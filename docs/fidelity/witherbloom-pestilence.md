# Fidelity report — Witherbloom Pestilence (Secrets of Strixhaven)

Source: `docs/decklists/witherbloom_pestilence.md` (official Wizards `soc` precon list, cross-checked
against live pool files and the current sacrifice / recursion / lifegain paths). Commander:
**Dina, Essence Brewer**. Backlog:
Engine increments for this grind all landed; backlog file removed.

Intake counts: 80 faithful / 1 approximated / 0 expressible / 3 needing engine work.
The classifier returned 83 A / 1 B / 0 missing, but the Witherbloom re-audit demoted three cards to
D: `Ominous Harvest` and `Plumb the Forbidden` still copy themselves from resolution instead of
cast-time context, and `Witherbloom Command`'s silent mandatory-vs-may land return becomes real in a
graveyard-resource deck. All three D increments (#1, #2) have since landed.

**Final state (2026-07-26): 84/84 nonbasic Witherbloom cards are in the pool — 83 fully faithful and
no deliberate approximations remaining. Both eligible engine increments (#1 cast-time self-copy,
#2 mandatory graveyard return) and #3 (`Final Act` missing modes) are LANDED.**

## A. In pool, faithful (83)

- [ ] Arcane Signet
- [ ] Assassin's Trophy
- [ ] Awakening Zone
- [ ] Beledros Witherbloom
- [ ] Blight Mound
- [ ] Blood Artist
- [ ] Bloodghast
- [ ] Blossoming Bogbeast
- [ ] Bojuka Bog
- [ ] Casualties of War
- [ ] Command Tower
- [ ] Creakwood Liege
- [ ] Culling Ritual
- [ ] Cultivate
- [ ] Deadly Brew
- [ ] Defiling Daemogoth
- [ ] Dina, Essence Brewer
- [ ] Dina, Soul Steeper
- [ ] Eccentric Pestfinder
- [ ] Elvish Mystic
- [ ] Exotic Orchard
- [ ] Fabled Passage
- [ ] Feral Appetite
- [ ] Festering Thicket
- [ ] Gilded Goose
- [ ] Gorma, the Gullet
- [ ] Grim Backwoods
- [ ] Gyome, Master Chef
- [ ] Haunted Mire
- [ ] Haywire Mite
- [ ] High Market
- [ ] Immoral Bargain
- [ ] Infernal Grasp
- [ ] Jadar, Ghoulcaller of Nephalia
- [ ] Llanowar Wastes
- [ ] Mazirek, Kraul Death Priest
- [ ] Merchant of Venom
- [ ] Moldervine Reclamation
- [ ] Morbid Opportunist
- [ ] Mortality Spear
- [ ] Mycoloth
- [ ] Necroblossom Snarl
- [ ] Nether Traitor
- [ ] Night's Whisper
- [ ] Ohran Frostfang
- [ ] Ominous Harvest — #1 (cast-time Gravestorm self-copy)
- [ ] Ophiomancer
- [ ] Path of Ancestry
- [ ] Pawn of Ulamog
- [ ] Pest Infestation
- [ ] Pest Rescuer
- [ ] Plumb the Forbidden — #1 (cast-time reflexive self-copy)
- [ ] Priest of Forgotten Gods
- [ ] Ribtruss Roaster
- [ ] Sakura-Tribe Elder
- [ ] Smothering Abomination
- [ ] Sol Ring
- [ ] Springbloom Druid
- [ ] Stensian Sanguinist
- [ ] Study Hall
- [ ] Teacher's Pest
- [ ] Temple of Malady
- [ ] Tendershoot Dryad
- [ ] Terramorphic Expanse
- [ ] Titan's Grave
- [ ] Toxic Deluge
- [ ] Trudge Garden
- [ ] Turbulent Fen
- [ ] Twilight Mire
- [ ] Umbral Collar Zealot
- [ ] Veinwitch Coven
- [ ] Vernal Fen
- [ ] Viridescent Bog
- [ ] Viscera Seer
- [ ] Wight of the Reliquary
- [ ] Witch of the Moors
- [ ] Witherbloom Campus
- [ ] Witherbloom Charm
- [ ] Witherbloom Command — #2 (mandatory land return)
- [ ] Woe Strider
- [ ] Woodland Cemetery
- [ ] Yahenni, Undying Partisan
- [ ] Zulaport Cutthroat

Notes on cards deliberately kept in A despite intake `ponytail:` prompts:

- **Infernal Grasp** — the note is explanatory only: the life-loss rider is already routed as life
  loss, not lifegain, so Witherbloom's lifegain watchers stay correct.
- **Morbid Opportunist** — the printed "one or more other creatures die" batch already collapses to
  one draw through the once-each-turn placement cap; the note documents the route, not a live
  residual.
- **Necroblossom Snarl** — auto-reveal is behaviorally exact here: revealing is strictly upside and
  no Witherbloom card or current pool card punishes showing the land.
- **Ohran Frostfang** — `snow = true` is authored; Into the North / Snow-Covered Forest make the
  snow supertype observable. No Witherbloom card reads snow as a payoff, but the residual is gone.
- **Toxic Deluge** — the note explains that X is paid as life, not mana. The cast-time life-payment
  behavior itself is landed.

## B. In pool, approximated at intake (0)

None. `Final Act` landed its two missing modes (2026-07-27) once `Invasion of Mercadia` (battle)
and `Infectious Inquiry` (poison) entered the pool as observers.

## C. New, expressible today (0)

None. All 84 Witherbloom nonbasics are already in the pool.

## D. In pool, not yet faithful; needs engine work (0)

All three demoted cards are now faithful:

- [x] Ominous Harvest — #1 LANDED (cast-time Gravestorm self-copy)
- [x] Plumb the Forbidden — #1 LANDED (cast-time reflexive self-copy)
- [x] Witherbloom Command — #2 LANDED (mandatory land return)

## Observability re-audit

### 1. Witherbloom validates most of the aristocrats stack that the decklist warned about

The highest-risk sacrifice / recursion cluster is mostly already landed, not newly broken:
`Blood Artist` and `Zulaport Cutthroat` have look-back death-watch coverage, `Morbid Opportunist`
already collapses simultaneous multi-death batches to one draw, `Dina, Soul Steeper` drains each
opponent without looping, `Bloodghast` / `Nether Traitor` / `Ophiomancer` / `Beledros Witherbloom`
exercise graveyard-functional and each-upkeep trigger paths, and `Mazirek`, `Priest of Forgotten
Gods`, and `Witch of the Moors` already have end-to-end tests on their main surfaces. The re-audit
therefore does **not** demote the deck's whole aristocrats shell just because it is dense.

### 2. `CopyThisSpell` is still a real cast-vs-resolution gap for this deck

`Ominous Harvest` and `Plumb the Forbidden` both authored their self-copy rider through the old
resolution-time `CopyThisSpell` path. That was observably late: the copies should exist from
cast-time context, above the original spell, rather than being minted only after the original
starts resolving. Deathdancer Xira already recorded this as a pool residual; Witherbloom is the
first SoC deck whose own game plan leans on both cards, so they moved to D here. This was increment
#1 — **now LANDED**: both cards copy via a `when_you_cast_this` / `copy_triggering_spell` cast
trigger, so the copies sit on the stack above the original.

### 3. `Witherbloom Command`'s mandatory-return shortcut is not harmless here

The file says the printed "you return a land card" is modeled as "you may return" because returning
a land is pure upside. Witherbloom falsifies that rationale: the deck deliberately uses the
graveyard as a resource (`Woe Strider` escape, long-game sacrifice loops, and multiple recursion
lines), so choosing to leave a land in the graveyard is sometimes strategically meaningful. The
deck therefore treats this as a real fidelity gap, not a harmless ponytail. This was increment #2 —
**now LANDED**: `may_return_from_graveyard` grew a `mandatory` flag and mode 0 sets it, so declining
with a legal land in the graveyard is illegal (no legal land still does nothing).

### 4. No stale pool-absence claim is falsified; the live problems are narrower

Unlike Silverquill's controller-vs-owner pass, Witherbloom does not expose a broad stale
"the-pool-has-no-X" premise. The old notes on `Infernal Grasp`, `Morbid Opportunist`,
`Necroblossom Snarl`, `Ohran Frostfang`, and `Toxic Deluge` are either explanatory or cosmetic and
stay in A; `Final Act` stays B because its explicit approximation is still real; the two live new
demotions are narrower silent gaps on cast-time copying and mandatory graveyard return.
