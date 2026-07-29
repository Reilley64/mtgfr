# Observability Ops

**Status:** Current (as of 2026-07-27)
**Module:** `iac/observability.tf`, `client/app/faro.ts`, `client/app/entry.ts`,
`client/server/plugins/otel.server.ts`, `client/server/routes/api/faro/`, `crates/server` (OTEL export)

Related: [production-topology-and-operations](2026-07-20-production-topology-and-operations.md)
(namespaces, env wiring), [ci-and-release](2026-07-20-ci-and-release.md),
[shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) (shell pointer only),
[OpenTelemetry Semantic Conventions design](2026-07-27-otel-semantic-conventions-design.md)
(approved design input).

---

## Problem Statement

Observability — structured logs, distributed traces, and metrics from browser through BFF to API
— is needed to debug friend-group games without full server access, without leaking private game
state (hands, libraries, intent payloads) into telemetry.

---

## Solution

Self-hosted **LGTM** (Grafana/Loki/Tempo/Prometheus) in namespace `observability`, with
**Grafana Alloy** as the sole ingest path, browser Faro telemetry via same-origin
`/api/faro/collect`, and OTEL from both BFF and API. Grafana is operator-only via `kubectl
port-forward`; no tunnel hostname for the observability plane. Exporters no-op locally unless
`OTEL_EXPORTER_OTLP_ENDPOINT` is set.

---

## User Stories

- As an **operator**, I run `kubectl -n observability port-forward svc/grafana 3000:80` and open
  Grafana; I see latency, error rate, and can correlate a browser trace to a BFF span to an API
  span via Tempo trace links in Loki.
- As an **operator**, I open the `mtgfr OTEL RED` dashboard and see BFF HTTP rate/error/latency by
  `http.route`, plus API gRPC rate/error/latency by `rpc.service`, `rpc.method`, and
  `rpc.grpc.status_code`.
- As an **operator**, I open Grafana (via port-forward) and see browser → BFF → API traces
  correlated by W3C `traceparent`, with no hand/library contents in any span.

---

## Behavior

### Semantic-convention contract

The deployed browser → BFF → API telemetry vocabulary follows the Medium OpenTelemetry Semantic
Conventions set pinned to **1.37.0**. The pin covers stable resource, HTTP, RPC/gRPC, safe DB, and
exception attributes plus deliberate `mtgfr.*` extensions. Bump the pin only when an adopted key is
renamed/removed or an already-adopted family needs a newer stable name.

One shared attribute dictionary owns the allow/deny rules; individual runtimes map to it and do not
invent free-form span keys. Scrub rules win over conventions.

| Family | Allow | Forbid |
|---|---|---|
| Resource | `service.name`, `service.version`, `service.instance.id`, `deployment.environment`, `vcs.ref.head.revision` | user/session tokens, deck lists, table secrets, auth material |
| HTTP | `http.request.method`, `http.response.status_code`, low-cardinality `http.route` or `url.path`, `url.scheme`, `server.address` | request/response bodies, sensitive query strings, auth/cookie headers as attributes |
| RPC / gRPC | `rpc.system=grpc`, `rpc.service`, short `rpc.method`, `rpc.grpc.status_code` | full HTTP/2 path as `rpc.method`, request/response bodies |
| DB | `db.system=postgresql`, `db.operation.name`, `db.namespace` | `db.query.text`, statement strings, parameters, row contents |
| Exceptions | span status `Error`, `exception.type`, safe `exception.escaped` | `exception.message` or stacks when they may include payloads |
| `mtgfr.*` | `mtgfr.table.id`, `mtgfr.intent.kind`, `mtgfr.intent.accepted`, `mtgfr.user.id` | `mtgfr.intent.payload`, hand/library fields, auth tokens, any unlisted game key |

Runtime ownership:

| Runtime | Sets | Does not own |
|---|---|---|
| Faro (`edh-web` browser) | App/resource identity and HTTP client attrs on same-origin `/api` fetches | gRPC, DB, `mtgfr.*` game fields |
| BFF (`edh-web` Nitro / Effect) | HTTP server attrs, outbound gRPC client attrs, safe DB attrs, exceptions | Engine internals |
| API (`edh-api` / tonic) | Inbound RPC server attrs, `mtgfr.*`, exceptions; safe DB attrs when DB spans are emitted | Browser RUM |
| Engine (`crates/engine`) | Local `tracing` spans only | OTEL exporters |

Span names stay low-cardinality: BFF HTTP spans are route-oriented, gRPC spans are
`{rpc.service}/{rpc.method}`, and span names never include card names, player names, intent bodies,
or private game state.

### Self-hosted LGTM (namespace `observability`)

