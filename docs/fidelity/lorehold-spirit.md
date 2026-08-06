# Fidelity report — Lorehold Spirit (Secrets of Strixhaven)

Source: `docs/decklists/lorehold_spirit.md` (official Wizards `soc` precon list, cross-checked
against the live pool files and the current planeswalker / graveyard / recursion paths). Commander:
**Quintorius, History Chaser**. Engine increments for this grind all landed; backlog file removed.

Intake counts: 79 faithful / 0 approximated / 0 expressible / 4 needing engine work.
The classifier returned 83 A / 0 B / 0 missing, including `Quintorius, History Chaser` from the
annotated commander lines; the re-audit demoted four cards to D once Lorehold's real trigger-shape
observers were considered.

**Current state (2026-07-26): 83/83 nonbasic Lorehold cards are in the pool and fully faithful
(0 residual blockers).**

## A. In pool, faithful after re-audit (83)

- [ ] Advanced Reconstruction
- [ ] Ao, the Dawn Sky
- [ ] Angel of Indemnity
- [ ] Anger
- [ ] Arcane Signet
- [ ] Archaeomancer's Map
- [ ] Atsushi, the Blazing Sky
- [ ] Augusta, Order Returned
- [ ] Balefire Liege
- [ ] Battlefield Forge
- [ ] Bitterthorn, Nissa's Animus
- [ ] Ceaseless Conflict
- [ ] Claim Jumper
- [ ] Clifftop Retreat
- [ ] Command Tower
- [ ] Containment Construct
- [ ] Conspiracy Theorist
- [ ] Currency Converter
- [ ] Drumbellower
- [ ] Emeria, the Sky Ruin
- [ ] Excava, the Risen Past
- [ ] Exotic Orchard
- [ ] Fabled Passage
- [ ] Faithless Looting
- [ ] Fateful Tempest
- [ ] Fellwar Stone
- [ ] Fields of Strife
- [ ] Furycalm Snarl
- [ ] Glittering Massif
- [ ] Guardian of Faith
- [ ] Guardian Scalelord
- [ ] Hofri Ghostforge
- [ ] Kami of Ancient Law
- [ ] Karmic Guide
- [ ] Kirol, History Buff
- [ ] Laelia, the Blade Reforged
- [ ] Lorehold Archivist
- [ ] Lorehold Campus
- [ ] Lorehold Charm
- [ ] Lotus Field
- [ ] Millikin
- [ ] Mind Stone
- [ ] Mistveil Plains
- [ ] Monologue Tax
- [ ] Moonshaker Cavalry
- [ ] Naktamun Lorespinner
- [ ] Patchwork Banner
- [ ] Path to Exile
- [ ] Perpetual Timepiece
- [ ] Primary Research
- [ ] Quintorius, Field Historian
- [ ] Quintorius, History Chaser
- [ ] Quintorius, Loremaster
- [ ] Radiant Summit
- [ ] Relic Retriever
- [ ] Remorseful Cleric
- [ ] Rip Apart
- [ ] Rugged Prairie
- [ ] Sacred Peaks
- [ ] Secret Rendezvous
- [ ] Seize the Spoils
- [ ] Selfless Spirit
- [ ] Serra Paragon
- [ ] Sevinne's Reclamation
- [ ] Skyclave Apparition
- [ ] Sol Ring
- [ ] Spirit of Resilience
- [ ] Squee, Goblin Nabob
- [ ] Staff of the Storyteller
- [ ] Study Hall
- [ ] Sun Titan
- [ ] Sunscorched Divide
- [ ] Swords to Plowshares
- [ ] Temple of Triumph
- [ ] Terramorphic Expanse
- [ ] Teshar, Ancestor's Apostle
- [ ] Tocasia's Welcome
- [ ] Tragic Arrogance
- [ ] Turbulent Steppe
- [ ] Vanguard of the Restless
- [ ] Venerable Warsinger
- [ ] Wave of Reckoning
- [ ] White Orchid Phantom

Notes on cards deliberately kept in A despite intake `ponytail:` prompts or Lorehold-risk warnings:

- **Advanced Reconstruction** — its level-2 "deals 2 damage to each opponent" shortcut still
  resolves as life loss, but Lorehold adds no player-damage-prevention, damage-redirection, or
  "dealt damage to a player" watcher that would distinguish the two.
