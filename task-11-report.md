## Task 11: Alt inspect with focused buttons

- Fixed board keyboard shortcut filtering so Alt and Escape still reach the board when focus is on a button.
- Kept Space and Enter guarded for button targets, and kept all board shortcuts guarded for input, textarea, and select targets.
- Added focused regression coverage in `client/app/board/html/keyboard-mount.test.ts`.
- Verified with `bunx vitest run app/board/html/keyboard-mount.test.ts app/board/inspect-pile-concede.test.ts`:
  - 2 files passed
  - 28 tests passed
