# Final fix report

## 2026-07-21 mulligan/beacon review fixes

- Fixed server wire mapping so mulligan lifecycle `VisibleEvent`s are omitted from delta event lists instead of serialized as unset protobuf oneofs.
- Added regression coverage for a delta containing only `MulliganTaken`, `HandKept`, and `MulligansFinished`; it now emits zero protobuf events.
- Added client coverage for applying a snapshot-sourced mulligan delta with no visible events.
- Added client mulligan chrome coverage for `can_mulligan: false` while Keep remains available.
- Updated stale `Game::next_u64` Rustdoc references to `Game::with_op_rng` / `crate::rng::OpRng`.

### TDD evidence

- RED: `cargo nextest run --profile ci -p server delta_omits_mulligan_lifecycle_events_from_wire` failed with `assertion failed: delta.events.is_empty()` before the mapper fix.
- GREEN: `cargo nextest run --profile ci -p server delta_omits_mulligan_lifecycle_events_from_wire` passed after switching the mapper to `Option<pb::VisibleEvent>` and `filter_map`.

### Verification

- `git diff --check` passed.
- `cargo nextest run --profile ci -p server` passed: 155 tests.
- `bun test src/store.test.ts src/lib/mulligan.test.ts` passed: 47 tests.

### Residual risks

- No full browser/live-game verification was run; coverage here is server wire mapping plus client delta/store unit coverage.
- Full proto variants for mulligan lifecycle events remain intentionally out of scope for this fix.
