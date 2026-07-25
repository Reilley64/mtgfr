# Task 7 report: Catalog coverage sync + feature specs

## Summary

- Added `client/lib/i18n/rustKeys.json` synced from `MessageKey::all()` (268 keys).
- Added `client/lib/i18n/catalogCoverage.test.ts`.
- Added an engine drift test that compares `MessageKey::all()` to the committed Rust-key fixture.
- Confirmed `enCatalog` covers every Rust-emitted key.
- Updated living MessageRef feature specs and marked the MessageRef wire/i18n design spec `Done`.
- Indexed the completed design spec in `docs/superpowers/specs/README.md`.

## TDD / focused checks

- RED: `cd client && bun run test lib/i18n/catalogCoverage.test.ts` failed on placeholder `__missing__`.
- GREEN: `cd client && bun run test lib/i18n/catalogCoverage.test.ts` passed.
- GREEN: `cargo test -p engine rust_keys_fixture_matches_message_key_all` passed.

## Full verify

- `just server-codegen` passed.
- `cargo fmt` ran.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo nextest run --profile ci -p engine -p schema -p server` passed: 2180 passed.
- `cd client && bun run check` is not defined in `client/package.json`; used the brief fallback.
- `just client-check` passed after applying Biome safe import fixes and regenerating token outputs:
  - typecheck passed.
  - client tests passed: 92 files, 955 tests.

## Notes

- Left pre-existing unrelated dirty files unstaged: `.superpowers/sdd/task-2-report.md` and `client/lib/deck-builder/scryfall.ts`.
- `just client-check` required safe Biome import-order fixes in existing MessageRef client tests.
