# production-and-ops Specification

## Purpose

Run mtgfr on a home k3s cluster behind Cloudflare Tunnel with reproducible Terraform/Argo rolls that preserve in-progress games, ship verified images via semantic-release and GHCR, and operate self-hosted LGTM/Faro telemetry without leaking private game state.

## Requirements

### Requirement: Public edge and network topology

Public traffic SHALL reach the cluster only through Cloudflare Tunnel (no inbound public ports on the k3s host). TLS SHALL terminate at Cloudflare; in-cluster traffic MAY be HTTP. The public hostname SHALL proxy to `edh-web` (`:8080`). Browser clients SHALL use same-origin `/api` only. Cloudflare Configuration Rules SHALL disable response buffering for SSE. The BFF SHALL NOT expose `/api/admin/*` or `/health/drain` on the public path; Axum health probes SHALL remain on API `:8080` inside the cluster. Public BFF meta SHALL be limited to health/version-style routes under `/api/meta/*` plus Faro collect.

#### Scenario: Player reaches the SPA over TLS without open cluster ports

- **WHEN** a player opens the public hostname in a browser
- **THEN** Cloudflare Tunnel delivers traffic to `edh-web` and no cluster node port needs to be publicly reachable

### Requirement: Kubernetes namespaces and ownership

Namespace `edh` SHALL host app workloads. Namespace `observability` SHALL host LGTM. Namespace `argocd` SHALL host Argo CD. Namespace `terraform` SHALL hold Terraform state Secret and lock Lease. Argo Application `edh` SHALL own API/web Deployments and ClusterIP Service `edh-api` (selector = newest `apiActiveInstanceId`) with sync waves and `PruneLast`. Terraform SHALL own headless Service `edh-api-headless` (`publishNotReadyAddresses=true`), Postgres StatefulSet, migrate Jobs, cloudflared, NetworkPolicies, tunnel/DNS, secrets, and the observability stack. Concurrent Terminating pods during a roll are in scope; same-image horizontal scale-out of the API registry is not.

#### Scenario: Newest-only seed service vs sticky headless dials

- **WHEN** a rolling deploy has a Ready new API Deployment and a Terminating prior Deployment
- **THEN** `edh-api` selects only the newest instance for seed/auth/decks/catalog while `edh-api-headless` remains dialable for in-game `table_routes` pod DNS

### Requirement: Rolling deploy and SIGTERM drain

`terraform apply` from an apply machine with remote kubeconfig SHALL: run Toasty migrate Job on `mtgfr`, run Drizzle migrate Job on `mtgfr_web`, let Argo sync-wave 0 roll the new API Deployment, retarget `edh-api` on wave 1, then prune the prior Deployment last. Pruned pods SHALL drain in-process on SIGTERM (`draining=true`, refuse Seed with 503, evict abandoned tables, exit at zero active tables or after `api_termination_grace_seconds`, default 24h). Distroless API images SHALL wait in-process without a shell `preStop`. `terraform apply` SHALL NOT wait for drain completion. Wire changes during drain SHALL follow expand-only rules in `docs/WIRE_COMPAT.md` except intentional majors.

#### Scenario: Migrate Jobs gate the image roll

- **WHEN** operators apply new API or web images that need schema changes
- **THEN** the corresponding migrate Job completes before Argo retargets traffic to the new workload generation

#### Scenario: In-progress game survives prune

- **WHEN** Argo prunes the prior API Deployment while a table still has players
- **THEN** the Terminating pod keeps serving that table via headless DNS until the game ends or grace expires

### Requirement: Runtime configuration

API `Settings` SHALL load defaults, then `config/mtgfr.toml`, then environment (`HOST`, `PORT`, `GRPC_PORT`, `DATABASE_URL`, `INSTANCE_ID`, `POD_DNS`, `COOKIE_SECURE`, `VERSION`, `OTEL_EXPORTER_OTLP_ENDPOINT`, `DEPLOYMENT_ENVIRONMENT`, optional `MTGFR_MASTER_SEED` / `master_seed`, and related keys). Web BFF SHALL receive `API_UPSTREAM`, `GRPC_UPSTREAM`, `WEB_DATABASE_URL`, `OTEL_EXPORTER_OTLP_ENDPOINT`, `FARO_COLLECT_UPSTREAM`, and `DEPLOYMENT_ENVIRONMENT`. OTEL exporters SHALL no-op when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset. Secrets and image tags SHALL live in gitignored `iac/terraform.tfvars`, not committed source.

