# Production Topology and Operations

**Status:** Current (as of 2026-07-25)
**Module:** `iac/`, `docker/server/Dockerfile`, `docker/web/Dockerfile`,
`crates/server/src/settings.rs`, `crates/server/src/health.rs`

Related: [ci-and-release](2026-07-20-ci-and-release.md), [observability-ops](2026-07-20-observability-ops.md).

---

## Problem Statement

A friend-group Commander game needs to be reachable over the public internet without a static IP
or inbound open ports on the home server. Deploys must not kill in-progress games: rolling out a
new binary while four people are mid-combat must not disconnect anyone. The infrastructure must be
reproducible from source; image builds and releases must be traceable to a specific git commit.

---

## Solution

The deployment stack is: home **k3s** cluster on a dedicated host, reached from the internet via
**Cloudflare Tunnel** (no inbound public ports on the cluster), managed entirely via **Terraform**
in `iac/` plus an **Argo CD** `Application` that owns rolling API and web Deployments. Releases
are fully automated via **semantic-release** (see [ci-and-release](2026-07-20-ci-and-release.md))
producing `v*` git tags that trigger **GHCR** container image builds. Operators run `terraform apply`
from a separate apply machine (workstation/laptop with kubeconfig access to the k3s API) to roll
images and infrastructure.

Observability is self-hosted **LGTM** in namespace `observability` — see
[observability-ops](2026-07-20-observability-ops.md). Grafana is operator-only via `kubectl
port-forward`; no tunnel hostname for the observability plane. Terraform provisions the
`mtgfr OTEL RED` Grafana dashboard with BFF HTTP and API gRPC RED panels.

---

## User Stories

- As a **player**, I open `https://edh.example.com/` in a browser; the Foldkit SPA loads,
  I log in, browse my decks, join a lobby, and start a game — all over TLS with no self-signed
  certs or port-forwarding.
- As an **operator**, I merge a `feat:` PR to `main`; CI runs verify, semantic-release cuts a
  `v*` tag, GHCR builds and pushes `mtgfr-server` and `mtgfr-web` images; I update image tags in
  `iac/terraform.tfvars` and run `terraform apply` from my laptop; the new API Deployment rolls
  while in-progress games finish on the Terminating pod.
- As a **developer**, I merge a `fix:` PR; a `v*.*.patch` tag is cut; I apply as above. Old pod
  drains within 60s of its last game ending; k3s then SIGKILLs it after the 24h grace if it
  somehow never drains (edge case for very long games).
- As an **operator**, I run `kubectl port-forward` to `edh-api:8080` and `GET /health/drain` to
  observe `{"active_tables": N, "draining": true}` while a Terminating pod is draining.

Operator Grafana / Tempo stories live in [observability-ops](2026-07-20-observability-ops.md).

---

## Behavior

### Network topology

```
Internet
  │ TLS (Cloudflare edge)
  ▼
Cloudflare Tunnel (orange-cloud DNS proxy)
  │ HTTP
  ▼
cloudflared Deployment (1 replica; HA: set cloudflared_replicas=2)
  │
  ▼
edh-web ClusterIP Service :8080
  │ Nitro BFF
  ├─ Lobby RPCs → mtgfr_web (Drizzle, Postgres)
  ├─ Auth/Decks/Cards → edh-api ClusterIP Service :50051 (newest pod)
  └─ In-game → table_routes lookup → pod DNS → edh-api-headless :50051 (any pod)
```

**Public hostname:** `edh.example.com` (Cloudflare DNS, CNAME to `<tunnel-id>.cfargotunnel.com`,
proxied). TLS terminates at Cloudflare. In-cluster traffic is HTTP (no mTLS for v1).

**Cloudflare Configuration Rules** on `edh.example.com` disable response buffering so SSE
(game stream) is not held at the edge. The ~100s Cloudflare idle timeout is countered by
`Heartbeat` frames in the `Game.Stream` protocol.

**Browser paths** all use same-origin `/api` — the BFF strips the prefix before Axum. Public
`/api/admin/*` and `/api/health/drain` return 404 from the BFF (not reachable publicly). The
Axum health probes are on `:8080` inside the cluster; the BFF exposes only `meta/health` and
`meta/version` publicly.

### Kubernetes namespaces and workloads

| Namespace | Contents |
|-----------|----------|
| `terraform` | tfstate Secret, lock Lease (kubernetes backend) |
| `argocd` | Argo CD Helm release; Application `edh` |
| `edh` | All app workloads (see table below) |
| `observability` | LGTM stack: Alloy, Loki, Tempo, Prometheus, Grafana (detail in [observability-ops](2026-07-20-observability-ops.md)) |

