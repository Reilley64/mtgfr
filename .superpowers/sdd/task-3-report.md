# Task 3: Engine mulligan phase report

## Status

Implemented and verified.

## Summary

- Added engine-native pre-game mulligan phase behind `Game::begin_mulligans()`.
- Added `hand_size_after_mulligans(n) = 7 - max(0, n - 1)` via saturating `u8` math.
- Added `Intent::KeepHand`, `Intent::Mulligan`, `Event::MulliganTaken`, `Event::HandKept`, `Event::MulligansFinished`, `MeaningfulAction::KeepHand`, `MeaningfulAction::Mulligan`, and `Reject::Mulliganing`.
- Added `Game::mulliganing()`, `Game::hand_kept()`, `Game::mulligans_taken()`, and public `Game::hand()`.
- Kept `Game::with_players` playable by default with `mulliganing: false`; only `begin_mulligans()` activates the phase.
- While mulliganing, only Keep/Mulligan/Concede or stored TakeAction dispatch are allowed; ordinary intents reject with `Reject::Mulliganing`.
- A mulligan moves the hand back into the library with existing hidden-zone events, shuffles, redraws to size, increments count, and auto-keeps at hand size 1.
- When all living players have kept, the engine emits `MulligansFinished` and begins the first turn.
- Added minimal schema projection/action-view support for exhaustiveness. Proto/client are intentionally not wired; server proto mapping drops mulligan schema events until later tasks.

## TDD evidence

RED:

- `cargo nextest run --profile ci -E 'test(friendly_mulligan) + test(second_mulligan) + test(mulligan_to_one) + test(all_keeps_begin) + test(simultaneous_keeps)'`
- Failed for missing `begin_mulligans`, `Intent::Mulligan`, `Intent::KeepHand`, accessors, and `Event::MulligansFinished`.

GREEN:

- Same focused command: 5 tests run, 5 passed.

Full Rust verification:

- `cargo fmt && just server-check`
- `cargo clippy --all-targets`: clean.
- `cargo nextest run --profile ci`: 2043 tests run, 2043 passed.

## Self-review

- Confirmed no server `seed_game`, proto, or client implementation was added.
- Confirmed unrelated pre-existing modified files are left unstaged.
- Confirmed constructor default remains non-mulligan playable.

## Concerns

- Mulligan events are engine/schema-visible but not proto/client-visible until Tasks 4-7.

## Review fix notes (2026-07-21)

- Fixed mulligan redraw shuffles to emit/apply `Event::LibraryShuffled` instead of mutating the library with a direct `Game::shuffle` call.
- Wired `Intent::Concede` through `finish_mulligans_if_all_kept` so a lost seat no longer blocks the pre-game mulligan phase.
- Kept lost seats non-blocking in the all-kept check and added a guard so the finisher is a no-op outside mulligans.
- Added regressions for event-sourced mulligan shuffle, concede finishing mulligans, and `PassPriority` rejecting with `Reject::Mulliganing`.

Review-fix TDD evidence:

- RED: `cargo nextest run --profile ci -p engine -E 'test(friendly_mulligan_emits_library_shuffled_before_redraw) + test(concede_after_other_player_kept_finishes_mulligans) + test(pass_priority_rejects_while_mulliganing)'` -> 3 run, 1 passed, 2 failed. Failures were missing `LibraryShuffled` and `mulliganing()` staying true after concede.
- GREEN: same command -> 3 run, 3 passed.
- Focused coverage: `cargo nextest run --profile ci -p engine -E 'test(friendly_mulligan) + test(second_mulligan) + test(mulligan_to_one) + test(all_keeps) + test(simultaneous_keeps) + test(concede)'` -> 8 run, 8 passed.
- Engine package on the final tree: `cargo nextest run --profile ci -p engine` -> 1818 run, 1818 passed.
