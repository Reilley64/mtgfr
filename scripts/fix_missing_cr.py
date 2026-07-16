#!/usr/bin/env python3
"""Add CR citations to comments flagged by scan_missing_cr.py.

Appends cites at comment *end* only — never mid-clause. Idempotent.
Run via: just engine-cr-fix
"""

from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from scan_missing_cr import (  # noqa: E402
    adjacent_slash_comment_block,
    has_cr,
    ponytail_paragraph_bounds,
    ponytail_window,
    scan_inline_rule_comments,
    scan_module_headers,
    scan_ponytail_without_cr,
    scan_test_docs,
)

# Repo conventions (see cast.rs module header) plus common chapters.
CR_PATTERNS: list[tuple[re.Pattern[str], str]] = [
    (re.compile(r"\bfirst strike\b", re.I), "702.7"),
    (re.compile(r"\bdouble strike\b", re.I), "702.4"),
    (re.compile(r"\bprotection from\b", re.I), "702.16"),
    (re.compile(r"\bhexproof\b", re.I), "702.11"),
    (re.compile(r"\bindestructible\b", re.I), "702.12"),
    (re.compile(r"\btrample\b", re.I), "702"),
    (re.compile(r"\bward\b", re.I), "702.21"),
    (re.compile(r"\bflashback\b", re.I), "702.34"),
    (re.compile(r"\bescape\b", re.I), "702.19"),
    (re.compile(r"\bdelve\b", re.I), "702.66"),
    (re.compile(r"\bproliferate\b", re.I), "701.27"),
    (re.compile(r"\bgoad(?:ed|ing)?\b", re.I), "701.38"),
    (re.compile(r"\bcycling\b", re.I), "702.28"),
    (re.compile(r"\bflash\b", re.I), "702.8"),
    (re.compile(r"\bcommander tax\b", re.I), "903.8"),
    (re.compile(r"\blegendary rule\b", re.I), "704.5j"),
    (re.compile(r"\bstate-based action|\bSBA\b", re.I), "704"),
    (re.compile(r"\bmana ability\b", re.I), "605"),
    (re.compile(r"\btriggered ability\b", re.I), "603"),
    (re.compile(r"\bdies trigger\b", re.I), "603.6"),
    (re.compile(r"\bactivated ability\b", re.I), "602"),
    (re.compile(r"\bcombat damage\b", re.I), "510"),
    (re.compile(r"\benters the battlefield\b", re.I), "603.6a"),
    (re.compile(r"\bpriority\b", re.I), "117"),
    (re.compile(r"\bascend\b", re.I), "702.131"),
    (re.compile(r"\bmagecraft\b", re.I), "702.138"),
    (re.compile(r"\bsurveil\b", re.I), "701.43"),
    (re.compile(r"\bscry\b", re.I), "701.42"),
    (re.compile(r"\bfight(?:ing|s)?\b", re.I), "701.12"),
    (re.compile(r"\bpopulate\b", re.I), "701.32"),
    (re.compile(r"\bconvoke\b", re.I), "702.52"),
    (re.compile(r"\bkicker\b", re.I), "702.33"),
    (re.compile(r"\bretrace\b", re.I), "702.83"),
    (re.compile(r"\bmodal\b", re.I), "700.2"),
    (re.compile(r"\bcouncil'?s? dilemma\b", re.I), "703.5"),
    (re.compile(r"\bcease(?:s|d)? to exist\b", re.I), "111.7"),
    (re.compile(r"\btoken\b", re.I), "111"),
    (re.compile(r"\battach(?:ed|ing)?\b", re.I), "303.4"),
    (re.compile(r"\bAura\b"), "303.4"),
    (re.compile(r"\bequip(?:ped|ping)?\b", re.I), "301.5"),
    (re.compile(r"\bcontrol(?:ler|led|s)?\b", re.I), "108.3"),
    (re.compile(r"\bowner(?:ship)?\b", re.I), "108.4"),
    (re.compile(r"\btarget(?:ing|ed|s)?\b", re.I), "601.2c"),
    (re.compile(r"\bcop(?:y|ies|ied)\b", re.I), "707"),
    (re.compile(r"\bexile(?:d|s)?\b", re.I), "406.5"),
    (re.compile(r"\bgraveyard\b", re.I), "403.5"),
    (re.compile(r"\blibrary\b", re.I), "400.3"),
    (re.compile(r"\bshuffle\b", re.I), "401.2"),
    (re.compile(r"\bhand\b", re.I), "402.5"),
    (re.compile(r"\bmana pool\b", re.I), "106.4"),
    (re.compile(r"\bland drop\b", re.I), "305.9"),
    (re.compile(r"\bplay land\b", re.I), "305.1"),
    (re.compile(r"\bstack\b", re.I), "405"),
    (re.compile(r"\bcast(?:ing|s)?\b", re.I), "601"),
    (re.compile(r"\bspell\b", re.I), "601"),
    (re.compile(r"\bdamage\b", re.I), "120.3"),
    (re.compile(r"\blife\b", re.I), "118.7"),
    (re.compile(r"\bbattlefield\b", re.I), "403.5"),
    (re.compile(r"\bcombat\b", re.I), "506"),
    (re.compile(r"\bblock(?:er|ing|s)?\b", re.I), "509"),
    (re.compile(r"\battack(?:er|ing|s)?\b", re.I), "508"),
    (re.compile(r"\bcommander\b", re.I), "903"),
    (re.compile(r"\bloyalty\b", re.I), "606"),
    (re.compile(r"\bcounter(?:s|ed)?\b", re.I), "122"),
    (re.compile(r"\btrigger(?:s|ed)?\b", re.I), "603"),
    (re.compile(r"\babil(?:ity|ities)\b", re.I), "113"),
    (re.compile(r"\bturn\b", re.I), "500"),
    (re.compile(r"\bstep\b", re.I), "104.3"),
    (re.compile(r"\bphase\b", re.I), "104"),
    (re.compile(r"\bzone\b", re.I), "400"),
    (re.compile(r"\bconcede\b", re.I), "104.3a"),
    (re.compile(r"\buntap\b", re.I), "502.1"),
    (re.compile(r"\bdraw\b", re.I), "121"),
    (re.compile(r"\bupkeep\b", re.I), "503"),
    (re.compile(r"\bcleanup\b", re.I), "514"),
    (re.compile(r"\bcharacteristic\b", re.I), "613"),
    (re.compile(r"\blayer\b", re.I), "613"),
]