#### Scenario: Local without OTEL endpoint is quiet

- **WHEN** a developer runs API or BFF without `OTEL_EXPORTER_OTLP_ENDPOINT`
- **THEN** OTLP export is a no-op and local suites do not require Alloy

### Requirement: Databases and migrations

Postgres SHALL provide databases `mtgfr` (API/Toasty: users, sessions, decks, catalog projection DDL) and `mtgfr_web` (BFF/Drizzle: lobbies, lobby_seats with `gravatar_hash`, table_routes). Production SHALL apply migrations via Jobs before rolls; API pods SHALL NOT rely on `push_schema()`. `mtgfr_web` migrations SHALL be the squashed v3 baseline forward; the web-migrate Job SHALL reconcile pre-squash journals when needed. Schema changes during rolling deploys SHALL be expand-only. The BFF SHALL NOT mutate schema at request time.

#### Scenario: Fresh mtgfr_web gets the lobby baseline

- **WHEN** `edh-web-migrate` runs against an empty `mtgfr_web`
- **THEN** `lobbies`, `lobby_seats` (including `gravatar_hash`), and `table_routes` exist before web serves lobby traffic

### Requirement: Container images

`mtgfr-server` SHALL build from `docker/server/Dockerfile` (Rust release binary, distroless `cc` nonroot runtime, card pool compiled in, Toasty migrations packaged; gRPC `:50051`, health `:8080`; optional per-instance action-log PVC at `/logs` retained across prune). `mtgfr-web` SHALL build from `docker/web/Dockerfile` (Bun build of Foldkit/Nitro with `preset: "bun"`, distroless Bun runtime). Images SHALL publish to GHCR as semver tags without a moving `latest` app tag; operators SHALL pin explicit versions in `terraform.tfvars`.

#### Scenario: Migrate Job reuses the server image entrypoint

- **WHEN** the Toasty migrate Job runs
- **THEN** it invokes the server image with `migration apply` against `DATABASE_URL` rather than requiring a separate migrate image

### Requirement: Card art CDN

Card art SHALL be served from Terraform-owned Cloudflare Worker + R2 (`edh-images.reilley.dev` / `edh-card-images`) without crossing the game Tunnel or Nitro BFF. The Worker SHALL be the only bucket reader/writer: cache hit serves stored WebP with long-lived `immutable` cache headers; miss fetches Scryfall image CDN bytes at the matched path, stores unchanged WebP, and serves; non-404 fill failures SHALL `302` to Scryfall; Scryfall `404` SHALL be `404`. Object keys SHALL be `{thumb|grid|display|art|crop}/{front|back}/{a}/{b}/{id}.webp` aligned with client `buildImageUrl`. `VITE_CARD_CDN` SHALL be baked at web image build time. Path layout SHALL be guarded by Worker tests against client URL construction.

#### Scenario: Cold miss fills from Scryfall image CDN

- **WHEN** a client requests a valid CDN path that is not yet in R2
- **THEN** the Worker fetches `cards.scryfall.io` at the same layout, stores the WebP on success, and serves it

#### Scenario: Invalid path does not proxy arbitrarily

- **WHEN** a request path fails the size/face/`a`/`b`/id layout check
- **THEN** the Worker returns 404 before any outbound fetch or bucket write

### Requirement: Observability plane

Self-hosted LGTM (Alloy, Loki, Tempo, Prometheus, Grafana) SHALL run in namespace `observability`. Grafana SHALL be operator-only via `kubectl port-forward` with no public tunnel hostname. Alloy SHALL be the sole ingest path. Loki retention SHALL be 7d; Tempo 7d with metrics-generator/`local-blocks` enabled for TraceQL metrics; Prometheus 15d. Terraform SHALL provision operator dashboards from `iac/dashboards/*.json` including `mtgfr OTEL RED` and `mtgfr Faro RUM`.

