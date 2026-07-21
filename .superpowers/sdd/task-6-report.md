# Task 6 Report: Cloudflare drand beacon at seed

## Status

Implemented.

## Changes

- Added `crates/server/src/beacon.rs` with `MasterEntropy`, `EntropySource`, env seed parsing, drand latest fetch from Cloudflare with api.drand.sh fallback, and retry handling.
- Changed `seed_game` to accept `[u8; 32]` and call `Game::with_master_seed`.
- Added `master_from_u64(seed: u64) -> [u8; 32]` for server tests.
- Replaced lobby `OsRng.next_u64()` seeding with beacon entropy.
- Persisted `Table.seed: [u8; 32]` and `Table.beacon_round: u64`.
- Added direct `reqwest` server dependency with `json` and `rustls-tls`.
- Added `ponytail:` note for skipping BLS signature verification in v1.

## TDD evidence

- Red: `cargo nextest run --profile ci -p server env_master_seed_skips_network beacon_failure_rejects_seed_with_503`
  - `env_master_seed_skips_network` failed because env seed still called the source.
  - `beacon_failure_rejects_seed_with_503` failed because the seed path still created a table.
- Green: same focused command passed after implementing resolver and lobby wiring.

## Verification

- `cargo nextest run --profile ci -p server`: 149 passed, 0 failed.
- `cargo clippy -p server --all-targets -- -D warnings`: passed.

## Concerns

- BLS signature verification is intentionally skipped in v1 per task scope; `beacon_round` is stored for audit.