MODULE_CR: dict[str, str] = {
    "crates/engine/src/zones.rs": (
        "Primary: CR 400 (zones), CR 121 (drawing a card), CR 106.4 (mana pool)."
    ),
    "crates/engine/src/priority.rs": (
        "Primary: CR 117 (priority), CR 500 (turn structure), CR 514 (cleanup). "
        "Also: CR 605 (mana abilities / auto-tap planning)."
    ),
    "crates/engine/src/pipeline.rs": (
        "Primary: CR 704 (SBA fixpoint), CR 603 (trigger enqueue / APNAP placement), CR 608 "
        "(priority rounds emptying the stack)."
    ),
    "crates/engine/src/spawn.rs": (
        "Test/setup helpers. Also: CR 903.8 (commander tax)."
    ),
    "crates/engine/src/lib.rs": (
        "Primary: CR 117 (priority), CR 405 (stack), CR 903 (Commander)."
    ),
}

SUFFIX_RE = re.compile(r"\s*\((?:CR [^)]+(?:, )?)+\)\s*$")


def infer_crs(text: str) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for pat, rule in CR_PATTERNS:
        if pat.search(text) and rule not in seen:
            seen.add(rule)
            out.append(rule)
    return out[:3]


def format_cr_suffix(rules: list[str]) -> str:
    return " (" + ", ".join(f"CR {r}" for r in rules) + ")"


