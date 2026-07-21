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
