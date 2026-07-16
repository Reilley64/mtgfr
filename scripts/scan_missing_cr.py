#!/usr/bin/env python3
"""Scan crates/engine for likely missing Comprehensive Rules citations.

Advisory only — heuristics flag places worth a human pass, not every hit is a bug.
Run via: just engine-cr-scan
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENGINE_SRC = ROOT / "crates" / "engine" / "src"
ENGINE_TESTS = ROOT / "crates" / "engine" / "tests"

# Module headers intentionally without chapter ownership (see docs/agent-navigation.md).
SKIP_MODULE_HEADERS = {
    "lib.rs",
    "de.rs",
    "label.rs",
    "characteristics_cache.rs",
    "types/mod.rs",
}

RULE_TERMS = re.compile(
    r"\b("
    r"state-based action|SBA|triggered ability|activated ability|"
    r"priority|combat damage|first strike|trample|protection from|"
    r"legendary rule|commander tax|flashback|escape|delve|proliferate|"
    r"goad|indestructible|hexproof|ward|flash\b|cycling|"
    r"enters the battlefield|dies trigger|mana ability"
    r")\b",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class Hit:
    path: str
    line: int
    snippet: str


CR_CITE = re.compile(r"CR \d")


def has_cr(text: str) -> bool:
    return bool(CR_CITE.search(text))


def snippet(line: str, max_len: int = 96) -> str:
    text = line.strip()
    for prefix in ("//!", "///", "//", "#"):
        if text.startswith(prefix):
            text = text[len(prefix) :].strip()
            break
    text = re.sub(r"\s+", " ", text)
    if len(text) > max_len:
        text = text[: max_len - 1] + "…"
    return text


def module_doc_lines(lines: list[str]) -> list[str]:
    doc: list[str] = []
    for line in lines:
        if line.startswith("//!"):
            doc.append(line[4:].strip())
        elif doc:
            break
    return doc


def scan_module_headers() -> list[Hit]:
    hits: list[Hit] = []
    for path in sorted(ENGINE_SRC.rglob("*.rs")):
        rel = path.relative_to(ENGINE_SRC).as_posix()
        if rel in SKIP_MODULE_HEADERS or path.name in SKIP_MODULE_HEADERS:
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        doc = module_doc_lines(lines)
        if not doc:
            continue
        if not has_cr("\n".join(doc)):
            rel = path.relative_to(ROOT).as_posix()
            hits.append(Hit(rel, 1, doc[0] if doc else "(empty module doc)"))
    return hits


def is_slash_comment_line(line: str) -> bool:
    stripped = line.lstrip()
    return stripped.startswith("//")


def starts_new_ponytail(line: str) -> bool:
    stripped = line.lstrip()
    for prefix in ("///", "//"):
        if stripped.startswith(prefix):
            body = stripped[len(prefix) :].lstrip()
            return body.lower().startswith("ponytail:")
    return False


def comment_body(line: str) -> str:
    stripped = line.lstrip()
    for prefix in ("///", "//!", "//"):
        if stripped.startswith(prefix):
            return stripped[len(prefix) :].lstrip()
    return stripped


def starts_new_doc_item(line: str) -> bool:
    """True when a `///` line begins a new parameter/doc item, not a ponytail continuation."""
    body = comment_body(line)
    if body.startswith(("`", "[", "Ward:", "Cascade", "Proliferate")):
        return True
    return bool(re.match(r"^[A-Z][A-Za-z0-9_]*:", body))


def ponytail_window(lines: list[str], index: int) -> tuple[int, int]:
    """0-based index on a `ponytail:` line — span of that approximation paragraph."""
    end = index
    while end + 1 < len(lines):
        nxt = lines[end + 1]
        if not is_slash_comment_line(nxt):
            break
        if end != index and starts_new_ponytail(nxt):
            break
        if end != index and starts_new_doc_item(nxt):
            break
        end += 1

    start = index
    while start > 0:
        prev = lines[start - 1]
        if not is_slash_comment_line(prev):
            break
        if starts_new_ponytail(prev) and start - 1 != index:
            break
        if prev.lstrip() in ("///", "//"):
            break
        start -= 1

    # Include one non-ponytail context line above a section-break `///` (e.g. "/// Ward: …").
    if start > 0 and lines[start - 1].lstrip() in ("///", "//"):
        ctx = start - 2
        if ctx >= 0 and is_slash_comment_line(lines[ctx]) and not starts_new_ponytail(lines[ctx]):
            start = ctx

    return start, end


def ponytail_paragraph_bounds(lines: list[str], index: int) -> tuple[int, int]:
    """0-based bounds of the `ponytail:` lines only (excludes context lines above)."""
    start, end = ponytail_window(lines, index)
    pony_start = index
    for j in range(start, end + 1):
        if starts_new_ponytail(lines[j]):
            pony_start = j
            break
    return pony_start, end


def adjacent_slash_comment_block(lines: list[str], index: int) -> tuple[int, int]:
    """0-based index on a plain `//` comment — merge consecutive `//` lines."""
    start = index
    while start > 0:
        prev = lines[start - 1].strip()
        if prev.startswith("//") and not prev.startswith(("///", "//!")):
            start -= 1
        else:
            break
    end = index
    while end + 1 < len(lines):
        nxt = lines[end + 1].strip()
        if nxt.startswith("//") and not nxt.startswith(("///", "//!")):
            end += 1
        else:
            break
    return start, end


def comment_window(lines: list[str], index: int) -> str:
    """0-based line index: nearby comment lines for CR lookup."""
    if "ponytail:" in lines[index].lower():
        start, end = ponytail_window(lines, index)
        return "\n".join(lines[start : end + 1])

    stripped = lines[index].strip()
    if stripped.startswith("//") and not stripped.startswith(("///", "//!")):
        start, end = adjacent_slash_comment_block(lines, index)
        return "\n".join(lines[start : end + 1])

    parts: list[str] = []
    for j in range(max(0, index - 2), min(len(lines), index + 2)):
        line = lines[j]
        if line.strip().startswith("//"):
            parts.append(line)
        elif "//" in line:
            parts.append(line.split("//", 1)[1])
    return "\n".join(parts)


def scan_ponytail_without_cr() -> list[Hit]:
    hits: list[Hit] = []
    for path in sorted(ENGINE_SRC.rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        for i, line in enumerate(lines):
            if "ponytail:" not in line.lower():
                continue
            window = comment_window(lines, i)
            pony_start, pony_end = ponytail_paragraph_bounds(lines, i)
            ponytail_text = "\n".join(lines[pony_start : pony_end + 1])
            if has_cr(ponytail_text):
                continue
            # Only flag approximations that discuss rules-shaped behavior.
            if not RULE_TERMS.search(window):
                continue
            hits.append(Hit(rel, i + 1, snippet(line)))
    return hits


def scan_inline_rule_comments() -> list[Hit]:
    hits: list[Hit] = []
    for base in (ENGINE_SRC, ENGINE_TESTS):
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            rel = path.relative_to(ROOT).as_posix()
            file_lines = path.read_text(encoding="utf-8").splitlines()
            for i, line in enumerate(file_lines, start=1):
                stripped = line.strip()
                comment_body = None
                if stripped.startswith("//") and not stripped.startswith(("///", "//!")):
                    comment_body = stripped
                elif "//" in stripped and not stripped.startswith(("///", "//!")):
                    comment_body = stripped.split("//", 1)[1].strip()
                if comment_body is None:
                    continue
                block_start, block_end = adjacent_slash_comment_block(file_lines, i - 1)
                block_text = "\n".join(
                    ln.strip()[2:].strip() for ln in file_lines[block_start : block_end + 1]
                )
                if has_cr(block_text) or "ponytail:" in block_text.lower() or "ADR " in block_text:
                    continue
                if RULE_TERMS.search(block_text) and len(block_text) > 24:
                    hits.append(Hit(rel, i, snippet(comment_body)))
    return hits


def scan_test_docs() -> list[Hit]:
    hits: list[Hit] = []
    for path in sorted(ENGINE_TESTS.rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        i = 0
        while i < len(lines):
            stripped = lines[i].strip()
            if not stripped.startswith("fn "):
                i += 1
                continue
            j = i - 1
            while j >= 0 and (lines[j].strip().startswith("#[") or lines[j].strip() == ""):
                j -= 1
            doc: list[str] = []
            while j >= 0 and lines[j].startswith("///"):
                doc.insert(0, lines[j][4:].strip())
                j -= 1
            if doc:
                text = " ".join(doc)
                if RULE_TERMS.search(text) and not has_cr(text) and len(text) > 40:
                    fn = stripped.split("(")[0].replace("fn ", "")
                    hits.append(Hit(rel, i + 1, f"{fn}: {doc[0]}"))
            i += 1
    return hits


def print_section(title: str, hits: list[Hit], limit: int) -> None:
    print(f"## {title} ({len(hits)})")
    print()
    if not hits:
        print("_None._")
        print()
        return
    for hit in hits[:limit]:
        print(f"- `{hit.path}:{hit.line}` — {hit.snippet}")
    if len(hits) > limit:
        print(f"- _… and {len(hits) - limit} more_")
    print()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--limit",
        type=int,
        default=40,
        help="max hits per section (default 40)",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run scanner sanity checks and exit",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        cases = [
            ("// priority pass", True),
            ("return; // ponytail: mana ability on stack (CR 605)", True),
            ("// ponytail: deduped Vec for Copy", False),
        ]
        for text, expect in cases:
            got = bool(RULE_TERMS.search(text))
            if got != expect:
                raise AssertionError(f"{text!r}: expected {expect}, got {got}")
        print("self-test ok")
        return 0

    modules = scan_module_headers()
    ponytail = scan_ponytail_without_cr()
    inline = scan_inline_rule_comments()
    tests = scan_test_docs()

    print("# Missing CR citation scan\n")
    print(
        "Heuristic pass over `crates/engine`. "
        "Regenerate the reverse index with `just engine-cr-index` after adding cites.\n"
    )
    print_section("Module `//!` headers with no CR cite", modules, args.limit)
    print_section("`ponytail:` approximations with no nearby CR cite", ponytail, args.limit)
    print_section("Inline `//` comments with rule terms but no CR", inline, args.limit)
    print_section("Test `///` docs with rule terms but no CR", tests, args.limit)

    total = len(modules) + len(ponytail) + len(inline) + len(tests)
    print(f"**Total flagged: {total}**")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
