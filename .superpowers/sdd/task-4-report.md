# Task 4 report

## Status

Complete.

## Scope shipped

- Added `client/app/domain/card-art/proxy-url.ts` with:
  - `proxiedCardArtUrl(remoteUrl)`
  - `resolveCardFaceUrls({ print, proxyArtUrl, size, face })`
- Extended `client/app/domain/ui/card-art.ts` so `cardArt` accepts optional
  `proxyArtUrl` and emits `data-art-url` / `data-art-fallback` from the shared
  resolver.
- Updated browser wire DTOs in `client/app/domain/wire/types.ts` for
  `proxy_art_url` / `commander_proxy_art_url` fields already present in generated
  proto output.
- Updated `docs/superpowers/specs/2026-07-20-deck-list-and-builder.md` so the
  shared card-art helper contract matches shipped behavior.

## Explicitly not shipped

- No board paint / HTML call-site wiring beyond `cardArt` itself.
- No Nitro BFF proxy route implementation.

## TDD evidence

### Red

`bun run test -- app/domain/card-art/proxy-url.test.ts app/domain/ui/card-art.test.ts`

- failed because `./proxy-url` did not exist
- failed because `cardArt` still emitted the print URL instead of the proxy URL

### Green

`bun run test -- app/domain/card-art/proxy-url.test.ts app/domain/ui/card-art.test.ts`

- passed: 2 files, 11 tests

## Verification

- `bun run typecheck`

## Notes

- I used `bun run test` instead of native `bun test` because these suites rely on
  Vitest + `@vitest-environment happy-dom`; native Bun test runner does not honor
  that file pragma in this repo.