def strip_existing_suffix(text: str) -> str:
    return SUFFIX_RE.sub("", text.rstrip())


def prefix_bare_rule_numbers(text: str) -> str:
    """Prefix standalone `601.2c`-style ids that aren't already `CR …`."""

    def repl(m: re.Match[str]) -> str:
        start = m.start()
        if start >= 3 and text[start - 3 : start] == "CR ":
            return m.group(0)
        if start >= 1 and text[start - 1] in "!.,":
            return m.group(0)
        return f"CR {m.group(1)}"

    return re.sub(r"(?<![CR \d.])(\d{3}(?:\.\d+[a-z]?)?)", repl, text)


def split_code_comment(line: str) -> tuple[str, str | None]:
    """Return `(code_part, comment_part)` for `foo(); // bar` lines."""
    if "//" not in line:
        return line, None
    idx = line.index("//")
    before = line[:idx]
    if before.count('"') % 2 == 1:
        return line, None
    return before.rstrip(), line[idx + 2 :].lstrip()


def append_suffix_to_comment_body(body: str, suffix: str = "", rules: list[str] | None = None) -> str:
    body = strip_existing_suffix(body)
    if has_cr(body):
        return prefix_bare_rule_numbers(body)
    resolved = rules if rules is not None else infer_crs(body)
    if not resolved:
        return prefix_bare_rule_numbers(body)
    suffix = suffix or format_cr_suffix(resolved)
    return prefix_bare_rule_numbers(body + suffix)


def is_last_doc_line(lines: list[str], index: int) -> bool:
    if index + 1 >= len(lines):
        return True
    nxt = lines[index + 1].lstrip()
    return not (nxt.startswith("///") or nxt.startswith("//!"))


def comment_block_range(lines: list[str], index: int, prefix: str) -> tuple[int, int]:
    start = index
    while start > 0 and lines[start - 1].lstrip().startswith(prefix):
        start -= 1
    end = index
    while end + 1 < len(lines) and lines[end + 1].lstrip().startswith(prefix):
        end += 1
    return start, end


def is_last_comment_line(lines: list[str], index: int) -> bool:
    stripped = lines[index].lstrip()
    if stripped.startswith("///") or stripped.startswith("//!"):
        return is_last_doc_line(lines, index)
    if stripped.startswith("//"):
        if index + 1 >= len(lines):
            return True
        nxt = lines[index + 1].lstrip()
        return not (nxt.startswith("//") and not nxt.startswith(("///", "//!")))
    return True


def inject_cr_into_line(line: str, lines: list[str] | None = None, index: int | None = None) -> str:
    stripped = line.lstrip()
    for prefix in ("///", "//!", "//"):
        if stripped.startswith(prefix):
            if lines is not None and index is not None and not is_last_comment_line(lines, index):
                body = stripped[len(prefix) :].lstrip()
                cleaned = prefix_bare_rule_numbers(body)
                if cleaned == body:
                    return line
                return f"{line[: len(line) - len(stripped)]}{prefix} {cleaned}"

            indent = line[: len(line) - len(stripped)]
            body = stripped[len(prefix) :].lstrip()
            block_text = body
            if lines is not None and index is not None:
                start, end = comment_block_range(lines, index, prefix)
                block_text = "\n".join(
                    lines[j].lstrip()[len(prefix) :].lstrip() for j in range(start, end + 1)
                )
            rules = None if has_cr(block_text) else infer_crs(block_text)
            if rules:
                new_body = append_suffix_to_comment_body(body, rules=rules)
            else:
                new_body = prefix_bare_rule_numbers(body)
            if new_body == body:
                return line
            return f"{indent}{prefix} {new_body}"

    code, comment = split_code_comment(line)
    if comment is not None:
        if has_cr(comment):
            new_comment = prefix_bare_rule_numbers(comment)
        else:
            new_comment = append_suffix_to_comment_body(comment, rules=None)
        if new_comment == comment:
            return line
        sep = " " if code and not code.endswith((";", "{", "}", "(", "[")) else ""
        return f"{code}{sep}// {new_comment}"

    return line


