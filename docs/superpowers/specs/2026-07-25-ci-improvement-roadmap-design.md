# CI improvement roadmap

**Status:** Wave 3 implemented — parallel Docker jobs (absorbed into [production-topology-and-operations](2026-07-20-production-topology-and-operations.md); further redesign remains evidence-gated)
**Date:** 2026-07-25  
**Module:** `.github/workflows/` (`ci.yml`, `verify-jobs.yml`, `verify-and-release.yml`, `docker.yml`), `justfile` check recipes, `scripts/check-docker-workflow-cache.sh`  
**Approach:** Waste-first multi-wave roadmap; keep current verify → semantic-release → `v*` → Docker cascade until later waves have evidence to change it

---

## Problem

PR CI is already parallelized (`verify-server` / `verify-client`) with content-hash pass-marker
skips, and release Docker builds recently gained Buildx GHA layer cache. Remaining gaps:

1. **Waste under rapid pushes:** `ci.yml` has no PR concurrency cancel. Cloud agents and
   iterative PRs stack overlapping runs; superseded work still burns minutes.
2. **Always-on terraform:** The terraform job runs on every PR even when `iac/` is untouched.
3. **Cheap local checks missing from CI:** `just engine-cr-index-check` and
   `just client-mana-oracle-check` exist locally but are not part of the CI verify path.
   `scripts/check-docker-workflow-cache.sh` is documented as a guard but is not enforced in
   PR CI.
4. **Structural inefficiencies (later):** Server jobs still provision Postgres on pass-marker
   hit; client skip hashes include broad `crates/**` invalidation; Docker image builds are
   sequential; Actions emit Node 20 deprecation warnings.

This design records an open audit and a phased plan. Implementation starts at Wave 1 only
after an implementation plan is written and approved.

## Goals

- Cut wasted PR CI minutes (especially rapid successive pushes on the same PR).
- Close cheap correctness gaps that local recipes already express.
- Keep the current release cascade as the default until Waves 1–2 land and run data
  justifies redesign.
- Document Waves 1–3 so later work has an explicit decision rule, not speculative rewrites.

## Non-goals (this design / Wave 1)

- Redesigning semantic-release, `RELEASE_TOKEN`, or the `v*` → `docker.yml` cascade.
- Changing the pass-marker skip model (beyond ensuring new check inputs are hashed).
- Booting Postgres conditionally, parallel Docker jobs, larger runners, or remote caches
  (Wave 2 / Wave 3).
- Adding Playwright / live-game verify to GitHub Actions.
- Estimating calendar time for waves.

## Decisions (from brainstorming)

1. **Open audit** — no single pain; recommend highest-leverage improvements from current setup.
2. **Roadmap first** — multi-wave plan; implement Wave 1 only after writing-plans + approval.
3. **Everything negotiable later** — release/Docker shape may change in Wave 3 if evidence
   warrants it; Waves 1–2 stay inside the current architecture.
4. **Waste-first (Approach 1)** — concurrency and cheap gates before correctness-max or
   pipeline redesign.

## Architecture (current; Waves 1–2 preserve)

| Workflow | Trigger | Role |
|----------|---------|------|
| `ci.yml` | PR → `main`/`master` | commitlint + `verify-jobs.yml` + terraform |
| `verify-jobs.yml` | `workflow_call` | Parallel server/client verify + pass-marker skips |
| `verify-and-release.yml` | Push → `main`/`master` | Verify then `npx semantic-release` (non-cancelling concurrency) |
| `docker.yml` | Push `v*` tags | Build/push GHCR images with Buildx `type=gha` cache |

Absorbed “what exists today” documentation lives in
[production-topology-and-operations](2026-07-20-production-topology-and-operations.md)
(Release and CI pipeline). This file is the brainstorming / roadmap record and is not a
separate indexed product surface.

## Wave 1 — Waste + cheap gates (implement first)

### 1. PR concurrency cancel

On `ci.yml`:

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

`verify-and-release.yml` keeps `cancel-in-progress: false` so release writes never overlap
or get cancelled mid-flight.

### 2. Path-skip terraform

