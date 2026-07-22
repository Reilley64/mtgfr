# Task 7 report — Activate "That ability isn't available"

**Status:** Complete.

**Branch:** `cursor/foldkit-migration-design-1ef0`

## Classification

Engine false-listed a non-mana activated ability as legal, then rejected the same stored `take_action` during payment as `CannotActivate`.

Not classified as:

- `UnknownAction`: current `intentEnvelopeToProto` emits `take_action.id` as `bigint` (`42n` in direct runtime inspection).
- Client-local cost rejection: radial non-mana activate emits a `SubmitIntent` with `kind: "take_action"` and the live `ActionView.id`; no pre-tap path was found.
- Missing sacrifice prompt: absent `sacrifice_choices` stays absent and `planCostPipeline` runs the action.

## Root cause

`Game::meaningful_actions` listed non-mana activated abilities using `available_mana` plus `affordable_from`. For a `{1}, {T}` non-mana ability on a land with its own `{T}: Add {C}` (for example `Arcane Lighthouse`), `available_mana` included the source's own `{C}`.

`Game::activate_ability` correctly settles the actual payment with `exclude = cost.taps_self.then_some(object)`, so the source cannot both tap for mana and pay the activation's tap cost. The action list and activation settlement disagreed; the radial showed the action, `take_action` used the stored action id, and settlement returned `CannotActivate`.

## Fix

`push_activatable_abilities` now checks activation affordability through `plan_auto_taps` with the same source exclusion and activation payment characteristics used by `activate_ability`. The client still sends one `take_action` and does not pre-tap.

## RED

Added `legal_actions_does_not_list_tap_cost_activate_paid_by_its_own_mana`.

Initial run:

- `cargo nextest run --profile ci legal_actions_does_not_list_tap_cost_activate_paid_by_its_own_mana`
- Failed as expected: `Arcane Lighthouse` was listed even though only its own `{T}: {C}` could pay its `{1}, {T}` non-mana ability.

## GREEN

Final targeted verification:

- `cargo nextest run --profile ci legal_actions_does_not_list_tap_cost_activate_paid_by_its_own_mana legal_actions_does_not_list_study_hall_paid_from_its_own_free_c take_action_activate_pays_a_filter_land_from_the_pool`
  - 3 passed.
- `cd client && bunx vitest run app/board/action/execution.test.ts app/board/scene.test.ts`
  - 2 files passed, 47 tests passed.

## Concerns

- No live MCP/browser capture was performed in this subagent run. The classification is from code tracing plus direct runtime inspection of the client proto/radial paths and the RED/GREEN engine regression.
- Full `cargo fmt` wanted to reformat unrelated existing files; I formatted only touched files with `rustfmt --edition 2024`. The tracked diff is the activation listing fix, its regression, and one clippy cleanup required by `-D warnings`.

## Follow-up review fix — Foldkit radial cost pipeline

**Status:** Complete.

**Finding:** The original engine fix is valid and was kept, but the Foldkit radial activate path still had a systematic client bug: `commitRadialIndex` called `runAction` directly for non-mana radial actions, bypassing `planCostPipeline`. A legal Viscera Seer-style activate with `sacrifice_choices: [id]` therefore submitted `take_action` with `sacrifice: []` instead of opening the sacrifice picker, producing the same user-facing reject class.

**Wire audit:** `fromProtoWire` still coerces proto `bigint` action ids to browser `number`; `buildTakeActionIntent` carries that number; `intentEnvelopeToProto`/`takeActionValueToProto` coerces only `take_action.id` back to `bigint`; `create(IntentEnvelopePbSchema)` preserves `91n` and does not zero the id. Working cast and activate paths both emit `take_action`; the broken difference was radial activate skipping the cost planner.

**RED:**

- `bunx vitest run app/board/scene.test.ts -t "RadialOptionPicked opens sacrifice picker"`
  - Failed as expected: `next.sacrificePick?.action` was `undefined`, proving radial submitted or cleared without staging the payable sacrifice choice.

**GREEN:**

- `bunx vitest run app/board/scene.test.ts app/board/action/execution.test.ts lib/wire/protoMap.test.ts`
  - 3 files passed, 51 tests passed.
- `cargo nextest run --profile ci take_action_activates_viscera_seer_with_a_creature_sacrifice legal_actions_does_not_list_tap_cost_activate_paid_by_its_own_mana legal_actions_does_not_list_study_hall_paid_from_its_own_free_c take_action_activate_pays_a_filter_land_from_the_pool`
  - 4 tests passed.
- `bun run typecheck`
  - passed.
- `bun run lint`
  - passed.
- `rustfmt --edition 2024 --check crates/engine/tests/game.rs`
  - passed for the touched Rust test file.

**Remaining concern:** No live MCP/browser capture was available in this subagent run. A live capture should still confirm the original "all abilities" report against an actual table by recording the listed `ActionView`, the emitted `SubmitIntent.args.intent`, and the proto `takeAction.id`/cost fields for mana dorks, Viscera Seer with fodder, and a no-cost non-mana activate.
