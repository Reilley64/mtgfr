# Final fix pass report

## 2026-07-26

- Fixed the `/play/:segment` collision risk by making `randomTableCode()` redraw until the minted six-character table code contains at least one letter.
- Added regression coverage for redraw-on-all-digit generation and for route normalization of mixed-code versus all-digit six-character `/play/...` segments.
- Updated living surface specs to point moved shared client modules at `client/app/domain/**` and aligned route/table-code wording with the current six-character mixed-alphanumeric generator contract.
- Verified with focused Vitest runs for `app/domain/lobby-store.test.ts` and `app/routes.test.ts`, then `just client-check` (format, lint, typecheck, full client test suite).
