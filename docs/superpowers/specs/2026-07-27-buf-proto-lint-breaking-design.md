# Buf proto lint + breaking (design)

**Status:** Proposed design input (2026-07-27). Design/spec only — do not treat as shipped
until an implementation PR updates the living surfaces listed below.
**Surfaces:** `ci-and-release` (`.github/workflows/verify-jobs.yml`, `justfile`),
`wire-protocol-and-visibility` (codegen / compat testing), `docs/WIRE_COMPAT.md` (ops rules +
major/`/v2` exception path).

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

## Goal

Machine-check proto quality and expand-only compatibility on every ordinary PR, with an
**explicit major-version / hard-break escape hatch** aligned to semantic-release majors and
`WIRE_COMPAT.md` `/v2` hard cuts — without replacing tonic Rust codegen, Buf Schema Registry,
or remote plugins.

## Locked decisions

| Decision | Choice |
|---|---|
| Where to run | Local `just` recipes **and** a dedicated `verify-wire` job in `verify-jobs.yml` (PR + main) |
| `buf lint` category | Keep `STANDARD`; except the four RPC/service naming rules that already fail on `mtgfr.proto` |
| `buf breaking` category | `WIRE` (drain-window parse compatibility), not `FILE` |
| Breaking baseline (PRs) | `origin/main` via `buf breaking --against '.git#branch=origin/main'` |
| Breaking on `main` push | Lint only; breaking is vacuous against self — do **not** require a tag baseline on main |
| Major / hard-break hatch | Skip `buf breaking` when the **PR title** is an Angular major (`!:` bang **or** contains `BREAKING CHANGE`) |
| Preferred hard-break shape | New package path `mtgfr/v2` (and `proto/mtgfr/v2/`) when practical; in-place breaks only via the major-title hatch |
| Tooling source | Existing `@bufbuild/buf` from `client/` (`bun install`); no new GHCR image bake for this PR |
| Rust codegen | Unchanged (`tonic_build` in `crates/server/build.rs`) |

## Approaches considered

1. **Dedicated `verify-wire` + `just proto-*` + PR breaking vs `origin/main` + major-title skip (chosen)** —
   Smallest enforceable gate. Lint always; breaking only where it matters (PRs). Major hatch
   matches squash-merge / semantic-release (PR title is the release subject). Failure UX lives
   in one job, not buried in full client-check.
2. Fold lint/breaking into `verify-client` only — Reuses Bun install and `proto/**` pass-marker
   invalidation, but couples wire gate latency/UX to format/lint/vitest and makes major-skip
   messaging harder to notice.
3. Breaking against last `v*` tag (PR and/or main) — Closer to “what operators may still pin,”
   but lags multi-commit expand-only sequences on `main`, needs tag-fetch edge cases, and
   after a major hard cut must rebaseline off the new tag anyway. Extra moving parts without
   better drain safety once every merge already passed vs `main`.

## Design

### Lint (`buf lint`)

Keep `proto/buf.yaml` on `STANDARD`. Today `buf lint` fails (~60 findings) solely on
intentional service/RPC naming in `proto/mtgfr/v1/mtgfr.proto` (`Auth`, `Game`, shared
`Empty`/`Ack`, etc.). Renaming those to satisfy Buf would churn generated API names without
improving wire safety.

**Lock:** except these rules globally (they are style, not wire safety):

- `SERVICE_SUFFIX`
- `RPC_REQUEST_STANDARD_NAME`
- `RPC_RESPONSE_STANDARD_NAME`
- `RPC_REQUEST_RESPONSE_UNIQUE`

Use buf v2 `lint.except` in `proto/buf.yaml`. Do not mass-rename existing services/RPCs in
this work. After exceptions, `buf lint` must be clean on current `main`.

### Breaking (`buf breaking`)

**Category:** `WIRE` in `proto/buf.yaml` (`breaking.use: [WIRE]`). This matches drain-window
reality: old and new binaries must parse the same bytes. Additive optional fields, new field
numbers, new `oneof` arms, and new RPCs/services are allowed. Removals, number reuse, and
wire-incompatible type changes fail.

