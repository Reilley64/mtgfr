# Turn, Priority, and Stack

**Status:** Current (as of 2026-07-20)
**Module:** `crates/engine` (`src/priority.rs`, `src/pipeline.rs`, `src/query.rs`, `src/playable.rs`)

---

## Problem Statement

A Commander game has a complex turn structure — thirteen named steps across five phases — with priority passing between two to four players at every step that grants it. Players need the game to advance automatically through "meaningless" priority windows (where no one can meaningfully act) while still pausing correctly for any player who can cast, activate, or declare. Additionally, the client needs rich chrome (Next, Stack Pass, Stack Yield, Turn Yield, End Turn) so players can express coarse-grained intent without manually passing every window.

---

## Solution

The engine models priority as a single integer field (`Game::priority: PlayerId`) plus a consecutive-pass counter (`Game::consecutive_passes: u8`). The server wraps the engine with auto-pass logic driven by `Game::has_meaningful_action`, submitting `PassPriority` intents on behalf of helpless seats (turn-priority-and-stack spec). Yield flags (stack yield, turn yield) are server-side chrome that extend the auto-pass set to include seats that *could* act but have opted out. The engine stays intent-only — it does not know about yields.

---

## User Stories

1. As a **player**, I want the game to advance automatically through steps where I have nothing to do, so I don't have to manually pass every window.
2. As a **player**, I want to see a **Next** button on an empty stack so I can advance to the next step with one press.
3. As a **player**, I want a **Stack Pass** button when a spell or ability is on the stack and I hold priority, so I can let it resolve without casting anything.
4. As a **player**, I want a **Stack Yield** toggle so I can opt out of the entire current stack, resuming control when it empties.
5. As a **player**, I want a **Turn Yield** toggle so I can auto-pass through every priority window until my next turn, without re-arming it after each stack.
6. As a **player on my own turn**, I want an **End Turn** control that auto-advances the rest of my turn while opponents still get windows to cast instants.
7. As an **attacked player**, I want turn yield to clear automatically when someone attacks me, so I can declare blockers even if I had turn-yielded.
8. As a **player**, I want the game to pause before resolving a stack item so the table can read the card (stack hold), with optional helpless dwell to buy a little extra time.
9. As a **player**, I want mana abilities (tap-for-mana) to resolve immediately without touching the stack or priority.
10. As a **player in cleanup**, I want the engine to automatically discard to hand size and remove marked damage without granting priority (unless a triggered ability fires).
11. As a **card**, I want triggered abilities to fire at the next priority window in APNAP order after the event that triggered them.
12. As a **starting player** in a two-player game, I want to skip my first draw step (CR 103.8a).
13. As a **player**, I want the untap step to untap my permanents without granting priority (CR 502.1).

---

## Behavior

### Steps and phases

The active player's turn progresses through these `Step` variants in order:

```
Untap → Upkeep → Draw → Main1 →
  BeginCombat → DeclareAttackers → DeclareBlockers →
  CombatDamage → EndCombat →
Main2 → End → Cleanup
```

Steps that **do not grant priority**: `Untap`, `Cleanup` (unless a triggered ability fires or a discard-to-hand-size is needed — in which case priority is granted after).

All other steps grant priority to the active player on entry.

### Turn-based actions (TBAs)

TBAs run automatically at the beginning of a step, before priority is granted (CR 703):

- **Untap**: untap all permanents the active player controls (CR 502.3). Clears turn-scoped tallies (`creatures_died_this_turn`, `spells_cast_this_turn`, `nontoken_creatures_entered_this_turn`, `land_entered_under_your_control_this_turn`, `permanents_died_this_turn`, `damaged_this_turn`). Clears goad entries for the (now-previous) active player. Advances Suspend time counters (decrements each; a card reaching 0 counters is cast for free).
- **Draw**: the active player draws one card. Skipped for the starting player in their first turn of a two-player game (armed in `begin_first_turn`, spent here).
- **Cleanup**: discard to hand size (7; raises `PendingChoice::DiscardToHandSize` if over), remove marked damage from all permanents, end until-EOT effects (`ControlEndedUntilEndOfTurn`, play-from-exile permissions expire, granted abilities expire). If a triggered ability fired during cleanup, priority is granted for a mini-priority-round before another cleanup pass.

