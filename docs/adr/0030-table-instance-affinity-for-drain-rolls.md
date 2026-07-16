# 0030 — Table routing for drain rolls (supersedes cookie affinity)

Status: **Accepted**; extends [0005](0005-in-process-fanout-ndjson-snapshot.md) / [0021](0021-process-local-in-memory-game-registry.md). Replaces the ConfigMap-peer + `mtgfr-instance` cookie model.

## Context

Live games remain process-local. Nested rolls need N Terminating API versions while new tables land only on newest. Cookie sticky + join fan-out across peers was replaced by BFF-owned lobby + Postgres routing.

## Decision

- Pre-game lobby lives on **SolidStart** against database **`mtgfr_web`** (Drizzle migrations; same Postgres instance as Axum `mtgfr`, separate schema).
- At **Start**, BFF calls `POST /tables/seed/v1` on Service **`edh-api`** (`mtgfr.io/api-role=active`). Response includes **`pod_dns`**; BFF writes `table_routes` (explicit delete + TTL).
- In-game `/api/tables/{table}/…` is proxied by looking up `table_routes` → `http://{pod_dns}:8080`. Headless Service **`edh-api-headless`** uses `publishNotReadyAddresses` so Terminating pods stay reachable. **No affinity cookie.**
- Intent/yield/dwell routes use **path** `{table}` (not body-only).
- Ship: `terraform apply` updates `server_image` / `web_image`. API Deployment gets SIGTERM; process drains until `active_tables=0` or `terminationGracePeriodSeconds` (default 24h). Argo CD is installed; optional Application mirrors `iac/charts/edh` when `argocd_repo_url` is set. ConfigMap `edh-api-peers` and the peer drain sequencer are retired.
- Web may roll with the API; expand-only wire across concurrent binaries ([WIRE_COMPAT.md](../WIRE_COMPAT.md)). Lobby UI shows active API image tag.

## Consequences

- Guests join lobbies on newest only (no fan-out). Mid-game survives via Postgres → pod DNS.
- Distroless API has no shell `preStop`; drain wait is in-process on SIGTERM.
- Horizontal same-tag replicas remain out of scope.