`FILE`-category renames (same number, new name) remain discouraged in `WIRE_COMPAT.md` prose
for human clarity; they are not this gate’s job.

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
| Ordinary PR (no major title) | Required | Required vs `origin/main` | Expand-only; N↔N−1 safe for drain |
| Major PR (`feat!:` / `fix!:` / title contains `BREAKING CHANGE`) | Required | **Skipped** (explicit hatch) | semantic-release major; **hard cut** — no N↔N−1 for that release |
| Preferred hard-break content | Required | May still pass if only additive under new `mtgfr.v2` package | `/v2` package bump per `WIRE_COMPAT.md`; old `v1` may remain or be removed in the same major |
| In-place hard break under `mtgfr.v1` | Required | Fails unless major-title hatch | Allowed only with major title; document hard cut in PR body |

**Who authorizes the exception:** the **PR title** alone (same artifact commitlint and
semantic-release already treat as the squash subject). No workflow_dispatch input, no label,
no post-merge rebaseline script — once the major merges, `origin/main` *is* the new baseline
for subsequent PRs.

**CI must print** when the hatch fires, e.g.:

```text
wire: PR title indicates a semver major — skipping buf breaking.
This release is a hard cut (no N↔N−1 drain coexistence). Prefer proto package mtgfr.v2
for intentional wire breaks. See docs/WIRE_COMPAT.md.
```

**Detection (implementation sketch):** on `pull_request`, if
`github.event.pull_request.title` matches `!:` (conventional bang before `:`) **or** contains
`BREAKING CHANGE` (case-sensitive is fine; document the exact match in the living CI spec),
skip the breaking step. Local `just proto-breaking` does **not** auto-skip — authors running
locally who intend a major pass `--force` / a documented env var only if implementers want
it; default local behavior stays strict.

### Local `just` recipes

Add focused recipes (names exact at implement time; suggested):

| Recipe | Behavior |
|---|---|
| `just proto-lint` | `buf lint` via `client`’s `@bufbuild/buf` on `proto/` |
| `just proto-breaking` | `buf breaking` against `origin/main` (document `git fetch origin main`) |
| `just proto-check` | `proto-lint` + `proto-breaking` |

Do **not** fold these into `just client-check` / `just check` in the first implementation
unless cheap and already fetching `origin/main`; CI ownership is `verify-wire`. Optional
follow-up: add `proto-lint` to `just check` (lint needs no against-ref).

### CI job (`verify-wire`)

New job in `verify-jobs.yml`, required by the same callers as today (`ci.yml` /
`verify-and-release.yml` already include the reusable workflow’s jobs via needs or as
parallel checks — wire the aggregator / branch-protection expectation the same way other
verify jobs are required).

```text
verify-wire
  checkout (fetch main / depth 0)
  setup-bun + bun install --frozen-lockfile in client/
  just proto-lint
  if pull_request and not major-title: just proto-breaking
  if pull_request and major-title: echo hard-cut notice; skip breaking
  if not pull_request (main): lint only
```

No pass-marker for v1 of this gate (job should be ~tens of seconds). If wall-clock later
warrants caching, that is a follow-up.

**Path filter:** always run with verify (proto is tiny). Optional later: paths-filter on
`proto/**` + workflow/just/buf.yaml — not required for the first ship.

### Failure UX

When `buf breaking` fails, the job should prefix stderr with a short, fixed blurb (script or
step `echo`), not only raw Buf output:

```text
wire: buf breaking failed against origin/main.
Expand-only changes are allowed (new fields/numbers, new oneof arms, new RPCs).
Do not rename/remove/repurpose field numbers while N↔N−1 drain coexistence is required.
If this is an intentional hard break: use PR title feat!: / fix!: (or BREAKING CHANGE)
so semantic-release cuts a major, and prefer package mtgfr.v2. See docs/WIRE_COMPAT.md.
```

Lint failures should mention `proto/buf.yaml` exceptions are intentional for existing RPC
naming — fix real lint issues; do not widen `except` without updating this design’s living
docs.

## At implement time — update these living docs/specs

Do **not** invent a new product-surface feature spec. Update existing living docs in the
same implementation change:

1. **`docs/superpowers/specs/2026-07-20-ci-and-release.md`** — Behavior: document
   `verify-wire` (lint always; breaking on non-major PRs vs `origin/main`; major-title skip).
   Implementation Decisions: Buf from `client`’s `@bufbuild/buf`; no pass-marker.
   Testing: local `just proto-check` / CI signal.
