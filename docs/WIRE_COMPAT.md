# Wire backwards compatibility

Durable rules for the proto / gRPC wire contract during a rolling deploy. See
[wire-protocol](../openspec/specs/wire-protocol/spec.md),
[lobby-and-live-game](../openspec/specs/lobby-and-live-game/spec.md),
[production-and-ops](../openspec/specs/production-and-ops/spec.md).

## Why this exists

Rolling deploy keeps **outgoing** API pods Terminating (SIGTERM drain) while **newest** accepts
new tables via Service `edh-api`. The Foldkit SPA may roll with newest; mid-game clients still
talk to older pods via BFF `table_routes` → pod DNS on the headless Service.

So every concurrent instance version must speak a wire protocol the current SPA/BFF can parse —
**expand-only** across the whole set until grace expires / pods exit.

The automated gate is `verify-wire` in `.github/workflows/verify-jobs.yml`; local equivalents
are `just proto-lint`, `just proto-breaking`, and `just proto-check`. Proto lint uses Buf
`STANDARD` with no silenced rules (`except`, `ignore`, or `ignore_only`), and proto breaking
uses Buf `WIRE` against `origin/main` for ordinary PRs. The `--against` input names
`subdir=proto`, because `proto/buf.yaml` — and therefore the import root the `.proto`
files are written against — lives under `proto/`, not the repository root.

## Transport migration (wire-protocol-and-visibility spec)

The OpenAPI/REST/SSE → Effect RPC + gRPC cutover is a **hard cut**: API and web ship together.
No N/N−1 coexistence between REST and gRPC is required for that release. In-flight tables may
drop. After that cut, the rules below apply to **gRPC/proto only**.

## 1. Compatibility window

All concurrent API binaries until each Terminating pod exits (tables empty or
`terminationGracePeriodSeconds`). No ConfigMap peer registry.

## 2. Expand-only during that window

Within one release's changes to `proto/` (including `common` / `catalog` / `intent` / `stream`)
and the generated Rust/TS bindings:

- **Additive optional fields only.** New protobuf fields use new field numbers; never reuse.
- **New RPCs / Intent / Event / PendingChoice variants** are safe to add — old peers never send
  them. New `oneof` arms need new field numbers inside the parent message.
- **Do not rename, remove, or repurpose** field numbers while any older binary may still serve a
  table the current SPA still reaches via `table_routes`.
- There is no JSON-in-proto escape hatch: game stream frames, intents, decks, cards, and seed
  are all native messages. Expand those trees the same way as any other proto message.

## 3. Hard breaks / majors

Service renames, gRPC path changes, field removals, field renumbering, and incompatible type
changes are hard breaks. Prefer a new proto package/path such as `mtgfr/v2` for later intentional
hard breaks when practical.

An in-place hard break under the current package is allowed only on a semver-major PR: the PR
body (squash commit body) must include an Angular `BREAKING CHANGE:` footer. That PR skips
`buf breaking`, cuts a major release through semantic-release, and is a hard cut: no N↔N−1
wire coexistence is promised for that release. After the merge, `main` is the new breaking
baseline for subsequent PRs. (`@commitlint/config-angular` forbids `!:` in the subject; do not
put a bang in the PR title.)

## 4. Lobby vs game

Lobby Effect RPC is owned by the BFF (`mtgfr_web`). Game stream/intent RPCs stay on tonic; the
BFF dials `{pod_dns}:50051` from `table_routes`.
