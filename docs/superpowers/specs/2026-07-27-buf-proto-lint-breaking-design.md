# Buf proto lint + breaking (design)

**Status:** Shipped (2026-07-27; full `STANDARD` via rename, no rule exceptions,
`verify-wire` gate live). Living surfaces now document shipped behavior.
**Surfaces:** `ci-and-release` (`.github/workflows/verify-jobs.yml`, `justfile`),
`wire-protocol-and-visibility` (codegen / compat testing + gRPC service names),
`docs/WIRE_COMPAT.md` (ops rules + major/`/v2` exception path), `proto/mtgfr/v1/*.proto`.

Related living docs: [ci-and-release](2026-07-20-ci-and-release.md),
[wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md),
[`docs/WIRE_COMPAT.md`](../../WIRE_COMPAT.md).

---

## Problem Statement

Expand-only wire rules for the rolling-deploy drain window live in prose
(`docs/WIRE_COMPAT.md`) and code review. `proto/buf.yaml` already declares `lint.use:
[STANDARD]`, and the client already runs `buf generate`, but **`buf lint` and `buf breaking`
are never run in CI or via `just`**. Accidental field renumbering, removals, or other hard
breaks can land on a non-major PR and break N↔N−1 pods during drain.

Today `buf lint` with full `STANDARD` also fails (~60 findings) on `mtgfr.proto` service and
RPC request/response naming. Those names should be brought up to Buf STANDARD — **not**
silenced with `lint.except` / `ignore` / `ignore_only`.

## Goal

1. Bring `proto/` to a clean `buf lint` under full `STANDARD` by renaming services and
   giving every RPC its own STANDARD-named request/response types.
2. Machine-check expand-only compatibility with `buf breaking` (`WIRE`) on ordinary PRs,
   with an explicit major-version / hard-break escape hatch aligned to semantic-release and
   `WIRE_COMPAT.md`.

Do not replace tonic Rust codegen, Buf Schema Registry, or remote plugins.

## Locked decisions

| Decision | Choice |
|---|---|
| Where to run | Local `just` recipes **and** a dedicated `verify-wire` job in `verify-jobs.yml` (PR + main) |
| `buf lint` category | Full `STANDARD` — **no** `except`, `ignore`, or `ignore_only` |
| Lint debt | Rename services + per-RPC request/response types until `buf lint` is clean |
| `buf breaking` category | `WIRE` (drain-window parse compatibility), not `FILE` |
| Breaking baseline (PRs) | `origin/main` via `buf breaking --against '.git#branch=origin/main'` |
| Breaking on `main` push | Lint only; breaking is vacuous against self — do **not** require a tag baseline on main |
| Major / hard-break hatch | Skip `buf breaking` when the **PR title** is an Angular major (`!:` bang **or** contains `BREAKING CHANGE`) |
| Shipped hatch detection | `verify-wire` also accepts `BREAKING CHANGE` in the **PR body** (squash commit footer); commitlint forbids `!:` in subjects |
| First implement PR shape | **One major PR** (`feat!:` …): STANDARD renames + call-site/codegen updates + turn on `verify-wire` / `just proto-*` together |
| Preferred later hard-break shape | New package path `mtgfr/v2` when practical; in-place breaks only via the major-title hatch |
| Tooling source | Existing `@bufbuild/buf` from `client/` (`bun install`); no new GHCR image bake for this work |
| Rust codegen | Unchanged generator (`tonic_build` in `crates/server/build.rs`); stubs regenerate under new service/message names |

## Approaches considered

### Gate placement / breaking baseline

1. **Dedicated `verify-wire` + `just proto-*` + PR breaking vs `origin/main` + major-title skip (chosen)** —
   Smallest enforceable ongoing gate. Lint always; breaking only where it matters (PRs).
   Major hatch matches squash-merge / semantic-release (PR title is the release subject).
2. Fold lint/breaking into `verify-client` only — Reuses Bun install, worse failure UX.
3. Breaking against last `v*` tag — More edge cases; little extra drain safety once every
   merge already passed vs `main`.

### Clearing today’s `STANDARD` failures

1. **`lint.except` the four naming rules (rejected)** — Silent debt; fights “bring lint up
   to scratch.”
2. **Rename to full `STANDARD` (chosen)** — Service `*Service` suffixes + unique
   `*Request`/`*Response` types per RPC. Service renames change gRPC paths
   (`/mtgfr.v1.Auth/Login` → `/mtgfr.v1.AuthService/Login`), so this ships as a **hard-cut
   major** (API + web together; no N↔N−1 for that release).
