# Task 5 Report: Primitive -> semantic tiers + color aliases

## Status

Complete.

## Changes

- Moved public token groups under `semantic`: `font`, `text`, `radius`, `spacing`, and `size`.
- Added `primitive.space` and made public spacing tokens semantic aliases, including `shell-gutter` -> `xxl`.
- Moved semantic color hex literals into `primitive.color`.
- Converted public semantic colors to aliases, with `playable-border` -> `semantic.color.snow-mint`.
- Regenerated `client/styles/tokens.generated.css`.
- Strengthened the focused token contract tests for primitive/semantic shape and aliases.

## Verification

- `cd /workspace/client && bun run gen:tokens:check` -> pass; generated outputs up to date.
- `cd /workspace/client && bunx vitest run app/domain/design-tokens.test.ts` -> pass; 7 tests.
- `cd /workspace && git diff --check` -> pass.
- CSS name spot-check: `--color-forest-floor` is present and no primitive or `--semantic-color-*` export appeared in `client/styles/tokens.generated.css`.

## Concerns

- Hex values remain in `primitive.color` by design for Task 6 OKLCH conversion.
