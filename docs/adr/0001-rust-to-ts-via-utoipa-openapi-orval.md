# 0001 — Rust→TypeScript wire types via utoipa → OpenAPI → Orval

Status: **Superseded** by [0018](0018-effect-generated-client-and-sse-stream.md) (Orval), then [0032](0032-effect-rpc-and-grpc-proto-wire.md) (OpenAPI/utoipa dropped; `.proto` is the wire contract).

## Decision

- Wire DTOs live in `crates/schema` with utoipa `ToSchema`; server serves `openapi.json`.
- Rust is the authored source of truth for the contract.

## Consequences

- Run `just server-codegen` after wire-type changes; client generates from the spec at build time (0018).
