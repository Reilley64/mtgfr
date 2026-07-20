# Choices, Actions, and Resolution

**Status:** Current (as of 2026-07-20)
**Module:** `crates/engine` (`src/types.rs` `PendingChoice`/`LegalAction`, `src/cast.rs`, `src/effects.rs`, `src/resolution.rs`, `src/priority.rs` `settle_payment`, `src/query.rs`, `src/playable.rs`)

---

## Problem Statement

Card effects pause the game for player input — choosing a target, paying a cost, deciding "may I", sacrificing a creature, arranging scried cards. Simultaneously, the client needs to know what actions it can offer without re-implementing rules. Both problems need a clean data contract: the pending choice (engine → client, "what do you need from me?") and the legal-action list (engine → client, "here is every meaningful play you may take right now").

---

## Solution

`Game::pending_choice: Option<PendingChoice>` is a plain data enum — no callbacks, no closures. While a choice is pending, `Game::legal_actions()` returns an empty list and only the matching answer intent from the awaited player is accepted by `submit`. When the choice is resolved, the engine continues executing the interrupted effect body (the `ResumeState` deferred-sequence mechanism).

`Game::legal_actions()` returns a `Vec<LegalAction>`, each carrying a stable `id`, the acting `player`, and a `MeaningfulAction` kind. The client submits `Intent::TakeAction { id, … }` to execute the action; the engine looks up the id and dispatches identically to the equivalent concrete intent. Stable ids survive state changes that do not remove the underlying action.

Payment is settled engine-side: `Game::settle_payment` auto-taps mana sources to cover a spell's or ability's cost, removing the need for the client to plan or sequence taps before casting.

---

## User Stories

1. As a **player**, I want to cast a spell with a single click (no prior tap-for-mana required), with the engine auto-tapping my lands for me.
2. As a **player**, I want to be prompted when a spell or ability needs a target I must choose, so the choice is explicit and not auto-decided.
3. As a **player**, I want to decline optional abilities ("you may…") rather than being forced to accept or having the engine decide for me.
4. As a **player**, I want to pick which creatures to sacrifice when an edict effect fires, unless only one option exists (in which case it auto-resolves).
5. As a **player**, I want to arrange the order of scried or surveiled cards by choosing which go on top and which go to the bottom.
6. As a **player**, I want to search my library for a card (tutor), browse the contents, and pick one (or decline to find).
7. As a **player**, I want triggered abilities from the same controller to be orderable when more than one fires simultaneously.
8. As a **player with a commander**, I want to be asked whether to redirect my commander to the command zone when it would leave the battlefield.
9. As a **player**, I want the engine to auto-resolve a choice with exactly one option (forced action), so trivially forced decisions don't stall the game.
10. As a **player**, I want legal actions filtered to exactly my seat's current options, so I'm never offered an action I can't legally take.
11. As a **client developer**, I want a stable `LegalAction.id` that persists across intents that don't remove the action, so holding an id across a tap-then-cast sequence never produces a stale error.
12. As a **card author**, I want to add a new forced choice by adding a `PendingChoice` variant and a resolver arm, without touching the broader submit path.

---

## Behavior

### PendingChoice variants

While `Game::pending_choice` is `Some`, no `LegalAction` is exposed and only the correct answer intent from the awaited player is accepted. The known choice kinds are:

- **`ChooseTarget { player, source, effect, legal, count, x, activated }`** — choose one or more targets from `legal` for the given effect. `count.min` is 0 for "up to N" choices. With `min == 0` and exactly one legal target, it is still not forced (the player may decline). Forced only when exactly one target AND `min >= 1` (or `min == legal.len()`).
- **`OrderTriggers { player, source, effects }`** — order simultaneous triggered abilities from the same controller. Forced when exactly one effect is in the list.
- **`MayYesNo { player, source, effect }`** — "you may" optional trigger. **Never forced** — declining is always a legal choice.
- **`PayCost { player, source, effect }`** — "you may pay a cost" to get an effect. Never forced.
- **`DiscardToHandSize { player, hand, count }`** — discard `count` cards from `hand`. Forced when `count == hand.len()` (entire hand must go). A partial discard (choose which cards) is never forced.
- **`SacrificeEdict { player, options, keep_one, filter, … }`** — sacrifice one or more permanents matching a filter. Forced when exactly one option AND `keep_one == false`. A `keep_one` edict (keep exactly one, sacrifice the rest) is never forced even with one option (keeping vs. sacrificing is a real decision).
- **`ArrangeTop { player, cards, total }`** — scry / surveil ordering (put some on top in order, rest to bottom). Never forced even for a single card (top vs. bottom is a real choice).
- **`SearchLibrary { player, filter, … }`** — tutor: browse a zone (library or graveyard) and pick a matching card. Fail-to-find is always legal, so this is never forced.
- **`ChooseMode { player, source, … }`** — pick from a modal spell's available modes. Forced when exactly one mode is available.
- **`CommanderRedirect { player, commander, … }`** — redirect a commander to the command zone instead of the graveyard/exile. Not forced (the player may allow it to go to the graveyard).
- **`AssignCombatDamage { player, attacker, blockers, total_damage }`** — assign trample damage among blockers and the defending player, with lethal-to-each-blocker minimums.
- **`ChooseAttachHost { player, attachment, legal }`** — pick a legal host for an Aura being deployed (e.g. from a tutor that puts it directly onto the battlefield).
- **`PayEchoOrSacrifice { player, permanent }`** — Echo (CR 702.31): pay the echo cost or sacrifice the permanent at upkeep.
- **`PayRecoverOrExile { player, card }`** — Recover (CR 702.59): pay the recover cost to return a creature from the graveyard, or exile it.
- **`PayCumulativeUpkeepOrSacrifice { player, permanent, cost }`** — Cumulative Upkeep (CR 702.24): pay all accumulated age-counter costs or sacrifice.
- **`Discard { player, hand, count }`** — a triggered discard effect (distinct from cleanup discard to hand size).
- **`ChooseSacrifice { player, options, filter }`** — a triggered edict that asks the player to pick which permanent(s) they sacrifice voluntarily (for a modal/optional effect).

