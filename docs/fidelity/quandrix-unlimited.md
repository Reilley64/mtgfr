# Fidelity report — Quandrix Unlimited (Secrets of Strixhaven)

Source: `docs/decklists/quandrix_unlimited.md` (official Wizards `soc` precon list, cross-checked
against live pool files and the current X-spell / counter / copy paths). Commander:
**Zimone, Infinite Analyst**. Backlog:
[quandrix-unlimited-increments.md](quandrix-unlimited-increments.md).

Intake counts: 85 faithful / 0 approximated / 0 expressible / 2 needing engine work.
The classifier returned 87 A / 0 B / 0 missing, but the Quandrix re-audit demoted two cards to D:
`Open the Way` still ignores its printed player-count ceiling on X, and `Zimone's Hypothesis` still
front-loads its resolution-time "counter a creature, then choose odd or even" decisions onto the
cast step.

**Current intake state (2026-07-26): 87/87 nonbasic Quandrix cards are in the pool. 85 are already
faithful; 2 still need engine work.**

## A. In pool, faithful at intake (85)

- [ ] Alchemist's Refuge
- [ ] Altered Ego
- [ ] Animist's Awakening
- [ ] Arcane Signet
- [ ] Astral Cornucopia
- [ ] Beast Within
- [ ] Benevolent Hydra
- [ ] Biomass Mutation
- [ ] Brass Infiniscope
- [ ] Command Tower
- [ ] Commander's Insight
- [ ] Curse of the Swine
- [ ] Decisive Denial
- [ ] Deekah, Fractal Theorist
- [ ] Elementalist's Palette
- [ ] Elusive Otter
- [ ] Entrancing Melody
- [ ] Eureka Moment
- [ ] Exotic Orchard
- [ ] Expansion Algorithm
- [ ] Fabled Passage
- [ ] Flooded Grove
- [ ] Forgotten Ancient
- [ ] Fractal Harness
- [ ] Goldvein Hydra
- [ ] Guardian Augmenter
- [ ] Hangarback Walker
- [ ] Hardened Scales
- [ ] Hinterland Harbor
- [ ] Hydroid Krasis
- [ ] Ingenious Prodigy
- [ ] Kami of Whispered Hopes
- [ ] Kinetic Ooze
- [ ] Lattice Library
- [ ] Lifeblood Hydra
- [ ] Mana Bloom
- [ ] Nature's Lore
- [ ] Nev, the Practical Dean
- [ ] Nexus Mentality
- [ ] Opal Palace
- [ ] Oran-Rief, the Vastwood
- [ ] Overflowing Basin
- [ ] Oversimplify
- [ ] Owlin Spiralmancer
- [ ] Ozolith, the Shattered Spire
- [ ] Paradox Gardens
- [ ] Path of Ancestry
- [ ] Perplexing Test
- [ ] Primal Might
- [ ] Primo, the Unbounded
- [ ] Primordial Hydra
- [ ] Pull from Tomorrow
- [ ] Quandrix Apprentice
- [ ] Quandrix Campus
- [ ] Quandrix Charm
- [ ] Quandrix Command
- [ ] Rain-Slicked Copse
- [ ] Rapid Hybridization
- [ ] Reliquary Tower
- [ ] Rogue's Passage
- [ ] Silkguard
- [ ] Sodden Verdure
- [ ] Sol Ring
- [ ] Steelbane Hydra
- [ ] Stonecoil Serpent
- [ ] Striding Shotcaller
- [ ] Stroke of Genius
- [ ] Study Hall
- [ ] Tanazir Quandrix
- [ ] Tangled Islet
- [ ] Temple of Mystery
- [ ] Temple of the False God
- [ ] Terramorphic Expanse
- [ ] The Goose Mother
- [ ] Three Visits
- [ ] Troyan, Gutsy Explorer
- [ ] Turbulent Wilderness
- [ ] Tyvar's Stand
- [ ] Unbound Flourishing
- [ ] Vineglimmer Snarl
- [ ] Yavimaya Bloomsage
- [ ] Yavimaya Coast
- [ ] Zimone, All-Questioning
- [ ] Zimone, Infinite Analyst
- [ ] Zimone, Quandrix Prodigy

Notes on cards deliberately kept in A despite intake `ponytail:` prompts:

- **Animist's Awakening** — the only shortcut is the random order of the bottomed nonlands. The
  deck never inspects that hidden order, so the deterministic stand-in stays unobservable here.
- **Guardian Augmenter** — the commander-only hexproof rider is implemented through the same
  creature-scoped commander anthem as its +2/+2 line. Quandrix's commanders are all creatures, so
  the creature-vs-commander distinction does not become observable in this deck.
