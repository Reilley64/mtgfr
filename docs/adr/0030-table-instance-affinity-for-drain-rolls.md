# 0030 — Table/instance affinity for drain rolls

Status: **Accepted**; extends [0005](0005-in-process-fanout-ndjson-snapshot.md) (single-instance fan-out) to a two-instance rolling deploy; deploy PRD §Table / instance affinity and §Rolling deployment model.

## Context

ADR 0005 assumes one server process per table for the life of that table — `tokio::broadcast` and
the in-memory `Registry` (ADR 0021) are process-local, so a table's SSE stream and game state live
and die with the pod that created it. The deploy PRD's rolling deploy needs a new server binary to
take over new traffic without killing tables still running on the old one, which means **two**
server processes can be live in the cluster at once (`edh-api` active, `edh-api-drain` outgoing).
ADR 0005's single-instance assumption doesn't say how a client finds the *right* one of the two.

## Decision

- Each server process gets a **stable** `instance_id` (`Settings::instance_id`), set once per
  Deployment via `INSTANCE_ID=edh-api` / `INSTANCE_ID=edh-api-drain` — **not** the pod name. Pod
  names change on restart/reschedule and would invalidate a cookie bound to one.
- On `POST /tables/v1`, `POST /tables/join/v1`, and any response that binds a user to a table, the
  server sets a **host-only** affinity cookie (no `Domain` attribute) on the public origin
  (`edh.example.com` via the SolidStart `/api` BFF):
  ```
  Set-Cookie: mtgfr-instance=<instance_id>; Path=/; Secure; SameSite=Lax; HttpOnly
  ```
- `edh-proxy` (nginx, `iac/proxy.tf`) reads `mtgfr-instance` and routes to the matching
  Service, defaulting to the **active** instance (`edh-api`) when the cookie is absent or names a
  peer that no longer exists. Public traffic reaches the proxy only through `edh-web` (`API_UPSTREAM`).
- If a request lands on the wrong instance (cookie points at a peer that doesn't have the table),
  the server returns `404`/`410`; the client's existing stream-reconnect path drops the stale
  cookie and retries.
- The affinity cookie is independent of the **auth session** cookie: both are host-only on edh
  when `COOKIE_DOMAIN` is empty; `mtgfr-instance` ignores `cookie_domain` either way. The BFF
  forwards `Cookie` / `Set-Cookie` on the ClusterIP hop to nginx.
- Terraform models the outgoing peer as a second Deployment/Service (`edh-api-drain`), gated by a
  variable (`api_drain_enabled`) rather than always present — it exists only for the duration of a
  roll (`iac/api.tf`).

## Consequences

- No `table_id` needed in every path or a routing service registry — a cookie carries the
  binding, and nginx's `map $cookie_mtgfr_instance` (not the nginx Plus `sticky` directive) is
  enough OSS-friendly stickiness for a two-instance topology.
- Extends, doesn't replace, ADR 0005: fan-out inside one process is still `tokio::broadcast`;
  affinity only decides *which* process a request reaches. The Redis scale-out path ADR 0005
  flags for the future would need a different affinity story (shared table registry) — out of
  scope here, which stays a two-instance rolling window (deploy PRD non-goals).
- Wire contract must tolerate the two instances briefly running different server versions (N on
  `edh-api-drain`, N+1 on `edh-api`) — see [docs/WIRE_COMPAT.md](../WIRE_COMPAT.md) for the
  expand-only rules this affinity model depends on.
- `edh-web` is deliberately held back until drain empties (deploy PRD §Client/server roll order):
  affinity fixes *API* routing, but a mid-game page refresh still needs the SPA that matches the
  API version its table cookie points at, and only the previous SPA build is guaranteed to.
- Admin/drain endpoints (`POST /admin/drain`, `GET /health/drain`) are deliberately **not** routed
  by the sticky proxy at all — `iac/network-policy.tf` blocks `cloudflared` from reaching
  `edh-api`/`edh-api-drain` (and `edh-proxy` is only reachable from `edh-web`), and
  `iac/proxy.tf` 404s those paths even if something hits the proxy. The apply machine reaches
  them only via `kubectl port-forward`.
