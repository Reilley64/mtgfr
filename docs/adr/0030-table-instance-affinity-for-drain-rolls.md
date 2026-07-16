# 0030 — Table/instance affinity for drain rolls

Status: **Accepted**; extends [0005](0005-in-process-fanout-ndjson-snapshot.md) (single-instance fan-out) to an **N-instance** rolling deploy (1 active + many draining); deploy PRD §Table / instance affinity and §Rolling deployment model.

## Context

ADR 0005 assumes one server process per table for the life of that table — `tokio::broadcast` and
the in-memory `Registry` (ADR 0021) are process-local, so a table's SSE stream and game state live
and die with the pod that created it. Rolling deploy needs new server binaries without killing
tables still running on older ones. A single drain slot cannot nest rolls: shipping again while a
peer is still draining would replace that peer and wipe its tables. Affinity must therefore scale
to **multiple concurrent draining instances**.

## Decision

- Each release is its own Kubernetes Deployment+Service whose name is a stable **`INSTANCE_ID`**
  derived from the image tag (e.g. `ghcr.io/…/mtgfr-server:1.2.3` → `edh-api-1-2-3`) — **not** the
  pod name. The image on a draining Deployment is never rewritten (that would restart the pod).
- Exactly one instance is **active**, derived from operator `server_image` (`edh-api-<slug(tag)>`);
  it accepts new tables. Drain peers are **not** listed in tfvars — scripts pass `api_peer_images`
  from Terraform outputs on every apply. Live drain is `POST /admin/drain` (never env/image churn).
- Cap: `api_max_instances` (default 4). A nested roll that would exceed the cap waits until a
  drain peer reports `active_tables=0` and is removed from the peer map.
- On table create/join (and other bind responses), the server sets a host-only affinity cookie:
  ```
  Set-Cookie: mtgfr-instance=<instance_id>; Path=/; Secure; SameSite=Lax; HttpOnly
  ```
- **SolidStart BFF** (`edh-web`, `API_UPSTREAMS` + `API_ACTIVE_INSTANCE_ID`) routes `/api/*` by that
  cookie (unknown/missing → active). `POST /tables/join/v1` **fans out** across all live peers
  until a non-`UnknownTable` lobby response (so cookieless guests can join a table on a drain
  peer); the winning `Set-Cookie` pins later requests. Public `/api/admin/*` and
  `/api/health/drain` return 404 after path normalization (reject `..`); apply-machine drain uses
  `kubectl port-forward` to the instance Service.
- Deploy cutover (`just deploy`): stage the new image as a peer while `server_image` stays on the
  previous tag → live-drain the previous active → flip `server_image` (old becomes a peer). GC
  removes a peer only when `active_tables=0` (never on probe failure). Cap waits time out
  (`API_CAP_WAIT_SECONDS`). Non-roll applies use `just tf-apply` so peers are not wiped.
- There is **no** nginx sticky proxy. NetworkPolicy: `cloudflared` → `edh-web` only; `edh-web` →
  pods labeled `mtgfr.io/component=api`.
- Auth session cookies are host-only on edh when `COOKIE_DOMAIN` is empty; the BFF forwards
  `Cookie` / `Set-Cookie` to the chosen API Service.
- `edh-web` image bumps only when **zero** drain peers remain (SPA pairing with mid-game tables).

## Consequences

- Nested API rolls are safe up to the cap: older drain peers keep their process memory until empty.
- Expand-only wire rules must hold across **all** concurrent instance versions until GC.
- Horizontal replicas of the same `INSTANCE_ID` remain out of scope (still one pod per instance).
- Redis / shared table registry remains future work (ADR 0005).
