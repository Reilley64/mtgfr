# 0018 — Effect v4 generated client + SSE stream

Status: **Superseded** by [0032](0032-effect-rpc-and-grpc-proto-wire.md).

## Decision (historical)

- `client/scripts/gen.sh` emits OpenAPI from `schema` crate → generates `api/generated.ts` (`-f httpclient-type-only`).
- `effect/client.ts` wraps fetch `HttpClient` + generated client; `run = Effect.runPromise`.
- `/stream` is SSE (`text/event-stream`); `streamSse()` typed in generated client. Reconnect wrapper in `effect/stream.ts`.

## Consequences (historical)

- `bun run gen` / `just server-codegen` needs Rust toolchain. Generated `openapi.json` + `client/src/api/generated.ts` are gitignored; CI and `just check` regenerate them. Two `sed` patches in `gen.sh`. Biome excludes the generated client.
- Pin `effect` to exact beta (matches 0019). utoipa/OpenAPI remains contract source.
