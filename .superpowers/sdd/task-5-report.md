# Task 5 report: Shell spec + verification

## Changes

- Updated `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md` so `/` describes the implemented deck tile grid, search, owned-first/precon ordering, Play links, and owned-only context menu.
- Added the returning-player deck-list user story to the shell spec.
- Marked `docs/superpowers/specs/2026-07-24-deck-list-tile-chooser-design.md` as implemented.
- Fixed two existing Scene tests to resolve the deck-list context-menu mounts that now render with the home deck-list surface.

## Verification

### `cd client && bun run typecheck && bunx vitest run`

Exit code: 0

```text
$ bun run gen
$ bun run gen:wire && bun run gen:tokens
$ PATH="$PWD/node_modules/.bin:$PATH" bunx --bun buf generate --template ../proto/buf.gen.yaml ../proto
$ node scripts/gen-tokens.mjs

css
✔︎ /tmp/mtgfr-tokens-00K5xe/tokens.generated.css

ts
✔︎ /tmp/mtgfr-tokens-00K5xe/design-tokens.generated.ts
wrote /workspace/client/styles/tokens.generated.css
wrote /workspace/client/lib/design-tokens.generated.ts
$ tsc --noEmit

 RUN  v4.1.10 /workspace/client


 Test Files  83 passed (83)
      Tests  865 passed (865)
   Start at  01:00:09
   Duration  10.22s (transform 2.73s, setup 0ms, import 18.67s, tests 1.64s, environment 1.93s)
```

### `cd client && bun run lint`

Exit code: 0

```text
$ biome check --formatter-enabled=false
biome.json:2:14 deserialize

  i The configuration schema version does not match the CLI version 2.5.5

Checked 253 files in 184ms. No fixes applied.
Found 1 info.
```

No Biome import-combine failure appeared for `client/app/shell/decks/list/visible.ts`.
