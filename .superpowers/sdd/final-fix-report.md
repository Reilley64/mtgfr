# Final fix report

## 2026-07-26

- Finding 1 red phase: `cargo nextest run --profile ci seed_table_prefers_non_empty_commander_proxy_art_when_commander_also_exists_in_the_99` failed because the live seat proxy-art map kept the deck-line URL (`tajic-line-proxy.png`) instead of the commander URL (`tajic-commander-proxy.png`).
- Finding 1 green phase: `crates/server/src/lobby.rs` now writes deck-line `proxy_art_url` values first and applies a non-empty `commander_proxy_art_url` last, so the live per-seat `card_id` overlay keeps the commander URL when both identities match.
- Finding 2 red phase: `cd client && bun run test -- app/domain/card-art/proxy-fetch.test.ts` failed in `fails within the timeout when dns lookup never resolves` because the unresolved DNS promise escaped the fetch timeout and the test's fallback timer won.
- Finding 2 green phase: `client/app/domain/card-art/proxy-fetch.ts` now races DNS lookup against the same `AbortSignal`, so stalled resolution returns `{ ok: false, status: 502 }` inside the configured timeout and never reaches `fetchImpl`.

## Verification

- `cargo nextest run --profile ci seed_table_prefers_non_empty_commander_proxy_art_when_commander_also_exists_in_the_99` — pass
- `cd client && bun run test -- app/domain/card-art/proxy-fetch.test.ts` — pass (`9` tests)
- `cd client && bun run typecheck` — pass

## Spec updates

- `docs/superpowers/specs/2026-07-20-accounts-decks-and-catalog.md`
- `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md`
# Final fix pass report

## 2026-07-26

- Fixed the `/play/:segment` collision risk by making `randomTableCode()` redraw until the minted six-character table code contains at least one letter.
- Added regression coverage for redraw-on-all-digit generation and for route normalization of mixed-code versus all-digit six-character `/play/...` segments.
- Updated living surface specs to point moved shared client modules at `client/app/domain/**` and aligned route/table-code wording with the current six-character mixed-alphanumeric generator contract.
- Verified with focused Vitest runs for `app/domain/lobby-store.test.ts` and `app/routes.test.ts`, then `just client-check` (format, lint, typecheck, full client test suite).