### Forced action

`Game::forced_action()` returns the single auto-submittable intent when a pending choice has exactly one legal answer and it is unambiguously the only option. Conservative: "may" choices and ArrangeTop are never forced. The server submits forced choices automatically and tags the resulting log events as `AUTO`.

### LegalAction and TakeAction

- `Game::refresh_actions()` rebuilds `Game::actions` from `Game::meaningful_actions(player)` for every living seat after every state change.
- While `pending_choice` is `Some`, `refresh_actions` produces an empty list.
- Each `LegalAction` carries: `id: u64` (stable monotonic), `player: PlayerId`, `kind: MeaningfulAction`.
- An action whose `(player, kind)` pair survives a state change keeps its id. A genuinely new action mints a fresh monotonic id. Dead ids are never recycled.
- `Intent::TakeAction { id, target, x, modes, sacrifice, discard_cost, graveyard_exile, attackers, blocks }` looks up `id` in `Game::actions`, checks `player` matches, and dispatches to the same private handler the equivalent concrete intent would invoke.
- `MeaningfulAction` kinds: `PlayLand`, `Cast`, `Activate`, `Cycle`, `ActivateHandAbility`, `Suspend`, `Encore`, `TurnFaceUp`, `CastPrepared`, `CastFaceDown`, `DeclareAttackers`, `DeclareBlockers`.

### Payment and auto-tap

`Game::settle_payment(player, cost)` is the single payment path for all casts, activations, cycling, and pay-cost choices (choices-actions-and-resolution spec):

1. **Verify affordability**: check that the player's mana pool plus the output of all available free-tap sources covers the cost.
2. **Auto-tap free sources**: tap untapped lands and free-tap mana abilities (Sol Ring, Arcane Signet, Llanowar Elves) in order — lands before non-lands, non-pain before pain sources, broader-color sources preferred (mana breadth heuristic) — to cover the shortfall against the pool.
3. **Auto-tap paid mana abilities**: filter lands (Fetid Heath) and karoos/signets with a generic activation cost are planned feed-first so that the nested `activate_ability → settle_payment` loop only spends mana, never generating more recursion.
4. Mana is deducted from the pool; tap events are emitted in the same delta as the cast.

The client does not plan or sequence payments; the cast is a single intent. Manual `Intent::TapForMana` for free-floating mana remains available. Paid mana abilities (those with a generic cost) appear as `Activate` actions in the action list (and on the activation radial in the UI) — they are never auto-tapped.

Net-zero converters (Study Hall — tap, pay {1}, get {1} of any color) are excluded from the planner to avoid infinite loops; they stay on the manual radial.

### Effect resolution and resume

When an effect body needs player input mid-resolution, it calls `pending::raise(choice)` which:
1. Sets `Game::pending_choice = Some(choice)`.
2. Returns control to the caller; the effect body is not re-entered.

Later, when the player answers, `pending::answer(game, intent)` resolves the choice and stores the answer in the `ResumeState`. `Game::resume_deferred_sequence` is then called at the tail of `submit_inner` to drain any deferred effect steps that were parked while the choice was pending. This allows a single effect body (`Effect::Sequence`, `Effect::Clash`, `Effect::Demonstrate`, etc.) to have multiple pause points without nesting callbacks.

### Cast flow (high level)

