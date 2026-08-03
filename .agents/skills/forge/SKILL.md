---
name: forge
description: Look up Card-Forge/forge card scripts, token scripts, and effect scripts as the rules-implementation reference when authoring or grinding mtgfr cards. Use for tricky rules interactions, fidelity gaps, or "how does Forge model this ability?"
---

# Forge reference

[Card-Forge/forge](https://github.com/Card-Forge/forge) is the external rules/script reference for mtgfr. Prefer it for *implementation shape* after Scryfall oracle text and rulings.

Vendored tree (committed like `./.repos/effect`): `./.repos/forge` — sparse snapshot of:

- `forge-gui/res/cardsfolder/` — card scripts (`.txt`)
- `forge-gui/res/tokenscripts/` — token scripts
- `forge-gui/res/effects/` — shared effect scripts

## Prerequisite

Before Forge-specific work, check that `./.repos/forge/forge-gui/res/cardsfolder` exists.

If it does not, restore from git (`git checkout -- .repos/forge`) or re-sync from upstream:

```sh
just forge          # scripts/sync-forge.sh — replace tree from upstream tip
```

Commit the `.repos/forge` diff after a sync so the vendor stays shared.

Do not nest a `.git` under `.repos/forge`. Do not expand to the full Forge monorepo / Java sources unless the user explicitly asks for `forge-game` / `forge-core`.

## When to use

| Need | Source |
|------|--------|
| Printed oracle / legality / rulings | Scryfall MCP (or Scryfall site) |
| How Forge scripts the card | This skill → `.repos/forge` |
| mtgfr DSL authoring bar | `card-dsl` |
| Deck-wide faithful grind | `fidelity-grind` |

Use Forge when deciding how to compose mtgfr effects, when a trigger/replacement is ambiguous, or when comparing keyword/ability encoding. Forge scripts are a *reference*, not a literal target language — translate into composable mtgfr DSL leaves; do not invent a one-off effect leaf per card when existing vocabulary can combine.

## Lookup

Card files live at:

```text
.repos/forge/forge-gui/res/cardsfolder/<first-letter>/<slug>.txt
```

Slug is lowercase, underscores for spaces/punctuation (e.g. Sol Ring → `s/sol_ring.txt`, Lightning Bolt → `l/lightning_bolt.txt`).

```sh
# By slug / name fragment
rg -n -i 'Name:Sol Ring' .repos/forge/forge-gui/res/cardsfolder
# Or open the expected path directly
rg -n 'A:|K:|SVar:' .repos/forge/forge-gui/res/cardsfolder/s/sol_ring.txt

# Ability / mode search across the pool
rg -n 'ChangeZone|DealDamage|Pump' .repos/forge/forge-gui/res/cardsfolder -g '*.txt' | head

# Tokens / shared effects
rg -n -i '<name>' .repos/forge/forge-gui/res/tokenscripts
rg -n -i '<pattern>' .repos/forge/forge-gui/res/effects
```

Read the matching `.txt` with the Read tool. Forge script lines of interest:

- `Name:` / `ManaCost:` / `Types:` / `Oracle:` — identity
- `A:SP$` / `A:AB$` / `T:` — spell, activated, triggered abilities
- `K:` — keywords
- `SVar:` — shared variables / cost helpers

## Research order for a sticky card

1. Scryfall oracle + rulings (MCP).
2. Forge card script (this tree).
3. Nearby Forge scripts with the same ability shape (`rg` on the mode name).
4. Author in mtgfr via `card-dsl` — compose existing effects; flag gaps in the deck's `docs/fidelity/<slug>-increments.md` rather than contorting.

## Setup reference

If the vendor bootstrap needs changing, see [`references/setup.md`](references/setup.md).