**Namespace `edh` workloads:**

| Resource | Owned by | Notes |
|----------|----------|-------|
| Deployment `edh-web` | Argo (`charts/edh`) | Nitro BFF; `API_UPSTREAM`, `WEB_DATABASE_URL` |
| Deployment `edh-api-<tag>` | Argo (`charts/edh`) | Newest active API binary; prior generations Terminating |
| Service `edh-api` | Argo (`charts/edh`) | Selects `app=<apiActiveInstanceId>` (newest only); for seed + auth/decks/cards |
| Service `edh-api-headless` | Terraform | `publishNotReadyAddresses=true`; for in-game pod DNS dial |
| StatefulSet `postgres` | Terraform | Official `postgres` image; `mtgfr` + `mtgfr_web` databases; PVC backed |
| Job `edh-migrate` | Terraform (run before API roll) | `server migration apply` on `mtgfr` |
| Job `edh-web-migrate` | Terraform (run before web roll) | Drizzle migrate on `mtgfr_web` |
| Job `postgres-create-web-db` | Terraform | `CREATE DATABASE mtgfr_web` (idempotent) |
| Deployment `cloudflared` | Terraform | Tunnel connector; auth via Secret |
| NetworkPolicy | Terraform | tunnel→web; web→api; api+migrates+web→postgres |

### Terraform ownership and layout

Terraform manages all infrastructure from the apply machine via the kubernetes provider (remote
kubeconfig). State is stored as a Secret + Lease in namespace `terraform` on the k3s cluster
(kubernetes backend — never local disk in steady state). Cloudflare provider manages the Zero
Trust tunnel, DNS record, and ingress rules.

```
iac/
  backend.tf           # kubernetes Secret+Lease state backend
  providers.tf         # kubernetes + helm + cloudflare (no docker-over-SSH)
  variables.tf
  namespace.tf          # edh + terraform namespaces
  api.tf               # edh-api-headless Service
  web.tf               # edh-web Service
  migrate.tf            # Toasty migration Job (gates API roll)
  web-migrate.tf       # Drizzle migration Job (gates web roll)
  postgres-web-db.tf   # CREATE DATABASE mtgfr_web Job
  postgres.tf          # StatefulSet + Service
  argocd.tf            # Argo CD Helm + Application edh
  observability.tf     # LGTM Helm + Alloy + NetworkPolicy
  tunnel.tf            # Cloudflare Tunnel + cloudflared + DNS CNAME
  network-policy.tf    # cluster-internal only for /health/drain
  secrets.tf           # DATABASE_URL, tunnel token, etc.
  terraform.tfvars     # gitignored: secrets + image tags
  charts/edh/          # API/web Deployments + edh-api Service (sync waves + PruneLast)
```

Key Terraform variables (`iac/terraform.tfvars`, gitignored):

| Variable | Purpose |
|----------|---------|
| `kubeconfig_path` | Apply machine → remote k3s API |
| `cloudflare_api_token` | DNS + Zero Trust |
| `mtgfr_db_password` | Postgres credential (composed into DSNs) |
| `server_image` | Active API image tag |
| `web_image` | Active web image tag |
| `argocd_repo_url` | Git source for Argo Application |
| `api_termination_grace_seconds` | Default 24h (86400) |

### Rolling deploy sequence

```
terraform apply (apply machine)
  │
  ├─ 1. Job edh-migrate:      server migration apply   (mtgfr Postgres)
  ├─ 2. Job edh-web-migrate:  drizzle migrate          (mtgfr_web Postgres)
  ├─ 3. Argo sync-wave 0:     new edh-api Deployment   (newest binary)
  ├─ 4. Argo sync-wave 1:     edh-api Service selector → app=<newId>
  └─ 5. Argo PruneLast:       prior API Deployment → SIGTERM drain
```

Migrate Jobs are `wait_for_completion = true` before Argo image param updates. Argo sync-wave
ordering prevents Service retargeting before the new Deployment is healthy. `PruneLast` delays
prune until all waves complete; pruned pods receive SIGTERM and drain in-process.

**In-game tables** stay on old pod via headless DNS. **New tables** seed only on Service `edh-api`
(newest). Web may roll with API in the same apply; expand-only wire protects concurrent binaries
(see `docs/WIRE_COMPAT.md`).

`terraform apply` does not wait for drain to complete. Monitor drain via `kubectl port-forward`
to the Terminating pod's `:8080` then `GET /health/drain`.

### SIGTERM drain (in-process)