### Priority model

- Priority begins with the active player on every step that grants it.
- After a player acts (casts, activates, plays a land), priority returns to the active player (CR 117.3b).
- After a stack item resolves, priority returns to the active player (CR 608.3).
- When all players pass priority in succession (`consecutive_passes == num_living_players`):
  - If the stack is non-empty, resolve the top item, reset passes to 0, return priority to the active player.
  - If the stack is empty, advance to the next step.
  - Combat declaration steps (DeclareAttackers, DeclareBlockers) remain until a valid declaration is made, then priority passes normally.

### Mana abilities

- A mana ability (CR 605) is an activated ability that produces mana and has no target.
- `Intent::TapForMana` and `Intent::ActivateAbility` for a mana-producing ability resolve **immediately** without the stack, without touching `priority` or `consecutive_passes`.
- `Game::tap_for_mana` validates controller/tapped status and dispatches to `activate_ability` for the appropriate ability index, or directly produces mana for the land's `produces` sugar.
- Bare mana production is never a "meaningful action" for auto-pass purposes (turn-priority-and-stack spec).

### Auto-pass and meaningful actions

`Game::has_meaningful_action(player)` returns `true` when `Game::meaningful_actions(player)` is non-empty. A meaningful action is one of:

- A land drop available in the player's hand during their main phase (sorcery speed, empty stack).
- A castable spell: timing, zone, affordability, at least one legal target (or enough playable modes). On an empty stack during a non-main / non-declare step, **only instant-speed casts count**; sorcery-speed spells do not stop auto-pass.
- An activatable non-mana ability on a permanent they control.
- A combat declaration that is awaited by the engine (DeclareAttackers / DeclareBlockers step).
- **Exception:** after attackers are declared, each defending player's Declare Attackers priority also counts empty-stack instants as meaningful, so defenders can respond before blockers (turn-priority-and-stack spec).

The server's auto-advance loop submits `PassPriority` for any seat where `has_meaningful_action` is false AND (`stack_yield || turn_yield`), bounded to 256 passes to prevent infinite loops.

### Stack yield, Turn yield, End Turn

These are **server-side chrome**, not engine concepts. The engine only receives `PassPriority` intents.

- **Stack yield**: once armed for a seat, auto-advance treats that seat as passing for the rest of the current stack. Clears when the stack empties.
- **Turn yield**: once armed, auto-advance treats the seat as passing until one of: (a) it becomes the active player at Untap, (b) it is attacked (any `AttackerDeclared` targeting it clears the flag), (c) it submits any intentional action (cast, activate, manual Pass / Next, or a non-empty combat declaration). Auto-submitted `PassPriority` (from auto-advance or the hold timer) does not clear it. Empty `DeclareAttackers` / `DeclareBlockers` are structural seals (same as auto-advance’s empty seal) and do not clear the flag — so a client “No attackers” racing `SetTurnYield` cannot cancel End Turn.
- **End Turn**: turn yield armed while that seat is the active player. Additional clears: (a) at Cleanup for that seat, (b) when any other seat submits a non-PassPriority intent (so the ending player can respond). While End Turn is active, `has_empty_stack_instant_play` is checked on other seats so they still get response windows.

### Stack hold and helpless dwell

- Before auto-resolving a stack item (when all players pass and the stack is non-empty), the server waits `STACK_HOLD` (2 s) so the table can read the card.
- During the hold, a seat with no meaningful action may register a **helpless dwell** via gRPC `Game.SetStackDwell` (BFF `/api/rpc/game/:table/stack-dwell`), postponing resolution until the dwell ends. A hard cap of `STACK_HOLD + 3 s` prevents indefinite delay.
- Seats that have a meaningful action are not helpless and cannot dwell-pause.
- The visible state carries `stack_hold_remaining_ms` for client countdown rendering.

