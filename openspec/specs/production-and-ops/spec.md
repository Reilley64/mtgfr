# production-and-ops Specification

## Purpose

Run mtgfr on a home k3s cluster behind Cloudflare Tunnel with reproducible Terraform/Argo rolls that preserve in-progress games, ship verified images via semantic-release and GHCR, and operate self-hosted LGTM/Faro telemetry without leaking private game state.

## Requirements

### Requirement: Public edge and network topology

Public traffic SHALL reach the cluster only through Cloudflare Tunnel (no inbound public ports on the k3s host). TLS SHALL terminate at Cloudflare; in-cluster traffic MAY be HTTP. The public hostname SHALL proxy to `edh-web` (`:8080`). Browser clients SHALL use same-origin `/api` only. Cloudflare Configuration Rules SHALL disable response buffering for SSE. The BFF SHALL NOT expose `/api/admin/*` or `/health/drain` on the public path; Axum health probes SHALL remain on API `:8080` inside the cluster. Public BFF meta SHALL be limited to health/version-style routes under `/api/meta/*` plus Faro collect. A Terraform-owned Cloudflare Cache Rule SHALL make `/api/meta/coverage/v1` cache-eligible at the edge while respecting origin cache directives; the BFF route SHALL own the TTLs via its `cache-control` response header (`s-maxage` for the edge, a short `max-age` for browsers, which a purge cannot reach). A Terraform apply that changes the deployed images SHALL purge that single URL (free-plan purge-by-URL; no tag or prefix purge).

#### Scenario: Player reaches the SPA over TLS without open cluster ports

- **WHEN** a player opens the public hostname in a browser
- **THEN** Cloudflare Tunnel delivers traffic to `edh-web` and no cluster node port needs to be publicly reachable

#### Scenario: Deploy purges the cached coverage meta

- **WHEN** an apply rolls a new `server_image` or `web_image`
- **THEN** the coverage meta URL is purged from the Cloudflare edge so the new faithful counts are not held for the remaining edge TTL

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

### Requirement: Debug builds expose an authoritative development API

Every Rust server build with debug assertions enabled SHALL register the unauthenticated `mtgfr.debug.v1.DebugService` on the existing gRPC listener automatically. The service SHALL expose exactly six RPCs: `ListTables`, an unfiltered structural `InspectTable`, atomic `MutateTable`, `CheckpointTable`, `RestoreCheckpoint`, and audit-only `GetDebugJournal`. Listing SHALL expose active table identifiers with debug revisions and table sequences without game contents. Inspection SHALL expose authoritative players, zones, objects, stack inspection, turn and priority state, the complete pending-orchestration counters, logical chrome, sorted checkpoint names, table sequence, debug revision, and whether the table was debug-mutated. An inspected library SHALL retain its authoritative order; an inspected hand SHALL be an unordered collection of object identifiers.

Mutation SHALL accept an ordered batch containing exactly fifteen typed operations: set life; set a poison or rad player counter; set turn state; set permanent tapped, damage, and +1/+1-counter state; set controller; set attachment; create a card; move a card; set library order; remove a card; coherently clear pending orchestration; replace the stack; push or pop stack entries; and set or clear an exact-object printing override. The clear SHALL reset pending choice, all deferred resume riders, resolution frame and finish state, pending enter-bonus counters, and clash scratch, SHALL optionally clear queued trigger groups and obligations, and SHALL reset logical yields, turn yields, and hold request. The API SHALL NOT accept a generic patch or internal snapshot. `CreateCard` and `MoveCard` SHALL require the supplied new object identifier to equal the arena's exact next identifier; a move SHALL tombstone the source and mint the fresh destination identity required by CR 400.7. The editor SHALL enforce structural safety and successful production projection, not whether normal Magic costs, priority, timing, or zone legality could have produced the state.