- **Ozolith, the Shattered Spire** — the printed gate is "artifact or creature you control"; the
  current replacement reads any permanent you control. That superset still stays unobservable in
  Quandrix because no deck card puts +1/+1 counters on a nonartifact, noncreature permanent.
- **Quandrix Apprentice** — the bottom-of-library order after the land pick stays hidden, so the
  fixed order stands in cleanly.
- **Striding Shotcaller** — the real "one or more creatures ... deal combat damage" trigger is
  batch-shaped, while the shortcut fires once per damaging creature. Both converge on the same
  prepared state, and `become_prepared` is idempotent, so Quandrix has no way to observe the
  difference.
- **Vineglimmer Snarl** — the reveal is auto-collapsed instead of chosen explicitly, but revealing
  is strictly upside and no Quandrix card punishes showing the land.

## B. In pool, approximated at intake (0)

None.

## C. New, expressible today (0)

None. All 87 Quandrix nonbasics are already in the pool.

## D. In pool, not yet faithful; needs engine work (2)

These cards are scripted, but Quandrix makes their remaining gaps observable. Each card points at
the increment that clears it.

- [ ] Open the Way — #1
- [ ] Zimone's Hypothesis — #2

## Observability re-audit

### 1. Quandrix's headline X shell stays in A after re-audit

The decklist's scariest surfaces are no longer open gaps. The current engine already covers:

- `Unbound Flourishing` doubling X on permanent spells and copying X instants / sorceries and
  X-cost activated abilities.
- `Zimone, Infinite Analyst` reducing the first X spell each turn by its own +1/+1 counters and
  separately growing by two counters on that first X spell.
- `Hydroid Krasis`, `Hangarback Walker`, `Primordial Hydra`, `Goldvein Hydra`, `Lifeblood Hydra`,
  `Steelbane Hydra`, and the rest of the X-creature shell reading cast-time X or last-known
  counters correctly.
- `Altered Ego` entering as a copy with its extra X counters intact.
- `Tanazir Quandrix` and `Biomass Mutation`'s base-power/base-toughness set effects.
- `Perplexing Test`, `Oversimplify`, and `Curse of the Swine` on their core bounce / exile /
  replacement-token behavior.

The deck is therefore **not** blocked on a broad "X on stack" or "copy plus X" subsystem. The
remaining work is narrower.

### 2. `Open the Way` falsifies its own "player-count cap is unneeded" premise

`open_the_way.toml` explicitly notes that `"X can't be greater than the number of players in the
game"` is not enforced because no other pool effect bounded X by player count. Quandrix itself now
falsifies that premise: `Open the Way` is in the target deck, and a real Commander pod can produce
enough mana to announce an illegal X above the seat count. The spell's reveal-until-X-lands body is
implemented and tested, but the cast-time legality gate is still too permissive. That makes the cap
a live fidelity gap, not a harmless ponytail, so the card moves to D as increment #1.

Its second note — bottoming the nonlands in deterministic order instead of random order — still
stays hidden in this deck and does not need its own increment.

### 3. `Zimone's Hypothesis` still turns resolution-time choices into stack-time mode/target picks

`zimones_hypothesis.toml` models "choose odd or even" as a modal choice and the optional primer as a
targeted `put_counters` clause. That gets the visible final board state right in goldfish tests, but
Quandrix makes the shortcut observable:

- the odd/even choice is exposed on cast instead of during resolution,
- the optional counter is treated as a stack target even though the printed spell does not target a
  creature for that step,
- and opponents can react to information, or to a targetability requirement, that should not exist
  before resolution.

The printed sequence is "you may put a +1/+1 counter on a creature. Then choose odd or even. Return
each creature with power of the chosen quality ...", all during the spell's own resolution. That is
a real fidelity gap for this deck's combat-trick / parity-sweeper shell, so the card moves to D as
increment #2.

### 4. The remaining ponytails stay harmless for this deck

No other explicit note becomes a real Quandrix blocker:

- `Animist's Awakening` and `Quandrix Apprentice` only hide bottom-of-library ordering.
- `Guardian Augmenter`'s commander-only creature scope is exact for Quandrix's creature commanders.
- `Ozolith, the Shattered Spire`'s type-gate superset never meets a nonartifact, noncreature
  +1/+1-counter placement in this deck.
- `Striding Shotcaller`'s per-creature trigger shortcut collapses to the same prepared/unprepared
  state.
- `Vineglimmer Snarl`'s reveal shortcut stays strictly upside.

The re-audit therefore files exactly two Quandrix increments.