- **Fateful Tempest** — the same player-damage-as-life-loss shortcut stays harmless here; the deck
  does not add a player-damage observer that would make the difference visible.
- **Furycalm Snarl** — the reveal is auto-collapsed to a hand scan, but revealing is strictly upside
  and no Lorehold card punishes showing the land.
- **Kirol, History Buff** — the prepared back face is still cast as the back-face spell itself
  rather than as a copy, but Lorehold has no magecraft / cast-copy observer that would distinguish
  the two.
- **Spirit of Resilience** — its copy payoff reads the chosen graveyard card's printed copyable
  values. That is exact here because the candidate list is built from cards that just left a
  graveyard, not battlefield permanents carrying copy-layer riders.

## B. In pool, approximated at intake (0)

None.

## C. New, expressible today (0)

None. All 83 Lorehold nonbasics are already in the pool.

## D. In pool, not yet faithful; needs engine work (0)

- [x] None. All former D cards landed and now sit in section A.

## Observability re-audit

### 1. Lorehold's headline Quintorius / Hofri / recursion shell stays in A

The decklist's loudest risks are already covered by the current engine and do **not** become new
Lorehold backlog items:

- **Planeswalker commander rules** are already live for `Quintorius, History Chaser`: loyalty costs
  are paid on activation, loyalty abilities are once per turn and sorcery-speed, creatures can
  attack a planeswalker, and combat damage removes loyalty instead of marking damage.
- **"Whenever one or more cards leave your graveyard"** already fires once per batch and threads the
  departed-card list through to Quintorius, `Advanced Reconstruction`, `Spirit of Resilience`, and
  Kirol's prepare trigger.
- **Hofri Ghostforge** already lands its full exile-then-copy-then-return loop: the dead nontoken
  creature is exiled, the token copy gains Spirit in addition to its other types, the token does
  not self-retrigger Hofri when it dies, and the exiled card returns to its owner's graveyard when
  the token leaves.
- **Lorehold's recursion shell** (`Karmic Guide`, `Sun Titan`, `Teshar, Ancestor's Apostle`,
  `Sevinne's Reclamation`, `Serra Paragon`, `Venerable Warsinger`, `Squee, Goblin Nabob`, `Anger`)
  reuses existing reanimate / flashback / play-from-graveyard / cast-from-nonhand-zone paths rather
  than demanding a new deck-local recursion subsystem.

The feared Quintorius loyalty / leaves-graveyard / Hofri / recursion work is therefore already
landed. Lorehold's live backlog is narrower.

### 2. `Ao` and `Atsushi` now choose triggered modes on placement (LANDED)

`Ao, the Dawn Sky` and `Atsushi, the Blazing Sky` both rode the triggered modal-choice fix from
increment #1:
their dies triggers now choose the branch when the trigger is put onto the stack, preserving the
real response window before resolution.

Ao's "creature or Vehicle" mode is faithful — `PermanentFilter.creature_or_vehicle` matches
creatures and Vehicle-subtype permanents; `Smuggler's Copter` is the Vehicle observer.

### 3. `Augusta, Order Returned` now creates the reflexive follow-up trigger correctly (LANDED)

`Augusta, Order Returned` now exiles the nonland cards first, then creates the reflexive trigger
only if at least one card was exiled, and that second trigger chooses its attacking-creature target
on its own stack object. This closes increment #2.

### 4. `Conspiracy Theorist` now batches nonland discards into one exile choice (LANDED)

`Conspiracy Theorist` now uses `timing = "you_discard_nonland"` +
`may_exile_discarded_nonland_may_play`: a land discard grants no impulse play, and a multi-card
nonland discard yields one "choose one of them" exile window. This closes increment #3.

### 5. The remaining ponytails stay harmless for this deck

No other explicit residual becomes a Lorehold blocker:

- `Advanced Reconstruction` and `Fateful Tempest` both stay within the engine's current
  player-damage-as-life-loss ceiling.
- `Furycalm Snarl`'s reveal shortcut stays strictly upside.
- `Kirol, History Buff`'s prepared back-face cast stays behaviorally exact for this deck.
- `Spirit of Resilience`'s copy-source note remains exact for graveyard cards.

The re-audit's three Lorehold increments are now closed; Lorehold needs no remaining deck-local
fidelity work.
