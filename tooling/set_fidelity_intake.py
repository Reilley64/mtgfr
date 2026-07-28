#!/usr/bin/env python3
"""Classify a whole Magic set against crates/cards/data for fidelity intake.

Deck grinds take an Archidekt link or a frozen decklist; a set grind takes a set code and
Scryfall as the source of truth. Caches the set fetch under the given --cache path so the
wave loop can re-baseline without re-hitting the API.

    python tooling/set_fidelity_intake.py 2ed
    python tooling/set_fidelity_intake.py 2ed --json out.json
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
import time
import urllib.request

SEARCH_URL = "https://api.scryfall.com/cards/search"
USER_AGENT = "mtgfr-fidelity-intake/1.0"
NAME_RE = re.compile(r'^name = "(.*)"', re.M)


def fetch_set(code: str) -> list[dict]:
    cards: list[dict] = []
    url = f"{SEARCH_URL}?q=set%3A{code}+unique%3Acards&order=name"
    while url:
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=30) as resp:
            page = json.load(resp)
        cards.extend(page["data"])
        url = page.get("next_page")
        if url:
            time.sleep(0.1)  # Scryfall asks for 50-100ms between requests
    return cards


def pool_names(data_dir: pathlib.Path) -> set[str]:
    names: set[str] = set()
    for path in data_dir.glob("*.toml"):
        names.update(NAME_RE.findall(path.read_text()))
    return names


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("set_code")
    parser.add_argument("--data-dir", default="crates/cards/data", type=pathlib.Path)
    parser.add_argument("--cache", default=None, type=pathlib.Path, help="Scryfall JSON cache (default .cache/<set>.json)")
    parser.add_argument("--refresh", action="store_true", help="ignore the cache and refetch")
    parser.add_argument("--json", dest="json_out", type=pathlib.Path, help="write the classification as JSON")
    args = parser.parse_args()

    cache = args.cache or pathlib.Path(f".cache/{args.set_code}.json")
    if cache.exists() and not args.refresh:
        cards = json.loads(cache.read_text())
    else:
        cards = fetch_set(args.set_code)
        cache.parent.mkdir(parents=True, exist_ok=True)
        cache.write_text(json.dumps(cards, indent=1))

    if not args.data_dir.is_dir():
        print(f"no such data dir: {args.data_dir}", file=sys.stderr)
        return 2

    pool = pool_names(args.data_dir)
    present = sorted(c["name"] for c in cards if c["name"] in pool)
    missing = sorted(c["name"] for c in cards if c["name"] not in pool)

    print(f"{args.set_code}: {len(cards)} unique cards — {len(present)} in pool, {len(missing)} missing")
    for name in missing:
        print(f"  MISSING  {name}")

    if args.json_out:
        args.json_out.write_text(json.dumps({"present": present, "missing": missing}, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
