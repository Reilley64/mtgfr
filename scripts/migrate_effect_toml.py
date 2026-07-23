#!/usr/bin/env python3
"""Rewrite flat Effect `type` strings to nested family + mode in card TOMLs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

EFFECT_TABLE_SUFFIXES = ("effects", "steps", "options", "then")
STRUCTURAL_TYPES = frozenset({"sequence", "conditional", "choose_one"})
TABLE_HEADER_RE = re.compile(r"^\[\[(.+)\]\]\s*$")
TYPE_LINE_RE = re.compile(r'^(\s*)type\s*=\s*"([^"]+)"\s*$')


def load_map(path: Path) -> dict[str, dict[str, str]]:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def _table_suffix(header: str) -> str | None:
    segment = header.rsplit(".", 1)[-1]
    if segment in EFFECT_TABLE_SUFFIXES:
        return segment
    return None


def migrate_text(text: str, mapping: dict[str, dict[str, str]]) -> str:
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    in_effect_table = False

    index = 0
    while index < len(lines):
        line = lines[index]
        header_match = TABLE_HEADER_RE.match(line.rstrip("\n"))
        if header_match is not None:
            in_effect_table = _table_suffix(header_match.group(1)) is not None
            out.append(line)
            index += 1
            continue

        if not in_effect_table:
            out.append(line)
            index += 1
            continue

        type_match = TYPE_LINE_RE.match(line.rstrip("\n"))
        if type_match is None:
            out.append(line)
            index += 1
            continue

        indent, old_type = type_match.group(1), type_match.group(2)
        entry = mapping.get(old_type)
        if entry is None or old_type in STRUCTURAL_TYPES:
            out.append(line)
            index += 1
            continue

        family = entry["family"]
        mode = entry["mode"]
        out.append(f'{indent}type = "{family}"\n')
        out.append(f'{indent}mode = "{mode}"\n')
        index += 1

    return "".join(out)


def find_unmigrated_types(
    text: str, mapping: dict[str, dict[str, str]]
) -> list[tuple[int, str]]:
    hits: list[tuple[int, str]] = []
    in_effect_table = False

    for line_number, line in enumerate(text.splitlines(), start=1):
        header_match = TABLE_HEADER_RE.match(line)
        if header_match is not None:
            in_effect_table = _table_suffix(header_match.group(1)) is not None
            continue

        if not in_effect_table:
            continue

        type_match = TYPE_LINE_RE.match(line)
        if type_match is None:
            continue

        old_type = type_match.group(2)
        if old_type in mapping and old_type not in STRUCTURAL_TYPES:
            hits.append((line_number, old_type))

    return hits


def migrate_file(path: Path, mapping: dict[str, dict[str, str]]) -> bool:
    original = path.read_text(encoding="utf-8")
    migrated = migrate_text(original, mapping)
    if migrated == original:
        return False
    path.write_text(migrated, encoding="utf-8")
    return True


def check_paths(
    paths: list[Path], mapping: dict[str, dict[str, str]]
) -> list[tuple[Path, list[tuple[int, str]]]]:
    failures: list[tuple[Path, list[tuple[int, str]]]] = []
    for path in paths:
        hits = find_unmigrated_types(path.read_text(encoding="utf-8"), mapping)
        if hits:
            failures.append((path, hits))
    return failures


def default_card_pool_glob(root: Path) -> list[Path]:
    return sorted((root / "crates" / "cards" / "data").glob("**/*.toml"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Migrate flat Effect type strings to family + mode."
    )
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        help="TOML files to rewrite in place",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Exit 1 if any card pool TOML still uses a mapped flat type",
    )
    parser.add_argument(
        "--map",
        type=Path,
        default=Path(__file__).resolve().parent / "effect_type_map.json",
        help="Path to effect type mapping JSON",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Repository root for --check card pool scan",
    )
    args = parser.parse_args(argv)

    mapping = load_map(args.map)

    if args.check:
        if args.paths:
            targets = args.paths
        else:
            targets = default_card_pool_glob(args.repo_root)
        failures = check_paths(targets, mapping)
        if failures:
            for path, hits in failures:
                for line_number, old_type in hits:
                    print(f"{path}:{line_number}: unmigrated type {old_type!r}", file=sys.stderr)
            return 1
        return 0

    if not args.paths:
        parser.error("paths required unless --check scans the default card pool")

    changed = 0
    for path in args.paths:
        if migrate_file(path, mapping):
            changed += 1
            print(f"migrated {path}")
    print(f"done ({changed} file(s) changed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
