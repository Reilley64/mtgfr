# Deployment PRD

Deploy mtgfr to a **Kubernetes cluster** reached via a **Cloudflare Tunnel** (no inbound ports on the cluster), with infrastructure-as-code, semver releases, and **draining** rolling deploys so in-progress games are not killed mid-hand.

## Goals

1. **Reachable by friends** — single public origin `https://edh.example.com` (SolidStart BFF: SPA + `/api` proxy); stable share links on that host.
2. **Reproducible infra** — Terraform in **this repo** (`iac/`) owns the edh namespace workloads, Postgres, Cloudflare Tunnel/DNS, and deploy config. `terraform apply` is the deploy path.
3. **Zero-downtime for active games** — during a release, players at a live table stay on the old server binary until that table is empty (game over or everyone left).
4. **Traceable releases** — every merge to `main`/`master` runs [semantic-release](https://semantic-release.org/) (default config) on green verify; container images build when the resulting `v*` tag is pushed (`docker.yml`).

## Non-goals (for this PRD)

- Multi-node horizontal scale of the **same** `INSTANCE_ID` / Redis shared registry (ADR 0005) — still future work. **Nested rolls** with multiple draining versioned instances (capped) **are** in scope.
- Durable game resume across server restart (explicitly out per ADR 0021).
- Managing the Kubernetes control plane / node OS (cluster bootstrap is assumed done; this repo deploys *into* the cluster).
- Automated card-art CDN deploy (ADR 0015; `cards.example.com` remains separate).
- Homelab Docker / Traefik — **retired** as the edh hosting path.

## Context (architecture constraints)

| Concern | Current state | Deploy implication |
|--------|---------------|-------------------|
| Live games | In-memory `Registry` per process (ADR 0021) | Concurrent versioned API instances **must not share** a table; need **table/instance affinity** (`mtgfr-instance`). |
| Fan-out | `tokio::broadcast` in-process (ADR 0005) | SSE streams are tied to the pod that owns the table. |
| Durable data | Postgres: users, sessions, decks (ADR 0010) | In-cluster Postgres; `DATABASE_URL` on every API instance. |
| Client | SolidStart 1.3 (Vinxi, `ssr: false`); same-origin `/api` BFF | Browser always calls `/api`; BFF sticky-routes by cookie to `API_UPSTREAMS`, strips `/api`. **Do not roll `edh-web` until zero drain peers remain.** |
| Dev DB bootstrap | `push_schema()` at boot (`db.rs`) | **Toasty migrations** in prod/CI; `push_schema` only for in-memory SQLite tests (per [Toasty schema guide](https://tokio-rs.github.io/toasty/nightly/guide/schema-management.html)). |
| Bind address | `127.0.0.1:8080` hard-coded | Load listen host/port from a `Settings` struct via the [`config`](https://docs.rs/config) crate. |
| Public edge | Was Traefik on homelab LAN | **Cloudflare Tunnel** (`cloudflared`) in-cluster; no public NodePort / LoadBalancer required for edh. |
| Cluster | Home **k3s** on a dedicated host | Terraform runs on a **different machine** (workstation / laptop), talking to the k3s API over the network via kubeconfig. Cluster bootstrap stays out of this repo. |

## Target topology

```
Apply machine (NOT the k3s host): terraform apply  (mtgfr/iac/)
    │  network → k3s API (kubeconfig)
    │  kubernetes/helm providers → remote cluster
    │  cloudflare provider → DNS + Zero Trust tunnel (Terraform-managed)
    │  state → kubernetes backend (Secret + Lease on the k3s cluster)
    ▼
Home k3s host (separate machine)
    │
    ├─ Namespace: terraform          (tfstate Secret + lock Lease)
    ├─ Namespace: edh
    │     ├─ Deployment edh-web              (SolidStart Node BFF, distroless)
    │     ├─ Deployment edh-api-<ver>…       (1 active + 0..N-1 draining; cap api_max_instances)
    │     ├─ Job edh-migrate                 (server migration apply, before new active)
    │     ├─ StatefulSet postgres            (official postgres image; mtgfr DB)
    │     └─ Deployment cloudflared          (tunnel connector)
    │
Internet ── Cloudflare (DNS + Tunnel) ──► cloudflared ──► edh-web:8080
                                              │
                                   /api/* cookie sticky (BFF)
                                              │
                         ┌────────────────────┼────────────────────┐
                         ▼                    ▼                    ▼
                  edh-api-1-2-0         edh-api-1-1-0         edh-api-1-0-0
                    (active)              (drain)               (drain)
```

**Apply machine vs cluster host:** `terraform` / `kubectl` always run on the apply machine. That machine needs network reachability to the k3s API server (LAN, VPN, or Tailscale — whatever you already use for remote kubeconfig). Do **not** install or run Terraform on the k3s node for this stack. Drain orchestration uses `kubectl port-forward` from the apply machine to in-cluster Services (still not via the public tunnel).

**Hostname** (Cloudflare `example.com` zone):

| Host | Serves | Tunnel ingress |
|------|--------|----------------|
| `edh.example.com` | SolidStart SPA + `/api` BFF (sticky to versioned API Services) | `http://edh-web.edh.svc:8080` |

Cookie sticky lives in **SolidStart** (`API_UPSTREAMS` JSON map + `API_ACTIVE_INSTANCE_ID`). Browser → tunnel → `edh-web` → chosen `edh-api-*` Service. NetworkPolicy: `cloudflared` → `edh-web` only; `edh-web` → pods with `mtgfr.io/component=api`.

Browser paths keep the `/api` prefix; the BFF strips it before Axum (routes are `/auth/...`, `/tables/...`, not `/api/auth/...`). Public `/api/admin/*` and `/api/health/drain` are 404'd by the BFF.

TLS terminates at **Cloudflare** (tunnel). In-cluster traffic is HTTP to ClusterIP Services (no mTLS between `cloudflared` and apps — accepted for friend-group v1). Pods do not need ACME or Traefik.

## Terraform (this repo)

Infrastructure lives in **`iac/`** in the mtgfr repo.

### State — Kubernetes backend

Use Terraform’s [**kubernetes** backend](https://developer.hashicorp.com/terraform/language/backend/kubernetes): state is a Secret, locks are Leases, stored **on the remote k3s cluster**. The apply machine never keeps prod state on its local disk after `init`; every plan/apply reads/writes state through the k3s API (same kubeconfig as the providers).

```hcl
# iac/backend.tf
terraform {
  backend "kubernetes" {
    secret_suffix = "mtgfr"
    namespace     = "terraform"
    config_path   = var.kubeconfig_path # or KUBE_CONFIG_PATH — points at remote k3s
  }
}
```

**Bootstrap (one-time, from the apply machine, before first `init` with this backend):**

```bash
kubectl --kubeconfig "$KUBE_CONFIG_PATH" create namespace terraform
# RBAC: the kubeconfig user needs secrets + coordination.k8s.io/leases in that namespace
```

Then `terraform init` on the apply machine. Prefer `KUBE_CONFIG_PATH` (or `-backend-config`) so the kubeconfig path is not baked into plan files.

**Trade-offs (accepted):** state lives with the cluster — if k3s/etcd is gone, so is state (and the workloads). Keep k3s etcd (or datastore) backups. State Secret size is fine for this stack (well under the ~1MiB Secret limit). Not for multi-cloud or multi-operator fleets. The apply machine must be able to reach the k3s API whenever you plan or apply.

### Providers

```hcl
# iac/providers.tf
provider "kubernetes" {
  # Remote k3s — never assumes Terraform runs on the cluster node
  config_path = var.kubeconfig_path
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

# Helm is not used — Postgres is a plain StatefulSet (postgres.tf).
provider "helm" {
  kubernetes {
    config_path = var.kubeconfig_path
  }
}
```

No Docker-over-SSH provider. No Traefik labels. No homelab data sources. No `in_cluster_config` — Terraform is not run as a pod in the cluster.

### Cloudflare Tunnel

mtgfr Terraform **fully owns** the Zero Trust tunnel, public hostname routes, DNS, and the in-cluster `cloudflared` Deployment + credentials Secret. No manual tunnel creation in the Cloudflare UI for steady state.

| Resource | Notes |
|----------|-------|
| Cloudflare Tunnel | Created/managed in Terraform |
| Tunnel config / ingress rules | `edh` → `edh-web` Service only (BFF proxies `/api` in-cluster) |
| Tunnel token / credentials | K8s Secret consumed by `cloudflared` |
| `cloudflare_dns_record` | `edh` → `<tunnel-id>.cfargotunnel.com`, **proxied** |

`cloudflared` runs in-cluster (Deployment, 2 replicas for connector HA). It authenticates with the tunnel token and forwards Cloudflare edge traffic to ClusterIP Services. The cluster needs **egress** to Cloudflare; it does **not** need inbound public ports for edh.

### Postgres — official image StatefulSet

Install a single-primary **Postgres StatefulSet** (`postgres:17` or pinned tag) into namespace `edh` with a PVC on k3s local-path (or whatever StorageClass the node has). Create role/database `mtgfr` via the image’s `POSTGRES_*` env. `DATABASE_URL` points at the Service DNS name `postgres`.

Skip CloudNativePG / Bitnami for v1 — more operators (and Bitnami’s image-catalog churn) than this friend-group deploy needs. **Backups for v1:** rely on k3s / PVC snapshots (and etcd/datastore backups that already protect cluster state). No separate Postgres dump cron until we need it.

### Sticky routing — SolidStart BFF

`edh-web` sets `API_UPSTREAMS` (JSON map of `INSTANCE_ID` → ClusterIP URL) and `API_ACTIVE_INSTANCE_ID`. The `/api` route reads `mtgfr-instance`, forwards to that Service (or the active id), strips `/api`, and streams SSE without buffering. Dev with an empty map falls back to `http://127.0.0.1:8080`.

### What mtgfr Terraform owns

| Resource | Notes |
|----------|-------|
| Cloudflare Tunnel + DNS | Single `edh` hostname → `edh-web` |
| `kubernetes_namespace.terraform` | Optional if bootstrapped by hand; state Secret/Lease live here |
| `kubernetes_namespace.edh` | Isolation boundary for workloads |
| `edh-web` | SolidStart BFF + sticky (`API_UPSTREAMS`) |
| Versioned `edh-api-*` Deployments + Services | Operator `server_image`; peers in ConfigMap `edh-api-peers`; cap `api_max_instances` |
| `edh-migrate` Job | `server migration apply` before new active (name hashed from active image) |
| StatefulSet `postgres` | Official `postgres` image; dedicated `mtgfr` role/DB; backups = k3s/PVC snapshots |
| NetworkPolicy | tunnel→web; web→api; api+migrate→postgres |
| Secrets | `DATABASE_URL`, tunnel token, admin token, etc. |
| `cloudflared` Deployment + Secret | Tunnel connector |

### Variables & secrets

`iac/terraform.tfvars` (gitignored) or env vars:

| Variable | Purpose |
|----------|---------|
| `kubeconfig_path` | Path on the **apply machine** to a kubeconfig that reaches remote k3s |
| `cloudflare_api_token` | DNS + Zero Trust tunnel |
| `mtgfr_db_password` | `DATABASE_URL` (composed in Terraform) |
| `auth_secret` | reserved — session signing if added later |
| `server_image` / `web_image` | Desired active API + web images (`just deploy` owns drain peers) |

### Apply

From the **apply machine** (repo checkout + Terraform CLI + network access to k3s API and Cloudflare):

```bash
export KUBE_CONFIG_PATH=~/.kube/config   # example — remote k3s
cd iac
terraform init
terraform apply          # safe during drain: peers live in ConfigMap edh-api-peers
just deploy              # roll to tfvars server_image / web_image (or SERVER_IMAGE / WEB_IMAGE env)
```

`just deploy` stages the new API, live-drains the previous active, flips `server_image`, GCs empty peers, then bumps web when no peers remain.

## DNS & Cloudflare

DNS for the public host is **owned by mtgfr Terraform** (with the tunnel resources in `tunnel.tf`):

| Record | FQDN | Type | Target |
|--------|------|------|--------|
| `edh` | `edh.example.com` | CNAME | `<tunnel-id>.cfargotunnel.com` |

**Proxy mode:** Tunnel hostnames are **proxied (orange cloud)** — that is how Cloudflare Tunnel works. TLS is at the edge; origin is the in-cluster `cloudflared` connector.

**SSE through Cloudflare (required):** Tunnel hostnames are orange-clouded, so Cloudflare's ~100s HTTP idle timeout applies. The server **must** send periodic SSE comments / keepalive events (e.g. every 15–30s) on `GET /tables/{table}/stream/v1` before phase 5 exit. Configuration Rules disable response buffering on `edh.example.com` so the BFF stream is not held at the edge. If keepalives still drop streams in playtests, revisit (WebSocket upgrade or Cloudflare settings).

**Table share links** use `https://edh.example.com/play/XXXXXX` — stable across deploys. Legacy `?table=` query links still parse on join.

## Rolling deployment model

### Definitions

| Term | Meaning |
|------|---------|
| **Active table** | A `table_id` still in the in-memory registry that counts for drain: a **started** game not yet torn down, **or** a lobby with ≥1 claimed seat that has not exceeded the idle lobby TTL. |
| **Drain mode** | Instance accepts traffic only for tables it already owns; rejects new table creation. |
| **Idle lobby TTL** | **30 minutes** since last lobby activity (seat claim/vacate, ready toggle, deck select, etc.). When the TTL fires with no started game, the table is removed and no longer counts as active. |
| **Finished** | Table removed because the game ended and no seats remain claimed, **or** the idle lobby TTL expired, **or** all seats vacated with no game. |

### Flow

```mermaid
sequenceDiagram
    participant TF as Terraform_deploy_script
    participant BFF as edh_web_BFF
    participant Old as edh_api_1_1_0
    participant New as edh_api_1_2_0
    participant Web as edh_web
    participant PG as Postgres

    Note over Web: Hold previous image until<br/>zero drain peers remain
    TF->>PG: Migrate Job (active image)
    TF->>New: Create Deployment (old stays active)
    TF->>BFF: API_UPSTREAMS includes New; active=Old
    TF->>Old: POST admin/drain live toggle
    Note over Old: Reject new tables<br/>Keep existing SSE
    TF->>BFF: Flip active=New
    Note over TF: Nested roll may add another<br/>peer before Old empties
    TF->>Old: GC when active_tables=0 only
    TF->>Web: Bump edh-web when only active remains
```

1. **Migrate**, then **stage** the new image in ConfigMap `edh-api-peers` while `server_image` stays on the previous tag. **Hold `edh-web`**.
2. **Mark previous active draining** via live `POST /admin/drain` (port-forward). Never flip `DRAIN` env / rewrite image.
3. **Flip** `server_image` to the new tag (old active moves into `edh-api-peers`).
4. **Affinity:** BFF routes `mtgfr-instance` to the matching Service; join fans out for cookieless guests.
5. **Nested rolls** allowed until `api_max_instances`; at cap, wait (with timeout) for a peer to empty and GC it first.
6. **GC** peers only when `active_tables=0` (never on probe failure); remove from `edh-api-peers`.
7. **Bump `edh-web`** only when the peer ConfigMap is empty.

**Client/server roll order (locked):** API rolls may nest; web only after **all** drain peers are gone. Expand-only wire across the whole concurrent set.

### Wire backwards compatibility (required doc)

Roll order reduces mid-game refresh skew; it does **not** remove the need for wire rules. During the drain window, **old SPA ↔ new API** still happens for new tables. Document durable backwards-compatibility rules for the OpenAPI / `crates/schema` contract (new ADR or short doc under `docs/`, linked from AGENTS.md and this PRD). At minimum cover:

1. **Compatibility window** — all concurrent instance versions until each drain peer GCs; no longer multi-version support required beyond that set.
2. **Expand-only during that window** — additive optional fields (`#[serde(default)]`), new endpoints, new intent/event variants the old client never sends; no rename/remove/type-change of wire fields until the prior API is gone (mirror Postgres migration rule).
3. **Hard breaks** — bump path version (`/v2`), run both until drain completes, then remove `/v1`; use sparingly.
4. **SSE / snapshots** — same expand-only rule on `VisibleState` and stream frames; do not rearrange discriminators mid-window.
5. **Authoring habit** — run `just server-codegen` with schema changes; prefer optional fields first, tighten only after a full drain cycle if desired.

This doc is an implementation deliverable of the deploy work, not optional follow-up.

### Table / instance affinity

ADR 0005 assumes a single instance. Nested rolling deploy requires:

1. Each server process has a **stable** `instance_id` = Deployment/Service name (e.g. `edh-api-1-2-3` from the image tag). **Not** the pod name.
2. On table bind responses, the server sets a **host-only** affinity cookie on edh:
   ```
   Set-Cookie: mtgfr-instance=<instance_id>; Path=/; Secure; SameSite=Lax; HttpOnly
   ```
3. **SolidStart BFF** routes on `mtgfr-instance` via `API_UPSTREAMS`; missing/unknown → `API_ACTIVE_INSTANCE_ID`. `POST /tables/join/v1` fans out across peers until the table is found (cookieless guests).
4. Wrong-instance joins surface as lobby `UnknownTable` and are retried on other peers by the BFF; stale cookies for GC'd peers fall through to active (then fan-out on join).

**Session cookies** (auth) are host-only on `edh.example.com` when `COOKIE_DOMAIN` is empty.

### Graceful shutdown

On `SIGTERM` (K8s pod termination):

1. Enter drain mode (same as live drain toggle).
2. Stop accepting new tables immediately.
3. Keep the process alive while `active_tables > 0` (poll every few seconds; configurable timeout with loud logging).
4. Close idle HTTP connections; **do not** cut active SSE streams until the table is finished or the hard timeout fires (hard timeout is a last resort — prefer waiting).
5. Prefer an explicit pre-delete drain wait (`active_tables=0`) over relying on a huge `terminationGracePeriodSeconds`. If a draining instance is still up after **24 hours**, **log loudly** (error-level, repeating) but do not auto-kill for v1 — operator decides. Force-kill after grace period is a last resort, not the happy path.

## Server changes required

Track as implementation work; not part of Terraform alone.

### Configuration (`config` crate)

All runtime/infra settings load through one **`Settings`** struct in `crates/server/src/settings.rs`, deserialized via the [`config`](https://docs.rs/config) crate. Replace ad-hoc `std::env::var` calls (`mtgfr.rs`, `auth.rs`, etc.) with `Settings::load()` at startup; pass `&Settings` (or an `Arc<Settings>`) into `AppState` where handlers need deploy flags.

**Source precedence** (later wins):

1. Built-in defaults (local dev ergonomics)
2. Optional `config/mtgfr.toml` in the repo (committed non-secret defaults)
3. Environment variables (what Terraform/K8s set in prod) — plain names, no prefix

```rust
// Illustrative — not the final API
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub host: String,           // default "127.0.0.1"; prod "0.0.0.0"
    pub port: u16,              // default 8080
    pub database_url: String,
    pub instance_id: String,    // stable per Deployment, e.g. "edh-api" — not pod name
    pub drain: bool,            // default false; also toggled live via admin endpoint
    pub cookie_secure: bool,    // default false; true behind HTTPS (Cloudflare)
    pub cookie_domain: String,   // default ""; prod ".example.com" for auth session only
    pub cors_origin: String,     // default ""; prod "https://edh.example.com"
    pub version: String,        // from VERSION or compile-time CARGO_PKG_VERSION
}

impl Settings {
    pub fn load() -> Result<Self, config::ConfigError> {
        // Prefer explicit env keys / a prefix. Avoid Environment::default().separator("_")
        // alone — it splits DATABASE_URL on `_` and breaks nested key parsing.
        config::Config::builder()
            .set_default("host", "127.0.0.1")?
            .set_default("port", 8080)?
            .set_default("drain", false)?
            .set_default("cookie_secure", false)?
            .add_source(config::File::with_name("config/mtgfr").required(false))
            .add_source(
                config::Environment::default()
                    .separator("__") // nested keys only; flat DATABASE_URL still works as one key
            )
            .build()?
            .try_deserialize()
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
```

**`config/mtgfr.toml`** (committed, local dev):

```toml
host = "127.0.0.1"
port = 8080
database_url = "postgresql://mtgfr:mtgfr@localhost:5432/mtgfr"
cookie_secure = false
cookie_domain = ""
cors_origin = ""
drain = false
```

No secrets in the TOML file — production credentials come only from Terraform-injected env / Secrets.

**Environment mapping** (Terraform / Deployment env on `edh-api`):

| `Settings` field | Env var | Prod value |
|------------------|---------|------------|
| `host` | `HOST` | `0.0.0.0` |
| `port` | `PORT` | `8080` |
| `database_url` | `DATABASE_URL` | `postgresql://mtgfr:<secret>@postgres:5432/mtgfr` |
| `instance_id` | `INSTANCE_ID` | stable Deployment id, e.g. `edh-api-1-2-3` |
| `drain` | `DRAIN` | startup default `false`; live drain via admin API, not Deployment env churn |
| `cookie_secure` | `COOKIE_SECURE` | `true` |
| `cookie_domain` | `COOKIE_DOMAIN` | empty (host-only session cookie on edh) |
| `cors_origin` | `CORS_ORIGIN` | empty (browser is same-origin via BFF) |
| `admin_token` | `ADMIN_TOKEN` | shared secret guarding `/admin/drain` + `/health/drain`; empty leaves them unauthenticated behind the NetworkPolicy |
| `version` | `VERSION` | image release tag |

`RUST_LOG` stays a standard tracing env var (not part of `Settings`) — set alongside the above in Terraform.

Affinity cookie value comes from `settings.instance_id` and is **host-only** (ignore `cookie_domain` for `mtgfr-instance`). Auth session cookies are also host-only when `cookie_domain` is empty. Health endpoints expose `settings.version` and `settings.drain`.

## Database migrations

Use Toasty's built-in [migration system](https://tokio-rs.github.io/toasty/nightly/guide/schema-management.html) — not refinery or hand-rolled SQL runners. Migrations are **generated from registered model diffs**, stored under `toasty/`, and applied via `toasty-cli`.

### Scope

| Object | Today | After migrations |
|--------|-------|------------------|
| `users`, `sessions`, `decks` | `push_schema()` in `db::connect` | `User` / `Session` / `Deck` models → `migration generate` → committed SQL in `toasty/migrations/` |
| `catalog_cards` | `CREATE TABLE IF NOT EXISTS` in `catalog_search::project` | DDL in the initial migration (append to generated SQL, or register a model so `generate` picks it up); **data** still refreshed by `project()` on boot |
| Future schema changes | `push_schema` / manual SQL | edit models → `migration generate --name …` → review SQL → commit |

`catalog_search::project()` remains a runtime **data** projection (truncate + reinsert). Remove `CREATE TABLE` from `project()` once migrations own the DDL.

### Tooling

Add `toasty-cli` and bump `toasty` to **0.8+** (migration CLI requires the version aligned with the guide). Layout:

```
Toasty.toml                  # [migration] path, prefix_style, …
toasty/
  history.toml
  migrations/
    0000_initial.sql         # generated; review before commit
  snapshots/
    0000_snapshot.toml
crates/server/
  src/
    main.rs                  # serve | openapi | static | migration …
```

**`Toasty.toml`** (repo root):

```toml
[migration]
path = "toasty"
prefix_style = "Sequential"
checksums = false
statement_breakpoints = true
```

**`crates/server/src/main.rs`** — single CLI. The `migration` subcommand forwards to [`toasty_cli::ToastyCli`](https://tokio-rs.github.io/toasty/nightly/guide/schema-management.html#setting-up-the-cli): load `Config` from `Toasty.toml`, connect with `DATABASE_URL`, register `User` / `Session` / `Deck` (and any future models), then `parse_from(["mtgfr", "migration", …])`.

| Command | When |
|---------|------|
| `migration generate [--name …]` | After model changes — diffs against last snapshot, writes SQL + snapshot |
| `migration apply` | Deploy, CI, local dev — runs pending migrations (`__toasty_migrations` table) |
| `migration snapshot` | Inspect current model schema as TOML |

**`just migrate`** → `cargo run -p server -- migration apply` (same locally, CI, and containers).

**`db::connect`** — remove `push_schema()` for Postgres; assume schema is current (`migration apply` ran first). Keep `push_schema()` behind `#[cfg(test)]` or sqlite-only for `sqlite::memory:` tests.

### Authoring workflow

1. Change `User` / `Session` / `Deck` (or add a model).
2. `cargo run -p server -- migration generate --name describe_change`
3. Review `toasty/migrations/000N_….sql` (Toasty emits Postgres-specific DDL when connected to Postgres).
4. Commit migration SQL, snapshot, and `history.toml` with the code change.
5. `just migrate` locally; CI and deploy run `migration apply` only.

First-time bootstrap: `migration generate --name initial` against Postgres, add `catalog_cards` DDL to the generated file if not covered by models, commit, then `migration apply`.

### Migration rules

1. **Forward-only** — no `migration drop` on applied history in prod; fix forward with `generate` + `apply`.
2. **Expand-only during rolling deploys** — nullable/default new columns; no rename/drop until the draining instance is gone.
3. **One writer** — only the migrate Job runs `migration apply`; API pods never call `push_schema` or DDL.
4. **Generated SQL is source of truth** — hand-edit generated files only when necessary (e.g. `catalog_cards`); prefer model changes + `generate`.

### Deploy integration

```
terraform apply / roll script
    │
    ├─ 1. Job: mtgfr-server:<tag> server migration apply
    ├─ 2. roll / update edh-api Deployment(s)  (web_image unchanged)
    ├─ 3. catalog projection runs on server boot (data only)
    ├─ 4. GC drain peers with active_tables=0; bump web when none remain
    └─ 5. bump edh-web to the same release tag
```

**Terraform** — Kubernetes Job before API roll. Use `generate_name` (not a fixed name with the image tag): Job names are immutable, and tags like `1.2.3` are awkward/invalid as sole DNS-1123 names. Wait for completion before updating API Deployments.

```hcl
resource "kubernetes_job" "edh_migrate" {
  metadata {
    generate_name = "edh-migrate-"
    namespace      = kubernetes_namespace.edh.metadata[0].name
  }
  wait_for_completion = true
  timeouts { create = "10m" }
  spec {
    template {
      spec {
        container {
          name    = "migrate"
          image   = var.server_image
          command = ["/server", "migration", "apply"]
          env {
            name  = "DATABASE_URL"
            value = "postgresql://mtgfr:${var.mtgfr_db_password}@postgres:5432/mtgfr"
          }
        }
        restart_policy = "Never"
      }
    }
    backoff_limit = 1
  }
}
```

Image must include the `server` binary, `Toasty.toml`, and committed `toasty/` tree. API Deployments depend on the migrate Job completing successfully.

### CI & local dev

```bash
docker compose up -d postgres
just migrate
cargo run -p server
```

CI: Postgres service → `just migrate` → `just check`.

- Server may enable **CORS** for `cors_origin` when set; same-origin BFF leaves it empty (browser does not need CORS).
- Auth **session** cookies are host-only on `edh.example.com` so `fetch(..., { credentials: "include" })` to `/api` sends them.
- Affinity cookie `mtgfr-instance` is host-only on `edh.example.com`; the BFF routes by that cookie (and fans out `POST /tables/join/v1` across peers when the guest has no sticky cookie).

### Client (production build)

Same-origin `/api` always — no separate API hostname bake:

| Env | Dev (`bun run dev`) | Prod (runtime) |
|-----|---------------------|----------------|
| (API origin) | `/api` → BFF → localhost (empty `API_UPSTREAMS`) | `/api` → BFF sticky (`API_UPSTREAMS` + `API_ACTIVE_INSTANCE_ID`) |
| `VITE_CARD_CDN` | optional | optional build-arg |

`client/src/effect/client.ts` always prepends `/api`. SSE stream URL follows the same origin.

### Other server work

| Item | Detail |
|------|--------|
| `Settings::load()` | Called once in `mtgfr serve`; bind `settings.listen_addr()`. |
| CORS middleware | Axum layer: allow `cors_origin`, credentials, needed methods/headers. |
| Cookie `Domain` | Auth session: empty `cookie_domain` (host-only on edh). Affinity `mtgfr-instance`: host-only. |
| `POST /admin/drain` | Live drain toggle — must not restart the pod. **Not on the public tunnel hostname.** NetworkPolicy blocks ingress from `cloudflared` / public paths. The apply machine reaches it via `kubectl port-forward` (through the k3s API), not by opening NodePorts on the k3s host. |
| `GET /health/live` | `200` if process is up; body includes `version`. |
| `GET /health/ready` | `200` if accepting traffic (not draining, or draining with tables — still "ready" for those tables). |
| `GET /health/drain` | JSON `{ "active_tables": N, "draining": bool }` — polled from the apply machine via port-forward; same NetworkPolicy posture as admin. |
| Active table count | Registry helper: started games, plus lobbies still inside the **30 min** idle TTL. |
| Affinity cookie | Set on table create/join; value = stable `instance_id`; host-only. |
| Idle lobby TTL | **30 min** — tear down idle lobbies so drain can finish. |
| SSE keepalive | **Required** — periodic comment/event so Cloudflare Tunnel idle timeout does not drop streams. |
| Migrations | See [Database migrations](#database-migrations) — not at request time in prod. |

## Container images

**Policy:** production runtime images use [Google distroless](https://github.com/GoogleContainerTools/distroless) (`gcr.io/distroless/*-debian12:nonroot`). Builder stages only (`rust:bookworm`, `oven/bun`) have shells and package managers; nothing based on `alpine`, `debian:slim`, or `nginx` ships to prod.

Distroless has no shell — use ephemeral debug containers for inspection. All config is env vars + baked files (`config/mtgfr.toml`, `Toasty.toml`, `toasty/`).

### `mtgfr-server`

Multi-stage `docker/server/Dockerfile`:

1. **Build:** `rust:1-bookworm` — `cargo build -p server --release` (cache mount on `target/`).
2. **Runtime:** `gcr.io/distroless/cc-debian12:nonroot` — copy `server`, `config/mtgfr.toml`, `Toasty.toml`, `toasty/`; `EXPOSE 8080`.
   - Use `cc` (glibc) variant for default dynamically-linked release binaries. Switch to `gcr.io/distroless/static-debian12:nonroot` only if we build fully static binaries (`musl`).
3. **Entrypoints:**
   - `server serve` — API container default (`CMD ["/server", "serve"]`).
   - `server migration apply` — migrate Job (same image, override `command`).
4. **Build-arg / env:** `VERSION` at build time; runtime env from Terraform/K8s (see [Configuration](#configuration-config-crate)).

Card pool and engine are compiled in — no runtime volume for `crates/cards/data/`.

### `mtgfr-web`

SolidStart (Vinxi / Nitro `node_server`) on distroless Node — SPA (`ssr: false`) plus same-origin `/api` BFF.

Multi-stage `docker/web/Dockerfile`:

1. **Deps:** `oven/bun` — `bun install --frozen-lockfile`.
2. **Build:** `node:22-bookworm` — `vinxi build` → `.output/` (optional `VITE_CARD_CDN` build-arg).
3. **Runtime:** `gcr.io/distroless/nodejs22-debian12:nonroot` — copy `.output`; `ENV HOST=0.0.0.0 PORT=8080`; sticky map from k8s env; `CMD [".output/server/index.mjs"]`.

#### Compression / streaming

| Hop | What |
|-----|------|
| Edge (Cloudflare → browser) | Cloudflare may encode with **gzip or brotli** on the orange-cloud hostname — expected. |
| BFF → API | Do **not** buffer `text/event-stream` (SSE); Configuration Rules disable response buffering on `edh`. |
| API (`serve`) | Optional gzip on JSON later; **never** gzip SSE. |

OpenAPI codegen in the client build must use the **committed** `openapi.json` at the image tag being built (release workflow checks they match).

### What not to use

| Avoid | Use instead |
|-------|-------------|
| `debian:bookworm-slim`, `alpine` runtime | distroless (`cc` for Rust API; `nodejs22` for web) |
| `nginx:alpine` or Rust `server static` for the SPA | SolidStart Node BFF on distroless |
| Shell-based entrypoint scripts | Node `.output/server/index.mjs` / `server` binary subcommands |

## Terraform layout

**Repo:** `iac/` in **mtgfr**.

```
mtgfr/
  iac/
    backend.tf           # kubernetes backend (Secret + Lease in namespace terraform)
    providers.tf         # kubernetes + helm + cloudflare
    variables.tf
    namespace.tf          # edh (+ terraform if not bootstrapped by hand)
    web.tf               # edh-web Deployment + Service
    api.tf               # versioned edh-api-* from server_image + ConfigMap edh-api-peers
    web.tf               # SolidStart BFF + API_UPSTREAMS sticky env
    postgres.tf          # StatefulSet + Service: official postgres image
    migrate.tf           # Job: toasty migration apply (generate_name + wait)
    tunnel.tf            # Cloudflare Tunnel + cloudflared + DNS records
    network-policy.tf    # cluster-internal only for admin/drain
    secrets.tf           # DATABASE_URL, etc.
    terraform.tfvars     # gitignored secrets + image tags
  docker/
    server/Dockerfile
    web/Dockerfile
```

**State:** kubernetes backend in namespace `terraform` (see above). Do not commit local state files.

**Images:** **public** GHCR packages (`mtgfr-server`, `mtgfr-web`). k3s nodes pull without `imagePullSecrets`. Do not push a moving `latest` tag — pin explicit versions in `terraform.tfvars`.

**API Deployment env (illustrative):**

```hcl
env = [
  { name = "HOST", value = "0.0.0.0" },
  { name = "PORT", value = "8080" },
  { name = "DATABASE_URL", value_from = secret_key_ref … },
  { name = "INSTANCE_ID", value = "edh-api-1-2-3" },  # Deployment name
  { name = "DRAIN", value = "false" },          # startup only; live drain via POST /admin/drain
  { name = "COOKIE_SECURE", value = "true" },
  { name = "COOKIE_DOMAIN", value = "" },
  { name = "CORS_ORIGIN", value = "" },
  { name = "VERSION", value = var.server_image_tag },
  { name = "RUST_LOG", value = "info" },
]
```

**Web Deployment env (illustrative):**

```hcl
env = [
  { name = "HOST", value = "0.0.0.0" },
  { name = "PORT", value = "8080" },
  { name = "API_UPSTREAMS", value = "{"edh-api-1-2-3":"http://edh-api-1-2-3.edh.svc:8080"}" },
  { name = "API_ACTIVE_INSTANCE_ID", value = "edh-api-1-2-3" },
]
```

## Release & versioning

### One-time history squash

Before the first semver tag, rewrite git history to a single root commit (preserving tree). This is a **manual, coordinated** step:

1. Announce to anyone with clones.
2. Squash (e.g. `git checkout --orphan release-root && git add -A && git commit`).
3. Force-push `main` (only acceptable because this is pre-release / friend group).
4. Tag `v0.1.0` (or `v1.0.0` if treating current state as first production).

After squash, **all** commits on `main` follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `chore:`, `feat!:` / `BREAKING CHANGE:` for majors.

### Semantic versioning

[semantic-release](https://semantic-release.org/) with **default configuration** — no `.releaserc`, no custom `releaseRules`, no extra plugins. Version bumps follow semantic-release's built-in commit analyzer (Angular convention): which commits trigger major/minor/patch releases is whatever the default does; do not override it in this repo.

| Artifact | Managed by |
|----------|------------|
| Git tag | `vMAJOR.MINOR.PATCH` — `@semantic-release/github` (default) |
| `package.json` `version` | `@semantic-release/npm` (default) — root `package.json` is `private: true` (not published to npm; satisfies the default plugin) |
| GitHub Release | release notes from `@semantic-release/release-notes-generator` (default) |
| GHCR image tags | `docker.yml` on `push` of `v*` tags, tagged with the release version |
| `Cargo.toml` version | **not** managed by semantic-release — pin `VERSION` / Terraform image tags from the **git tag** / GitHub Release name |

Commits that do not warrant a release under the default analyzer produce no tag and no image build (`docker.yml` does not fire).

### GitHub Actions

Follow the official [semantic-release GitHub Actions recipe](https://semantic-release.org/recipes/ci-configurations/github-actions/): verify first, then `npx semantic-release` with `GITHUB_TOKEN`. Terraform in `iac/` consumes GHCR images built after the GitHub Release is published.

#### Workflow overview

```
pull_request ──► ci.yml                    (migrate + just check)

push to main/master ──► verify-and-release.yml
                            ├─ job: verify   (migrate + just check)
                            └─ job: release  (npx semantic-release — default config)

v* tag pushed ──────► docker.yml           (build + push GHCR images)

workstation (apply machine) ──► terraform apply (iac/) → remote k3s API
```

Three workflows: **`ci.yml`** (PRs), **`verify-and-release.yml`** (official recipe), **`docker.yml`** (images only — not part of semantic-release).

#### `ci.yml` — pull requests

Triggers: `pull_request` to `main` or `master`. Same verify steps as below; no release.

#### `verify-and-release.yml` — official recipe (adapted for Rust)

Triggers: `push` to `main` and `master`. Structure matches the [verify-and-release example](https://semantic-release.org/recipes/ci-configurations/github-actions/); only the **verify** job steps differ (Rust/Bun instead of `npm test`).

```yaml
name: Verify and Release

on:
  push:
    branches:
      - main
      - master

permissions:
  contents: read # verify job

jobs:
  verify:
    name: Verify
    runs-on: ubuntu-latest
    services:
      postgres: # … image + env for migrate
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      # rust toolchain, bun, caches
      - run: cargo run -p server -- migration apply
      - run: just check

  release:
    name: Release
    runs-on: ubuntu-latest
    needs: verify
    permissions:
      contents: write       # publish GitHub release + tag
      issues: write         # comment on released issues (default plugin behavior)
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0   # required — semantic-release needs full history
      - uses: actions/setup-node@v4
        with:
          node-version: "lts/*"
        # do NOT set registry-url — conflicts with semantic-release (see recipe pitfalls)
      - run: npm clean-install   # installs semantic-release devDependencies from package.json
      - name: Release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: npx semantic-release
```

**Do not** use `cycjimmy/semantic-release-action`, a custom `.releaserc.json`, or `@semantic-release/git` / `@semantic-release/exec` unless we later decide to depart from defaults. Versioning is entirely whatever `npx semantic-release` does out of the box.

**Root `package.json`** (release tooling only):

```json
{
  "name": "mtgfr",
  "private": true,
  "version": "0.0.0",
  "devDependencies": {
    "semantic-release": "^24"
  }
}
```

#### `docker.yml` — images on `v*` tag push

Triggers: `push` of tags matching `v*`. semantic-release must push that tag with repo secret `RELEASE_TOKEN` (PAT: `contents` + `workflow`) — default `GITHUB_TOKEN` cannot cascade workflow runs. Verify already ran `just check` on the tagged commit — this workflow only builds and pushes images.

```yaml
on:
  push:
    tags: ["v*"]

permissions:
  contents: read
  packages: write

jobs:
  docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # docker/login-action → ghcr.io (GITHUB_TOKEN)
      # build-push mtgfr-server + mtgfr-web tagged ${GITHUB_REF_NAME#v} (lowercase owner)
```

Do **not** push a moving `latest` tag — pin explicit versions in `terraform.tfvars`.

#### Repo files to add (mtgfr)

```
.github/workflows/ci.yml
.github/workflows/verify-and-release.yml
.github/workflows/docker.yml
package.json
package-lock.json
Toasty.toml
toasty/                         # history.toml, migrations/, snapshots/
crates/server/src/main.rs  # serve | openapi | static | migration
docker/server/Dockerfile    # distroless cc: server (serve + migration)
docker/web/Dockerfile       # distroless cc: server static + dist/
iac/
```

No `.releaserc.json`, `scripts/bump-version.sh`, or `CHANGELOG.md` committed by automation (release notes live on the GitHub Release).

#### Secrets & permissions

| Name | Where | Purpose |
|------|-------|---------|
| `GITHUB_TOKEN` | built-in | GHCR push in `docker.yml`; fallback for semantic-release if `RELEASE_TOKEN` unset |
| `RELEASE_TOKEN` | repo secret | PAT (`contents` + `workflow`) for semantic-release so its `v*` tag push triggers `docker.yml` |

No `NPM_TOKEN` — we are not publishing to npm (`private: true`). No `id-token: write` unless npm trusted publishing is added later.

#### End-to-end release (steady state)

1. Open PR with conventional commits; `ci.yml` runs migrate + `just check`.
2. Merge to `main`.
3. `verify-and-release.yml`: verify passes → `npx semantic-release` (default) → git tag + GitHub Release (if commits warrant a release).
4. `docker.yml` builds and pushes GHCR images when that `v*` tag is pushed (requires `RELEASE_TOKEN` for semantic-release cascades).
5. Set **`server_image`** (and optionally **`web_image`**) in `iac/terraform.tfvars` to the new release tag and run **`just deploy`** from the apply machine (stages API, drains, flips active, GCs, bumps web when peers are empty). Override with `SERVER_IMAGE` / `WEB_IMAGE` env if needed.
6. Non-roll infra changes: bare **`terraform apply`** is fine — drain peers are in ConfigMap `edh-api-peers`, not a TF variable.

### Deploy

Rolling sequence uses versioned `edh-api-*` instances with SolidStart sticky. Nested API rolls allowed up to `api_max_instances`. From the apply machine: live `POST /admin/drain` and `GET /health/drain` via `kubectl port-forward`. **Do not** bump `edh-web` until zero drain peers remain.

## Phases

| Phase | Deliverable | Exit criteria |
|-------|-------------|---------------|
| **0 — Bootstrap** | History squash, `v0.1.0` tag, conventional-commit note in AGENTS.md | Tag exists; CI green |
| **1 — Migrations** | `toasty-cli`, `Toasty.toml`, initial `migration generate`, `server migration` subcommand, drop prod `push_schema` | `just migrate` + tests green against Postgres |
| **2 — Containerize** | Distroless Dockerfiles (API `cc`, web `nodejs22` SolidStart), `config` crate + `Settings` | Local image smoke test (compose or kind) |
| **3 — CI** | `.github/workflows/ci.yml` (migrate + `just check`) | PRs and `main` run migrate then `just check` |
| **4 — Release automation** | `verify-and-release.yml` + `docker.yml`, root `package.json`, `RELEASE_TOKEN` | Merge to `main` → semantic-release (default) → `v*` tag push → GHCR images |
| **5 — Cluster + tunnel** | `iac/` from apply machine → remote k3s: Postgres StatefulSet, BFF sticky, Terraform-managed tunnel + DNS, SSE keepalives | Friends reach edh; SSE survives idle |
| **6 — Drain + affinity** | Health/drain + live `/admin/drain`, stable `INSTANCE_ID`, SolidStart BFF cookie sticky + join fan-out, N-Deployment roll; **web image held until all drain peers empty**; **wire backwards-compat doc** (ADR or `docs/`) | Deploy while a game runs on the prior version without disconnect; mid-game refresh keeps old SPA ↔ old API; schema authors have written expand/contract rules |
| **7 — Deploy ergonomics** | `just deploy` (tfvars `server_image` → drain/GC/web); peers in ConfigMap | Release → bump `server_image` → `just deploy`; bare apply OK for infra |

## Decisions (locked)

| Topic | Decision |
|-------|----------|
| Cluster | Home **k3s** on its own host; Terraform / kubectl on a **separate apply machine** via remote kubeconfig |
| Terraform state | **Kubernetes backend** on that k3s cluster (Secret + Lease in `terraform`); apply machine reaches it over the k3s API |
| Postgres | Official **`postgres` image** StatefulSet in `edh`; single primary + PVC |
| Postgres backups (v1) | **k3s / PVC snapshots** (+ existing etcd/datastore backups); no dump cron yet |
| Sticky routing | **SolidStart BFF** (`API_UPSTREAMS` + `mtgfr-instance` cookie; join fan-out for cookieless guests) |
| Tunnel | **Fully Terraform-managed** Zero Trust tunnel + DNS + in-cluster `cloudflared` |
| Idle lobby TTL | **30 minutes** — idle lobbies drop so drain can complete |
| Drain stuck >24h | **Log loudly**; no auto-kill for v1 — operator decides |
| GHCR | **Public** packages; no `imagePullSecret` on nodes |
| Admin / drain endpoints | **Not public** (NetworkPolicy blocks tunnel); apply machine uses `kubectl port-forward` |
| Client vs API roll order | **API (+ drain) first; `edh-web` only after `active_tables=0`** — avoids new SPA ↔ draining old API on refresh |
| Wire backwards compatibility | **Documented rules required** (expand-only across N↔N+1 drain window; `/v2` for hard breaks) — see [Wire backwards compatibility](#wire-backwards-compatibility-required-doc) |
| Static asset compression | Cloudflare edge gzip/brotli OK; **no SSE buffering/gzip** through the BFF |

## Open questions

None blocking implementation. Refine which lobby events reset the 30 min TTL if playtesting shows false drains.

## Success criteria

- [ ] `https://edh.example.com/` loads the client; `/api` reaches Axum via the BFF; auth, deck builder, and lobby work against prod Postgres.
- [ ] Traffic reaches the cluster via Cloudflare Tunnel only (no public NodePort/LB required for edh).
- [ ] Auth + affinity cookies are host-only on edh; BFF routes `mtgfr-instance` and fans out join when the cookie is missing.
- [ ] SSE keepalives keep streams alive through the tunnel under idle play (≥2 min without user actions).
- [x] `just migrate` runs Toasty `migration apply` on dev Postgres; server starts without `push_schema`.
- [ ] `terraform apply` from the apply machine (not the k3s host) uses the kubernetes backend on home k3s and reproduces the edh stack; plan fails clearly if the cluster/tunnel prerequisites are missing.
- [ ] Migrate Job (`generate_name` + wait) completes before `edh-api` rolls; schema version matches the deployed image.
- [ ] Live `POST /admin/drain` marks the old Deployment draining **without** restarting it; `GET /health/drain` is polled from the apply machine via port-forward; neither is reachable via the public tunnel.
- [ ] Idle lobbies expire after 30 minutes and stop blocking drain.
- [ ] Deploying `vX.Y.Z+1` while a four-player game runs on `vX.Y.Z` completes without SSE drop or `UnknownAction` spikes.
- [ ] During that roll, `edh-web` stays on `vX.Y.Z` until drain empties; a mid-game refresh still serves the old SPA against the draining API; web bumps to `vX.Y.Z+1` only afterward.
- [x] Wire backwards-compatibility rules are documented (ADR or `docs/`) and linked from AGENTS.md — expand-only across the drain window, `/v2` for hard breaks.
- [ ] Old server pods exit within minutes after the last table clears (or stay up harmlessly if a game runs long; loud logs after 24h draining).
- [ ] Every production deploy is a semver GitHub Release with changelog; **public** GHCR images exist for that version.
- [ ] Merging releasable conventional commits to `main` triggers semantic-release (default rules) → GitHub Release → GHCR images.

## References

- Home **k3s** — workload host; apply machine is separate and uses remote kubeconfig for Terraform providers + kubernetes state backend
- [Terraform kubernetes backend](https://developer.hashicorp.com/terraform/language/backend/kubernetes) — state Secret + lock Lease
- Official `postgres` image StatefulSet — in-cluster Postgres
- SolidStart BFF sticky (`API_UPSTREAMS` / `mtgfr-instance`) — no nginx hop
- Cloudflare Tunnel / Zero Trust — Terraform-managed public hostnames → in-cluster `cloudflared`
- ADR 0005 — in-process fan-out (affinity extends this)
- ADR 0010 — Postgres via Toasty (`push_schema` dev-only; migrations in prod)
- [Toasty schema management](https://tokio-rs.github.io/toasty/nightly/guide/schema-management.html) — `migration generate` / `migration apply`
- ADR 0018 — Effect client + SSE stream; same-origin `/api` (SolidStart BFF)
- ADR 0021 — live games in-memory (motivates drain deploy)
- `docker-compose.yml` — dev Postgres defaults
- [`config`](https://docs.rs/config) — layered server `Settings` (TOML + env)
- `justfile` — `migrate`, `build-server`, `build-client`, `check`
- [Google distroless](https://github.com/GoogleContainerTools/distroless) — production runtime base images
- [semantic-release GitHub Actions recipe](https://semantic-release.org/recipes/ci-configurations/github-actions/) — `verify-and-release.yml` template