3. **Add `mtgfr.v2` package with corrected names and leave `v1` (rejected for this pass)** —
   Correct long-term shape for some hard breaks, but doubles surface area and leaves
   `STANDARD`-unclean `v1` still loaded. Prefer in-place rename under `mtgfr.v1` for this
   lint cleanup; reserve `/v2` for later intentional wire breaks.

## Design

### Lint (`buf lint`) — full STANDARD via rename

Keep `proto/buf.yaml` on `STANDARD` with **no rule turn-offs**. Measured on current `main`
(`@bufbuild/buf@1.72.0`), all failures are in `proto/mtgfr/v1/mtgfr.proto`:

| Rule | Count | Fix |
|---|---|---|
| `SERVICE_SUFFIX` | 6 | Rename services to `*Service` |
| `RPC_REQUEST_STANDARD_NAME` | 16 | Per-RPC `*Request` (or `ServiceMethodRequest`) types |
| `RPC_RESPONSE_STANDARD_NAME` | 18 | Per-RPC `*Response` types |
| `RPC_REQUEST_RESPONSE_UNIQUE` | 22 | No shared RPC input/output types (`Empty`, `Ack`, `AuthSession`, … must not be the direct RPC type for more than one method; request≠response type on the same RPC) |

Other proto files under `mtgfr/v1/` are already clean.

#### Service renames (gRPC path break)

| Today | Target |
|---|---|
| `Auth` | `AuthService` |
| `Decks` | `DecksService` |
| `Ratings` | `RatingsService` |
| `Cards` | `CardsService` |
| `Game` | `GameService` |
| `Tables` | `TablesService` |

Paths become `/mtgfr.v1.AuthService/…`, etc. Tonic server registration, Effect-gRPC clients
(`AuthClient` → generated `AuthServiceClient` or equivalent), BFF dials, and any path-based
trace tests (`/mtgfr.v1.Game/SubmitIntent`) must update in the same change.

#### RPC message renames (pattern)

Prefer Buf’s short form (`SignupRequest` / `SignupResponse`) when valid; use the
service-prefixed form only when needed for clarity. Every RPC gets **its own** request and
response message types (satisfies `RPC_REQUEST_RESPONSE_UNIQUE`).

Shared domain payloads (`Me`, `DeckDetail`, `CatalogCard`, `IntentEnvelope`, stream frames,
etc.) may remain as **nested field types** inside those wrappers when wrapping would
otherwise force awkward duplication — but the **RPC signature types** themselves must be
unique and STANDARD-named.

Empty RPCs: use dedicated empty messages (`LogoutRequest`, `LogoutResponse`, …), not a
shared `Empty`. Same for `Ack`-shaped outcomes (`SubmitIntentResponse`, …) — either rename
per RPC with the same field layout (`accepted`, `reject_reason`, …) or nest a shared
non-RPC `Ack` message behind a single field **only if** the implement PR accepts that
payload layout change as part of the hard cut. Prefer **same field numbers/types as today’s
direct response** on each new `*Response` type so binary layouts stay familiar even though
the release is a hard cut.

Illustrative Auth slice after rename:

```protobuf
service AuthService {
  rpc Signup(SignupRequest) returns (SignupResponse);
  rpc Login(LoginRequest) returns (LoginResponse);
  rpc Logout(LogoutRequest) returns (LogoutResponse);
  rpc GetMe(GetMeRequest) returns (GetMeResponse);
}
```

(Exact field bodies are implement-time; `buf lint` must be clean with zero exceptions.)

`GetMe` already avoids naming the RPC `Me` (method/type shadowing). Keep that constraint when
choosing response type names.

#### Call-site / codegen blast radius (same PR)

- Regenerate TS (`bun run gen` / `gen:wire`) and fix `client/app/domain/wire/**` + BFF usages.
- Rebuild server tonic stubs via normal `build.rs`; update `crates/server/src/grpc/*_svc.rs`
  and registration.
- Update tests and comments that hard-code old service paths or message type names.
- No Buf Rust codegen migration.

### Breaking (`buf breaking`)

**Category:** `WIRE` in `proto/buf.yaml` (`breaking.use: [WIRE]`). Additive optional fields,
new field numbers, new `oneof` arms, and new RPCs/services are allowed. Removals, number
reuse, and wire-incompatible type changes fail.

`FILE`-category renames remain discouraged in `WIRE_COMPAT.md` prose for ordinary PRs; the
lint-cleanup rename is explicitly a **major hard cut**, not an ordinary expand-only change.