#### Scenario: Operator opens Grafana privately

- **WHEN** an operator port-forwards `svc/grafana` in namespace `observability`
- **THEN** they can use provisioned dashboards without a public Grafana hostname

### Requirement: OpenTelemetry semantic conventions

Deployed browser → BFF → API telemetry SHALL follow OpenTelemetry Semantic Conventions **1.37.0** plus deliberate `mtgfr.*` extensions under a shared allow/deny dictionary. Scrub rules SHALL win over conventions. Allowed families include resource (`service.name`, `service.version`, `service.instance.id`, `deployment.environment`, `vcs.ref.head.revision`), HTTP (method, status, low-cardinality route/path, scheme, server.address), RPC/gRPC (`rpc.system=grpc`, `rpc.service`, short `rpc.method`, `rpc.grpc.status_code`), safe DB (`db.system=postgresql`, `db.operation.name`, `db.namespace`), safe exceptions (`exception.type`, safe `exception.escaped`), and `mtgfr.table.id` / `mtgfr.intent.kind` / `mtgfr.intent.accepted` / `mtgfr.user.id`. Forbidden attributes include auth/session tokens, cookies, request/response bodies, SQL text/parameters, hand/library fields, `mtgfr.intent.payload`, and unlisted game keys. Span names SHALL stay low-cardinality and SHALL NOT include card names, player names, or intent bodies. Engine code SHALL emit local `tracing` only and SHALL NOT export OTEL.

#### Scenario: Submit span carries allowlisted mtgfr keys only

- **WHEN** the API records a game submit span
- **THEN** attributes may include `mtgfr.table.id`, `mtgfr.intent.kind`, `mtgfr.intent.accepted`, and `mtgfr.user.id` and must not include intent payload or hand/library contents

#### Scenario: BFF DB span omits SQL text

- **WHEN** the BFF opens a `mtgfr_web` database span
- **THEN** it sets safe DB attributes only and does not attach `db.query.text` or statement strings

### Requirement: Trace propagation and Faro

Browser Faro SHALL post to same-origin `/api/faro/collect`; the BFF SHALL proxy to Alloy `faro.receiver`. `traceparent` propagation SHALL be same-origin `/api` only. The BFF SHALL continue inbound `traceparent` as parent only when sampled (`traceFlags & 0x01`); unsampled Faro spans SHALL be ignored to avoid Tempo orphans. BFF gRPC outbound calls SHALL carry trace context via an explicit per-request env bag (not Node AsyncLocalStorage) across separate ManagedRuntimes. Faro collect SHALL reject bodies over 512 KiB with HTTP 413. Faro log streams SHALL carry low-cardinality `kind` and `app_name` labels; web-vitals measurements SHALL be mirrored into Prometheus histograms via Alloy. TOON action traces under `ACTION_LOG_DIR` SHALL stay off Loki and stdout observability paths.

#### Scenario: Oversized Faro payload is rejected

- **WHEN** a browser posts more than 512 KiB to `/api/faro/collect`
- **THEN** the BFF returns 413 without forwarding the body to Alloy

#### Scenario: Unsamped browser parent is dropped

- **WHEN** a request arrives with `traceparent` whose flags are unsampled
- **THEN** the BFF starts a new root span rather than parenting under a span Tempo will never receive

### Requirement: Commit convention and release authorship

Commits and squash-merge PR titles SHALL follow Angular conventional commits. Husky `commit-msg` SHALL run commitlint locally; Cursor Cloud SHALL chain the same hook after `npm clean-install`. semantic-release (default Angular analyzer, no custom `.releaserc`) SHALL be the only writer of `v*` tags and GitHub Releases. Hand-created version tags are forbidden. Repo secret `RELEASE_TOKEN` (PAT with `contents` + `workflow`) SHALL be required so tag push can cascade `docker.yml`. Squash-merge means semantic-release analyzes the PR title (plus major `BREAKING CHANGE` footer) only.

