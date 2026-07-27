#!/usr/bin/env python3
"""Classify a frozen SoC decklist against crates/cards/data for fidelity intake."""
from __future__ import annotations

import argparse
import pathlib
import re
import sys

BASICS = {
    "plains",
    "island",
    "swamp",
    "mountain",
    "forest",
    "wastes",
}

STOP_RE = re.compile(
    r"^##\s+(Unusual|Implementation|Engine|Hard|Notes|Gaps|Observ)",
    re.M | re.I,
)


def parse_decklist(text: str) -> set[str]:
    body = STOP_RE.split(text, maxsplit=1)[0]
    found: set[str] = set()
    for line in body.splitlines():
        line = line.strip()
        m = re.match(r"^(?:[-*]\s+)?(\d+)\s+x\s+(.+)$", line, re.I)
        if m:
            name = re.sub(r"\s*\(.*\)$", "", m.group(2)).strip()
            name = re.sub(r"\s*\*.*$", "", name).strip()
            if name:
                found.add(name)
            continue
        m = re.match(r"^\|\s*(\d+)\s*\|\s*([^|]+)\|", line)
        if not m:
            continue
        name = m.group(2).strip()
        if name.lower() in {"card", "count"} or set(name) <= set("-: "):
            continue
        name = re.sub(r"\s*\*.*$", "", name).strip()
        found.add(name)
    return {n for n in found if n.lower() not in BASICS}


def load_pool(data_dir: pathlib.Path) -> dict[str, dict]:
    pool: dict[str, dict] = {}
    for path in data_dir.glob("*.toml"):
        text = path.read_text(encoding="utf-8", errors="ignore")
        m = re.search(r'^name\s*=\s*"(.*)"', text, re.M)
        if not m:
            continue
        name = m.group(1)
        approx = re.search(r'^approximates\s*=\s*"(.*)"', text, re.M)
        pool[name.lower()] = {
            "name": name,
            "file": path.name,
            "approximates": approx.group(1) if approx else None,
            "ponytails": len(re.findall(r"ponytail:", text)),
        }
    return pool


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("decklist", type=pathlib.Path)
    ap.add_argument(
        "--data",
        type=pathlib.Path,
        default=pathlib.Path("crates/cards/data"),
    )
    args = ap.parse_args()
    cards = parse_decklist(args.decklist.read_text(encoding="utf-8"))
    pool = load_pool(args.data)
    a, b, missing = [], [], []
    for name in sorted(cards, key=str.lower):
        info = pool.get(name.lower())
        if info is None:
            missing.append(name)
        elif info["approximates"]:
            b.append((name, info["approximates"], info["file"]))
        else:
            a.append((name, info["file"], info["ponytails"]))
    print(f"# Intake — {args.decklist.name}")
    print(f"nonbasics={len(cards)} A={len(a)} B={len(b)} missing={len(missing)}")
    print("\n## A. In pool, no approximates")
    for name, file, pony in a:
        suffix = f"  ; ponytail×{pony}" if pony else ""
        print(f"- [ ] {name} (`{file}`){suffix}")
    print("\n## B. In pool, approximated")
    for name, note, file in b:
        print(f"- [ ] {name} (`{file}`): {note}")
    print("\n## Missing from pool")
    for name in missing:
        print(f"- [ ] {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
