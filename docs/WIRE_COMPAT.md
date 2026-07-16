# Wire backwards compatibility

Durable rules for the OpenAPI / `crates/schema` wire contract during a rolling deploy. Deploy PRD
§Wire backwards compatibility is the source requirement; this doc is the implementation
deliverable it calls for. See also [ADR 0030](adr/0030-table-instance-affinity-for-drain-rolls.md)
(why old SPA ↔ new API coexistence is unavoidable during a drain window) and
[ADR 0021](adr/0021-live-games-in-memory-only.md) (why games can't just restart on the new
binary).

## Why this exists

The rolling deploy model (deploy PRD §Rolling deployment model) keeps **zero or more** outgoing
versioned API instances alive (draining), serving their existing tables, while one **active**
instance takes all new traffic. `edh-web` stays on the previous release until **all** drain peers
report empty and are GC'd. During that window:

- **New** tables are created against the **active** API by the **held** SPA.
- **Old** tables keep talking to their versioned API via the sticky `mtgfr-instance` cookie
  ([ADR 0030](adr/0030-table-instance-affinity-for-drain-rolls.md)), routed by the SolidStart BFF.

So every concurrent instance version must speak a wire protocol the held SPA and peer versions can
parse — expand-only across the whole set until GC. Nested rolls widen that set until peers empty.

## 1. Compatibility window

All concurrent instance versions must coexist until each drain peer reports `active_tables: 0`
and is removed from `api_instances`. No longer-lived multi-version support is required beyond that
set — once a peer is torn down, only remaining instances' contracts need to work.

## 2. Expand-only during that window

Within one release's changes to `crates/schema`, `crates/server`, and `openapi.json`:

- **Additive optional fields only.** New fields on request/response DTOs, `Intent` variants, or
  `Event` variants get `#[serde(default)]` (or `Option<T>`) so an old client that never sends them,
  and an old server that has never heard of them, both still deserialize successfully. The
  codebase already leans on this pattern heavily — see the `#[serde(default)]` fields throughout
  `crates/schema/src/intent.rs` and `crates/schema/src/event.rs`.
- **New endpoints, new `Intent`/`Event` variants** are always safe to add — an old peer simply
  never sends/produces them.
- **No rename, no remove, no type change** of an existing wire field, path, `Intent` variant, or
  `Event` variant until the prior API is fully gone (drain empty, old Deployment deleted). This
  mirrors the Postgres rule almost exactly: nullable/default new columns, no rename/drop mid-roll.
- **Enum/discriminator stability.** `Intent` and `Event` use a `#[serde(tag = "kind", rename_all =
  "snake_case")]` discriminator. Adding a new `kind` value is expand-only and safe. Renaming an
  existing `kind` string, or changing what payload shape an existing `kind` carries, is not — it
  is a hard break (§3).
- **Run `just server-codegen`** after any schema change (gitignored `openapi.json` +
  `client/src/api/generated.ts`). CI and `just check` regenerate them; docker image builds
  expect codegen to have run on the host before `docker build` so the web stage can typecheck.

## 3. Hard breaks — `/v2`

Some changes are not expressible as an additive field: splitting a request body's shape,
repurposing a `kind` discriminator value, changing an intent's semantics in a way an old client's
UI can't safely drive. For those:

1. Add the new shape under a new path version (`/v2/...`), alongside the existing `/v1/...`
   handler — do not mutate `/v1` in place.
2. Run both versions until the drain window that introduced `/v2` has fully closed (the release
   that added `/v2` has completed its own rolling deploy).
3. Remove `/v1` only in a **subsequent** release, once no draining instance can still be serving
   an old SPA that only knows `/v1`.

Use this sparingly — it is an escape hatch for genuine breaks, not a routine versioning habit. Most
changes should fit the expand-only rule in §2.

## 4. SSE / snapshots

`GET /tables/{table}/stream/v1` carries the same expand-only rule as request/response DTOs:

- `VisibleState` and stream frame payloads only grow fields (`#[serde(default)]`); existing fields
  keep their shape and meaning for the life of the compatibility window.
- Do not rearrange or repurpose discriminators on stream frame variants mid-window (same rule as
  `Event::kind` in §2).
- SSE keepalive comments (deploy PRD §DNS & Cloudflare — required so Cloudflare Tunnel's ~100s
  idle timeout doesn't drop the stream) are transport-level, not part of the versioned contract;
  adding/adjusting their cadence is not a wire compatibility change.
- A stale `mtgfr-instance` cookie pointing at a torn-down peer surfaces as `404`/`410` on the next
  request (deploy PRD §Table / instance affinity) — the client's existing stream-reconnect path
  handles this without a wire version bump.

## 5. Authoring habit

- Prefer optional fields first; tighten (make required, narrow a type) only after a full drain
  cycle has passed for the release that added the field, if tightening is even necessary.
- When touching `crates/schema`, ask: "if an old client sent/received this without the new field,
  would it still work?" If no, the change needs `/v2` (§3), not a same-version edit.
- `just server-codegen` regenerates `openapi.json` and `client/src/api/generated.ts` from the
  `schema` crate — run it as part of the same change, not a follow-up.
- Every wire-shape PR should say, in its description, whether it's expand-only or a `/v2` break,
  so reviewers know which rule to check it against.