**PR baseline:**

```bash
git fetch origin main
buf breaking --against '.git#branch=origin/main' proto
```

(Exact cwd/`--path` flags are implementation detail; module root is `proto/` per
`buf.yaml`.)

**Checkout:** `verify-wire` uses `fetch-depth: 0` (or an explicit `git fetch origin main`) so
the against-ref resolves.

**Main push:** run `buf lint` only. After merge, `HEAD` *is* `main`, so breaking against
`origin/main` is empty. Do not add a last-tag baseline in this design.

### Major-version / hard-break path

| Path | `buf lint` | `buf breaking` | Release / ops meaning |
|---|---|---|---|
| Ordinary PR (no major title) | Required (full STANDARD) | Required vs `origin/main` | Expand-only; N↔N−1 safe for drain |
| Major PR (`feat!:` / `fix!:` / title contains `BREAKING CHANGE`) | Required | **Skipped** (explicit hatch) | semantic-release major; **hard cut** — no N↔N−1 for that release |
| **First implement PR (this design)** | Required after renames | Skipped via major title | Ships STANDARD renames + CI gate; gRPC service path hard cut |
| Later preferred hard-break content | Required | May still pass if only additive under new `mtgfr.v2` package | `/v2` package bump per `WIRE_COMPAT.md` |
| Later in-place hard break under `mtgfr.v1` | Required | Fails unless major-title hatch | Allowed only with major title; document hard cut in PR body |

**Who authorizes the exception:** the **PR title** alone (same artifact commitlint and
semantic-release already treat as the squash subject). No workflow_dispatch input, no label,
no post-merge rebaseline script — once the major merges, `origin/main` *is* the new baseline
for subsequent PRs.

**CI must print** when the hatch fires, e.g.:

```text
wire: PR title indicates a semver major — skipping buf breaking.
This release is a hard cut (no N↔N−1 drain coexistence). Prefer proto package mtgfr.v2
for intentional wire breaks after the STANDARD rename lands. See docs/WIRE_COMPAT.md.
```

**Detection (implementation sketch):** on `pull_request`, if
`github.event.pull_request.title` matches `!:` (conventional bang before `:`) **or** contains
`BREAKING CHANGE`, skip the breaking step. Local `just proto-breaking` does **not**
auto-skip — default local behavior stays strict (authors expecting a major either use a
documented override env var or rely on CI hatch).

### Local `just` recipes

| Recipe | Behavior |
|---|---|
| `just proto-lint` | `buf lint` via `client`’s `@bufbuild/buf` on `proto/` |
| `just proto-breaking` | `buf breaking` against `origin/main` (document `git fetch origin main`) |
| `just proto-check` | `proto-lint` + `proto-breaking` |

Do **not** fold these into `just client-check` / `just check` in the first implementation
unless cheap and already fetching `origin/main`; CI ownership is `verify-wire`. Optional
follow-up: add `proto-lint` to `just check` (lint needs no against-ref).

### CI job (`verify-wire`)

New job in `verify-jobs.yml`, required alongside existing verify jobs.

```text
verify-wire
  checkout (fetch main / depth 0)
  setup-bun + bun install --frozen-lockfile in client/
  just proto-lint
  if pull_request and not major-title: just proto-breaking
  if pull_request and major-title: echo hard-cut notice; skip breaking
  if not pull_request (main): lint only
```

No pass-marker for v1 of this gate. Path filter optional later — not required for first ship.

### Failure UX

When `buf breaking` fails:

```text
wire: buf breaking failed against origin/main.
Expand-only changes are allowed (new fields/numbers, new oneof arms, new RPCs).
Do not rename/remove/repurpose field numbers while N↔N−1 drain coexistence is required.
If this is an intentional hard break: use PR title feat!: / fix!: (or BREAKING CHANGE)
so semantic-release cuts a major, and prefer package mtgfr.v2. See docs/WIRE_COMPAT.md.
```

When `buf lint` fails: fix the proto to satisfy `STANDARD`. **Do not** add `except` /
`ignore` / `ignore_only` to silence rules. The living CI / `WIRE_COMPAT` docs should state
that policy explicitly at implement time.

## At implement time — update these living docs/specs

Do **not** invent a new product-surface feature spec. Update existing living docs in the
same implementation change:

