# 0034 — Self-hosted LGTM + Faro + OpenTelemetry

Status: **Accepted**

## Context

The cluster had health probes and an unused `RUST_LOG` env, but no metrics, logs, or distributed traces. Debugging friend-group games required pod logs and local TOON action traces. We want browser→BFF→API continuity without leaking private hand/library state into storage.

## Decision

- **Self-hosted LGTM** in namespace `observability` via Terraform Helm: Grafana, Loki, Tempo, Prometheus, plus **Grafana Alloy** as the sole ingest path.
- **Grafana UI** is operator-only via `kubectl port-forward` (no Cloudflare Tunnel hostname).
- **Browser:** Grafana Faro (`@grafana/faro-web-sdk` + `@grafana/faro-web-tracing`) via `src/plugins/otel.client.ts` (imported from `entry-client`); posts to same-origin `/api/faro/collect`; the BFF proxies to Alloy `faro.receiver`. Faro app name is `edh-web` (same as BFF; tell them apart with `telemetry.sdk.name`). Session sampling is forced to 100%; resumed `sessionStorage` sessions are repaired if stuck with `isSampled=false` (legacy Faro storage otherwise never exports browser spans while still injecting `traceparent`).
- **BFF:** Nitro plugin `src/plugins/otel.server.ts` installs a process-scoped `@effect/opentelemetry` `ManagedRuntime` once; exports OTLP when `OTEL_EXPORTER_OTLP_ENDPOINT` is set; continues inbound W3C `traceparent` as the BFF span parent **only when sampled** (unsampled Faro non-recording injects are ignored so Tempo is not left with `<root span not yet received>` orphans) and injects the *BFF* span into gRPC metadata (so Tempo shows browser → web → api).
- **Propagation rule:** Effect parent spans live in fiber Context. The gRPC client uses a **separate** `ManagedRuntime`, so context does not survive `runPromise` into outbound calls. Always capture `currentTraceparent` on the OTEL fiber and pass it explicitly (`RpcEnv.traceparent` / `grpcClient(addr, tp)` / lobby helpers). Do not use Node AsyncLocalStorage for this handoff.
- **API:** `tracing` + `opentelemetry-otlp` (HTTP) in `crates/server`; `tracing` spans in `crates/engine` (no exporters in engine).
- **Metrics:** app OTEL only (no kube/node scrapes). **Logs:** OTLP from apps → Alloy → Loki (not stdout scrape).
- **Sampling:** 100%. **Retention:** 7d traces/logs, 15d metrics.
- **Scrub:** identifiers + timing + error classes only. Never hand/library contents, intent payloads, or auth headers. Keep TOON `action_log` out of Loki. Faro collect is size-capped (512KiB); Alloy Faro rate-limits; browser `traceparent` propagation is same-origin `/api` only.
- **Local/dev:** exporters no-op when OTLP endpoint unset; `RUST_LOG` still drives fmt. OTLP exporter build failures soft-fall back to fmt-only.

## Consequences

- New Terraform: `iac/observability.tf`, NetworkPolicy for Alloy, edh chart env (`OTEL_*`, `FARO_COLLECT_UPSTREAM`).
- Cross-namespace Alloy ingress from `edh-web` / `edh-api`.
- Operator docs: port-forward Grafana; admin password in Secret `grafana-admin`.