1. `AppState.draining` set to `true` (atomic bool).
2. `Tables.Seed` returns 503 immediately.
3. Drain loop polls: `evict_abandoned()` (removes tables with no subscribers for ≥60s) then
   checks `active_table_count()`.
4. Loop exits when `active_table_count() == 0`. Process exits cleanly.
5. If count never reaches 0, SIGKILL fires after `terminationGracePeriodSeconds` (default 24h).
6. Active SSE streams (Game.Stream) are not severed until the game ends or SIGKILL fires.
7. Distroless runtime has no shell; drain wait is entirely in-process (`main.rs`). No `preStop`
   hook needed.

### Configuration (`Settings`, `config` crate)

Loaded once at startup via `Settings::load()`:

| Env var | Setting | Default | Prod value |
|---------|---------|---------|-----------|
| `HOST` | `host` | `127.0.0.1` | `0.0.0.0` |
| `PORT` | `port` | `8080` | `8080` |
| `GRPC_PORT` | `grpc_port` | `50051` | `50051` |
| `DATABASE_URL` | `database_url` | — | `postgresql://mtgfr:<pw>@postgres:5432/mtgfr` |
| `INSTANCE_ID` | `instance_id` | `local` | Deployment name (`edh-api-1-2-3`) |
| `POD_DNS` | `pod_dns` | `""` | `{instanceId}.edh-api-headless.$(NS).svc.cluster.local` |
| `DRAIN` | `drain` | `false` | `false` (SIGTERM sets live flag) |
| `COOKIE_SECURE` | `cookie_secure` | `false` | `true` |
| `COOKIE_DOMAIN` | `cookie_domain` | `""` | `""` (host-only) |
| `CORS_ORIGIN` | `cors_origin` | `""` | `""` (same-origin via BFF) |
| `VERSION` | `version` | crate version | image release tag |
| `RUST_LOG` | (tracing) | `info` | `info` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | (OTEL exporter) | — | `http://alloy.observability.svc:4318` |
| `DEPLOYMENT_ENVIRONMENT` | (OTEL resource attr) | — | `production` |

Source precedence (later wins): built-in defaults → `config/mtgfr.toml` (committed, non-secret)
→ environment variables (`__` separator for nested keys; flat vars like `DATABASE_URL` also work).

Web BFF env (Nitro `edh-web`):

| Env var | Purpose |
|---------|---------|
| `HOST` / `PORT` | Bind address (`0.0.0.0:8080`) |
| `API_UPSTREAM` | `http://edh-api.edh.svc:8080` |
| `GRPC_UPSTREAM` | `edh-api.edh.svc:50051` |
| `WEB_DATABASE_URL` | `postgresql://mtgfr:<pw>@postgres:5432/mtgfr_web` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://alloy.observability.svc:4318` |
| `FARO_COLLECT_UPSTREAM` | `http://alloy.observability.svc:12347/collect` |
| `DEPLOYMENT_ENVIRONMENT` | `production` OTEL resource attr |

OTEL / Faro exporter behavior and scrub rules: [observability-ops](2026-07-20-observability-ops.md).

### Database migrations

**`mtgfr` (API Postgres):** Toasty ORM migration system. CLI: `cargo run -p server -- migration
apply`. Justfile: `just migrate`. Models: `User`, `Session`, `Deck`. `catalog_cards` DDL in the
initial migration (data refreshed by `catalog_search::project()` on boot). Forward-only; expand-
only during rolling deploys (nullable/default new columns; no rename/drop until drain completes).

**`mtgfr_web` (BFF Postgres):** Drizzle ORM (`drizzle-orm` / `drizzle-kit` pinned to `1.0.0-rc.4`).
CLI: `bunx drizzle-kit migrate` (`client/scripts/migrate.sh`). Justfile: `just client-migrate`.
Migrations are a single squashed **v3 baseline** (`db/migrations/20260726160151_init_lobby_routes`)
that creates `lobbies`, `lobby_seats` (with `gravatar_hash text NOT NULL DEFAULT ''`), and
`table_routes`, plus the `lobby_seats (table_id, user_id)` unique index and the
`lobby_seats → lobbies` cascade FK. Forward-only, expand-only from here. The `edh-web-migrate` Job
is Terraform-owned (hash of `client/db/migrations/**` plus a script rev); an Argo-only web image
roll does **not** apply new Drizzle files — run `terraform apply` when migrations change. The BFF
does **not** mutate or probe schema at request time; if `gravatar_hash` is missing, join/lobby GET
500 (client Unreachable).

