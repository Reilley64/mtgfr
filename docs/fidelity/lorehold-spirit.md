# Fidelity report — Lorehold Spirit (Secrets of Strixhaven)

Source: `docs/decklists/lorehold_spirit.md` (official Wizards `soc` precon list, cross-checked
against the live pool files and the current planeswalker / graveyard / recursion paths). Commander:
**Quintorius, History Chaser**. Backlog:
[lorehold-spirit-increments.md](lorehold-spirit-increments.md).

Intake counts: 79 faithful / 0 approximated / 0 expressible / 4 needing engine work.
The classifier returned 83 A / 0 B / 0 missing, including `Quintorius, History Chaser` from the
annotated commander lines; the re-audit demoted four cards to D once Lorehold's real trigger-shape
observers were considered.

**Current state (2026-07-26): 83/83 nonbasic Lorehold cards are in the pool — 79 fully faithful,
with 4 scripted cards still blocked on trigger-shape engine work.**

## A. In pool, faithful after re-audit (79)

- [ ] Advanced Reconstruction
- [ ] Angel of Indemnity
- [ ] Anger
- [ ] Arcane Signet
- [ ] Archaeomancer's Map
- [ ] Balefire Liege
- [ ] Battlefield Forge
- [ ] Bitterthorn, Nissa's Animus
- [ ] Ceaseless Conflict
- [ ] Claim Jumper
- [ ] Clifftop Retreat
- [ ] Command Tower
- [ ] Containment Construct
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
- **White Orchid Phantom** — "may search" still collapses to fail-to-find on the search itself, but
  declining to search is indistinguishable from searching and choosing no basic land.

## B. In pool, approximated at intake (0)

None.

## C. New, expressible today (0)

None. All 83 Lorehold nonbasics are already in the pool.

## D. In pool, not yet faithful; needs engine work (4)

- [ ] Ao, the Dawn Sky — #1
- [ ] Atsushi, the Blazing Sky — #1
- [ ] Augusta, Order Returned — #2
- [ ] Conspiracy Theorist — #3

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

### 2. `Ao` and `Atsushi` make the triggered modal-choice shortcut observable

`Atsushi, the Blazing Sky` already declares the residual openly: its dies trigger chooses the mode
at resolution instead of when the triggered ability is put onto the stack. `Ao, the Dawn Sky` uses
the same triggered `choose_one` path, so it carries the same shape even though its file does not
spell it out.

Lorehold makes that visible because these are real multiplayer, instant-speed trigger windows:
players should know which branch was chosen before deciding whether to respond, and the engine's
current pause happens only after the trigger starts resolving. The live gap is therefore the
trigger-shape rule itself, not either card's dig / Treasure / counter payload. This is increment
[#1](lorehold-spirit-increments.md#1-triggered-modal-choices-must-be-chosen-on-placement--2-cards-m).

Ao's separate "creature or Vehicle" trim still stays harmless here — Lorehold fields no Vehicle card
and no Lorehold card distinguishes the absent type line.

### 3. `Augusta, Order Returned` still front-loads a reflexive-trigger target

`Augusta, Order Returned` currently chooses its target attacking creature **before** the graveyard
exiles happen, then pays out counters in the same resolving ability. The printed card is a reflexive
trigger: only if one or more nonland cards were exiled this way should a second trigger be created,
and that second trigger should choose its attacking-creature target afterward, on its own stack
object.

Lorehold's own graveyard-exit shell makes the count live every game, so the target timing and
missing response window are not harmless documentation noise. This is increment
[#2](lorehold-spirit-increments.md#2-reflexive-trigger-follow-up-targeting--1-card-m).

### 4. `Conspiracy Theorist` falsifies its own discard-trigger ponytail

`Conspiracy Theorist`'s file still claims two shortcuts are harmless:

- the trigger fires on **every** discarded card, not once per discard event, and
- it fires on discarded **lands** too, not only on "one or more nonland cards".

Lorehold falsifies both. The deck can discard lands to Quintorius's +1, to `Conspiracy Theorist`'s
own attack trigger, and to spells like `Faithless Looting` / `Seize the Spoils`; under the current
shape, a discarded land can still be exiled and played, and multiple discarded nonlands would yield
multiple single-card triggers instead of one "choose one of them" window. That is a live fidelity
gap, not a harmless upside note. This is increment
[#3](lorehold-spirit-increments.md#3-discard-trigger-batch-filter-and-choose-one--1-card-m).

### 5. The remaining ponytails stay harmless for this deck

No other explicit residual becomes a Lorehold blocker:

- `Advanced Reconstruction` and `Fateful Tempest` both stay within the engine's current
  player-damage-as-life-loss ceiling.
- `Furycalm Snarl`'s reveal shortcut stays strictly upside.
- `Kirol, History Buff`'s prepared back-face cast stays behaviorally exact for this deck.
- `Spirit of Resilience`'s copy-source note remains exact for graveyard cards.
- `White Orchid Phantom`'s fail-to-find stand-in is indistinguishable from declining the search.

The re-audit therefore files exactly three Lorehold increments affecting four cards, not a broader
rewrite of the deck's headline planeswalker or recursion shell.