#### Scenario: feat PR cuts a release tag

- **WHEN** a `feat:` PR is squash-merged to `main` and verify passes
- **THEN** semantic-release creates a `v*` tag and GitHub Release without a human pushing the tag

#### Scenario: docs-only PR skips a version bump

- **WHEN** a `docs:`-titled PR merges and verify is green
- **THEN** semantic-release does not cut a new version tag

### Requirement: PR and main verify

`ci.yml` on PRs SHALL use concurrency `ci-${{ github.ref }}` with `cancel-in-progress: true`, lint the PR title with commitlint, call reusable `verify-jobs.yml`, and run `terraform validate` when `iac/**` (or the workflow) changes. `verify-and-release.yml` on push to `main` SHALL run `verify-jobs.yml` then `npx semantic-release`.

`verify-jobs.yml` SHALL provide:

- **verify-server**: pass-marker gate (`verify-server-v3-*` content hash) with restore-only gate and save-only mark after success; on miss, parallel `verify-server-lint`, three nextest shards (`cargo nextest run --profile ci --partition count:i/3`), and `verify-server-migrate` inside `ghcr.io/<owner>/mtgfr-ci:latest` (`--user root`); lint includes CR index, card schema/DSL/pool checks, fmt, and clippy; nextest shards SHALL NOT start Postgres; migrate SHALL use Postgres 16 + `just migrate` only; shared `Swatinem/rust-cache` key `verify-server`; aggregator job green on cache hit or full miss success
- **verify-client**: Bun-only `just client-check` with its own pass marker hashing client/proto/tokens/workflow inputs (not `crates/**`)
- **verify-wire**: no pass marker; `buf lint` under full `STANDARD` with no `except`/`ignore`/`ignore_only`; on pull requests, `buf breaking` category `WIRE` against `origin/main` unless PR title or body contains `BREAKING CHANGE`; main pushes run lint only
- **verify-openspec**: no pass marker; install pinned `@fission-ai/openspec` and run `just openspec-check` (`openspec validate --all --strict --no-interactive`) so living specs and active change artifacts keep valid structure

#### Scenario: Server pass-marker hit skips miss-path jobs

- **WHEN** `verify-server-gate` restores a matching `.ci-pass` marker
- **THEN** lint, nextest shards, migrate, and mark jobs are skipped and the aggregator is green

#### Scenario: Wire breaking skips on major marker

- **WHEN** a PR title or body contains `BREAKING CHANGE`
- **THEN** `verify-wire` runs `buf lint` and skips `buf breaking`, treating the release as a hard cut

#### Scenario: Nextest shards need no database service

- **WHEN** server verify misses the pass marker
- **THEN** the three nextest partition jobs run without Postgres while migrate alone starts Postgres and applies Toasty migrations

#### Scenario: OpenSpec structural validate runs on every verify

- **WHEN** `verify-jobs.yml` runs on a PR or main push
- **THEN** `verify-openspec` installs the pinned OpenSpec CLI and fails if living specs or active changes fail strict validation

### Requirement: Release images and CI toolchain image

`docker.yml` on `v*` tags SHALL build/push `mtgfr-server` and `mtgfr-web` to GHCR in parallel with Buildx GHA layer cache scopes `mtgfr-server` / `mtgfr-web`, then attempt to mark packages public. `ci-image.yml` SHALL build/push `ghcr.io/<owner>/mtgfr-ci:latest` when `docker/ci/**` or that workflow changes on `main` (or `workflow_dispatch`), with cache scope `mtgfr-ci`. Server verify miss-path jobs SHALL pull that CI image rather than installing Rust/protoc/just/nextest on the runner.

#### Scenario: Tag push builds both app images

- **WHEN** semantic-release pushes a new `v*` tag
- **THEN** `docker.yml` publishes version-tagged `mtgfr-server` and `mtgfr-web` images to GHCR

#### Scenario: CI image publishes for verify consumers

- **WHEN** `docker/ci/**` changes land on `main`
- **THEN** `ci-image.yml` pushes an updated `mtgfr-ci:latest` used by server verify containers
