# Deployment PRD

Deploy mtgfr to a **Kubernetes cluster** reached via a **Cloudflare Tunnel** (no inbound ports on the cluster), with infrastructure-as-code, semver releases, and **draining** rolling deploys so in-progress games are not killed mid-hand.

> **Routing / drain model:** [ADR 0030](../adr/0030-table-instance-affinity-for-drain-rolls.md) — BFF lobby on `mtgfr_web` (Drizzle / `@effect/sql-pg`), in-game `table_routes` → pod DNS on headless Service. `terraform apply` bumps Argo Application helm params (after migrate Jobs complete); Argo syncs Deployments then Service `edh-api` (sync waves + `PruneLast`); pruned API pods drain on SIGTERM.

## Goals

1. **Reachable by friends** — single public origin `https://edh.example.com` (SolidStart BFF: SPA + `/api` proxy); stable share links on that host.
2. **Reproducible infra** — Terraform in **this repo** (`iac/`) owns headless/web Services, secrets, Postgres, Cloudflare Tunnel/DNS, migrate Jobs, and the Argo Application params. Argo owns API/web Deployments and Service `edh-api` (`iac/charts/edh`). `terraform apply` is the operator ship path.
3. **Zero-downtime for active games** — during a release, players at a live table stay on the old server binary until that table is empty (game over or everyone left).
4. **Traceable releases** — every merge to `main`/`master` runs [semantic-release](https://semantic-release.org/) (default config) on green verify; container images build when the resulting `v*` tag is pushed (`docker.yml`).

## Non-goals (for this PRD)

- Multi-node horizontal scale of the **same** image tag / Redis shared registry (ADR 0005) — still future work. Concurrent **Terminating** API pods during a roll (newest active + draining old) **are** in scope; same-tag horizontal replicas are not.
- Durable game resume across server restart (explicitly out per ADR 0021).
- Managing the Kubernetes control plane / node OS (cluster bootstrap is assumed done; this repo deploys *into* the cluster).
- Automated card-art CDN deploy (ADR 0015; `cards.example.com` remains separate).
- Homelab Docker / Traefik — **retired** as the edh hosting path.

## Context (architecture constraints)

| Concern | Current state | Deploy implication |
|--------|---------------|-------------------|
| Live games | In-memory `Registry` per process (ADR 0021) | Concurrent API pods **must not share** a table; BFF pins each table to the owning pod via `mtgfr_web.table_routes` → pod DNS. |
| Fan-out | `tokio::broadcast` in-process (ADR 0005) | gRPC streams are tied to the pod that owns the table. |
| Durable data | Postgres `mtgfr`: users, sessions, decks (ADR 0010); `mtgfr_web`: lobbies + routes | API uses `DATABASE_URL` → `mtgfr`; BFF uses `WEB_DATABASE_URL` → `mtgfr_web` (Drizzle). |
| Client | SolidStart 1.3 (Vinxi, `ssr: false`); same-origin `/api` BFF | Browser always calls `/api`; lobby stays on BFF; game paths resolve `table_id` → pod DNS; auth/decks → Service `edh-api`. Web **may** roll with the API (expand-only wire). |
| Dev DB bootstrap | `push_schema()` at boot (`db.rs`) | **Toasty** for `mtgfr` in prod/CI; **Drizzle** for `mtgfr_web`; `push_schema` only for in-memory SQLite tests (per [Toasty schema guide](https://tokio-rs.github.io/toasty/nightly/guide/schema-management.html)). |
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
    ├─ Namespace: argocd             (Helm argo-cd; Application edh — owns Deployments)
    ├─ Namespace: edh
    │     ├─ Deployment edh-web              (SolidStart BFF; Argo-owned)
    │     ├─ Deployment edh-api-<tag>        (newest active; prior generations Terminating + SIGTERM drain)
    │     ├─ Service edh-api                 (Argo-owned; selects app=<apiActiveInstanceId> after new Deployment healthy)
    │     ├─ Service edh-api-headless        (TF; publishNotReadyAddresses — in-game dial)
    │     ├─ Job edh-migrate                 (Toasty apply on mtgfr)
    │     ├─ Job edh-web-migrate             (Drizzle apply on mtgfr_web)
    │     ├─ Job postgres-create-web-db      (CREATE DATABASE mtgfr_web)
    │     ├─ StatefulSet postgres            (official postgres; mtgfr + mtgfr_web)
    │     └─ Deployment cloudflared          (tunnel connector)
    │
Internet ── Cloudflare (DNS + Tunnel) ──► cloudflared ──► edh-web:8080
                                              │
                         lobby → mtgfr_web │ game → table_routes → pod DNS
                                              │
                         ┌────────────────────┼────────────────────┐
                         ▼                    ▼                    ▼
                      edh-api              Terminating          Terminating
                   (newest only)            old pod               older pod
```

**Apply machine vs cluster host:** `terraform` / `kubectl` always run on the apply machine. That machine needs network reachability to the k3s API server (LAN, VPN, or Tailscale — whatever you already use for remote kubeconfig). Do **not** install or run Terraform on the k3s node for this stack. `GET /health/drain` (observation) uses `kubectl port-forward` from the apply machine (not the public tunnel).

**Hostname** (Cloudflare `example.com` zone):

| Host | Serves | Tunnel ingress |
|------|--------|----------------|
| `edh.example.com` | SolidStart SPA + `/api` BFF | `http://edh-web.edh.svc:8080` |

Browser → tunnel → `edh-web`. Lobby CRUD hits Postgres `mtgfr_web`. Start seeds Service `edh-api` and writes `table_routes`. In-game `/api/tables/{table}/…` proxies to `http://{pod_dns}:8080` on the headless Service. Auth/decks/cards always go to `edh-api`. NetworkPolicy: `cloudflared` → `edh-web` only; `edh-web` → pods with `mtgfr.io/component=api`.

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

`cloudflared` runs in-cluster (Deployment, **1** replica by default — enough for a small cluster; set `cloudflared_replicas = 2` for connector HA). It authenticates with the tunnel token and forwards Cloudflare edge traffic to ClusterIP Services. The cluster needs **egress** to Cloudflare; it does **not** need inbound public ports for edh.

### Postgres — official image StatefulSet

Install a single-primary **Postgres StatefulSet** (`postgres:17` or pinned tag) into namespace `edh` with a PVC on k3s local-path (or whatever StorageClass the node has). Create role/database `mtgfr` via the image’s `POSTGRES_*` env. `DATABASE_URL` points at the Service DNS name `postgres`. A one-shot Job creates database **`mtgfr_web`** on the same instance for the SolidStart BFF (Drizzle); API pods never see that DB.

Skip CloudNativePG / Bitnami for v1 — more operators (and Bitnami’s image-catalog churn) than this friend-group deploy needs. **Backups for v1:** rely on k3s / PVC snapshots (and etcd/datastore backups that already protect cluster state). No separate Postgres dump cron until we need it.

### Table routing — SolidStart BFF

`edh-web` sets `API_UPSTREAM` (Service `edh-api`) and `WEB_DATABASE_URL` (`mtgfr_web`). Lobby create/join/ready/start run in SolidStart against Drizzle. **Start** calls `POST /tables/seed/v1` on `edh-api` and writes `table_routes` (`table_id` → `pod_dns`). In-game `/api/tables/{table}/stream|intent|…` looks up that row and proxies to `http://{pod_dns}:8080` (headless Service keeps Terminating pods reachable). Auth/decks/cards always proxy to `API_UPSTREAM`. Dev without `WEB_DATABASE_URL` falls back to localhost for game paths. **No** `mtgfr-instance` cookie / `API_UPSTREAMS` map.

### What mtgfr Terraform owns

| Resource | Notes |
|----------|-------|
| Cloudflare Tunnel + DNS | Single `edh` hostname → `edh-web` |
| `kubernetes_namespace.terraform` | Optional if bootstrapped by hand; state Secret/Lease live here |
| `kubernetes_namespace.edh` | Isolation boundary for workloads |
| `kubernetes_namespace.argocd` + Helm `argo-cd` | Control plane; Application owns API/web Deployments (`argocd_repo_url` required) |
| `edh-web` | SolidStart BFF (`API_UPSTREAM` + `WEB_DATABASE_URL`) |
| `edh-api-<tag>` Deployment + Services | Desired `server_image` (Argo); Service selects `app=<instanceId>`; headless for pod DNS |
| `edh-migrate` Job | Toasty `migration apply` on `mtgfr` before API roll |
| `edh-web-migrate` Job | Drizzle migrate on `mtgfr_web` before web roll |
| `postgres-create-web-db` Job | Idempotent `CREATE DATABASE mtgfr_web` |
| StatefulSet `postgres` | Official `postgres` image; `mtgfr` + `mtgfr_web`; backups = k3s/PVC snapshots |
| NetworkPolicy | tunnel→web; web→api; api+migrate+web(+web-migrate)→postgres |

| Secrets | `DATABASE_URL`, tunnel token, admin token, etc. |
| `cloudflared` Deployment + Secret | Tunnel connector |

### Variables & secrets

`iac/terraform.tfvars` (gitignored) or env vars:

| Variable | Purpose |
|----------|---------|
| `kubeconfig_path` | Path on the **apply machine** to a kubeconfig that reaches remote k3s |
| `cloudflare_api_token` | DNS + Zero Trust tunnel |
| `mtgfr_db_password` | `DATABASE_URL` / `WEB_DATABASE_URL` (composed in Terraform) |
| `auth_secret` | reserved — session signing if added later |
| `server_image` / `web_image` | Desired active API + web images |
| `argocd_repo_url` | Required; git source for Argo Application (`iac/charts/edh`) |
| `api_termination_grace_seconds` | SIGTERM drain ceiling (default 24h) |

### Apply

From the **apply machine** (repo checkout + Terraform CLI + network access to k3s API and Cloudflare):

```bash
export KUBE_CONFIG_PATH=~/.kube/config   # example — remote k3s
cd iac
terraform init
terraform apply          # migrate Jobs complete → bump Argo helm params; sync waves + PruneLast
```

Drain is in-process on SIGTERM until `active_tables=0` or grace expires. Apply does not wait on grace.

**Roll ordering (avoids seed blackhole):** Argo sync-wave `0` brings up the new API Deployment; wave `1` retargets Service `edh-api` to `app=<newId>`; `PruneLast` then deletes the prior Deployment (SIGTERM drain). Mid-game stays on headless pod DNS.

**One-time cutover** from TF-owned Deployments / `edh-api` Service:

1. Chart (`iac/charts/edh`) must already be on the git revision in `argocd_target_revision` — Argo reads git, not the apply machine working tree.
2. Set `argocd_repo_url` / `argocd_target_revision` and image tags, then `terraform apply`. Terraform will destroy the old Deployments / `edh-api` Service still in state; Argo creates the replacements. Expect a short outage and any in-memory games to die.

Optional: `terraform state rm` those resources first if you want Argo to adopt without a delete/create gap.

If a prior apply left a failed `kubernetes_manifest.edh_application` in state, remove it (`terraform state rm 'kubernetes_manifest.edh_application'`) — the Application is now a `helm_release.edh_application` (`argocd-apps`).

**Failure mode:** If Service `edh-api` ever selects an instance id with no Ready pods (mis-ordered sync without waves), Start/seed/auth return connection errors until the new Deployment is healthy.

## DNS & Cloudflare

DNS for the public host is **owned by mtgfr Terraform** (with the tunnel resources in `tunnel.tf`):

| Record | FQDN | Type | Target |
|--------|------|------|--------|
| `edh` | `edh.example.com` | CNAME | `<tunnel-id>.cfargotunnel.com` |

**Proxy mode:** Tunnel hostnames are **proxied (orange cloud)** — that is how Cloudflare Tunnel works. TLS is at the edge; origin is the in-cluster `cloudflared` connector.

**Game stream through Cloudflare (required):** Tunnel hostnames are orange-clouded, so Cloudflare's ~100s HTTP idle timeout applies. The BFF bridges gRPC `Game.Stream` to the browser as SSE on `/api/rpc/.../stream` and must keep that edge stream alive (heartbeats from tonic → SSE). Configuration Rules disable response buffering on `edh.example.com` so the BFF stream is not held at the edge.

**Table share links** use `https://edh.example.com/play/XXXXXX` — stable across deploys. Legacy `?table=` query links still parse on join.

## Rolling deployment model

### Definitions

| Term | Meaning |
|------|---------|
| **Active table** | A `table_id` still in the in-memory registry that counts for drain: a **started** game not yet torn down. (Pre-game lobbies live on the BFF / `mtgfr_web` and do not block API drain.) |
| **Drain mode** | Instance rejects new seeds (`POST /tables/seed/v1` → 503); keeps serving tables it already owns. |
| **Idle lobby TTL** | **30 minutes** since last lobby activity on `mtgfr_web`. Swept by the BFF so abandoned lobbies do not linger. |
| **Finished** | Table removed from the API registry because the game ended / seats vacated; BFF may `DELETE` the `table_routes` row (TTL is the safety net). |

### Flow

```mermaid
sequenceDiagram
    participant TF as terraform_apply
    participant BFF as edh_web_BFF
    participant Old as edh_api_old
    participant New as edh_api_new
    participant WebDB as mtgfr_web

    TF->>New: Roll Deployment server_image (role=active)
    Note over Old: SIGTERM → in-process drain<br/>reject new seeds
    TF->>BFF: May roll web_image same apply
    BFF->>New: Start → POST /tables/seed/v1
    New-->>BFF: pod_dns
    BFF->>WebDB: INSERT table_routes
    Note over BFF: Mid-game hops stay on Old<br/>via pod DNS + headless
    Note over Old: Exit when active_tables=0<br/>or grace (default 24h)
```

1. **Migrate** `mtgfr` (Toasty Job) and `mtgfr_web` (Drizzle Job).
2. **`terraform apply`** waits for image-keyed migrate Jobs, then updates Argo helm params. Argo syncs the new API Deployment (wave 0), retargets Service `edh-api` (wave 1), then `PruneLast` deletes the previous Deployment; pruned pods receive SIGTERM and drain in-process.
3. **New tables** only seed on Service `edh-api` (active). BFF writes `table_routes` with the seed response’s `pod_dns`.
4. **In-game** traffic uses path `{table}` → `table_routes` → headless pod DNS (Terminating pods stay dialable via `publishNotReadyAddresses`).
5. **Web may roll with the API**; expand-only wire while old pods hold games ([WIRE_COMPAT.md](../WIRE_COMPAT.md)).
6. Old pods exit when `active_tables=0` or `terminationGracePeriodSeconds` (default 24h) elapses.

**Client/server roll order (locked):** Web may roll with newest API. Expand-only wire across concurrent binaries until Terminating pods exit.

### Wire backwards compatibility (required doc)

Roll order reduces mid-game refresh skew; it does **not** remove the need for wire rules. During the drain window, **new SPA ↔ old API** still happens for mid-game tables (and new SPA ↔ new API for new tables). Durable rules live in [WIRE_COMPAT.md](../WIRE_COMPAT.md):

1. **Compatibility window** — all concurrent API binaries until each Terminating pod exits.
2. **Expand-only during that window** — additive optional protobuf fields (new field numbers), new RPCs, new intent/event variants the old client never sends; no rename/remove/reuse of field numbers until older binaries are gone. See [WIRE_COMPAT.md](../WIRE_COMPAT.md).
3. **Hard breaks** — bump proto package / service version, run both until drain completes, then remove the old; use sparingly. The REST→gRPC migration (ADR 0032) was an intentional hard cut of API+web together.
4. **Game stream** — same expand-only rule on native protobuf `StreamFrame` / `VisibleState` trees (ADR 0032; not JSON-in-proto).
5. **Authoring habit** — change `proto/` first; keep `client/src/wire/types.ts` aligned with `crates/schema` serde shapes; prefer optional fields first.

### Table → pod routing

ADR 0005 assumes a single instance. Rolling deploy requires:

1. Each API pod publishes **`POD_DNS`** (`{apiActiveInstanceId}.{headless}.$(POD_NAMESPACE).svc.cluster.local`, with Pod `hostname`/`subdomain` set so CoreDNS serves that A record) and returns it from `Tables.Seed` (gRPC).
2. BFF stores `table_id` → `pod_dns` in `mtgfr_web.table_routes` (explicit delete + TTL).
3. In-game BFF dials tonic at `{pod_dns}:50051`; missing/expired route → 404 `UnknownTable`.
4. Guests join lobbies on the BFF only (no Axum lobby / no join fan-out across peers).

**Session cookies** (auth) are host-only on `edh.example.com` when `COOKIE_DOMAIN` is empty.

### Graceful shutdown

On `SIGTERM` (K8s pod termination):

1. Enter drain mode (same as live drain toggle).
2. Stop accepting new tables immediately.
3. Keep the process alive while `active_tables > 0` (poll every few seconds; configurable timeout with loud logging).
4. Close idle HTTP connections; **do not** cut active SSE streams until the table is finished or the hard timeout fires (hard timeout is a last resort — prefer waiting).
5. Prefer waiting for `active_tables=0` over relying on a huge grace alone. Drain is in-process on SIGTERM (distroless has no shell `preStop`). Default `terminationGracePeriodSeconds` is **24h**; if still draining after that, kube SIGKILLs. Loud logs while draining are expected for long games.

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
    pub drain: bool,            // default false; flipped live by SIGTERM
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
| `grpc_port` | `GRPC_PORT` | `50051` (ADR 0032 — tonic gRPC, the wire contract's authoritative transport) |
| `database_url` | `DATABASE_URL` | `postgresql://mtgfr:<secret>@postgres:5432/mtgfr` |
| `instance_id` | `INSTANCE_ID` | stable Deployment id, e.g. `edh-api-1-2-3` |
| `drain` | `DRAIN` | startup default `false`; live drain via SIGTERM |
| `cookie_secure` | `COOKIE_SECURE` | `true` |
| `cookie_domain` | `COOKIE_DOMAIN` | empty (host-only session cookie on edh) |
| `cors_origin` | `CORS_ORIGIN` | empty (browser is same-origin via BFF) |
| `version` | `VERSION` | image release tag |
| `pod_dns` | `POD_DNS` | `{instanceId}.{headless}.$(POD_NAMESPACE).svc.cluster.local` (Pod `hostname`/`subdomain`); returned by seed |

`RUST_LOG` stays a standard tracing env var (not part of `Settings`) — set alongside the above in Terraform.

Auth session cookies are host-only when `cookie_domain` is empty. Health endpoints expose `settings.version` and `settings.drain`. Seed returns `pod_dns` for BFF `table_routes` (no affinity cookie).

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
    main.rs                  # serve | static | migration …
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
terraform apply
    │
    ├─ 1. Job: mtgfr-server:<tag> server migration apply (mtgfr)
    ├─ 2. Job: drizzle migrate (mtgfr_web) before edh-web
    ├─ 3. roll edh-api Deployment (SIGTERM drain on old pods)
    ├─ 4. catalog projection runs on server boot (data only)
    └─ 5. roll edh-web (may be same apply as API)
```

**Terraform** — Kubernetes Jobs before API/web rolls. Image-hash Job names stay stable when the image is unchanged. Wait for completion before updating Deployments.

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

CI: parallel `verify-server` / `verify-client` jobs (see §GitHub Actions); server job applies Toasty migrations then `just server-check`.

- Server may enable **CORS** for `cors_origin` when set; same-origin BFF leaves it empty (browser does not need CORS).
- Auth **session** cookies are host-only on `edh.example.com` so `fetch(..., { credentials: "include" })` to `/api` sends them.
- In-game routing is **`table_routes` → pod DNS** (no affinity cookie).

### Client (production build)

Same-origin `/api` always — no separate API hostname bake:

| Env | Dev (`bun run dev`) | Prod (runtime) |
|-----|---------------------|----------------|
| API | `/api` → BFF → `http://127.0.0.1:8080` | `/api` → BFF → `API_UPSTREAM` / `table_routes` |
| `WEB_DATABASE_URL` | local `mtgfr_web` for lobby | composed DSN to in-cluster `mtgfr_web` |
| `VITE_CARD_CDN` | optional | optional build-arg |

`client/src/effect/client.ts` always prepends `/api`. SSE stream URL follows the same origin.

### Other server work

| Item | Detail |
|------|--------|
| `Settings::load()` | Called once in `mtgfr serve`; bind `settings.listen_addr()`. |
| CORS middleware | Axum layer: allow `cors_origin`, credentials, needed methods/headers. |
| Cookie `Domain` | Auth session: empty `cookie_domain` (host-only on edh). |
| `GET /health/drain` | JSON `{ "active_tables": N, "draining": bool }` — observation only (SIGTERM sets `draining`). **Not on the public tunnel hostname.** NetworkPolicy blocks ingress from `cloudflared` / public paths. The apply machine reaches it via `kubectl port-forward` (through the k3s API). |
| `GET /health/live` | `200` if process is up; body includes `version`. |
| `GET /health/ready` | `200` if accepting traffic (not draining, or draining with tables — still "ready" for those tables). |
| Active table count | Registry helper: started games only (lobbies are on the BFF). |
| `POST /tables/seed/v1` | BFF Start → seed seats/decks; response includes `pod_dns`. |
| Idle lobby TTL | **30 min** on `mtgfr_web` — BFF sweep so abandoned lobbies drop. |
| SSE keepalive | **Required** — periodic comment/event so Cloudflare Tunnel idle timeout does not drop streams. |
| Migrations | Toasty for `mtgfr`; Drizzle for `mtgfr_web` — see [Database migrations](#database-migrations). |

## Container images

**Policy:** production runtime images use [Google distroless](https://github.com/GoogleContainerTools/distroless) (`gcr.io/distroless/*-debian12:nonroot`). Builder stages only (`rust:bookworm`, `oven/bun`) have shells and package managers; nothing based on `alpine`, `debian:slim`, or `nginx` ships to prod.

Distroless has no shell — use ephemeral debug containers for inspection. All config is env vars + baked files (`config/mtgfr.toml`, `Toasty.toml`, `toasty/`).

### `mtgfr-server`

Multi-stage `docker/server/Dockerfile`:

1. **Build:** `rust:1-bookworm` — install `protobuf-compiler`, copy `proto/` + crates, then `cargo build -p server --release` (cache mount on `target/`). tonic stubs land in Cargo `OUT_DIR` via `build.rs` (never committed). Test-only Toasty sqlite is a `[dev-dependencies]` feature, so it is not compiled into this image.
2. **Runtime:** `gcr.io/distroless/cc-debian12:nonroot` — copy `server`, `config/mtgfr.toml`, `Toasty.toml`, `toasty/`; `EXPOSE 8080` (health) / gRPC is `:50051` in chart/TF.
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
2. **Build:** `node:22-bookworm` — `.proto` is the sole wire contract (ADR 0032). Effect-gRPC clients under `client/src/wire/generated/` are gitignored and regenerated in-image via `bun run gen` / `scripts/gen.sh`; `client/src/wire/types.ts` stays hand-maintained. Then `tsc` + `vinxi build` → `.output/` (optional `VITE_CARD_CDN` build-arg).
3. **Runtime:** `gcr.io/distroless/nodejs22-debian12:nonroot` — copy `.output`; `ENV HOST=0.0.0.0 PORT=8080`; sticky map from k8s env; `CMD [".output/server/index.mjs"]`.

#### Compression / streaming

| Hop | What |
|-----|------|
| Edge (Cloudflare → browser) | Cloudflare may encode with **gzip or brotli** on the orange-cloud hostname — expected. |
| BFF → API | Do **not** buffer `text/event-stream` (SSE); Configuration Rules disable response buffering on `edh`. |
| API (`serve`) | Optional gzip on JSON later; **never** gzip SSE. |

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
    api.tf               # edh-api-headless Service (Deployments + edh-api Service in charts/edh)
    web.tf               # edh-web Service (Deployment in charts/edh)
    web-migrate.tf       # Drizzle Job for mtgfr_web
    postgres-web-db.tf   # CREATE DATABASE mtgfr_web
    postgres.tf          # StatefulSet + Service: official postgres image
    migrate.tf           # Job: toasty migration apply (gates Application image bumps)
    argocd.tf            # Argo CD Helm + Application (owns API/web Deployments + edh-api)
    observability.tf     # LGTM (Grafana/Loki/Tempo/Prometheus) + Alloy (ADR 0034)
    charts/edh/          # API/web Deployments + edh-api Service (sync waves + PruneLast)
    tunnel.tf            # Cloudflare Tunnel + cloudflared + DNS records
    network-policy.tf    # cluster-internal only for /health/drain (+ Alloy ingress in observability.tf)
    secrets.tf           # DATABASE_URL, etc.
    terraform.tfvars     # gitignored secrets + image tags
  docker/
    server/Dockerfile
    web/Dockerfile
```

### Observability (ADR 0034)

Self-hosted LGTM in namespace `observability` (Terraform Helm): Alloy ingest, Loki/Tempo/Prometheus storage, Grafana UI.

**Grafana (operator only — no tunnel hostname):**

```bash
kubectl -n observability port-forward svc/grafana 3000:80
# admin user/password: terraform output -raw grafana_admin_password
# or: kubectl -n observability get secret grafana-admin -o jsonpath='{.data.admin-password}' | base64 -d
```

Datasources (Prometheus, Loki, Tempo) are provisioned with trace↔log links. Retention: 7d traces/logs, 15d metrics.

**App wiring:** `OTEL_EXPORTER_OTLP_ENDPOINT=http://alloy.observability.svc:4318` and `FARO_COLLECT_UPSTREAM=http://alloy.observability.svc:12347/collect` on edh Deployments. Browser Faro → same-origin `/api/faro/collect` → Alloy. Empty OTLP endpoint disables export (local `just dev`).

**State:** kubernetes backend in namespace `terraform` (see above). Do not commit local state files.

**Images:** **public** GHCR packages (`mtgfr-server`, `mtgfr-web`). k3s nodes pull without `imagePullSecrets`. Do not push a moving `latest` tag — pin explicit versions in `terraform.tfvars`.

**API Deployment env (illustrative):**

```hcl
env = [
  { name = "HOST", value = "0.0.0.0" },
  { name = "PORT", value = "8080" },
  { name = "GRPC_PORT", value = "50051" },      # ADR 0032
  { name = "DATABASE_URL", value_from = secret_key_ref … },
  { name = "INSTANCE_ID", value = "edh-api-1-2-3" },  # Deployment name
  { name = "POD_DNS", value = "edh-api-<tag>.edh-api-headless.$(POD_NAMESPACE).svc.cluster.local" },
  # Pod template also sets hostname=<instanceId> + subdomain=edh-api-headless (required for that A record).
  { name = "DRAIN", value = "false" },          # startup only; SIGTERM for live drain
  { name = "COOKIE_SECURE", value = "true" },
  { name = "COOKIE_DOMAIN", value = "" },
  { name = "CORS_ORIGIN", value = "" },
  { name = "VERSION", value = var.server_image },
  { name = "RUST_LOG", value = "info" },
  { name = "OTEL_EXPORTER_OTLP_ENDPOINT", value = "http://alloy.observability.svc:4318" },
  { name = "OTEL_SERVICE_NAME", value = "edh-api" },
]
```

**Web Deployment env (illustrative):**

```hcl
env = [
  { name = "HOST", value = "0.0.0.0" },
  { name = "PORT", value = "8080" },
  { name = "WEB_DATABASE_URL", value_from = secret_key_ref … },
  { name = "API_UPSTREAM", value = "http://edh-api.edh.svc:8080" },
  { name = "GRPC_UPSTREAM", value = "edh-api.edh.svc:50051" },
  { name = "OTEL_EXPORTER_OTLP_ENDPOINT", value = "http://alloy.observability.svc:4318" },
  { name = "OTEL_SERVICE_NAME", value = "edh-web" },
  { name = "FARO_COLLECT_UPSTREAM", value = "http://alloy.observability.svc:12347/collect" },
]
```

**Web Deployment env (illustrative):**

```hcl
env = [
  { name = "HOST", value = "0.0.0.0" },
  { name = "PORT", value = "8080" },
  { name = "API_UPSTREAM", value = "http://edh-api.edh.svc:8080" },
  { name = "WEB_DATABASE_URL", value = "postgresql://mtgfr:…@postgres:5432/mtgfr_web" },
]
```

## Release & versioning

### One-time history squash

Before the first semver tag, rewrite git history to a single root commit (preserving tree). This is a **manual, coordinated** step:

1. Announce to anyone with clones.
2. Squash (e.g. `git checkout --orphan release-root && git add -A && git commit`).
3. Force-push `main` (only acceptable because this is pre-release / friend group).
4. Tag `v0.1.0` (or `v1.0.0` if treating current state as first production).

After squash, **all** commits on `main` follow the [Angular commit message guidelines](https://github.com/angular/angular/blob/main/contributing-docs/commit-message-guidelines.md) (enforced by commitlint / `@commitlint/config-angular`): `feat:`, `fix:`, `build:`, `ci:`, …, with majors via a `BREAKING CHANGE:` footer.

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

**Squash merges:** GitHub writes one commit on `main` whose subject is the **PR title**. That is the only line semantic-release sees for the merge — branch commits (`feat:` inside the PR, `build:` title, etc.) do not matter after squash. Title the PR for the release outcome you want (`feat:` / `fix:` / breaking footer), not merely for the largest file churn. See also [AGENTS.md](../../AGENTS.md) § Commits & releases.

### GitHub Actions

Follow the official [semantic-release GitHub Actions recipe](https://semantic-release.org/recipes/ci-configurations/github-actions/): verify first, then `npx semantic-release` with `GITHUB_TOKEN`. Terraform in `iac/` consumes GHCR images built after the GitHub Release is published.

#### Workflow overview

```
pull_request ──► ci.yml
                   ├─ verify → verify-jobs.yml
                   │            ├─ verify-server  (just server-check; Postgres)
                   │            └─ verify-client  (just client-check)
                   └─ terraform (iac/)

push to main/master ──► verify-and-release.yml
                            ├─ verify → verify-jobs.yml  (same parallel jobs)
                            └─ release  (npx semantic-release — default config)

v* tag pushed ──────► docker.yml           (build + push GHCR images)

workstation (apply machine) ──► terraform apply (iac/) → remote k3s API
```

Four workflows: **`ci.yml`** (PRs), **`verify-jobs.yml`** (reusable server/client verify), **`verify-and-release.yml`** (official recipe), **`docker.yml`** (images only — not part of semantic-release).

**Content-hash skip:** each verify job caches a tiny pass marker keyed by `hashFiles` of that side’s inputs (`verify-server-v1-…` / `verify-client-v1-…`). On cache hit, setup and checks are skipped. PRs restore markers written on `main`, so a client-only PR can skip the server job (and vice versa). Same-branch re-runs no-op when the tree is unchanged. Post-merge `main` still re-runs (GHA cache from a PR branch is not visible on the default branch).

#### `ci.yml` — pull requests

Triggers: `pull_request` to `main` or `master`. Calls `verify-jobs.yml`; no release. Terraform job validates `iac/` in parallel.

#### `verify-jobs.yml` — reusable verify

`on: workflow_call`. Two parallel jobs:

| Job | Recipe | Needs |
|-----|--------|-------|
| `verify-server` | `just server-check` (fmt + clippy + migrate + nextest) | Rust, nextest, Postgres |
| `verify-client` | `just client-check` (proto → Effect-gRPC codegen + format + lint + typecheck + vitest) | Bun only; no Postgres |

Local `just check` stays sequential (codegen → format → lint → typecheck → test).

#### `verify-and-release.yml` — official recipe (adapted for Rust)

Triggers: `push` to `main` and `master`. Structure matches the [verify-and-release example](https://semantic-release.org/recipes/ci-configurations/github-actions/); verify is the reusable workflow instead of `npm test`.

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
    uses: ./.github/workflows/verify-jobs.yml

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

Triggers: `push` of tags matching `v*`. semantic-release must push that tag with repo secret `RELEASE_TOKEN` (PAT: `contents` + `workflow`) — default `GITHUB_TOKEN` cannot cascade workflow runs. Verify already ran `just server-check` / `just client-check` on the tagged commit — this workflow only builds and pushes images.

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
.github/workflows/verify-jobs.yml
.github/workflows/verify-and-release.yml
.github/workflows/docker.yml
package.json
package-lock.json
Toasty.toml
toasty/                         # history.toml, migrations/, snapshots/
crates/server/src/main.rs  # serve | static | migration
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

1. Open PR with Angular-format commits (commitlint); `ci.yml` runs parallel `verify-server` / `verify-client` (content-hash skip when unchanged vs `main`).
2. Merge to `main`.
3. `verify-and-release.yml`: both verify jobs pass → `npx semantic-release` (default) → git tag + GitHub Release (if commits warrant a release).
4. `docker.yml` builds and pushes GHCR images when that `v*` tag is pushed (requires `RELEASE_TOKEN` for semantic-release cascades).
5. Set **`server_image`** / **`web_image`** in `iac/terraform.tfvars` and run **`terraform apply`** from the apply machine.
6. Old API pods drain on SIGTERM; mid-game traffic stays on them via `table_routes` → headless pod DNS until tables clear or grace expires.

### Deploy

`terraform apply` updates Argo helm params after migrate Jobs. Argo sync-wave 0 = new Deployments; wave 1 = Service `edh-api` → `app=<apiActiveInstanceId>`; `PruneLast` then drains the prior API Deployment. Terminating pods remain reachable on the headless Service. From the apply machine: optional `GET /health/drain` via `kubectl port-forward`. Web may roll with the API.

## Phases

| Phase | Deliverable | Exit criteria |
|-------|-------------|---------------|
| **0 — Bootstrap** | History squash, `v0.1.0` tag, Angular commit-message note in AGENTS.md | Tag exists; CI green |
| **1 — Migrations** | `toasty-cli`, `Toasty.toml`, initial `migration generate`, `server migration` subcommand, drop prod `push_schema` | `just migrate` + tests green against Postgres |
| **2 — Containerize** | Distroless Dockerfiles (API `cc`, web `nodejs22` SolidStart), `config` crate + `Settings` | Local image smoke test (compose or kind) |
| **3 — CI** | `.github/workflows/ci.yml` + `verify-jobs.yml` (parallel server/client) | PRs and `main` run `just server-check` / `just client-check` |
| **4 — Release automation** | `verify-and-release.yml` + `docker.yml`, root `package.json`, `RELEASE_TOKEN` | Merge to `main` → semantic-release (default) → `v*` tag push → GHCR images |
| **5 — Cluster + tunnel** | `iac/` from apply machine → remote k3s: Postgres, BFF, Terraform-managed tunnel + DNS, SSE keepalives | Friends reach edh; SSE survives idle |
| **6 — Drain + routing** | SIGTERM drain, `POD_DNS`, BFF lobby + `table_routes`, active + headless Services, [WIRE_COMPAT.md](../WIRE_COMPAT.md) | Deploy while a game runs on the prior binary without disconnect; mid-game hops stay on old pod DNS |
| **7 — Deploy ergonomics** | `terraform apply` of images; Argo Application owns Deployments (`prune` on) | Release → bump images → apply |

## Decisions (locked)

| Topic | Decision |
|-------|----------|
| Cluster | Home **k3s** on its own host; Terraform / kubectl on a **separate apply machine** via remote kubeconfig |
| Terraform state | **Kubernetes backend** on that k3s cluster (Secret + Lease in `terraform`); apply machine reaches it over the k3s API |
| Postgres | Official **`postgres` image** StatefulSet in `edh`; single primary + PVC |
| Postgres backups (v1) | **k3s / PVC snapshots** (+ existing etcd/datastore backups); no dump cron yet |
| Sticky routing | **SolidStart BFF**: lobby on `mtgfr_web` (Drizzle); in-game `table_routes` → pod DNS on headless Service (ADR 0030) |
| Tunnel | **Fully Terraform-managed** Zero Trust tunnel + DNS + in-cluster `cloudflared` |
| Idle lobby TTL | **30 minutes** — idle lobbies drop so drain can complete |
| Drain stuck >24h | **`terminationGracePeriodSeconds` default 24h** then SIGKILL; SIGTERM waits for `active_tables=0` in-process |
| GHCR | **Public** packages; no `imagePullSecret` on nodes |
| Admin / drain endpoints | **Not public** (NetworkPolicy blocks tunnel); apply machine uses `kubectl port-forward` |
| Client vs API roll order | **Web may roll with API**; expand-only wire while Terminating pods hold tables |
| Wire backwards compatibility | **Expand-only** across concurrent binaries — see [WIRE_COMPAT.md](../WIRE_COMPAT.md) |
| Static asset compression | Cloudflare edge gzip/brotli OK; **no SSE buffering/gzip** through the BFF |
| Control plane | **`terraform apply`** bumps Argo helm params after migrate Jobs; Argo owns API/web Deployments + `edh-api` (`argocd_repo_url` required; sync waves + `PruneLast`) |

## Open questions

None blocking implementation. Refine which lobby events reset the 30 min TTL if playtesting shows false drains.

## Success criteria

- [ ] `https://edh.example.com/` loads the client; `/api` reaches Axum via the BFF; auth, deck builder, and lobby work against prod Postgres (`mtgfr` + `mtgfr_web`).
- [ ] Traffic reaches the cluster via Cloudflare Tunnel only (no public NodePort/LB required for edh).
- [ ] Auth session cookies are host-only on edh; in-game hops use `table_routes` → pod DNS (no affinity cookie).
- [ ] SSE keepalives keep streams alive through the tunnel under idle play (≥2 min without user actions).
- [x] `just migrate` runs Toasty `migration apply` on dev Postgres; server starts without `push_schema`.
- [ ] `just client-migrate` applies Drizzle migrations to `mtgfr_web`.
- [ ] `terraform apply` from the apply machine (not the k3s host) uses the kubernetes backend on home k3s and reproduces the edh stack; plan fails clearly if the cluster/tunnel prerequisites are missing.
- [ ] Migrate Jobs complete before `edh-api` / `edh-web` roll; schema versions match the deployed images.
- [ ] SIGTERM drain rejects new seeds with 503 while existing tables keep SSE; `/health/drain` is not reachable via the public tunnel.
- [ ] Idle lobbies expire after 30 minutes on `mtgfr_web`.
- [ ] Deploying `vX.Y.Z+1` while a four-player game runs on `vX.Y.Z` completes without SSE drop or `UnknownAction` spikes.
- [ ] During that roll, mid-game traffic stays on the Terminating pod via headless DNS; new tables seed on active; web may roll with newest.
- [x] Wire backwards-compatibility rules are documented ([WIRE_COMPAT.md](../WIRE_COMPAT.md)) — expand-only across the drain window, `/v2` for hard breaks.
- [ ] Old server pods exit within minutes after the last table clears (or stay up harmlessly if a game runs long; SIGKILL after default 24h grace).
- [ ] Every production deploy is a semver GitHub Release with changelog; **public** GHCR images exist for that version.
- [ ] Merging releasable Angular-format commits to `main` triggers semantic-release (default rules) → GitHub Release → GHCR images.

## References

- Home **k3s** — workload host; apply machine is separate and uses remote kubeconfig for Terraform providers + kubernetes state backend
- [Terraform kubernetes backend](https://developer.hashicorp.com/terraform/language/backend/kubernetes) — state Secret + lock Lease
- Official `postgres` image StatefulSet — in-cluster Postgres (`mtgfr` + `mtgfr_web`)
- SolidStart BFF + `table_routes` → pod DNS — [ADR 0030](../adr/0030-table-instance-affinity-for-drain-rolls.md)
- Cloudflare Tunnel / Zero Trust — Terraform-managed public hostnames → in-cluster `cloudflared`
- ADR 0005 — in-process fan-out (routing extends this)
- ADR 0010 — Postgres via Toasty (`push_schema` dev-only; migrations in prod)
- [Toasty schema management](https://tokio-rs.github.io/toasty/nightly/guide/schema-management.html) — `migration generate` / `migration apply`
- ADR 0032 — Effect RPC + gRPC/tonic; proto-owned wire; hard cut from OpenAPI/SSE
- ADR 0021 — live games in-memory (motivates drain deploy)
- `docker-compose.yml` — dev Postgres defaults
- [`config`](https://docs.rs/config) — layered server `Settings` (TOML + env)
- `justfile` — `migrate`, `client-migrate`, `server-check`, `client-check`, `check`
- [Google distroless](https://github.com/GoogleContainerTools/distroless) — production runtime base images
- [semantic-release GitHub Actions recipe](https://semantic-release.org/recipes/ci-configurations/github-actions/) — `verify-and-release.yml` template