`MutateTable` SHALL support independent optional `expected_debug_revision` and `expected_table_seq` guards. `CheckpointTable` SHALL support an optional `expected_table_seq` guard. `RestoreCheckpoint` SHALL support both optional guards and SHALL reject a revision mismatch before a table-sequence mismatch when both are stale. Every write SHALL lock the table once, preflight every guard, checked counter increment, structural/projection condition, and storage limit, and commit atomically only after all preconditions pass. Any failed write SHALL leave the game, logical chrome, debug revision, live sequence and broadcast sequence, checkpoints, journal totals and ordinal, and stream output unchanged; capacity failures SHALL NOT evict retained records or checkpoints.

Checkpoint names SHALL contain 1 through 64 ASCII bytes and match `[A-Za-z0-9._-]+`. A table SHALL retain at most 16 checkpoints, at most 4,096 arena object slots in one checkpoint, and at most 32,768 slots across its checkpoints; the slot count SHALL include moved and removed tombstones. Replacing a name SHALL require explicit `replace_existing = true`, SHALL account for `current total - replaced slots + new slots` with checked arithmetic, and SHALL keep the prior checkpoint when any later preflight fails.

A checkpoint SHALL contain a clone of `Game`, the server-owned exact-object printing override map, logical yield flags, logical turn-yield flags, whether a hold was requested, the source table sequence for provenance, and its logical slot count. The logical slot count SHALL include both retained engine arena slots and retained exact-object override entries for per-checkpoint and aggregate limits. It SHALL NOT store or restore the live sequence, broadcast sequence, sender, quiet-since state, timer or wall-clock instants, dwell flags, checkpoint map, or journal. A successful mutation or restore SHALL advance the live sequence, broadcast sequence, and debug revision exactly once and publish one complete replacement snapshot freshly filtered for every owner, opponent, and spectator. Restore SHALL install the saved game and logical chrome without copying a timer, use a new monotonically increasing live sequence rather than the saved source sequence, synchronously arm any still-valid requested hold against that new sequence while locked, publish the armed snapshot, release the registry lock, and only then spawn timer polling. A racing ordinary intent or pre-restore timer SHALL make stale post-lock work harmless through the sequence guard. Later accepted ordinary intents SHALL continue through the normal event path and publish deltas after the replacement snapshot.

The per-table debug journal SHALL retain at most 1,024 successful records and at most 1,048,576 aggregate bytes measured by `prost::Message::encoded_len()` on each original mutation, checkpoint, or restore protobuf request before destructuring. Capacity SHALL be preflighted before every game swap or checkpoint insert/replace, with no eviction. Successful mutation batches, checkpoint creates/replacements, and restores SHALL appear in ordinal order with their result revision, result table sequence, encoded request bytes, and operator-order timestamp. A successful `MutationCommitted` record SHALL deliberately retain and return the complete ordered typed operations for audit and fixture provenance, including card identifiers and hidden-zone object identifiers; because this approved debug-only service is unauthenticated, callers of a debug build may observe those values. Mutation payloads SHALL remain excluded from application logs and telemetry. Failed writes, ordinary intents, and checkpoint bodies SHALL be absent from the journal. The journal is audit/provenance only and SHALL NOT claim complete replay because ordinary intents may interleave.

#### Scenario: A guarded development mutation commits atomically

- **WHEN** an unauthenticated debug caller supplies matching optional guards and all typed operations, structural checks, projections, and storage preflights succeed
- **THEN** the candidate replaces the table, the debug revision and both live transport watermarks advance exactly once, the audit record is appended, and each owner, opponent, and spectator stream receives a fresh snapshot filtered for that viewer

#### Scenario: Ordinary play makes a debug guard stale

- **WHEN** an ordinary accepted intent advances the table after a debug caller observed its sequence
- **THEN** a checkpoint, mutation, or restore carrying that old table-sequence guard returns `ABORTED` and changes no authoritative, storage, or stream state

#### Scenario: Capacity failure is transactional and never evicts

