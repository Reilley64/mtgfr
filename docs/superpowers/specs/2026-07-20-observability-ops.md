# Observability Ops

**Status:** Current (as of 2026-07-25)
**Module:** `iac/observability.tf`, `client/app/faro.ts`, `client/app/entry.ts`,
`client/server/plugins/otel.server.ts`, `client/server/routes/api/faro/`, `crates/server` (OTEL export)

Related: [production-topology-and-operations](2026-07-20-production-topology-and-operations.md)
(namespaces, env wiring), [ci-and-release](2026-07-20-ci-and-release.md),
[shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) (shell pointer only).

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

### Self-hosted LGTM (namespace `observability`)

**Self-hosted LGTM** in namespace `observability` (Terraform Helm via `iac/observability.tf`):
- **Grafana Alloy** — sole ingest path for all telemetry.
- **Loki** — structured log store (7d retention).
- **Tempo** — distributed trace store (7d retention).
- **Prometheus** — metrics (15d retention).
- **Grafana** — dashboards and trace/log correlation; operator-only via `kubectl port-forward`.

Cluster placement and NetworkPolicy context:
[production-topology-and-operations](2026-07-20-production-topology-and-operations.md).

### Browser (Faro)

**Browser (Faro):** `client/app/faro.ts` (called from `client/app/entry.ts`) installs
`@grafana/faro-web-sdk` + `@grafana/faro-web-tracing`. Posts to same-origin `/api/faro/collect`;
the BFF proxies to Alloy `faro.receiver`. Session sampling forced to 100%; stale sessions
(`isSampled=false` in `sessionStorage`) are repaired. `traceparent` propagation is same-origin
`/api` only.

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

**API:** `tracing` + `opentelemetry-otlp` (HTTP export) in `crates/server`. Engine
(`crates/engine`) emits `tracing` spans but no OTEL exporters (engine is pure).

Prod endpoint wiring (`OTEL_EXPORTER_OTLP_ENDPOINT`, `FARO_COLLECT_UPSTREAM`) is listed in the
Settings / BFF env tables of
[production-topology-and-operations](2026-07-20-production-topology-and-operations.md).

### Scrub rules

**Scrub rules:** identifiers, timing, error classes only. No hand/library contents, intent
payloads, or auth headers. Faro collect is capped at 512 KiB (`FARO_MAX_BODY_BYTES`); oversize
requests return 413. TOON action traces (`ACTION_LOG_DIR`) must stay off Loki. Alloy Faro
rate-limits ingest.

### Grafana access (operator only)

```bash
kubectl -n observability port-forward svc/grafana 3000:80
# admin password: terraform output -raw grafana_admin_password
# or: kubectl -n observability get secret grafana-admin -o jsonpath='{.data.admin-password}' | base64 -d
```

### RED dashboard

Grafana provisions one operator dashboard from
`iac/grafana/dashboards/mtgfr-otel-red.json` via the Helm chart's `dashboardProviders` and
`dashboards` values. The dashboard uses Tempo TraceQL metrics panels rather than Prometheus
spanmetrics because the current Alloy/Tempo topology does not configure a spanmetrics connector or
Tempo metrics-generator.

Panels:
- BFF HTTP rate, 5xx rate, and p95 latency grouped by `http.route` for `service.name=edh-web`.
- API gRPC rate, non-OK rate, and p95 latency grouped by `rpc.service`, `rpc.method`, and
  `rpc.grpc.status_code` for `service.name=edh-api`.
- A Tempo trace-search table for API spans carrying `mtgfr.table.id`.

### Local / dev

**Local/dev:** OTEL exporters no-op when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset. `RUST_LOG`
still drives `tracing` / fmt output.

---

## Implementation Decisions

- **OTEL propagation pattern:** Faro injects `traceparent` same-origin only; BFF continues it as a
  span parent when sampled; BFF passes its span to gRPC via `grpcRequestEnv` bag (not Node
  AsyncLocalStorage) so context survives `runPromise` across the `@effect/rpc` boundary.
- **Faro unsampled span ignore.** Faro's tracing sampler often marks sessions `NOT_RECORD` while
  the fetch instrumentation still injects a `traceparent` for the non-recording span. Parenting
  BFF spans under an unsampled span leaves Tempo with `<root span not yet received>` orphans.
  The BFF rejects inbound `traceparent` where `traceFlags & 0x01 === 0`.
- **Action traces off observability:** TOON files are on a dedicated PVC, not stdout or Loki.
  Retained on pod prune. PVC names include `instanceId` to avoid collisions across rolling
  Deployments ([production-topology-and-operations](2026-07-20-production-topology-and-operations.md)
  container image notes).
- **RED dashboard queries traces directly:** Terraform provisions the dashboard through Grafana
  Helm values; panels use Tempo datasource UID `tempo` and TraceQL metrics functions because
  Prometheus does not receive generated spanmetrics in this stack.

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
