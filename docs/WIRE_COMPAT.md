# Wire backwards compatibility

Durable rules for the proto / gRPC wire contract during a rolling deploy. See
[ADR 0030](adr/0030-table-instance-affinity-for-drain-rolls.md),
[ADR 0032](adr/0032-effect-rpc-and-grpc-proto-wire.md), and
[ADR 0021](adr/0021-live-games-in-memory-only.md).

## Why this exists

Rolling deploy keeps **outgoing** API pods Terminating (SIGTERM drain) while **newest** accepts
new tables via Service `edh-api`. The SolidStart SPA may roll with newest; mid-game clients still
talk to older pods via BFF `table_routes` → pod DNS on the headless Service.

So every concurrent instance version must speak a wire protocol the current SPA/BFF can parse —
**expand-only** across the whole set until grace expires / pods exit.

## Transport migration (ADR 0032)

The OpenAPI/REST/SSE → Effect RPC + gRPC cutover is a **hard cut**: API and web ship together.
No N/N−1 coexistence between REST and gRPC is required for that release. In-flight tables may
drop. After that cut, the rules below apply to **gRPC/proto only**.

## 1. Compatibility window

All concurrent API binaries until each Terminating pod exits (tables empty or
`terminationGracePeriodSeconds`). No ConfigMap peer registry.

## 2. Expand-only during that window

Within one release's changes to `proto/` (including `common` / `catalog` / `intent` / `stream`)
and the generated Rust/TS bindings:

- **Additive optional fields only.** New protobuf fields use new field numbers; never reuse.
- **New RPCs / Intent / Event / PendingChoice variants** are safe to add — old peers never send
  them. New `oneof` arms need new field numbers inside the parent message.
- **Do not rename, remove, or repurpose** field numbers while any older binary may still serve a
  table the current SPA still reaches via `table_routes`.
- There is no JSON-in-proto escape hatch: game stream frames, intents, decks, cards, and seed
  are all native messages. Expand those trees the same way as any other proto message.

## 3. Lobby vs game

Lobby Effect RPC is owned by the BFF (`mtgfr_web`). Game stream/intent RPCs stay on tonic; the
BFF dials `{pod_dns}:50051` from `table_routes`.