**Self-hosted LGTM** in namespace `observability` (Terraform Helm via `iac/observability.tf`):
- **Grafana Alloy** — sole ingest path for all telemetry.
- **Loki** — structured log store (7d retention).
- **Tempo** — distributed trace store (7d retention), with the metrics-generator enabled running
  the `local-blocks` processor so TraceQL metrics (`{...} | rate()`) work in the RED dashboard and
  Grafana Drilldown > Traces. Generator WAL and RF1 blocks live on the Tempo PVC under
  `/var/tempo/generator/`.
- **Prometheus** — metrics (15d retention).
- **Grafana** — dashboards and trace/log correlation; operator-only via `kubectl port-forward`.

Cluster placement and NetworkPolicy context:
[production-topology-and-operations](2026-07-20-production-topology-and-operations.md).

### Browser (Faro)

**Browser (Faro):** `client/app/faro.ts` (called from `client/app/entry.ts`) installs
`@grafana/faro-web-sdk` + `@grafana/faro-web-tracing`. Posts to same-origin `/api/faro/collect`;
the BFF proxies to Alloy `faro.receiver`. Session sampling forced to 100%; stale sessions
(`isSampled=false` in `sessionStorage`) are repaired. `traceparent` propagation is same-origin
`/api` only. Faro app identity remains `app.name=edh-web`, aligned with `service.name=edh-web`.

Same-origin `/api` requests are made by Effect's `HttpClient` (`client/app/domain/rpc-client.ts`,
`client/app/domain/lobby/client.ts`), which opens its own span per request and injects that span's
`traceparent`. `browserTracerLayer` (`client/app/domain/faro/tracer.ts`, merged into
`client/app/resources.ts`) backs Effect's tracer with the global OTEL provider Faro registers, so
those spans are real browser spans that Faro ships to Tempo and the injected `traceparent` names a
span that actually arrives. Without Faro (local dev) the global provider is a no-op, the injected
`traceparent` is unsampled, and the BFF drops it and becomes the root.

