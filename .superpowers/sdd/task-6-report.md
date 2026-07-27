# Task 6 report: BFF HTTP attrs -> 1.37 + status code

## Status

Done.

## Changes

- Replaced BFF HTTP span annotations in `client/server/lobby-http.ts` with `httpServerAttrs`.
- Replaced `/api/rpc` HTTP span annotations in `client/server/routes/api/rpc/[...path].ts` with `httpServerAttrs`.
- Removed HTTP-span `rpc.path` emission from the BFF gateway path.
- Annotated `http.response.status_code` when response status is available without reading bodies:
  - lobby auth/meta responses
  - `/api/rpc` JSON and empty outcomes
- Added regression coverage for legacy key removal and semconv output shape.
- Added a tiny `vi.hoisted` fallback so the brief's exact `bun test ...` command works for `lobby-http.test.ts`.

## Verification

- Red: `bun run test app/domain/otel/semconv.test.ts server/lobby-http.test.ts` failed on the new source migration assertion before implementation.
- Green: `bun test app/domain/otel/semconv.test.ts server/lobby-http.test.ts` -> 19 pass.
- Green: `bun run lint`.
- Green: `bun run typecheck`.
- Green: `bun run test app/domain/otel/semconv.test.ts server/lobby-http.test.ts` -> 2 files passed, 19 tests passed.
- Green: no matches for `"http\.method"|rpc\.path` in `client/server/lobby-http.ts`.
- Green: no matches for `"http\.method"|rpc\.path` in `client/server/routes/api/rpc`.

## Concerns

- Stream RPC outcomes do not annotate a status at dispatch time because connect-time stream failures are only known when `streamResponse` pulls the first frame.
- The worktree contains unrelated pre-existing or formatter-touched dirty files; they were left unstaged.