def fix_ponytail_at_line(lines: list[str], lineno: int) -> bool:
    """Append CR suffix to the last line of the ponytail paragraph at `lineno` (1-based)."""
    idx = lineno - 1
    if idx < 0 or idx >= len(lines) or "ponytail:" not in lines[idx].lower():
        return False

    start, end = ponytail_window(lines, idx)
    pony_start, pony_end = ponytail_paragraph_bounds(lines, idx)
    block_text = "\n".join(
        lines[j].lstrip()[3:].lstrip() if lines[j].lstrip().startswith("///") else lines[j].split("//", 1)[1].strip()
        for j in range(start, end + 1)
    )
    ponytail_text = "\n".join(
        lines[j].lstrip()[3:].lstrip() if lines[j].lstrip().startswith("///") else lines[j].split("//", 1)[1].strip()
        for j in range(pony_start, pony_end + 1)
    )
    if has_cr(ponytail_text):
        return False

    rules = infer_crs(block_text)
    if not rules:
        return False

    last = lines[pony_end]
    stripped = last.lstrip()
    for prefix in ("///", "//"):
        if stripped.startswith(prefix):
            indent = last[: len(last) - len(stripped)]
            body = stripped[len(prefix) :].lstrip()
            lines[end] = f"{indent}{prefix} {append_suffix_to_comment_body(body, rules=rules)}"
            return True
    return False


def fix_comment_block(lines: list[str], lineno: int) -> bool:
    """Append one CR suffix to the last line of the `//`/`///`/`//!` block containing `lineno`."""
    idx = lineno - 1
    if idx < 0 or idx >= len(lines):
        return False
    stripped = lines[idx].lstrip()
    prefix = None
    for candidate in ("///", "//!", "//"):
        if stripped.startswith(candidate):
            prefix = candidate
            break
    if prefix is None:
        return False

    start, end = comment_block_range(lines, idx, prefix)
    block_text = "\n".join(
        lines[j].lstrip()[len(prefix) :].lstrip() for j in range(start, end + 1)
    )
    if has_cr(block_text):
        for j in range(start, end + 1):
            line = lines[j]
            s = line.lstrip()
            if s.startswith(prefix):
                body = s[len(prefix) :].lstrip()
                cleaned = prefix_bare_rule_numbers(body)
                if cleaned != body:
                    indent = line[: len(line) - len(s)]
                    lines[j] = f"{indent}{prefix} {cleaned}"
        return False

    rules = infer_crs(block_text)
    if not rules:
        return False

    for j in range(start, end):
        line = lines[j]
        s = line.lstrip()
        if s.startswith(prefix):
            body = s[len(prefix) :].lstrip()
            cleaned = prefix_bare_rule_numbers(body)
            if cleaned != body:
                indent = line[: len(line) - len(s)]
                lines[j] = f"{indent}{prefix} {cleaned}"

    last = lines[end]
    s = last.lstrip()
    body = s[len(prefix) :].lstrip()
    indent = last[: len(last) - len(s)]
    lines[end] = f"{indent}{prefix} {append_suffix_to_comment_body(body, rules=rules)}"
    return True


def inject_cr_into_test_doc(lines: list[str], fn_line: int) -> bool:
    """Append one suffix to the last `///` line before `fn_line` (1-based)."""
    j = fn_line - 2
    while j >= 0 and (lines[j].strip().startswith("#[") or lines[j].strip() == ""):
        j -= 1
    if j < 0 or not lines[j].startswith("///"):
        return False
    doc_end = j
    doc_start = j
    while doc_start >= 0 and lines[doc_start].startswith("///"):
        doc_start -= 1
    doc_start += 1
    doc_text = "\n".join(line[4:] for line in lines[doc_start : doc_end + 1])
    if has_cr(doc_text):
        return False
    rules = infer_crs(doc_text)
    if not rules:
        return False
    old = lines[doc_end]
    body = old[4:].rstrip()
    lines[doc_end] = f"/// {append_suffix_to_comment_body(body, rules=rules)}"
    return True