Buffered browser spans are force-flushed when the page hides or unloads (`registerSpanFlushOnHide`,
registered before `initializeFaro` so it runs ahead of faro-core's own hide flush). The game stream
`/api/rpc/game/:table/stream` is traced like any other request: Effect's client span ends when the
response headers arrive, not when the SSE body closes. See Implementation Decisions.

### BFF (OTEL)

**BFF (OTEL):** `client/server/plugins/otel.server.ts` (Nitro plugin) installs a process-scoped
`@effect/opentelemetry` `ManagedRuntime` once at server start via `initOtel()`. Exports OTLP when
`OTEL_EXPORTER_OTLP_ENDPOINT` is set; no-ops otherwise. `runTracedRequest(traceparent, spanName, body)`
is the standard edge entry: it continues inbound W3C `traceparent` as the BFF span parent **only when
sampled** (unsampled Faro non-recording spans are ignored — avoids `<root span not yet received>`
Tempo orphans), opens `spanName`, and runs on the OTEL runtime.

**Trace propagation rule:** Effect parent spans live in **fiber Context**. The gRPC `ManagedRuntime`
is separate — context does not survive `runPromise` into outbound calls. `grpcRequestEnv` captures
`{ sessionToken, traceparent }` once per request edge under `runTracedRequest`, then passes it into
every gRPC call. Do not use Node AsyncLocalStorage or per-helper optional `traceparent` args.

BFF propagates its span into gRPC metadata so Tempo shows browser → web → API trace chains.
Outbound BFF gRPC calls from `client/app/domain/wire/grpcClient.ts` open client spans named
`{rpc.service}/{rpc.method}` with `rpc.system=grpc`, `rpc.service`, and `rpc.method` attributes.
`GrpcStatusError` failures annotate only `rpc.grpc.status_code` and `exception.type`; gRPC status
messages, request bodies, and intent payloads stay out of span attributes.

BFF `mtgfr_web` database work that crosses the temporary `runWebDb` Promise bridge opens a
`db.mtgfr_web` span with hand-set `db.system=postgresql`, `db.operation.name=QUERY`, and
`db.namespace=mtgfr_web` attributes only. The BFF does not attach SQL strings, `db.query.text`, or
`db.statement` to DB spans.

Lobby HTTP helpers in `client/server/lobby-http.ts` annotate traced failures with `exception.type`
only (`LobbyDbError` for wrapped DB/upstream failures). JSON error bodies may include truncated
messages for clients; those strings never appear on span attributes.

### API (OTEL)

**API:** `tracing` + `opentelemetry-otlp` (HTTP export) in `crates/server`. Inbound tonic spans
use `rpc.system=grpc`, `rpc.service`, short `rpc.method`, `rpc.grpc.status_code`, and
`{rpc.service}/{rpc.method}` names. Tower `TraceLayer` records `rpc.grpc.status_code` from
response headers only; mid-stream trailer-only failures may default to OK (`0`). Game submit spans
use `mtgfr.table.id`, `mtgfr.intent.kind`,
`mtgfr.intent.accepted`, and `mtgfr.user.id` only; legacy bare `table_id`, `intent.kind`,
`accepted`, and `user_id` are not emitted as OTEL attribute keys. Engine (`crates/engine`) emits
`tracing` spans but no OTEL exporters (engine is pure).

Prod endpoint wiring (`OTEL_EXPORTER_OTLP_ENDPOINT`, `FARO_COLLECT_UPSTREAM`) is listed in the
Settings / BFF env tables of
[production-topology-and-operations](2026-07-20-production-topology-and-operations.md).

### Scrub rules

**Scrub rules:** identifiers, timing, error classes, and allowlisted convention keys only. No
hand/library contents, intent payloads, SQL/query bodies, auth headers, cookies, or free-form
attributes outside the dictionary. Faro collect is capped at 512 KiB (`FARO_MAX_BODY_BYTES`);
oversize requests return 413. TOON action traces (`ACTION_LOG_DIR`) must stay off Loki. Alloy Faro
rate-limits ingest.

### Grafana access (operator only)

```bash
kubectl -n observability port-forward svc/grafana 3000:80
# admin password: terraform output -raw grafana_admin_password
# or: kubectl -n observability get secret grafana-admin -o jsonpath='{.data.admin-password}' | base64 -d
```

### RED dashboard

Grafana provisions one operator dashboard from
`iac/dashboards/mtgfr-otel-red.json` via the Helm chart's `dashboardProviders` and
`dashboards` values. The directory is deliberately not named `grafana/` — Helm resolves `chart` as
a local path before the remote `repository`, so `iac/grafana/` shadows the upstream chart. The dashboard uses Tempo TraceQL metrics panels rather than Prometheus
spanmetrics because the current Alloy/Tempo topology does not configure a spanmetrics connector or
Tempo metrics-generator.

Panels:
- BFF HTTP rate, 5xx rate, and p95 latency grouped by `http.route` for `service.name=edh-web`.
- API gRPC rate, non-OK rate, and p95 latency grouped by `rpc.service`, `rpc.method`, and
  `rpc.grpc.status_code` for `service.name=edh-api`.
- A Tempo trace-search table for API spans carrying `mtgfr.table.id`.

Expected good Tempo tree:

```text
Faro fetch (edh-web, HTTP client attrs)
  └─ BFF HTTP server span (http.request.method, http.route / url.path, status)
       └─ BFF RPC client span (rpc.system=grpc, rpc.service, rpc.method)
            └─ API RPC server span (same rpc.*; optional mtgfr.table.id /
               mtgfr.intent.kind / mtgfr.intent.accepted; status / exception.type only)
```

Privacy check: open the API leaf span for a sampled submit path and confirm no hand/library
contents, no intent payload, no SQL text, and no Authorization/cookie values are present.

### Local / dev

**Local/dev:** OTEL exporters no-op when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset. `RUST_LOG`
still drives `tracing` / fmt output.

---

## Implementation Decisions

- **OTEL propagation pattern:** the browser injects `traceparent` same-origin only; BFF continues it
  as a span parent when sampled; BFF passes its span to gRPC via `grpcRequestEnv` bag (not Node
  AsyncLocalStorage) so context survives `runPromise` across the `@effect/rpc` boundary.
- **Faro unsampled span ignore.** Faro's tracing sampler often marks sessions `NOT_RECORD` while
  the fetch instrumentation still injects a `traceparent` for the non-recording span. Parenting
  BFF spans under an unsampled span leaves Tempo with `<root span not yet received>` orphans.
  The BFF rejects inbound `traceparent` where `traceFlags & 0x01 === 0`.
- **Effect's tracer must share Faro's provider.** Effect's `HttpClient` opens a span for every
  request and injects its `traceparent` — always, and always flagged sampled, even with no ambient
  span and no OTEL configured. Backed by Effect's own tracer that span is exported by nothing:
  Faro's `fetch` instrumentation never sees these requests (the RPC client binds `globalThis.fetch`
  at module evaluation, before `initFaro` patches it), and Faro's SDK has no knowledge of Effect
  spans. The BFF then parents `rpc <path>` under an id that never arrives, the API parents under
  the BFF, and Tempo holds a rootless trace — `<root span not yet received>` on every RPC call,
  regardless of navigation, and no browser-origin spans in Tempo at all. `browserTracerLayer`
  installs `OtelTracer.layerGlobal`, so Effect's request spans *are* Faro's spans. The BFF's
  sampled-only guard is what makes the Faro-less case degrade cleanly to a BFF-rooted trace.
