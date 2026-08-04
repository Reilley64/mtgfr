#!/usr/bin/env python3
"""Cross-check a fidelity report against the card pool and the increment backlog.

Engine work has repeatedly landed without its card, its test, or its tick, and nothing goes red
when it does: the suite only knows about tests that exist. Three grind waves each found a stale
entry a wave or more after the work was actually finished — one increment read as open for two
full waves with all six of its cards already scripted.

This is the standing Phase-5 check. It answers three questions the suite cannot:

  1. Which cards are still unticked in the report but *already scripted* in the pool? Each is
     either a missed tick or a partial script blocked by an open increment — the audit says
     which, by naming the increments that mention the card and whether they are landed.
  2. Which increments are marked LANDED but still have unticked cards?
  3. Which cards are ticked in the report but have no TOML at all?

Landed-ness is read from the `###` header only. A `*Landed:*` paragraph in an increment's body
is prose and is deliberately not consulted — pick one marker and this is it.

    python tooling/fidelity_report_audit.py leg
    python tooling/fidelity_report_audit.py leg --quiet   # exit 1 on findings, print nothing
"""

from __future__ import annotations

import argparse
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
DATA = REPO / "crates" / "cards" / "data"
FIDELITY = REPO / "docs" / "fidelity"

CARD_RE = re.compile(r"^- \[([ x])\] \*\*(.+?)\*\*", re.M)
HEADER_RE = re.compile(r"^### (\d+)\. (.+)$", re.M)
NAME_RE = re.compile(r'^name = "(.*)"', re.M)


def slugify(name: str) -> str:
    """"Gabriel Angelfire" -> "gabriel-angelfire", the form increment titles use."""
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


def pool_names() -> dict[str, str]:
    """Card name -> TOML basename, for every face of every script in the pool.

    Every `name =` in the file, not just the first: a double-faced card carries its back face's
    name further down (Dirgur Focusmage // Braingeyser), and the report ticks the face it means.
    """
    found = {}
    for path in sorted(DATA.glob("*.toml")):
        for name in NAME_RE.findall(path.read_text()):
            found.setdefault(name, path.name)
    return found


def increments(slug: str) -> list[tuple[int, bool, str]]:
    """(number, landed, body) per increment, landed read from the header alone."""
    path = FIDELITY / f"{slug}-increments.md"
    if not path.exists():
        return []
    parts = HEADER_RE.split(path.read_text())
    # split with two groups yields [pre, num, rest_of_header, body, num, ...]
    return [
        (int(parts[i]), "LANDED" in parts[i + 1], parts[i + 1] + parts[i + 2])
        for i in range(1, len(parts), 3)
    ]


def audit(slug: str) -> list[str]:
    report = (FIDELITY / f"{slug}.md").read_text()
    cards = [(name, tick == "x") for tick, name in CARD_RE.findall(report)]
    pool = pool_names()
    incs = increments(slug)

    findings = []

    # A card is fairly unticked while *any* increment that mentions it is still open — most cards
    # here are named by several, and one open increment is enough to hold the tick back. Both
    # checks below share this, or the second one re-reports every card the first one excused.
    # A single-card increment is often named for its card and never spells it out in prose
    # (`### 39. gabriel-angelfire`), so the slug counts as a mention too.
    blocked = {
        name
        for name, _ in cards
        if any(name in body or slugify(name) in body for _, landed, body in incs if not landed)
    }

    for name, ticked in cards:
        if ticked or name not in pool or name in blocked:
            continue
        owners = [f"#{n}" for n, _, body in incs if name in body]
        where = " ".join(owners) or "no increment mentions it"
        findings.append(f"scripted but unticked, nothing blocking it: {name} ({pool[name]}, {where})")

    for number, landed, body in incs:
        if not landed:
            continue
        stale = [name for name, ticked in cards if not ticked and name not in blocked and name in body]
        if stale:
            findings.append(f"increment #{number} is LANDED but these are unticked: {', '.join(stale)}")

    for name, ticked in cards:
        if ticked and name not in pool:
            findings.append(f"ticked with no TOML in the pool: {name}")

    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("slug", help="fidelity report slug, e.g. leg or 2ed")
    parser.add_argument("--quiet", action="store_true", help="exit 1 on findings, print nothing")
    args = parser.parse_args()

    findings = audit(args.slug)
    if args.quiet:
        return 1 if findings else 0

    if not findings:
        print(f"{args.slug}: report, pool and backlog agree")
        return 0

    print(f"{args.slug}: {len(findings)} finding(s)")
    for line in findings:
        print(f"  {line}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
