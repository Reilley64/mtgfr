#!/usr/bin/env python3
"""Rewrite flat Effect `type` strings to nested family + mode in card TOMLs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

EFFECT_TABLE_SUFFIXES = ("effects", "steps", "options", "then", "modes", "rider")
INLINE_NESTED_KEYS = ("effects", "modes", "on_expiry", "options", "steps", "then")
STRUCTURAL_TYPES = frozenset({"sequence", "conditional", "choose_one"})
TABLE_HEADER_RE = re.compile(r"^\[\[(.+)\]\]\s*$")
SINGLE_TABLE_HEADER_RE = re.compile(r"^\[(.+)\]\s*$")
TYPE_LINE_RE = re.compile(r'^(\s*)type\s*=\s*"([^"]+)"\s*$')
MODES_LINE_RE = re.compile(r"^(\s*)modes(\s*=\s*\[.*)$")
INLINE_NESTED_START_RE = re.compile(
    rf'^\s*(?:{"|".join(INLINE_NESTED_KEYS)})\s*=\s*\['
)
INLINE_TYPE_RE = re.compile(r'type\s*=\s*"([^"]+)"')
INLINE_FAMILY_MODE_RE = re.compile(r'type\s*=\s*"([^"]+)"\s*,\s*mode\s*=\s*"([^"]+)"')
MODE_LINE_RE = re.compile(r'^(\s*)mode\s*=\s*"([^"]+)"\s*$')

NESTED_MODE_REWRITES = {
    ("destroy", "destroy_all"): {"family": "destroy", "mode": "all"},
    ("destroy", "destroy_target"): {"family": "destroy", "mode": "target"},
    (
        "destroy",
        "destroy_triggering_damaged_creature",
    ): {"family": "destroy", "mode": "triggering_damaged_creature"},
    ("destroy", "exile_all"): {"family": "exile", "mode": "all"},
    ("destroy", "exile_all_graveyards"): {"family": "exile", "mode": "all_graveyards"},
    ("destroy", "exile_graveyard"): {"family": "exile", "mode": "graveyard"},
    ("destroy", "exile_object"): {"family": "exile", "mode": "object"},
    ("destroy", "exile_target"): {"family": "exile", "mode": "target"},
    (
        "destroy",
        "exile_target_minting_illusion_on_leave",
    ): {"family": "exile", "mode": "target_minting_illusion_on_leave"},
    ("destroy", "exile_until_source_leaves"): {
        "family": "exile",
        "mode": "until_source_leaves",
    },
    ("destroy", "sacrifice_enchanted_creature"): {
        "family": "sacrifice",
        "mode": "enchanted_creature",
    },
    ("destroy", "sacrifice_object"): {"family": "sacrifice", "mode": "object"},
    ("destroy", "sacrifice_source"): {"family": "sacrifice", "mode": "source"},
}


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


def _nested_mapping_entry(
    family: str, mode: str
) -> dict[str, str] | None:
    return NESTED_MODE_REWRITES.get((family, mode))


def _rewrite_inline_family_modes(line: str) -> str:
    def replace(match: re.Match[str]) -> str:
        entry = _nested_mapping_entry(match.group(1), match.group(2))
        if entry is None:
            return match.group(0)
        return f'type = "{entry["family"]}", mode = "{entry["mode"]}"'

    return INLINE_FAMILY_MODE_RE.sub(replace, line)


def _rewrite_inline_types(line: str, mapping: dict[str, dict[str, str]]) -> str:
    line = _rewrite_inline_family_modes(line)

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

    for match in INLINE_FAMILY_MODE_RE.finditer(line):
        family = match.group(1)
        mode = match.group(2)
        if _nested_mapping_entry(family, mode) is not None:
            hits.append(f"{family}/{mode}")

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


def _rewrite_choose_one_modes_key(line: str) -> str:
    match = MODES_LINE_RE.match(line.rstrip("\n"))
    if match is None:
        return line

    newline = "\n" if line.endswith("\n") else ""
    return f'{match.group(1)}options{match.group(2)}{newline}'


def migrate_text(text: str, mapping: dict[str, dict[str, str]]) -> str:
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    in_effect_table = False
    inline_nested_depth = 0

    index = 0
    while index < len(lines):
        line = lines[index]
        rewritten_header = line.replace(".modes]]", ".options]]")
        header_match = TABLE_HEADER_RE.match(rewritten_header.rstrip("\n"))
        if header_match is None:
            header_match = SINGLE_TABLE_HEADER_RE.match(rewritten_header.rstrip("\n"))
        if header_match is not None:
            in_effect_table = _table_suffix(header_match.group(1)) is not None
            inline_nested_depth = 0
            out.append(rewritten_header)
            index += 1
            continue

        if not in_effect_table:
            out.append(line)
            index += 1
            continue

        line = _rewrite_choose_one_modes_key(line)

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

        if TYPE_LINE_RE.match(line.rstrip("\n")) is None:
            rewritten = _rewrite_inline_types(line, mapping)
            if rewritten != line:
                out.append(rewritten)
                index += 1
                continue

        type_match = TYPE_LINE_RE.match(line.rstrip("\n"))
        if type_match is None:
            out.append(line)
            index += 1
            continue

        indent, old_type = type_match.group(1), type_match.group(2)
        if index + 1 < len(lines):
            mode_match = MODE_LINE_RE.match(lines[index + 1].rstrip("\n"))
            if mode_match is not None and lines[index + 1].startswith(indent):
                nested_entry = _nested_mapping_entry(old_type, mode_match.group(2))
                if nested_entry is not None:
                    out.append(f'{indent}type = "{nested_entry["family"]}"\n')
                    out.append(f'{indent}mode = "{nested_entry["mode"]}"\n')
                    index += 2
                    continue

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
        normalized_header = line.replace(".modes]]", ".options]]")
        header_match = TABLE_HEADER_RE.match(normalized_header)
        if header_match is None:
            header_match = SINGLE_TABLE_HEADER_RE.match(normalized_header)
        if header_match is not None:
            in_effect_table = _table_suffix(header_match.group(1)) is not None
            inline_nested_depth = 0
            continue

        if not in_effect_table:
            continue

        if MODES_LINE_RE.match(line):
            hits.append((line_number, "modes"))
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

        if TYPE_LINE_RE.match(line) is None:
            inline_hits = _iter_unmigrated_inline_types(line, mapping)
            if inline_hits:
                for old_type in inline_hits:
                    hits.append((line_number, old_type))
                continue

        type_match = TYPE_LINE_RE.match(line)
        if type_match is None:
            continue

        indent, old_type = type_match.group(1), type_match.group(2)
        line_index = line_number - 1
        remaining = text.splitlines()
        if line_index + 1 < len(remaining):
            mode_match = MODE_LINE_RE.match(remaining[line_index + 1])
            if mode_match is not None and remaining[line_index + 1].startswith(indent):
                if _nested_mapping_entry(old_type, mode_match.group(2)) is not None:
                    hits.append((line_number, f"{old_type}/{mode_match.group(2)}"))
                    continue

        if _mapping_entry(old_type, mapping) is None:
            continue

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


def expand_paths(paths: list[Path]) -> list[Path]:
    expanded: list[Path] = []
    seen: set[Path] = set()

    for path in paths:
        if path.is_dir():
            for child in sorted(path.glob("**/*.toml")):
                resolved = child.resolve()
                if resolved in seen:
                    continue
                seen.add(resolved)
                expanded.append(child)
            continue

        resolved = path.resolve()
        if resolved in seen:
            continue
        seen.add(resolved)
        expanded.append(path)

    return expanded


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
            targets = expand_paths(args.paths)
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
    for path in expand_paths(args.paths):
        if migrate_file(path, mapping):
            changed += 1
            print(f"migrated {path}")
    print(f"done ({changed} file(s) changed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
