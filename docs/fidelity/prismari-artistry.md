# Fidelity report — Prismari Artistry (Secrets of Strixhaven)

Source: `docs/decklists/prismari_artistry.md` (official Wizards `soc` precon list, cross-checked
against live pool files and the current copy / cast-trigger / mana-payment paths). Commander:
**Rootha, Mastering the Moment**. Backlog:
[prismari-artistry-increments.md](prismari-artistry-increments.md).

Intake counts: 75 faithful / 0 approximated / 0 expressible / 10 needing engine work.
The classifier returned 85 A / 0 B / 0 missing, but the Prismari re-audit demoted 10 cards to D:
the deck's own copy shell falsifies the engine's "first-generation copy only" shortcut, and
`Goldspan Dragon` turns the current "two independent any-color credits" mana note into a real
gameplay bug.

**Current state (2026-07-26): 85/85 nonbasic Prismari cards are in the pool — 75 fully faithful
and 10 still blocked on two engine increments (#1 copy-effect exception riders becoming copiable,
#2 linked same-color mana credits).**

## A. In pool, faithful at intake (75)

- [ ] Abrade
- [ ] Abstract Performance
- [ ] Aether Gale
- [ ] Arcane Denial
- [ ] Arcane Signet
- [ ] Archmage Emeritus
- [ ] Big Score
- [ ] Blasphemous Act
- [ ] Brazen Borrower
- [ ] Cascade Bluffs
- [ ] Chain Reaction
- [ ] Chaos Warp
- [ ] Coastal Peak
- [ ] Command Tower
- [ ] Creative Technique
- [ ] Curiosity Crafter
- [ ] Dance with Calamity
- [ ] Deep Analysis
- [ ] Dig Through Time
- [ ] Dirgur Focusmage
- [ ] Exotic Orchard
- [ ] Expressive Iteration
- [ ] Fabled Passage
- [ ] Faerie Mastermind
- [ ] Fellwar Stone
- [ ] Ferrous Lake
- [ ] Frostboil Snarl
- [ ] Furygale Flocking
- [ ] Galazeth Prismari
- [ ] Hall of Oracles
- [ ] Harmonic Prodigy
- [ ] Inspired Skypainter
- [ ] Leitmotif Composer
- [ ] Lightning Greaves
- [ ] Magma Opus
- [ ] Mana Geyser
- [ ] Manaform Hellkite
- [ ] Mirrorwing Dragon
- [ ] Molten Tributary
- [ ] Mystic Sanctuary
- [ ] Path of Ancestry
- [ ] Plargg and Nassari
- [ ] Prismari Campus
- [ ] Prismari Charm
- [ ] Prismari Command
- [ ] Prismari Pianist
- [ ] Reality Shift
- [ ] Reliquary Tower
- [ ] Renegade Bull
- [ ] Resculpt
- [ ] Restless Spire
- [ ] Rootha, Mastering the Moment
- [ ] Rootha, Mercurial Artist
- [ ] Rousing Refrain
- [ ] Scorched Geyser
- [ ] Shivan Reef
- [ ] Sol Ring
- [ ] Solemn Simulacrum
- [ ] Spectacle Summit
- [ ] Storm-Kiln Artist
- [ ] Stormcatch Mentor
- [ ] Study Hall
- [ ] Sulfur Falls
- [ ] Surge to Victory
- [ ] Talisman of Creativity
- [ ] Temple of Epiphany
- [ ] Temple of the False God
- [ ] Terramorphic Expanse
- [ ] Throes of Chaos
- [ ] Thunderclap Drake
- [ ] Treasure Cruise
- [ ] Turbulent Springs
- [ ] Veyran, Voice of Duality
- [ ] Volcanic Salvo
- [ ] Volcanic Torrent

Notes on cards deliberately kept in A despite intake `ponytail:` prompts or deck-risk warnings:

- **Dig Through Time** — the only shortcut is the bottom-of-library order after the keep. The
  library stays hidden, Prismari has no card that inspects that exact bottom order, and the
  deterministic stand-in remains unobservable here.
- **Frostboil Snarl** — auto-revealing via `HandHasLandWithSubtype` is still behaviorally exact for
  this deck: revealing is strictly upside and no Prismari card punishes showing the land.
- **Leitmotif Composer** — the note is explanatory, not a residual. Its unblockable filter already
  reads the permanent's current copied name, so Prismari's copy effects do not create a second
  hidden name seam on this card.
- **Magecraft / trigger doubling shell** — `Archmage Emeritus`, `Storm-Kiln Artist`,
  `Manaform Hellkite`, `Veyran, Voice of Duality`, and `Harmonic Prodigy` stay faithful. The risky
  part of the deck is the copy shell, not trigger multiplication: Prismari's permanent-sourced
  cast/copy triggers still fit the engine's landed trigger-doubling path.

## B. In pool, approximated at intake (0)

None.

Every live Prismari gap exposed by the re-audit needs engine work, so the affected cards move
straight to D instead of sitting in B with a papered-over note.

## C. New, expressible today (0)

None. All 85 Prismari nonbasics are already in the pool.

## D. In pool, not yet faithful; needs engine work (10)

These cards are scripted, but Prismari makes their remaining engine gaps observable. Each card
points at the increment(s) that clear it.

- [ ] Brudiclad, Telchor Engineer — #1
- [ ] Cursed Mirror — #1
- [ ] Determined Iteration — #1
- [ ] Goldspan Dragon — #2
- [ ] Muddle, the Ever-Changing — #1
- [ ] Redoubled Stormsinger — #1
- [ ] Replication Technique — #1
- [ ] Rite of Replication — #1
- [ ] Rionya, Fire Dancer — #1
- [ ] Twinflame — #1

## Observability re-audit

### 1. Prismari falsifies the "no card copies something already under a copy effect" premise

`pending/handlers/edict.rs` and `cast.rs` still carry the old premise that no pool card will copy
an object whose copy effect already added a copiable rider outside its printed `CardDef`. Prismari
breaks that immediately. `Twinflame`, `Rionya, Fire Dancer`, and `Determined Iteration` make token
copies that should stay "that creature, except it has haste"; `Muddle, the Ever-Changing` becomes a
copy "except it has myriad"; then `Brudiclad, Telchor Engineer`, `Redoubled Stormsinger`,
`Replication Technique`, `Rite of Replication`, `Cursed Mirror`, and `Muddle` itself can copy those
objects again.

Today those second-generation copy paths mostly read only `def_id_of(...)` and model the
copy-exception keyword as a temporary boost, so the follow-on copy loses the copiable rider. A
Brudiclad copy of a `Twinflame` token should keep haste; a `Replication Technique` token copy of a
temporary `Cursed Mirror` creature should keep that copy's haste; a copy of Muddle's copied form
should keep myriad. This is increment #1.

### 2. `Goldspan Dragon`'s same-color Treasure note is now a real rules gap

`goldspan_dragon.toml` still says its "Add two mana of any one color" rider is harmlessly modeled
as `mana = ["any", "any"]`. Prismari falsifies that premise because the deck is full of blue-red
costs and color-intensive follow-ups. One Goldspan-boosted Treasure can currently split as one blue
and one red toward cards like `Prismari Command`, `Muddle, the Ever-Changing`, or
`Rootha, Mastering the Moment`, even though both mana must be the same chosen color. This is
increment #2.

### 3. Prismari's magecraft-doubling risk stays in A after re-audit

The decklist flagged `Veyran, Voice of Duality` plus `Harmonic Prodigy` as a likely engine seam,
but Prismari's own watchers stay inside the trigger-doubling implementation that already exists.
`Archmage Emeritus`, `Storm-Kiln Artist`, and Veyran's own magecraft trigger are all
permanent-sourced triggers caused directly by casting or copying an instant or sorcery; Harmonic's
subtype gate also stays exact for Prismari's Wizard/Shaman shell. The current issue is therefore
not "trigger doubling is missing," but "copy-generated copiable riders are not preserved when those
objects get copied again."

### 4. The remaining ponytails stay harmless for this deck

No other `approximates` / `ponytail:` claim is falsified by Prismari's intake:

- `Dig Through Time`'s bottom-order shortcut stays hidden.
- `Frostboil Snarl`'s reveal shortcut stays strictly upside.
- `Leitmotif Composer`'s name-matters note is already satisfied by the current copied name the
  filter reads.

The re-audit therefore files exactly two Prismari-specific increments, not a wider sweep.