### Mana pool

- Each player's mana pool is a `ManaPool` struct tracking colored (WUBRG), colorless `{C}`, any, either (dual-color pairs), and restricted-palette mana separately.
- The pool empties at the end of every step and phase (CR 106.4). Mana added during a step persists through that step's priority round but empties before the next step's TBAs.
- `persist: true` mana (e.g. from Rousing Refrain's suspended delay) is not cleared at step boundaries — it carries over until used or the end of the turn.

---

## Implementation Decisions

- **Step advancement is in `priority.rs` (`Game::advance_step`).** It handles the step-by-step TBA dispatch and fires `StepBegan` events. Combat-step gating (DeclareAttackers, DeclareBlockers awaiting declarations) is also in this module.
- **`PostIntentPipeline` phases are stable and ordered.** `pipeline.rs` defines `PostIntentPhase::ALL` as a fixed const slice; new phases must be inserted in rules order.
- **`CastPlayKind` distinguishes list, one-click, and full-validate paths** (`playable.rs`). The list path (for `meaningful_actions` / auto-pass) checks timing, zone, affordability, and target availability but does not require chosen inputs. The full path also validates chosen discard picks, graveyard exile, etc.
- **`has_empty_stack_instant_play` is a separate predicate from `has_meaningful_action`** (turn-priority-and-stack spec). It is broader: it includes empty-stack instant-speed casts that `has_meaningful_action` omits. Used only by the End Turn server chrome to grant opponent response windows.
- **`consecutive_passes` resets on every act, every resolution.** It does not reset on the step boundary — advancing a step is a direct state mutation in `advance_step`, not driven by the pass counter.
- **Engine is intent-only.** Yield flags, hold timers, dwell registration, and auto-advance loops live entirely in the server layer, not in the engine. Adding a new yield dimension = server chrome only.

---

## Testing Decisions

- **Priority round tests**: submit `PassPriority` for each seat in sequence and assert the step advances (or the stack resolves) after the correct number of passes.
- **Auto-pass tests**: construct a board where a player has no meaningful action, verify `has_meaningful_action` returns false, submit their `PassPriority` and confirm priority advances correctly.
- **Mana ability tests**: tap a land via `TapForMana`, assert mana pool increases and priority is unchanged.
- **Step TBA tests**: start from Untap, submit `begin_first_turn`, assert permanents are untapped, draw happened, and priority is now in Upkeep.
- **Cleanup tests**: give a player more than 7 cards, reach Cleanup, assert `PendingChoice::DiscardToHandSize` is raised.
- **Prior art**: `tests/game.rs` contains full-turn integration tests; the engine's in-module tests in `lib.rs` cover `forced_action` and `refresh_actions` edge cases.

---

## Out of Scope

- **Client chrome rendering** (how the context bar is drawn, button labels, toggle animations) — this spec covers engine and server behavior only.
- **Server auto-advance implementation details** (HTTP routes, session management) — those live in `crates/server`.
- **Phasing** (CR 702.26) — not yet implemented; all permanents are treated as never phased out. Schedule via a deck's fidelity increments when needed.
- **Two-headed giant / other formats** — the engine targets 2–4 player free-for-all Commander only.

---

## Further Notes

- See `2026-07-20-engine-core-and-event-model.md` for the post-intent pipeline that runs after each priority action.
- See `2026-07-20-choices-actions-and-resolution.md` for how `PendingChoice` interacts with the priority gate.
- See `2026-07-20-combat-and-commander-rules.md` for combat-step specifics (DeclareAttackers, DeclareBlockers, combat damage).
- `CONTEXT.md` defines **meaningful action**, **auto-pass**, **stack yield**, **turn yield**, **End Turn**, and related terms.