- **Browser spans flush on page hide.** faro-core's transport flushes its batch on
  `visibilitychange: hidden`, but faro-web-tracing wraps a plain OTel `BatchSpanProcessor` (1 s
  timer, 30-span batch) whose `forceFlush` nothing calls, so spans ending in that last second die
  with the page and orphan their trace. `registerSpanFlushOnHide` is registered *before*
  `initializeFaro` so it fires first; `FaroTraceExporter.export` pushes synchronously, so the spans
  reach the transport buffer in time for Faro's own flush (sent with `keepalive`).
- **Action traces off observability:** TOON files are on a dedicated PVC, not stdout or Loki.
  Retained on pod prune. PVC names include `instanceId` to avoid collisions across rolling
  Deployments ([production-topology-and-operations](2026-07-20-production-topology-and-operations.md)
  container image notes).
- **RED dashboard queries traces directly:** Terraform provisions the dashboard through Grafana
  Helm values; panels use Tempo datasource UID `tempo` and TraceQL metrics functions because
  Prometheus does not receive generated spanmetrics in this stack.
- **TraceQL metrics need the metrics-generator.** The `grafana/tempo` chart ships
  `tempo.metricsGenerator.enabled: false`, which renders a `tempo.yaml` with no
  `metrics_generator.storage.path`. Tempo then logs "metrics-generator is not configured", swaps the
  generator for an idle service, and never joins the generator ring — so every TraceQL metrics query
  500s with `error finding generators: empty ring`, because the querier serves the recent window
  (`query_frontend.metrics.query_backend_after`, 15m) from generators. `iac/observability.tf` enables
  it, lists `local-blocks` in `overrides.defaults.metrics_generator.processors` (a processor a tenant
  does not ask for is not run), sets `filter_server_spans: false` so client spans count, and
  `flush_to_storage: true` so queries reach past the live window.
- **Semantic-convention pin:** the living contract pins OpenTelemetry Semantic Conventions 1.37.0.
  New telemetry families require a design pass before they join the shared dictionary.

---

## Testing Decisions

- Operator path: port-forward Grafana and confirm browser → BFF → API correlation in Tempo for a
  same-origin `/api` request (manual / friend-group ops check).
- Local: with `OTEL_EXPORTER_OTLP_ENDPOINT` unset, exporters no-op; client/server unit suites do not
  require Alloy.
- BFF outbound gRPC semantic-convention spans are covered by
  `client/app/domain/wire/grpcClient.semconv.test.ts`.
- BFF DB semantic-convention attrs are covered by
  `client/app/domain/otel/semconv.test.ts`.
- Build metadata consumed by BFF OTEL resource attributes is covered by
  `client/app/domain/build-meta.test.ts` ([shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)).
- Dashboard provisioning changes are validated with JSON syntax checks and `terraform validate`
  from `iac/`; dashboard query behavior is checked manually through operator Grafana after
  port-forwarding.
- API Rust unit tests cover RPC/gRPC and `mtgfr.*` span fields in `grpc/trace.rs` and
  `otel_semconv.rs` (including legacy bare game key exclusion on span field names), plus
  `deployment.environment` trim/omit behavior in `telemetry.rs`. They do not golden-fixture full
  resource attribute sets, exception attrs, or payload/SQL/auth scrubbing.
- BFF TypeScript golden fixtures assert HTTP server, outbound gRPC, safe DB, exception, and build
  metadata resource attributes match the shared dictionary and omit forbidden body/query/auth keys.
- Manual golden trace verification uses the good Tempo tree shown in Behavior and the API leaf-span
  privacy check; live LGTM is not required for every PR.

---

## Out of Scope

- Grafana Faro public dashboard (operator-only via port-forward).
- Putting hand/library contents or intent payloads in telemetry (hard rule).
- Engine OTEL exporters (engine stays pure).

---

## Further Notes

- **Faro size cap.** `FARO_MAX_BODY_BYTES = 512 KiB`. The `/api/faro/collect` route rejects
  oversized payloads with 413 before reading the full body (`contentLengthTooLarge` checks
  `Content-Length` header first; `readBodyCapped` streams and checks).
- Production source maps (`build.sourcemap: true`) help Faro deminify frames; see
  [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md).
- Shell routes only keep a short pointer here; do not duplicate Faro/BFF OTEL Behavior in the
  shell surface spec.
