# 0030 — Table routing for drain rolls (supersedes cookie affinity)

Status: **Accepted**; extends [0005](0005-in-process-fanout-ndjson-snapshot.md) / [0021](0021-process-local-in-memory-game-registry.md). Replaces the ConfigMap-peer + `mtgfr-instance` cookie model.

## Context

Live games remain process-local. Nested rolls need N Terminating API generations while new tables land only on newest. Cookie sticky + join fan-out across peers was replaced by BFF-owned lobby + Postgres routing.

## Decision

- Pre-game lobby lives on **SolidStart** against database **`mtgfr_web`** (Drizzle migrations; same Postgres instance as Axum `mtgfr`, separate schema).
- At **Start**, BFF calls `POST /tables/seed/v1` on Service **`edh-api`**, which selects **`app=<apiActiveInstanceId>`** (newest Deployment only). Response includes **`pod_dns`**; BFF writes `table_routes` (explicit delete + TTL).
- In-game `/api/tables/{table}/…` is proxied by looking up `table_routes` → `http://{pod_dns}:8080`. Headless Service **`edh-api-headless`** uses `publishNotReadyAddresses` so Terminating pods stay reachable. **No affinity cookie.**
- Intent/yield/dwell routes use **path** `{table}` (not body-only).
- Ship: `terraform apply` updates image helm params on the Argo Application **after** migrate Jobs complete. Argo syncs `iac/charts/edh`: new API Deployment (sync-wave 0), then Service **`edh-api`** selector `app=<apiActiveInstanceId>` (wave 1), then `PruneLast` deletes the prior Deployment → **SIGTERM** drain until `active_tables=0` or `terminationGracePeriodSeconds` (default 24h). Apply does not wait on grace. Headless Service stays in Terraform. ConfigMap `edh-api-peers` and the peer drain sequencer are retired. No live `POST /admin/drain` on the ship path (`GET /health/drain` is observation only).
- Web may roll with the API; expand-only wire across concurrent binaries ([WIRE_COMPAT.md](../WIRE_COMPAT.md)). Lobby UI shows active API image tag.

## Consequences

- Guests join lobbies on newest only (no fan-out). Mid-game survives via Postgres → pod DNS. Nested rolls yield 1 Ready + N−1 Terminating generations.
- Distroless API: drain wait is in-process on SIGTERM (no shell `preStop`).
- Horizontal same-tag replicas remain out of scope.
