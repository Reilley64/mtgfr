# Task 8 report: Docs + verification

## Status

Completed.

## Docs updated

- `docs/superpowers/specs/2026-07-20-engine-core-and-event-model.md`
  - Records `[u8; 32]` master seed construction.
  - Records derive-per-op RNG via `BLAKE3(master_seed || player || op_iteration)`.
  - Records simultaneous pre-game mulligans, friendly mulligan sizing, auto-keep at 1, and first-turn start after all keeps.
- `docs/superpowers/specs/2026-07-20-lobby-table-routing-and-live-game.md`
  - Records Cloudflare/drand seed fetch, fallback relay, fixed dev/test seeds, persisted `seed` + `beacon_round`, and 503 behavior when beacon entropy is unavailable.
  - Records that `seed_game` deals opening hands, enters mulligans, and delays `begin_first_turn()` until all living seats keep.
- `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md`
  - Records `KeepHand` / `Mulligan` intents.
  - Records `VisibleState.mulliganing` and per-player `mulligans_taken`, `hand_kept`, `can_mulligan`.
  - Records that mulligan UI must use snapshot/delta state fields because the current protobuf `VisibleEvent` oneof lacks mulligan event arms.
- `docs/superpowers/specs/2026-07-21-mulligan-and-beacon-seed-design.md`
  - Marks the design implemented on `cursor/mulligan-and-beacon-seed-design-1e1a`.

## Verification

- `just server-check`: PASS.
  - Ran `cargo fmt`, `cargo clippy --all-targets`, Toasty migrations, and `just server-test`.
  - Nextest summary: 2059 tests run, 2059 passed, 0 skipped.
- `just client-check`: PASS.
  - Ran codegen, format, lint, typecheck, and Vitest.
  - Biome reported warnings in existing generated/client files but recipe exit code was 0.
  - TypeScript `tsc --noEmit`: PASS.
  - Vitest summary: 74 files passed, 706 tests passed.

## Concerns

- `crates/server/src/grpc/map/stream.rs` still maps `VisibleEvent::MulliganTaken`, `VisibleEvent::HandKept`, and `VisibleEvent::MulligansFinished` to an empty protobuf `VisibleEvent` because `stream.proto` has no mulligan event variants. This is now documented; snapshot/delta state fields are the source of truth for mulligan UI.
- `just server-check` / `just client-check` rewrote unrelated source/generated files through formatter/codegen. The worktree was clean before verification, so those side effects were restored and only the requested docs/report changes remain.