1. Client submits `Intent::TakeAction { id: cast_id, target, x, … }` (or legacy `Intent::Cast`).
2. `Game::cast_with_kind` validates: timing (sorcery/instant speed), zone (hand/command/graveyard/exile/etc.), affordability, modal mode count, target legality.
3. If targeting, `PendingChoice::ChooseTarget` is raised (for multi-target spells or when the client didn't supply a target in the one-click path).
4. `Game::settle_payment` auto-taps sources and deducts cost.
5. `Event::SpellCast` is emitted; the card moves from hand to stack as `Object::Spell`.
6. Cast triggers fire (`CastSpell` triggers, magecraft, etc.) at the next priority window (via `enqueue_triggers`).
7. When the stack resolves (all players pass), `Game::resolve_top` calls `Game::resolve_spell` for the top spell.
8. Permanents enter the battlefield (`Event::PermanentEntered`); instants/sorceries run their effects via `Game::run` and move to the graveyard.

---

## Implementation Decisions

- **`PendingChoice` is plain data, not a trait object or callback.** New choice kinds = new variant + one arm in `pending::answer`. The serializable shape is critical for the wire contract (VisibleState carries the pending choice for the client).
- **Stable action ids (lobby-table-routing-and-live-game spec) enable tap-then-cast.** A client can tap a land (changing mana state but not removing the cast action), then submit `TakeAction { id: cast_id }` with the id it fetched before the tap. The id survives because the action's `(player, kind)` pair is unchanged.
- **`CastPlayKind` separates listing from execution.** The `List` path checks timing, zone, affordability, and target availability without needing chosen inputs; it drives `meaningful_actions` and auto-pass. The `OneClick` and `Full` paths additionally validate chosen discard picks, graveyard-exile selections, and other cost components.
- **`ResumeState` is not event-sourced.** The deferred-sequence resume stack (`Vec<ResumeFrame>`) is transient orchestration state on `Game`, consistent with how `pending_choice` is handled. Games are in-memory only (lobby-table-routing-and-live-game spec), so there is no replay concern.
- **`forced_action` is conservative by design.** It errs on the side of not forcing rather than accidentally making a choice for the player. A real decision must never be auto-submitted.
- **Payment planner preference order**: free-tap lands before non-land free-tap sources; non-pain sources before pain sources; higher-breadth (more color-versatile) sources preferred. This is a heuristic: the planner aims to minimize color waste without guaranteeing an optimal plan (optimization is bounded and practically good for the pool's cards).

---

## Testing Decisions

- **`forced_action` is unit-testable** without a board: construct `PendingChoice` directly on a bare `Game`, assert `forced_action()` returns the correct `Some`/`None`. See the in-module tests in `lib.rs`.
- **Choice resume tests**: construct a board with a choice-raising effect (e.g. Scry), submit the triggering action, assert `pending_choice` is set, then submit the answer intent and assert the game continues correctly.
- **Payment tests**: give a player specific untapped lands, verify `settle_payment` taps the correct sources and the correct mana is deducted. Test pain-land preference (non-pain lands tapped first).
- **Stable-id tests**: construct a board, call `refresh_actions`, record an id, mutate state in a way that doesn't remove the action, call `refresh_actions` again, assert the id is unchanged. See in-module tests in `lib.rs`.
- **`TakeAction` dispatch test**: verify that submitting `TakeAction { id }` produces the same events as the equivalent concrete intent for each `MeaningfulAction` kind.
- **Multi-choice sequence test**: effect with two pause points (e.g. a Clash followed by a Scry) — answer each in order and verify the deferred sequence drains correctly.

---

## Out of Scope

- **Triggered ability stacking choices for multiple controllers** (full APNAP ordering with per-controller `OrderTriggers` for each controller's simultaneous triggers) — the engine raises `OrderTriggers` for a single controller's group; cross-controller ordering is correct via the APNAP placement loop but does not prompt for inter-controller ordering (CR 603.3b deliberate simplification).
- **Wish effects** (retrieving cards from outside the game) — not implemented; the pool does not include cards that rely on a sideboard.
- **Split-second** (CR 702.61) — not implemented; no pool card currently requires it.
- **Splice onto Arcane** (CR 702.46) — not implemented.
- **General damage prevention choices** (choice to redirect or prevent damage before it is dealt, CR 615 full framework) — partial implementations exist for specific cards; the general replacement-effect engine is backlog.

---

## Further Notes

- See `2026-07-20-engine-core-and-event-model.md` for the `submit` / `apply` / pipeline path that wraps all choices and actions.
- See `2026-07-20-turn-priority-and-stack.md` for priority mechanics that gate when actions and choices are legal.
- See `2026-07-20-card-dsl-and-card-pool.md` for how `Effect` variants map to choice kinds.
- `CONTEXT.md` defines **choice / pending choice**, **intent**, **legal targets**, **forced-action**, **auto-tap**, and related terms.
- Per-deck `docs/fidelity/<slug>-increments.md` files track engine gaps surfaced by cards in that grind.
