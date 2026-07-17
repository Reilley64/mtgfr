# 0032 — Effect RPC + gRPC (proto-owned wire)

Status: **Accepted**; supersedes [0018](0018-effect-generated-client-and-sse-stream.md) (OpenAPI Effect client + SSE). Extends [0030](0030-table-instance-affinity-for-drain-rolls.md) (BFF routing unchanged; transport to API pods becomes gRPC).

## Context

The browser talked same-origin `/api` REST + SSE through a SolidStart BFF that mostly `proxyRequest`ed to Axum. Wire types were Rust `schema` → utoipa OpenAPI → Effect `HttpClient`. That worked, but split Effect ergonomics (atoms/`Stream`) from an HTTP-shaped contract and kept cookie pass-through to every API pod.

## Decision

- **`.proto` is the sole wire contract.** Rust uses prost/tonic; TypeScript uses Effect Schema + Rpc groups generated from the same protos.
- **Browser → BFF:** `@effect/rpc` over HTTP/JSON (embedded in SolidStart Vinxi; no second port).
- **BFF → API:** `@effect-grpc/effect-grpc` (Connect native-gRPC transport + Effect Rpc clients generated from `.proto`) → **tonic** on the API pod (`:50051`).
- **Health stays HTTP:** Axum serves only `/health/live|ready|drain` on `:8080`. BFF keeps `meta/health` and `meta/version`.
- **Lobby stays BFF-local** (Drizzle/`mtgfr_web`) but is exposed on the same Effect Rpc surface; `Tables.Seed` remains a tonic RPC the BFF calls at start.
- **Cookies terminate at the BFF.** Session token goes to tonic as gRPC metadata (`x-session-token`).
- **Hard cutover** for this migration: ship API + web together. No REST/SSE dual-serve; in-flight tables may drop. After cutover, ADR 0030 drain rolls continue with **gRPC-only** expand-only proto rules ([WIRE_COMPAT.md](../WIRE_COMPAT.md)).

## Consequences

- Drop utoipa OpenAPI emit, `openapi.json`, `client/scripts/gen.sh` OpenAPI path, and SSE `/stream`.
- Headless/`edh-api` Services expose gRPC port `50051`; BFF dials `{pod_dns}:50051` for game affinity.
- **All tonic payloads are native protobuf** (`proto/mtgfr/v1/{common,catalog,intent,stream,mtgfr}.proto`) — no JSON-in-string wrappers. `crates/schema` remains the projection model; `crates/server/src/grpc/map` converts at the edge. Browser JSON shapes stay in `client/src/wire/types.ts` (BFF maps via `protoMap.ts`).
- Pin Effect betas together with `@effect-grpc/effect-grpc` / Buf codegen tooling (`bun run gen` → gitignored `client/src/wire/generated`).