- **WHEN** a checkpoint or journal write would exceed an exact count, object-slot, record, or encoded-request-byte bound
- **THEN** it returns `RESOURCE_EXHAUSTED`, preserves every retained checkpoint and journal record, and changes no game, chrome, revision, sequence, ordinal, or stream

#### Scenario: Restore invalidates old hold work

- **WHEN** a checkpoint with a logical hold request is restored after the live table and an older hold task have advanced
- **THEN** restore uses a new live sequence, arms only a still-valid hold at that sequence while locked, starts polling after unlock, and the older task cannot submit or clear the new hold

#### Scenario: Journal order is provenance rather than replay

- **WHEN** successful debug writes and ordinary intents interleave
- **THEN** journal ordinals order only the successful debug mutation, checkpoint, and restore records, omit ordinary intents and checkpoint bodies, and make no replay-completeness claim

#### Scenario: A stale or invalid batch rolls back

- **WHEN** either supplied guard is stale or any operation, structural invariant, viewer projection, counter increment, or storage preflight fails
- **THEN** the service returns the stable typed failure without changing authoritative state, revisions, provenance, retained storage, or stream output

#### Scenario: Structurally safe rule-illegal state is accepted

- **WHEN** a typed batch creates a representable, referentially sound, projectable state that ordinary Magic play could not legally produce
- **THEN** the debug service accepts it without treating rules legality as a structural invariant

### Requirement: Debug failures and tooling do not disclose payloads

Debug RPC failures SHALL use stable gRPC statuses (`NOT_FOUND`, `ALREADY_EXISTS`, `INVALID_ARGUMENT`, `FAILED_PRECONDITION`, `ABORTED`, or `RESOURCE_EXHAUSTED` as applicable) plus a typed detail containing an optional operation index, stable reason, and a bounded sanitized list of non-secret structural violation codes. The public status message and violation text SHALL be generic and SHALL NOT echo table identifiers, checkpoint names, card identities, request fields, or private state. Debug request and response payloads, unfiltered inspections, table identifiers, checkpoint names, hidden identities, and journal bodies SHALL NOT enter application logs or telemetry.

The checked-in `mtgfr-debug` CLI SHALL expose all six service workflows as `tables`, `inspect <table> [--out <file>]`, `mutate <protobuf-json-file> [--out <file>]`, `checkpoint <table> <name>`, `restore <table> <name>`, and `journal <table>`, plus a checked `stack-fixture <table>` workflow that inspects authoritative identifiers before sending one typed mutation batch, with command-specific guards and output flags, a global endpoint flag, `MTGFR_DEBUG_ENDPOINT` fallback, and the local debug listener as its default. An optional mutation table-sequence flag SHALL agree with any value already present in the protobuf JSON. The CLI SHALL emit protobuf JSON, render stable typed failures with operation indexes, write requested output atomically with private default permissions, and SHALL NOT log request bodies automatically. The corresponding `debug-tables`, `debug-inspect`, `debug-mutate`, `debug-checkpoint`, `debug-restore`, `debug-journal`, and `debug-stack-seven` recipes SHALL preserve every argument as a literal argv element without shell evaluation.

#### Scenario: A failed CLI mutation is safe to retain

- **WHEN** the CLI receives a typed debug failure
- **THEN** it prints only the stable status, reason, optional operation index, and sanitized violations without printing the request body or hidden identity

### Requirement: Release artifacts omit the authoritative debug API

Release compilation SHALL omit debug protobuf exposure, all six service implementations and registrations, engine/server raw-editor hooks, checkpoint and journal storage, and identifying service, route, and implementation-marker strings from the production server. A release build of the checked-in CLI target SHALL be only a non-operational stub, and the production image SHALL contain the gated release server rather than the CLI. The production descriptor SHALL omit the `mtgfr.debug.v1` package, browser wire generation SHALL exclude that package and `DebugService`, and production service authentication and per-viewer visibility requirements SHALL remain unchanged.

