# OpenTelemetry Semantic Conventions (design)

**Status:** Approved design input (2026-07-27).
**Surfaces:** `observability-ops` (primary); `production-topology-and-operations` if resource/env or Grafana dashboard provisioning changes.

Related living specs: [observability-ops](2026-07-20-observability-ops.md),
[production-topology-and-operations](2026-07-20-production-topology-and-operations.md),
[wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) (no intent bodies in spans),
[shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) (Faro pointer only).

This document is **design input only**. It does not replace updating living surface specs at
implement time (see [At implement time](#at-implement-time--living-docs)).

---

## Problem Statement

Browser → BFF → API telemetry already flows into self-hosted LGTM (Faro + Effect OTEL + Rust
OTLP), but span names and attributes are inconsistent with OpenTelemetry Semantic Conventions:
API uses a generic `grpc` span with `rpc.method` set to the full path; game fields use
free-form `table_id` / `intent.kind` / `accepted`; HTTP/RPC/DB vocabulary is not shared across
runtimes. Operators cannot rely on standard Tempo/Prometheus queries, and ad-hoc attributes
risk drift against the hard scrub rule (identifiers, timing, error classes only — never
hand/library contents, intent payloads, or auth headers).

## Goal

Adopt the **smallest Medium convention set** that makes browser → BFF → API traces uniformly
queryable in LGTM, with an explicit attribute allowlist/denylist, deliberate `mtgfr.*`
extensions, RED dashboards for BFF HTTP and API gRPC, and unit/golden verification — without
weakening scrub rules or inventing a new product-surface feature spec.

## Locked decisions

| Decision | Choice |
|---|---|
| Adoption depth | **Medium** — resource + HTTP + RPC + safe DB + exception type/status |
| Custom game fields | Keep; rename to `mtgfr.*` (`mtgfr.table.id`, `mtgfr.intent.kind`, `mtgfr.intent.accepted`, `mtgfr.user.id`) |
| Dashboards | **In implement PR** — RED panels for BFF HTTP and API gRPC |
| DB attributes | Allowlist safe keys only; **forbid** `db.query.text` / statement bodies |
| Verification | Unit/golden fixtures only (no CI whole-tree grep gate in v1) |
| Convention pin | OpenTelemetry Semantic Conventions **1.37.0** |
| Scrub vs convention | Scrub wins — never emit forbidden data because “the convention says so” |

## Approaches considered

1. **Minimal** — resource + HTTP + RPC only; skip DB and exception detail. Fastest, but leaves
   Postgres latency opaque and weaker error taxonomy.
2. **Medium (chosen)** — Minimal plus safe DB attrs, `exception.type` + span status, pinned
   semconv release, shared dictionary, RED dashboards, golden fixtures.
3. **Broad** — full registry families, deeper Faro web normalization, messaging, stricter
   static scans. Excess churn vs the “smallest uniformly queryable” bar.

---

## Design

### Framing and ownership

**Convention pin:** Target OpenTelemetry Semantic Conventions **1.37.0** for the stable
resource / HTTP / RPC / DB / exception attribute names used below. Document the pin in this
design and in living `observability-ops` at implement time.

**Churn policy:** Do not chase every registry bump. Bump the pin only when (a) an adopted key
is renamed or removed, or (b) a family already in this Medium set needs a newer stable name.
New families (messaging, GenAI, etc.) require a new design pass.

**Dictionary ownership:** One shared attribute dictionary lives in `observability-ops`
(allowlist / denylist / `mtgfr.*`). Runtimes map to it — no per-runtime free-form keys.

| Runtime | Sets | Does not own |
|---|---|---|
| Faro (`edh-web` browser) | Resource/app identity; HTTP client attrs on same-origin `/api` | gRPC, DB, `mtgfr.*` game fields |
| BFF (`edh-web` Nitro / Effect) | HTTP server attrs, outbound RPC client attrs, safe DB attrs, exceptions | Engine internals |
| API (`edh-api` / tonic) | Inbound RPC server attrs, safe DB attrs, `mtgfr.*`, exceptions | Browser RUM |
| Engine (`crates/engine`) | `tracing` spans only (no OTEL exporters) | — |

**Hard rule:** Scrub wins. If a convention encourages query text, bodies, headers, or intent
payloads — forbid it.

### Attribute allowlist / denylist

#### Resource (all services)

| Key | Status | Notes |
|---|---|---|
| `service.name` | **Allow** | `edh-web` (Faro + BFF), `edh-api` (API); honor `OTEL_SERVICE_NAME` when set |
| `service.version` | **Allow** | App version / image tag (already partially wired) |
| `service.instance.id` | **Allow** | API via `INSTANCE_ID` when present |
| `deployment.environment` | **Allow** | Wire from existing env if present; omit if unset — never invent |
| `vcs.ref.head.revision` | **Allow** | Git commit (already set on BFF/API) |

**Forbid on resource:** user/session tokens, deck lists, table secrets, auth material.

#### HTTP (Faro client + BFF server)

| Key | Status | Notes |
|---|---|---|
| `http.request.method` | **Allow** | |
| `http.response.status_code` | **Allow** | |
| `http.route` or `url.path` | **Allow** | Prefer low-cardinality route templates over raw paths with ids when both exist |
| `url.scheme` | **Allow** | |
| `server.address` | **Allow** | Host only |
| `http.request.body.*` / `http.response.body.*` | **Forbid** | |
| Full `url.query` with tokens / sensitive preselect | **Forbid** | Do not promote query strings that may hold session or deck secrets into attrs |
| `Authorization` / cookie / raw `traceparent` as attributes | **Forbid** | Propagation stays in headers, not span attributes |

#### RPC / gRPC (BFF client → API server)

| Key | Status | Notes |
|---|---|---|
| `rpc.system` | **Allow** | Always `grpc` |
| `rpc.service` | **Allow** | e.g. `mtgfr.v1.Game` |
| `rpc.method` | **Allow** | Short method name (e.g. `SubmitIntent`), **not** the full HTTP/2 path |
| `rpc.grpc.status_code` | **Allow** | |

**Change from today:** API `TraceLayer` currently names the span `grpc` and sets
`rpc.method` to the full path (e.g. `/mtgfr.v1.Game/SubmitIntent`). Implement should emit
semconv-shaped `rpc.system` / `rpc.service` / `rpc.method` and a span name of the form
`{rpc.service}/{rpc.method}` (or the 1.37.0-recommended equivalent).

#### DB (BFF Drizzle / `@effect/sql-pg` + API Toasty)

| Key | Status | Notes |
|---|---|---|
| `db.system` | **Allow** | `postgresql` |
| `db.operation.name` | **Allow** | Operation name only |
| `db.namespace` | **Allow** | Database name only (`mtgfr` / `mtgfr_web`) |
| `db.query.text` / statement / parameter bodies | **Forbid** | Privacy — may embed private game data |
| Row contents | **Forbid** | |

Instrument DB spans only when query text can be omitted. If auto-instrumentation cannot
guarantee that, hand-set system / operation / namespace only.

#### Exceptions

| Key | Status | Notes |
|---|---|---|
| Span status `Error` | **Allow** | |
| `exception.type` | **Allow** | Error class / tonic status name |
| `exception.escaped` | **Allow** | Only if instrumentation sets it safely |
| `exception.message` / stacks with payloads | **Forbid** | Prefer status + type when in doubt |

#### Deliberate `mtgfr.*` extensions (API game spans)

| Key | Meaning | Justification |
|---|---|---|
| `mtgfr.table.id` | Table id | Correlate ops without payload |
| `mtgfr.intent.kind` | Intent type enum only | Filter `SubmitIntent` without body |
| `mtgfr.intent.accepted` | bool ack outcome | Success/reject without payload |
| `mtgfr.user.id` | Authenticated user id | Pre-existing `user_id` on submit path; identifier only |

**Migrate away from:** bare `table_id`, `intent.kind`, `accepted`, `user_id` (as span
attribute keys). Also migrate BFF’s current non-semconv annotations (`http.method` →
`http.request.method`; drop free-form `rpc.path` in favor of proper `rpc.*` on the
outbound gRPC client span).

**Forbid:** `mtgfr.intent.payload`, hand/library fields, auth tokens, free-form keys outside
this dictionary.

#### Span names

- BFF HTTP: route-oriented (match the route template / Effect edge span).
- gRPC: `{rpc.service}/{rpc.method}` style.
- Never put card names, player names, or intent bodies in span names.

### Runtime alignment (auto vs hand-set)

**Faro (`client/app/faro.ts`):** Keep existing web/fetch instrumentation and same-origin
`/api` `traceparent` propagation. Map app identity to `service.name=edh-web` plus version /
commit. Do not broaden CORS propagation beyond `/api`.

**BFF (`client/app/domain/otel`, Nitro plugin):** Keep `runTracedRequest` + `grpcRequestEnv`
as the edge (fiber Context does not survive `runPromise` alone). Hand-set HTTP server attrs
on the request span; outbound gRPC client spans get `rpc.*`. Safe DB attrs only under the
forbid-query-text rule.

**API (`crates/server` `telemetry.rs`, `grpc/trace.rs`, game submit path):** Fix inbound RPC
attrs and naming; migrate game fields to `mtgfr.*`; safe DB attrs for Toasty under the same
rule. Engine remains pure — no OTEL exporters in `crates/engine`.

**Propagation unchanged:** Faro → BFF (sampled `traceparent` only) → gRPC metadata via
`grpcRequestEnv`. Unsamped Faro parents stay ignored (Tempo orphan prevention).

### Privacy vs conventions (scrub map summary)

| Candidate | Verdict |
|---|---|
| Resource `service.*` / `vcs.*` / `deployment.environment` | Allow |
| HTTP method / status / route | Allow |
| HTTP/gRPC bodies, auth headers as attrs | Forbid |
| `rpc.*` + status codes | Allow |
| `db.system` / operation / namespace | Allow |
| `db.query.text` | Forbid |
| `exception.type` + span status | Allow |
| `exception.message` with payloads | Forbid |
| `mtgfr.table.id` / `mtgfr.intent.kind` / `mtgfr.intent.accepted` / `mtgfr.user.id` | Allow (deliberate) |
| Intent payload / hand / library | Forbid |

### Dashboards / Tempo (implement PR)

Today Grafana Helm values provision datasources only — no product dashboards
(`iac/observability.tf`). Implement provisions **one** operator dashboard (Grafana Helm
dashboard provisioning or equivalent ConfigMap) with RED panels:

1. **BFF HTTP** — rate / error rate / latency by `http.route` (or path template) and
   `service.name`
2. **API gRPC** — rate / error rate / latency by `rpc.service`, `rpc.method`, and
   `rpc.grpc.status_code`

Optional row: Explore links filtered by `mtgfr.table.id` / `mtgfr.intent.kind` (identifiers
only). No panels that require body or query-text attributes.

What becomes easier once conventions are stable: RED by route/method, gRPC error class
filters, browser→BFF→API correlation without remembering bespoke attribute names.

### Verification (implement PR)

No live LGTM required in every PR.

1. **Unit/golden fixtures** (BFF TS + API Rust) for resource builders and span attribute
   helpers: assert allowlisted keys present and denylisted keys absent.
2. **Privacy fixture:** sample attribute maps for a SubmitIntent-shaped path include
   `mtgfr.intent.kind` / `mtgfr.intent.accepted` only — never payload, hand, or library
   fields.
3. **Documented good Tempo tree** (manual ops check via port-forward, not CI):

```text
Faro fetch (edh-web, HTTP client attrs)
  └─ BFF HTTP server span (http.request.method, http.route / url.path, status)
       └─ BFF RPC client span (rpc.system=grpc, rpc.service, rpc.method)
            └─ API RPC server span (same rpc.*; optional mtgfr.table.id /
               mtgfr.intent.kind / mtgfr.intent.accepted; status / exception.type only)
```

Privacy check on a sample: open the API leaf span; confirm no hand/library contents, no
intent payload, no Authorization/cookie attribute values.

Suggested implement commands (extend adjacent suites; living specs name exact tests):

```bash
# API: telemetry resource attrs + grpc/trace span attribute helpers
cargo nextest run --profile ci telemetry span_for_http_request
# BFF: domain otel / build-meta resource + span helper fixtures
bun test client/app/domain/otel client/app/domain/build-meta
just server-lint       # when Rust span helpers change
just client-typecheck  # when BFF otel helpers change
```

### At implement time — living docs

Update these in the **same** implementation change (design sidecars do not replace surface
specs):

1. **`docs/superpowers/specs/2026-07-20-observability-ops.md`** (primary) — Behavior /
   Implementation / Testing: convention pin, shared dictionary, runtime mapping, scrub
   allow/deny, `mtgfr.*` extensions, RED dashboard, golden verification.
2. **`docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`** — if
   `deployment.environment` / resource env wiring or Grafana dashboard provisioning is added.
3. **`AGENTS.md` Observability one-liner** — only if implement finds agents need an explicit
   “allowlisted OTel keys; scrub wins; no intent payloads in spans” reminder. Prefer
   skipping if the living obs spec alone is enough.
4. **Pointers only (no behavior duplication):** `wire-protocol-and-visibility` still forbids
   intent bodies in spans; `shell-routes-and-auth` remains a Faro pointer.

Do **not** create a new indexed product-surface feature spec for this work.

Optional local plan (gitignored): `docs/superpowers/plans/2026-07-27-otel-semantic-conventions.md`.

---

## Out of Scope

- New collectors or Alloy topology changes
- GenAI semantic conventions
- OpenTelemetry Arrow
- Replacing Grafana Faro
- OTEL exporters in `crates/engine` (engine stays pure)
- CI whole-tree attribute greps / allowlist scanners (v1 uses unit/golden fixtures only)
- Messaging semantic conventions
- Broadening Faro `traceparent` propagation beyond same-origin `/api`
- Emitting `db.query.text`, intent payloads, hand/library contents, or auth headers as
  span/log attributes

---

## Further Notes

- Current API custom fields (`table_id`, `intent.kind`, `accepted` in `grpc/trace.rs` /
  submit path) are **identifiers / enums / bools** today and remain privacy-safe after the
  `mtgfr.*` rename — the rename is vocabulary hygiene, not a scrub relaxation.
- Faro collect remains capped at 512 KiB; TOON action traces stay off Loki.
- Exporters continue to no-op locally unless `OTEL_EXPORTER_OTLP_ENDPOINT` is set.
