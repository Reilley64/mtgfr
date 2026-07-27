#!/usr/bin/env python3
"""Emit the frame block of a card TOML — the part Scryfall owns, not the rules.

Every field here is a frame-audit field (name, cost, type, P/T, legendary, verbatim oracle,
printing metadata), so scaffolding them from Scryfall makes the audit pass by construction. The
abilities are yours to write. Prints and default_print come from `/cards/named?exact=`, cached
under .cache/frames/.

    python tooling/scaffold_card_frame.py "Air Elemental" "Craw Wurm"
    python tooling/scaffold_card_frame.py --write "Air Elemental"    # create data/<slug>.toml
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
import time
import urllib.parse
import urllib.request

NAMED_URL = "https://api.scryfall.com/cards/named"
USER_AGENT = "mtgfr-fidelity-intake/1.0"
CACHE = pathlib.Path(".cache/frames")
DATA = pathlib.Path("crates/cards/data")

COLORS = {"W": "white", "U": "blue", "B": "black", "R": "red", "G": "green"}
BASIC_LAND_MANA = {"Plains": "white", "Island": "blue", "Swamp": "black", "Mountain": "red", "Forest": "green"}
# Scryfall `keywords` that map 1:1 onto a bare `Keyword` variant. Parameterized ones (ward,
# protection) and ones the engine can't express yet (banding, landwalk) are deliberately absent —
# they need a hand-written ability or an increment, so silently emitting them would be a lie.
BARE_KEYWORDS = {
    "flying", "first strike", "vigilance", "haste", "trample", "deathtouch", "reach", "menace",
    "double strike", "lifelink", "defender", "indestructible", "flash", "hexproof", "shroud",
    "prowess", "skulk", "shadow", "fear", "intimidate",
}


def get(url: str) -> dict:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
    for attempt in range(5):
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.load(resp)
        except urllib.error.HTTPError as e:
            if e.code != 429 or attempt == 4:
                raise
            time.sleep(2 ** attempt)
    raise SystemExit("unreachable")


def set_index() -> dict:
    """Card objects already pulled by set_fidelity_intake.py — one less request per card."""
    if not hasattr(set_index, "_by_name"):
        by_name = {}
        for path in sorted(pathlib.Path(".cache").glob("*.json")):
            payload = json.loads(path.read_text())
            for card in payload["data"] if isinstance(payload, dict) else payload:
                by_name.setdefault(card["name"], card)
        set_index._by_name = by_name
    return set_index._by_name


def fetch(name: str) -> dict:
    CACHE.mkdir(parents=True, exist_ok=True)
    path = CACHE / f"{slug(name)}.json"
    if path.exists():
        return json.loads(path.read_text())
    card = set_index().get(name)
    if card is None:
        card = get(f"{NAMED_URL}?exact={urllib.parse.quote(name)}")
        time.sleep(0.3)
    sets, url = set(), card["prints_search_uri"]
    while url:  # basics run to thousands of printings, well past one page
        page = get(url)
        time.sleep(0.3)
        sets.update(p["set"] for p in page["data"])
        url = page.get("next_page") if page.get("has_more") else None
    card["_sets"] = sorted(sets)
    path.write_text(json.dumps(card, indent=1))
    return card


def slug(name: str) -> str:
    """Pool filename convention: apostrophes vanish, every other separator becomes `_`
    (`Ajani's Chosen` → `ajanis_chosen`, `Man-o'-War` → `man_o_war`)."""
    return re.sub(r"[^a-z0-9]+", "_", name.lower().replace("'", "").replace("’", "")).strip("_")


def toml_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


def wrap(text: str, prefix: str = "# ", width: int = 96) -> list[str]:
    out: list[str] = []
    for para in text.split("\n"):
        line = prefix.rstrip() if not para.strip() else prefix
        for word in para.split():
            if len(line) + len(word) + 1 > width and line.strip() != prefix.strip():
                out.append(line.rstrip())
                line = prefix
            line += word + " "
        out.append(line.rstrip())
    return out


def cost_block(mana_cost: str) -> list[str]:
    """`{3}{W}{W}` → a [cost] table. Hybrid/phyrexian pips are not in 2ed and are not handled."""
    pips = re.findall(r"\{([^}]+)\}", mana_cost or "")
    if not pips:
        return []
    generic, colored, colorless = 0, {}, 0
    for pip in pips:
        if pip.isdigit():
            generic += int(pip)
        elif pip == "X":
            pass  # `x_cost = true` is the DSL's own flag; emitted below
        elif pip == "C":
            colorless += 1
        elif pip in COLORS:
            colored[COLORS[pip]] = colored.get(COLORS[pip], 0) + 1
        else:
            raise SystemExit(f"unhandled mana pip {{{pip}}} in {mana_cost}")
    lines = ["", "[cost]"]
    if "X" in pips:
        lines.append("x = true")
    if generic:
        lines.append(f"generic = {generic}")
    if colorless:
        lines.append(f"colorless = {colorless}")
    for color in ("white", "blue", "black", "red", "green"):
        if color in colored:
            lines.append(f"{color} = {colored[color]}")
    return lines if len(lines) > 2 else []


def kind_block(card: dict) -> list[str]:
    type_line = card["type_line"]
    subtypes = type_line.split("—")[1].split() if "—" in type_line else []
    if "Land" in type_line:
        produces = [BASIC_LAND_MANA[s] for s in subtypes if s in BASIC_LAND_MANA]
        lines = ["", "[kind]", 'type = "land"']
        if "Basic" in type_line:
            lines.append("basic = true")
        if subtypes:
            lines.append("subtypes = [" + ", ".join(toml_str(s) for s in subtypes) + "]")
        if len(produces) == 1:
            lines.append(f"produces = {toml_str(produces[0])}")
        elif produces:
            lines.append("produces = [" + ", ".join(toml_str(p) for p in produces) + "]")
        return lines
    if "Creature" in type_line:
        also = [t for t in ("Artifact", "Enchantment") if t in type_line.split("—")[0]]
        lines = ["", "[kind]", 'type = "creature"', f"power = {card['power']}", f"toughness = {card['toughness']}"]
        if also:
            lines.append("also = [" + ", ".join(toml_str(t.lower()) for t in also) + "]")
        return lines
    for word, kind in (("Artifact", "artifact"), ("Enchantment", "enchantment"),
                       ("Instant", "instant"), ("Sorcery", "sorcery")):
        if word in type_line:
            if kind == "enchantment" and "Aura" in subtypes:
                return ["", "[kind]", 'type = "aura"']
            return ["", "[kind]", f'type = "{kind}"']
    raise SystemExit(f"unhandled type line: {type_line}")


def frame(card: dict) -> str:
    oracle = card.get("oracle_text") or ""
    type_line = card["type_line"]
    subtypes = type_line.split("—")[1].split() if "—" in type_line else []

    lines: list[str] = []
    lines += wrap(oracle) if oracle else ["# (no rules text — vanilla creature)"]
    lines.append(f"name = {toml_str(card['name'])}")
    lines.append("sets = [" + ", ".join(toml_str(s) for s in card["_sets"]) + "]")
    lines.append(f"id = {toml_str(card['oracle_id'])}")
    lines.append(f"default_print = {toml_str(card['id'])}")
    if oracle:
        lines.append(f"oracle = {toml_str(oracle)}")
    if "Legendary" in type_line:
        lines.append("legendary = true")
    if re.search(r"\benters tapped\b", oracle):
        lines.append("enters_tapped = true")
    if subtypes and "Land" not in type_line:
        lines.append("subtypes = [" + ", ".join(toml_str(s) for s in subtypes) + "]")
    keywords = [k.lower() for k in card.get("keywords", []) if k.lower() in BARE_KEYWORDS]
    if keywords:
        lines.append("keywords = [" + ", ".join(toml_str(k.replace(" ", "_")) for k in keywords) + "]")
    lines += cost_block(card.get("mana_cost", ""))
    lines += kind_block(card)
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("names", nargs="+")
    parser.add_argument("--write", action="store_true", help="write crates/cards/data/<slug>.toml")
    args = parser.parse_args()

    for name in args.names:
        text = frame(fetch(name))
        if not args.write:
            print(text)
            continue
        path = DATA / f"{slug(name)}.toml"
        if path.exists():
            print(f"exists, skipped: {path}", file=sys.stderr)
            continue
        path.write_text(text)
        print(f"wrote {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