CI SHALL build and scan the production-equivalent target (`server` package, release profile, `server` binary), proving that the debug service FQN, RPC path prefix, and implementation marker are absent from its bytes and that exactly one production descriptor contains no debug package. The production Docker build SHALL independently build the equivalent package/profile/binary target and repeat the forbidden-byte gate on the exact server binary it copies into the runtime image. Browser generation exclusion, release all-target compilation of the CLI stub, and CLI recipe argument safety SHALL remain verification gates.

The shared engine and production projection contract SHALL include stable stack-entry identities and an exhaustive source-independent public-ghost representation. Only a debug-assertion-only engine constructor MAY create that constrained source-less, targetless, no-op ghost; release builds SHALL retain the inert engine enum and wire/projection shape solely so projection is total, and SHALL NOT contain that constructor or an operational service that can create the ghost.

The production projection and self-contained stream publication shape SHALL accept a transient exact-object Printing UUID map without adding it to serialized `Game` state or to the wire payload. Live ownership of that map SHALL exist only inside the debug-assertion-only table transaction state; release tables and every release opening, delta, and hold publication SHALL supply an empty map. The internal debug transaction SHALL require an exact live object ID and a nonblank UTF-8 printing identifier of at most 64 bytes when setting an override, allow idempotent clearing after that identity dies, carry a source card's exact override to the exact next spell object created by checked known-spell construction, and remove overrides whose exact object becomes moved or removed without migrating them across unrelated zone changes. Debug checkpoints SHALL snapshot and restore the map with the game and logical chrome so replacement projection and publication use one atomic presentation state. Carrying the always-present inert publication input SHALL NOT expose a browser debug route and remains subject to debug-only release isolation.

The debug-assertion-only server domain SHALL use one ordered `TableMutation` transaction for the existing eleven engine mutations, checked stack replacement/push/pop, and exact-object printing overrides. It SHALL clone game, printing overrides, and logical chrome; prune overrides whose exact object identity is no longer live before applying the candidate; report every engine, stack-entry, and printing failure at the containing global operation index; validate and project every role from the complete candidate; preflight all counters and audit bounds; and then swap, journal, and publish exactly once. Clearing an override SHALL be idempotent even when its exact object is already dead; setting one SHALL still require a live exact object. Every accepted ordinary intent SHALL prune dead exact-object overrides under the same table lock before projection and publication, without migrating an override across a zone-change identity. The debug protobuf SHALL append the typed stack and print operation messages so every successfully retained journal operation has a total faithful wire representation. The gRPC request adapter SHALL accept all fifteen typed arms, preserve the original full protobuf encoded length before destructuring, validate every nested union and narrowing conversion, and report stack, print, and engine failures at the containing global operation index. Inspection SHALL expose stable stack-entry identity, authoritative targets, optional bounded public-ghost metadata, and presence-aware authoritative next-object and next-stack-entry allocator frontiers whose absence means that identity domain is exhausted. The checked stack-fixture CLI workflow SHALL inspect first, require both authoritative allocator frontiers, preflight enough sequential in-range zero-based object identities for six source objects and their exact six spell conversions, plus seven nonzero stack-entry identities, then submit six checked targetless known spells and one public ghost in a single atomic mutation without calling mutation when preflight fails. It SHALL NOT add a browser/BFF debug route or accept wire-supplied arbitrary effects.

#### Scenario: Release artifacts exclude the authoritative debug service

- **WHEN** release-isolation, production-image, descriptor, browser-generation, and release all-target gates inspect production artifacts
- **THEN** no operational debug service, debug package, debug RPC path, implementation marker, browser binding, public-ghost constructor, or production-runtime debug executable is present, while stable stack identities and the inert production projection shape remain available

#### Scenario: Inert stack shape exposes no debug operation

- **WHEN** a release build compiles the source-independent stack enum and production projection arms
- **THEN** no release caller can construct a public ghost or invoke stack replacement, push, pop, printing override, fixture authoring, or arbitrary-effect operations through a debug service; the engine-only checked editor is absent with debug assertions disabled

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