An existing `mtgfr_web` provisioned before the v3 squash carries pre-squash rows in its
`__drizzle_migrations` journal. The `edh-web-migrate` Job reconciles that journal to the v3
baseline (tables already match the baseline DDL) before `drizzle-kit migrate`: it verifies
`lobby_seats.gravatar_hash` exists, replaces pre-squash journal rows with the baseline hash/name,
then runs migrate. A freshly created `mtgfr_web` skips reconcile and applies the baseline directly.

**`push_schema()` is dev/SQLite-test only.** Production pods assume `migration apply` ran
first (via the K8s Job before the Deployment roll).

### Container images

Both images use [Google distroless](https://github.com/GoogleContainerTools/distroless) runtime
bases (no shell, no package manager):

**`mtgfr-server` (`docker/server/Dockerfile`):**
1. Build: `rust:1-bookworm` — `protobuf-compiler`, copy `proto/` + crates, `cargo build -p server --release`.
2. Runtime: `gcr.io/distroless/cc-debian12:nonroot` — copy `server`, `config/mtgfr.toml`,
   `Toasty.toml`, `toasty/` migrations, `EXPOSE 8080`; gRPC on `50051`.
3. Entrypoints: `server serve` (default) | `server migration apply` (override for migrate Job).
4. Card pool compiled in — no runtime volume for `crates/cards/data/`.
5. Action traces: per-`instanceId` RWO PVC mounted at `/logs` (`fsGroup: 65532`); PVC retained
   on Argo prune (`Delete=false`, `helm.sh/resource-policy: keep`).

**`mtgfr-web` (`docker/web/Dockerfile`):**
1. Deps: `oven/bun:1.3.14` — `bun install --frozen-lockfile` (pin matches `client/package.json`).
2. Build: `oven/bun:1.3.14` — copy `client/`, `proto/`, and repo-root `design.tokens.json`;
   `bun run build` (Effect-gRPC codegen, DTCG token gen, typecheck, Nitro/Vite → `.output/`).
   Nitro is pinned to `preset: "bun"` in `client/nitro.config.ts` so `.output` is the Bun.serve
   build (not Node auto-detect).
3. Runtime: `oven/bun:1.3.14-distroless` as `nonroot` — copy `.output/`; entrypoint `bun`,
   `CMD [".output/server/index.mjs"]`.

Both images are pushed to public GHCR (`ghcr.io/<owner>/mtgfr-server`, `mtgfr-web`) tagged with
the release semver. No moving `latest` tag — pin explicit versions in `terraform.tfvars`.
CI/Buildx cache and release tag cascade: [ci-and-release](2026-07-20-ci-and-release.md).

---

## Implementation Decisions

- **k3s on a dedicated host; apply machine is separate** (this spec): Terraform/kubectl
  always run on the apply machine, never on the k3s node. Remote kubeconfig over LAN/VPN/Tailscale.
- **Kubernetes backend for Terraform state**: state Secret + Lease live on the same k3s cluster.
  State is gone if k3s/etcd is gone — maintain k3s etcd backups. No local state files on the
  apply machine in steady state.
- **Argo CD owns API/web Deployments + edh-api Service** ([lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md)): sync waves enforce roll
  order; `PruneLast` ensures prior Deployment prune happens after Service retarget. Terraform
  owns the headless Service (always in Terraform, never pruned by Argo).
- **Official `postgres` image StatefulSet, not CloudNativePG/Bitnami**: simple for a friend-group
  scale; less operator surface. v1 backups = k3s PVC snapshots + etcd backups.
- **Cloudflare Tunnel fully Terraform-managed**: no manual tunnel creation in the Cloudflare UI
  for steady state. Tunnel credentials as a K8s Secret consumed by `cloudflared`.
- **Distroless runtime images** (this spec): no shell in prod. Ephemeral debug containers
  for inspection. `cc` variant for dynamically-linked Rust; `oven/bun:*-distroless` for the web BFF.
- **Action traces off observability** ([observability-ops](2026-07-20-observability-ops.md)): TOON files are on a dedicated PVC, not stdout
  or Loki. Retained on pod prune. PVC names include `instanceId` to avoid collisions across
  rolling Deployments.

---

## Testing Decisions

- `iac/` is validated in CI via `terraform validate` (plan not run in CI — apply machine needs
  cluster access).
- Grafana dashboard JSON is syntax-checked locally; Terraform validation covers dashboard provider
  and file wiring.
- `crates/server/src/settings.rs` unit tests cover: default loading, env-var override, toml
  override, cors\_origin validation (valid and invalid).
- `crates/server/src/health.rs` unit tests cover: `live` version, `ready` always 200 while
  draining, `drain` status initial state.
- Container images are smoke-tested locally with `docker compose` or `kind` before production apply.
- The rolling-deploy invariant (mid-game SSE survives a binary roll) is validated by the
  `verify/SKILL.md` two-player end-to-end game verify skill.

---

## Card art CDN (`iac/card-cdn.tf`, `iac/workers/card-cdn.js`)

`edh-images.reilley.dev` is a Cloudflare Worker over the `edh-card-images` R2 bucket, both
Terraform-owned. The Worker is the only reader/writer of the bucket: a hit serves the stored
bytes with a year-long `immutable` cache, a miss fetches the print from `cards.scryfall.io`,
stores the JPEG unchanged, and serves it. A failed fill `302`s there; a Scryfall `404` is a
`404`. Fills go to Scryfall's image CDN rather than its rate-limited API.
The bucket starts empty and fills from real traffic — there is no bulk upload and no cron.

The hostname is a first-level subdomain so free Universal SSL covers it. Nothing metered is in
the request path (Workers' free 100k/day rejects rather than bills, R2 egress is free, no
Cloudflare Images subscription exists), so only R2 storage can accrue — bounded by the catalog
at roughly 20 GB fully warm. Image traffic deliberately does **not** cross the Cloudflare
Tunnel or the Nitro BFF.

`VITE_CARD_CDN` is baked at image build time from the same-named GitHub Actions repo variable
([ci-and-release](2026-07-20-ci-and-release.md)); changing the CDN needs a rebuild. Path layout
is owned by `buildImageUrl` ([deck-list-and-builder](2026-07-20-deck-list-and-builder.md)) and
duplicated in the Worker's matcher, with a round-trip test at `iac/workers/card-cdn.test.ts`
guarding the drift. It matches `cards.scryfall.io`'s own layout, so a miss maps to the upstream
URL one-for-one.

---

## Out of Scope

- **Multi-node horizontal scale of same image tag**: not in scope. Concurrent Terminating
  pods during a roll are in scope; same-image horizontal replicas require Redis or equivalent.
- **Automated Postgres dump cron**: v1 relies on k3s PVC snapshots. A dump cron is deferred.
- **Cluster bootstrap**: k3s node setup is out of scope for this repo. Terraform deploys *into*
  an existing cluster.
- **Homelab Docker/Traefik**: retired as the hosting path. Only k3s + Cloudflare Tunnel.
- **Multi-environment (staging vs prod) Terraform**: one `iac/` tree targets one cluster.
  Staging can be a second `terraform workspace` or a separate directory, not currently set up.
- **mTLS between in-cluster services**: accepted for v1. Cloudflare Tunnel provides edge TLS;
  in-cluster is HTTP only.
- CI workflow internals and semantic-release rules: [ci-and-release](2026-07-20-ci-and-release.md).
- LGTM/Faro/BFF OTEL detail: [observability-ops](2026-07-20-observability-ops.md).

---

## Further Notes

- `terraform.tfvars` is gitignored. Never commit secrets or image tags to the repo. Use
  `KUBE_CONFIG_PATH` (or `-backend-config`) to avoid baking the kubeconfig path into plan files.
- **Bootstrap (one-time, from the apply machine):** `kubectl --kubeconfig "$KUBE_CONFIG_PATH"
  create namespace terraform`, then `terraform init`. The kubernetes backend namespace must exist
  before `init` uses it.
- **One-time cutover from TF-owned Deployments to Argo:** chart must be at `argocd_target_revision`
  in git before `terraform apply`. Consider `terraform state rm` for resources Argo will adopt
  to avoid a delete/create gap.
- **Failure mode:** if Service `edh-api` selects a new instance id before the Deployment is
  Ready (mis-ordered sync without waves), Start/seed returns connection errors until the new
  Deployment becomes healthy. Sync waves prevent this in steady state.
- **Idle lobby TTL = 30 minutes** on `mtgfr_web`. Sweep is BFF-driven. Refine the TTL-reset
  events if playtesting reveals false drains (e.g. a long deck-selection phase without a ready
  toggle).
- `docs/WIRE_COMPAT.md` governs the proto expand-only rule during the drain window. This spec
  describes the deployment mechanics; `WIRE_COMPAT.md` remains the authoritative wire rule
  reference for operators and code reviewers.
- The Argo Application is a `helm_release.edh_application` (using the `argocd-apps` chart);
  if a prior apply left a failed `kubernetes_manifest.edh_application` in state, remove it with
  `terraform state rm 'kubernetes_manifest.edh_application'` before re-applying.
- Release token / tag cascade requirements: [ci-and-release](2026-07-20-ci-and-release.md).