Add a tiny `paths` (or `dorny/paths-filter`) job in `ci.yml` that detects changes under
`iac/**` and `.github/workflows/ci.yml`. The terraform job runs only when that filter is
true (`if:`). Untouched PRs skip setup-terraform entirely. Fail closed: any change to those
paths runs validate.

### 3. Wire missing cheap checks

| Check | Where it runs |
|-------|----------------|
| `just engine-cr-index-check` | Always as part of `server-check` (same as fmt/clippy/migrate/nextest). Extend server pass-marker `hashFiles` to include `docs/CR_INDEX.md` and `scripts/gen_cr_index.py` (and any other inputs the check reads). |
| `just client-mana-oracle-check` | Fold into `client-check` before format/lint so local and CI cannot diverge. Pass-marker already hashes `client/**`. |
| `scripts/check-docker-workflow-cache.sh` | Always as a cheap job/step on `ci.yml` (seconds; no path filter required). |

### 4. Docs when Wave 1 ships

Update production-topology Release and CI section to describe concurrency, terraform skip,
and the new gates. Keep this roadmap file as the multi-wave record; mark Wave 1 status
implemented when absorbed.

### Wave 1 out of scope

Postgres-on-skip, hash redesign, parallel Docker, release cascade changes, Action major
version upgrades beyond what’s needed for the above.

## Wave 2 — Job structure / DX (roadmap only)

- Avoid provisioning the Postgres service when the server pass-marker hits (split
  “cache lookup” from “full verify + services”).
- Document, then only tighten, client skip `hashFiles` (today `crates/**` forces client
  re-verify — keep or narrow with explicit rationale).
- Vitest / client test summary parity with Rust nextest JUnit reporting.
- Clear Node 20 deprecation warnings from Actions pins when safe.
- Optional: clearer step names / fail-fast for agent diagnosis.

## Wave 3 — Optional redesign (evidence-gated)

Open only after Waves 1–2 land and roughly a week of run data exists (PR cancel rate,
pass-marker hit rate, p50/p90 PR CI duration). Candidates:

- Parallel `mtgfr-server` / `mtgfr-web` Docker jobs.
- Revisit pass-marker model, path filters for verify, or release/Docker cascade.
- Larger runners / alternate cache backends if wall-clock remains the bottleneck.

No speculative rewrite without that evidence.

## Testing decisions

Wave 1 verification:

1. **Static guards** — small shell script(s) (same spirit as
   `check-docker-workflow-cache.sh`) asserting: PR concurrency present; terraform skip
   wiring present; mana-oracle + CR index checks appear on the recipes/jobs CI runs;
   docker-cache script still green.
2. **Local** — run new/updated `just` checks and the docker-cache script.
3. **On the Wave 1 PR** — push twice quickly and confirm the first run cancels; open or
   simulate a non-`iac` change and confirm terraform skips; prove CR index / mana-oracle
   gates fail closed (intentional stale commit or scripted check).

**Failure modes:**

- Cancelled superseded runs must not block merge of the latest push (branch protection
  should require the latest run, not historical cancelled ones).
- Terraform skip must still run when `iac/**` or the defining CI workflow changes.
- New check inputs must be included in pass-marker `hashFiles` so a skip cannot hide a
  stale index or mana CSS after those files change.

## Success criteria (Wave 1 done)

1. Superseded PR CI runs cancel.
2. Non-`iac` PRs do not run terraform validate.
3. Stale `docs/CR_INDEX.md` fails CI.
4. Stale mana-oracle CSS fails CI.
5. Docker Buildx GHA cache wiring remains guarded in CI.
6. Production-topology CI section matches shipped behavior.

## Implementation sequence

1. This design approved.
2. `writing-plans` produces `docs/superpowers/plans/2026-07-25-ci-improvement-wave-1.md`.
3. Implement Wave 1 only; update production-topology; leave Waves 2–3 as roadmap until
   separately planned.

## Further notes

Audit snapshot that motivated the waves (not normative requirements):

- Full client verify ~1.5–2 minutes when hashes miss; both sides skipped ~30s floor
  (commitlint + terraform + checkout/cache still run).
- Recent PR failures observed were real client lint errors, not flakes.
- Docker GHA cache design already absorbed into production-topology (2026-07-24).
