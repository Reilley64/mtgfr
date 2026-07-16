# 0018 — Effect v4 generated client + SSE stream

Status: **Accepted**; supersedes [0001](0001-rust-to-ts-via-utoipa-openapi-orval.md) (Orval) and NDJSON half of [0005](0005-in-process-fanout-ndjson-snapshot.md).

## Decision

- `client/scripts/gen.sh` emits OpenAPI from `schema` crate → generates `api/generated.ts` (`-f httpclient-type-only`).
- `effect/client.ts` wraps fetch `HttpClient` + generated client; `run = Effect.runPromise`.
- `/stream` is SSE (`text/event-stream`); `streamSse()` typed in generated client. Reconnect wrapper in `effect/stream.ts`.

## Consequences

- `bun run gen` needs Rust toolchain. Generated file gitignored; two `sed` patches in `gen.sh`.
- Pin `effect` to exact beta (matches 0019). utoipa/OpenAPI remains contract source.