1. **`docs/superpowers/specs/2026-07-20-ci-and-release.md`** — Behavior: document
   `verify-wire` (full STANDARD lint always; breaking on non-major PRs vs `origin/main`;
   major-title skip). Implementation Decisions: Buf from `client`’s `@bufbuild/buf`; no
   pass-marker; no lint rule exceptions.
2. **`docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md`** — Behavior /
   Implementation: gRPC service names are `*Service`; Testing Decisions: replace
   “expand-only enforced by code review only” with automated `buf lint` / `buf breaking`;
   note major-title / `/v2` hard-cut path. Codegen lifecycle unchanged (tonic +
   `buf generate`).
3. **`docs/WIRE_COMPAT.md`** — Keep prose expand-only rules authoritative. Add:
   - Pointer to the automated gate (`just proto-*`, `verify-wire`); full STANDARD, no
     silenced rules.
   - **Hard breaks / majors** section (today §3 is only “Lobby vs game”; wire-protocol’s
     Out of Scope already refers to a missing hard-breaks § — add it here): service renames
     and other gRPC path changes are hard cuts; `/v2` package preferred for later breaks;
     in-place breaks only with semver major PR title; no N↔N−1 for that release; after
     merge, `main` is the new breaking baseline.
4. **`docs/superpowers/specs/README.md`** — Index already present; refresh blurb if needed
   when status flips from proposed → shipped design input.

Optional: root `README.md` / `AGENTS.md` Commands — one line for `just proto-check`.

## Verification plan (implementation PR)

| Scenario | Command / signal | Expected |
|---|---|---|
| Clean tree after renames | `just proto-lint` | Exit 0 with **zero** lint exceptions configured |
| Lint policy | Inspect `proto/buf.yaml` | No `except` / `ignore` / `ignore_only` |
| Local expand-only edit (post-merge baseline) | Add optional field with new number; `just proto-breaking` | Exit 0 |
| Local hard break without hatch | Delete/renumber a field; `just proto-breaking` | Non-zero; failure blurb visible |
| Non-major PR with break | PR **without** `!:` / `BREAKING CHANGE`; remove a field | `verify-wire` red on `buf breaking` |
| Major PR with break | First implement PR titled `feat!: …`; service renames present | `verify-wire` green; hard-cut skip notice; lint clean |
| Call sites | `just server-check` / `just client-check` (or `just check`) | Green against renamed services/messages |
| Main after major merge | Push/merge to `main` | `verify-wire` lint-only; later PRs compare against new `main` |
| CI signal | `verify-wire` job in Actions | Required check; distinct from Verify (client) / Verify (server) |

## Out of Scope

- Buf Schema Registry, remote Buf modules, or locked BSR images as the breaking baseline
- Replacing `tonic_build` / Rust stubs with Buf Rust codegen
- Changing Effect-gRPC / `protoc-gen-es` plugins or `buf.gen.yaml` unless required for
  lint/breaking to run or for regenerated client names
- OpenAPI / AsyncAPI
- `lint.except` / `ignore` / `ignore_only` for any STANDARD rule
- Moving the cleanup into a new `mtgfr.v2` package (in-place `mtgfr.v1` rename is the chosen
  hard cut for this pass)
- `FILE`-category breaking enforcement
- Breaking-against-last-`v*`-tag on main
- Pass-marker / paths-filter optimization for `verify-wire`
- Baking `buf` into `mtgfr-ci` (may revisit if a non-Bun job needs it)

## Error / degradation

| Condition | Behavior |
|---|---|
| `origin/main` missing locally | `proto-breaking` fails clearly; docs say `git fetch origin main` |
| Fork PR cannot fetch base | Use GHA `checkout` with adequate fetch + `git fetch origin main` from the base repo |
| Accidental `!:` in unrelated PR title | Breaking skipped — acceptable; title already implies a major release |
| Someone adds lint `except`/`ignore` | Reject in review; living docs forbid silencing STANDARD rules |

## Further Notes

- Measured on current `main` with `@bufbuild/buf@1.72.0`: only
  `SERVICE_SUFFIX`, `RPC_REQUEST_STANDARD_NAME`, `RPC_RESPONSE_STANDARD_NAME`, and
  `RPC_REQUEST_RESPONSE_UNIQUE` fail — all on `mtgfr/v1/mtgfr.proto`.
- Wire-protocol Out of Scope already reserves **Proto package versioning (`/v2`)** and cites
  `WIRE_COMPAT.md` hard breaks — the implement PR must add that section so the citation is
  real.
- This document is design input only. Per AGENTS.md Feature specs, shipping behavior belongs
  in the living surfaces listed above, updated in the implementation change.