def fix_module_header(path: Path) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    extra = MODULE_CR.get(rel)
    if not extra:
        return False
    lines = path.read_text(encoding="utf-8").splitlines()
    doc = [line for line in lines if line.startswith("//!")]
    if has_cr("\n".join(doc)):
        return False
    for i, line in enumerate(lines):
        if line.startswith("//!"):
            lines.insert(i + 1, f"//! {extra}")
            path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            return True
    return False


def module_doc(lines: list[str]) -> list[str]:
    doc: list[str] = []
    for line in lines:
        if line.startswith("//!"):
            doc.append(line)
        elif doc:
            break
    return doc


def self_test() -> None:
    assert set(infer_crs("priority and combat damage")) == {"510", "117", "120.3"}
    assert infer_crs("flashback from graveyard") == ["702.34", "403.5"]
    assert infer_crs("escape cost") == ["702.19"]
    assert set(infer_crs("trample damage")) == {"702", "120.3"}

    line = "// Casting resets priority."
    fixed = inject_cr_into_line(line)
    assert fixed.endswith("(CR 117, CR 601)")
    assert inject_cr_into_line(fixed) == fixed

    line = 'return; // ponytail: scry on empty library.'
    fixed = inject_cr_into_line(line)
    assert "CR 701.42" in fixed
    assert "CR (CR" not in fixed

    line = "// width, CR 601.2c's maximum"
    fixed = inject_cr_into_line(line)
    assert "CR (CR" not in fixed
    assert "CR 601.2c" in fixed

    block = [
        "/// ponytail: ward tax at cast time, not",
        "/// countered on resolution unless you pay.",
    ]
    fixed = inject_cr_into_line(block[1], block, 1)
    assert "CR 601" in fixed and "CR 122" in fixed
    assert "unless you (CR" not in block[0]

    lines = [
        "/// Pass priority until done.",
        "/// Also handles goad.",
        "fn helper() {}",
    ]
    assert inject_cr_into_test_doc(lines, 3)
    assert "CR 701.38" in lines[1] and "CR 117" in lines[1]


def main() -> int:
    if "--self-test" in sys.argv:
        self_test()
        print("self-test ok")
        return 0

    hits_by_file: dict[str, set[int]] = defaultdict(set)
    test_fn_lines: dict[str, set[int]] = defaultdict(set)

    for hit in scan_ponytail_without_cr():
        hits_by_file[hit.path].add(hit.line)
    for hit in scan_inline_rule_comments():
        hits_by_file[hit.path].add(hit.line)
    for hit in scan_test_docs():
        test_fn_lines[hit.path].add(hit.line)

    changed_files = 0
    for rel in sorted(set(hits_by_file) | set(test_fn_lines)):
        path = ROOT / rel
        lines = path.read_text(encoding="utf-8").splitlines()
        original = list(lines)

        for lineno in sorted(hits_by_file.get(rel, []), reverse=True):
            idx = lineno - 1
            is_ponytail = (
                0 <= idx < len(lines) and "ponytail:" in lines[idx].lower()
            )
            if is_ponytail:
                if not fix_ponytail_at_line(lines, lineno):
                    idx = lineno - 1
                    if 0 <= idx < len(lines):
                        lines[idx] = inject_cr_into_line(lines[idx], lines, idx)
            elif not fix_comment_block(lines, lineno):
                idx = lineno - 1
                if 0 <= idx < len(lines):
                    lines[idx] = inject_cr_into_line(lines[idx], lines, idx)

        for lineno in sorted(test_fn_lines.get(rel, []), reverse=True):
            inject_cr_into_test_doc(lines, lineno)

        if lines != original:
            path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            changed_files += 1

    for rel in MODULE_CR:
        if fix_module_header(ROOT / rel):
            changed_files += 1

    print(f"updated {changed_files} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
