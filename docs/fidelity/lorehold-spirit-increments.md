# Lorehold Spirit deck increments (2026-07-26)

Deck report: [lorehold-spirit.md](lorehold-spirit.md). This file is the sole
engine-capability backlog for this deck (ranked increments plus the concrete Lorehold cards they
unblock).

From `docs/decklists/lorehold_spirit.md` (official Wizards `soc` precon; commander Quintorius,
History Chaser). Lorehold's marquee planeswalker-commander / leaves-graveyard / Hofri / recursion
shell is already landed. The live backlog is narrower trigger work: two modal dies triggers still
choose on resolution, one reflexive trigger still front-loads its target, and one discard trigger
still treats every discarded card as a separate eligible impulse source.

### 1. `triggered-modal-choices-must-be-chosen-on-placement` — 2 cards, M

**Depends on:** none.
**Cards:** `ao_the_dawn_sky.toml`, `atsushi_the_blazing_sky.toml`
**Sketch:** Triggered modal abilities choose their mode when they are put onto the stack, not after
players have passed priority into resolution (CR 603.3d, CR 700.2). Today Lorehold's two modal dies
triggers pause on `ChooseMode` only once the trigger is already resolving, which hides information
that should have been public before responses. The smallest clean fix is to let a triggered
`Effect::ChooseOne` raise its mode choice at trigger-placement time, store the chosen branch on the
placed trigger itself, and then resolve directly down that branch with no mid-resolution mode pause.
Regression bar:

- `Ao, the Dawn Sky` and `Atsushi, the Blazing Sky` expose their chosen branch before opponents pass
  on the trigger,
- their existing payload behavior (Ao selection/counters, Atsushi exile-or-Treasures) stays
  unchanged once the mode is chosen,
- the trigger no longer pauses on `ChooseMode` after resolution has begun.

### 2. `reflexive-trigger-follow-up-targeting` — 1 card, M

**Depends on:** none.
**Cards:** `augusta_order_returned.toml`
**Sketch:** `Augusta, Order Returned` prints a reflexive trigger: first, each player exiles a card
from their graveyard; then, **when one or more nonland cards are exiled this way**, a second trigger
goes onto the stack targeting an attacking creature and putting that many +1/+1 counters on it.
Today the engine chooses the target attacking creature up front and pays out in the same resolving
ability. Add a small reflexive-trigger follow-up path that can:

1. remember the counted result from the first resolving effect,
2. create no follow-up at all when the count is 0,
3. create a new stack object when the count is positive, and
4. choose the attacking-creature target on that follow-up, not before the graveyard fan-out.

Regression bar:

- all-lands exile creates no second trigger and no target prompt,
- a positive nonland count creates a second trigger that targets an attacking creature afterward,
- players can respond in the real window between the graveyard fan-out and the counter placement.

### 3. `discard-trigger-batch-filter-and-choose-one` — 1 card, M

**Depends on:** none.
**Cards:** `conspiracy_theorist.toml`
**Sketch:** `Conspiracy Theorist` needs a discard-trigger shape stricter than the current
`Trigger::YouDiscard` plumbing. Today the trigger queues once per discarded card and threads only one
discarded-card id, which is good enough for `Containment Construct` but not for "Whenever you
discard **one or more nonland cards**, you may exile **one of them** ...". Lorehold makes that
difference live because the deck routinely discards lands and can discard multiple cards in a turn.
Add a batch discard trigger context that carries the full discarded-card set for one discard event,
filters it to nonlands, and then pauses on one "choose one of them" decision before exiling the
chosen card to play. Regression bar:

- discarding only a land does **not** trigger `Conspiracy Theorist`,
- discarding two nonland cards in one event yields one trigger and one choice among those two cards,
- `Conspiracy Theorist`'s own attack-loot ability still draws after a discard, but discarding a land
  there does not incorrectly grant impulse play of that land.