2. **`docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md`** — Testing
   Decisions: replace “expand-only enforced by code review only” with the automated
   `buf lint` / `buf breaking` gate; note major-title / `/v2` hard-cut path. Codegen
   lifecycle unchanged (tonic + `buf generate`).
3. **`docs/WIRE_COMPAT.md`** — Keep prose expand-only rules authoritative. Add:
   - Pointer to the automated gate (`just proto-*`, `verify-wire`).
   - **Hard breaks / majors** section (today §3 is only “Lobby vs game”; wire-protocol’s
     Out of Scope already refers to a missing hard-breaks § — add it here): `/v2` package
     preferred; in-place breaks only with semver major PR title; hard cut means no N↔N−1
     for that release; after merge, `main` is the new breaking baseline.
4. **`docs/superpowers/specs/README.md`** — Index this design under Process/policy (done in
   the design PR; implement PR does not need a second index churn unless the title/status
   changes).

Optional (only if recipes are added to developer docs that already list `just` commands):
root `README.md` / `AGENTS.md` Commands — one line for `just proto-check`. Prefer living
specs over drive-by AGENTS expansion.

## Verification plan (implementation PR)

| Scenario | Command / signal | Expected |
|---|---|---|
| Clean tree matches policy | `just proto-lint` | Exit 0 after `buf.yaml` exceptions |
| Local expand-only edit | Add optional field with new number; `just proto-breaking` (after `git fetch origin main`) | Exit 0 |
| Local hard break without hatch | Delete/renumber a field; `just proto-breaking` | Non-zero; failure blurb visible |
| Non-major PR with break | Open PR **without** `!:` / `BREAKING CHANGE`; remove a field | `verify-wire` red on `buf breaking` |
| Major PR with break | Same break; PR title `feat!: wire hard cut` (or `BREAKING CHANGE` in title) | `verify-wire` green; log shows hard-cut skip notice; lint still runs |
| Main after major merge | Push/merge to `main` | `verify-wire` runs lint; no false breaking fail; later PRs compare against new `main` |
| CI signal | `verify-wire` job in Actions | Required check; distinct from Verify (client) / Verify (server) |

Manual smoke: temporarily break a field on a branch, push, confirm log text; revert before
merge. Do not leave a broken proto on the design-only PR.

## Out of Scope

- Buf Schema Registry, remote Buf modules, or locked BSR images as the breaking baseline
- Replacing `tonic_build` / Rust stubs with Buf Rust codegen
- Changing Effect-gRPC / `protoc-gen-es` plugins or `buf.gen.yaml` unless required for
  lint/breaking to run
- OpenAPI / AsyncAPI
- Renaming existing `mtgfr.v1` services/RPCs to satisfy `SERVICE_SUFFIX` / RPC_*_NAME
- `FILE`-category breaking enforcement
- Breaking-against-last-`v*`-tag on main
- Pass-marker / paths-filter optimization for `verify-wire`
- Baking `buf` into `mtgfr-ci` (may revisit if a non-Bun job needs it)
- Auto-creating `proto/mtgfr/v2/` in this work — only the policy path is designed

## Error / degradation

| Condition | Behavior |
|---|---|
| `origin/main` missing locally | `proto-breaking` fails clearly; docs say `git fetch origin main` |
| Fork PR cannot fetch base | Use GHA `checkout` with adequate fetch + `git fetch origin main` from the base repo |
| Accidental `!:` in unrelated PR title | Breaking skipped — acceptable; title already implies a major release |
| Someone widens `lint.except` to silence real issues | Living `WIRE_COMPAT` / CI spec review; do not except wire-safety rules |

## Further Notes

- Measured on current `main` with `@bufbuild/buf@1.72.0`: `STANDARD` lint is clean aside from
  the four excepted rule IDs on `mtgfr/v1/mtgfr.proto` (Buf’s
  `--error-format=config-ignore-yaml` lists exactly those).
- Wire-protocol Out of Scope already reserves **Proto package versioning (`/v2`)** and cites
  `WIRE_COMPAT.md` hard breaks — the implement PR must add that section so the citation is
  real.
- This document is design input only. Per AGENTS.md Feature specs, shipping behavior belongs
  in the living surfaces listed above, updated in the implementation change.
