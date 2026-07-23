#!/usr/bin/env python3
"""Rewrite flat Effect `type` strings to nested family + mode in card TOMLs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

EFFECT_TABLE_SUFFIXES = ("effects", "steps", "options", "then")
INLINE_NESTED_KEYS = ("then", "modes", "options", "steps")
STRUCTURAL_TYPES = frozenset({"sequence", "conditional", "choose_one"})
TABLE_HEADER_RE = re.compile(r"^\[\[(.+)\]\]\s*$")
TYPE_LINE_RE = re.compile(r'^(\s*)type\s*=\s*"([^"]+)"\s*$')
INLINE_NESTED_START_RE = re.compile(
    rf'^\s*(?:{"|".join(INLINE_NESTED_KEYS)})\s*=\s*\['
)
INLINE_TYPE_RE = re.compile(r'type\s*=\s*"([^"]+)"')
MODE_LINE_RE = re.compile(r'^(\s*)mode\s*=\s*"([^"]+)"\s*$')


def load_map(path: Path) -> dict[str, dict[str, str]]:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def _table_suffix(header: str) -> str | None:
    segment = header.rsplit(".", 1)[-1]
    if segment in EFFECT_TABLE_SUFFIXES:
        return segment
    return None


def _mapping_entry(
    old_type: str, mapping: dict[str, dict[str, str]]
) -> dict[str, str] | None:
    if old_type in STRUCTURAL_TYPES:
        return None
    return mapping.get(old_type)


def _table_type_already_migrated(lines: list[str], index: int, indent: str) -> bool:
    if index + 1 >= len(lines):
        return False
    next_line = lines[index + 1].rstrip("\n")
    return MODE_LINE_RE.match(next_line) is not None and next_line.startswith(indent)


def _rewrite_inline_types(line: str, mapping: dict[str, dict[str, str]]) -> str:
    def replace(match: re.Match[str]) -> str:
        old_type = match.group(1)
        entry = _mapping_entry(old_type, mapping)
        if entry is None:
            return match.group(0)

        tail = line[match.end() :]
        if re.match(r'\s*,\s*mode\s*=\s*"', tail):
            return match.group(0)

        return f'type = "{entry["family"]}", mode = "{entry["mode"]}"'

    return INLINE_TYPE_RE.sub(replace, line)


def _inline_bracket_delta(line: str) -> int:
    return line.count("[") - line.count("]")


def _iter_unmigrated_inline_types(
    line: str, mapping: dict[str, dict[str, str]]
) -> list[str]:
    hits: list[str] = []
    for match in INLINE_TYPE_RE.finditer(line):
        old_type = match.group(1)
        entry = _mapping_entry(old_type, mapping)
        if entry is None:
            continue

        tail = line[match.end() :]
        if re.match(r'\s*,\s*mode\s*=\s*"', tail):
            continue

        hits.append(old_type)
    return hits


def migrate_text(text: str, mapping: dict[str, dict[str, str]]) -> str:
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    in_effect_table = False
    inline_nested_depth = 0

    index = 0
    while index < len(lines):
        line = lines[index]
        header_match = TABLE_HEADER_RE.match(line.rstrip("\n"))
        if header_match is not None:
            in_effect_table = _table_suffix(header_match.group(1)) is not None
            inline_nested_depth = 0
            out.append(line)
            index += 1
            continue

        if not in_effect_table:
            out.append(line)
            index += 1
            continue

        if inline_nested_depth == 0 and INLINE_NESTED_START_RE.match(line.rstrip("\n")):
            rewritten = _rewrite_inline_types(line, mapping)
            inline_nested_depth = max(0, _inline_bracket_delta(rewritten))
            out.append(rewritten)
            index += 1
            continue

        if inline_nested_depth > 0:
            rewritten = _rewrite_inline_types(line, mapping)
            inline_nested_depth = max(0, inline_nested_depth + _inline_bracket_delta(rewritten))
            out.append(rewritten)
            index += 1
            continue

        type_match = TYPE_LINE_RE.match(line.rstrip("\n"))
        if type_match is None:
            out.append(line)
            index += 1
            continue

        indent, old_type = type_match.group(1), type_match.group(2)
        entry = _mapping_entry(old_type, mapping)
        if entry is None:
            out.append(line)
            index += 1
            continue

        if _table_type_already_migrated(lines, index, indent):
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
    inline_nested_depth = 0

    for line_number, line in enumerate(text.splitlines(), start=1):
        header_match = TABLE_HEADER_RE.match(line)
        if header_match is not None:
            in_effect_table = _table_suffix(header_match.group(1)) is not None
            inline_nested_depth = 0
            continue

        if not in_effect_table:
            continue

        if inline_nested_depth == 0 and INLINE_NESTED_START_RE.match(line):
            for old_type in _iter_unmigrated_inline_types(line, mapping):
                hits.append((line_number, old_type))
            inline_nested_depth = max(0, _inline_bracket_delta(line))
            continue

        if inline_nested_depth > 0:
            for old_type in _iter_unmigrated_inline_types(line, mapping):
                hits.append((line_number, old_type))
            inline_nested_depth = max(0, inline_nested_depth + _inline_bracket_delta(line))
            continue

        type_match = TYPE_LINE_RE.match(line)
        if type_match is None:
            continue

        indent, old_type = type_match.group(1), type_match.group(2)
        if _mapping_entry(old_type, mapping) is None:
            continue

        line_index = line_number - 1
        remaining = text.splitlines()
        if _table_type_already_migrated(remaining, line_index, indent):
            continue

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
