# GHA server verify sharding (design)

**Status:** Approved design input (2026-07-26).
**Surfaces:** `ci-and-release` (`.github/workflows/verify-jobs.yml`, `.config/nextest.toml`, `justfile` server recipes).

Related living spec: [ci-and-release](2026-07-20-ci-and-release.md) — update Behavior / Implementation in the same change when this ships.

---

## Problem Statement

Cold PR / main `verify-server` wall-clock is dominated by nextest (~5–6 minutes for ~2.4k tests on a single `ubuntu-latest` runner). Pass-marker caching already skips unchanged server trees; engine/card PRs still wait on the full sequential suite. Client verify (~1 minute cold) is not the bottleneck.

## Goal

Cut cold **server verify wall-clock** with a **modest** parallelization budget: **exactly two** nextest shards. Best effort — no hard time target. Prefer GHA minutes spent on two shards over introducing a new test runner (Maelstrom) or large matrices.

## Locked decisions

| Decision | Choice |
|---|---|
| Optimize for | Cold `verify-server` wall-clock |
| Parallelism | 2 nextest shards (`count:1/2` and `count:2/2`) |
| Runner | Stay on `ubuntu-latest` (no larger / self-hosted runners in this pass) |
| Local commands | `just server-test` / `just server-check` remain unsharded |
| Pass marker | Same content-hash idea; **write only after lint + both shards succeed** |
| Required-check shape | Keep a single aggregator job named like today’s server gate |
| Client / docker / release | Unchanged |

## Approaches considered

1. **Two-shard nextest matrix + separate lint + mark aggregator (chosen)** — Targets the nextest wall; modest minutes bump; stays on nextest/JUnit.
2. Lint job parallel to one nextest job — Simpler; only recovers ~clipy overlap (~30–60s); leaves the 5–6 min suite intact.
3. Larger single runner — Minimal workflow shape change; billed-minute / plan dependent; weaker fit for the two-shard preference.

## Design

### Job graph (`verify-jobs.yml`)

```text
verify-server-gate          (restore pass marker; emit cache-hit output)
        |
        +-- miss --> verify-server-lint     (CR index + fmt + clippy; no Postgres)
        |              verify-server-test   (matrix partition 1, 2; Postgres + migrate + nextest shard)
        |                    |
        +--------------------+--> verify-server-mark  (write pass marker only on full success + miss)
        |
        +-- hit  --> lint/test/mark skipped or no-op

verify-server               (aggregator: needs lint + both test shards; single status check)
```

Naming in the Actions UI should stay readable, e.g. `Verify (server lint)`, `Verify (server test) (1, 2)`, `Verify (server)` for the aggregator. Exact `name:` strings are implementation detail as long as one stable aggregator remains for branch protection / mental model.

### Lint job

On cache miss:

- `engine-cr-index-check`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`

No Postgres service, no migrate, no nextest. Runs in parallel with the test matrix.

### Test matrix

```yaml
strategy:
  fail-fast: false
  matrix:
    partition: [1, 2]
```

On cache miss, each shard:

1. Postgres 16 service (same as today’s `verify-server`)
2. Toolchain + rust-cache + nextest + protoc + just (same setup as today)
3. `cargo run -p server -- migration apply`
4. `cargo nextest run --profile ci --partition count:${{ matrix.partition }}/2`

JUnit: keep the `ci` profile path; upload per-shard artifacts (e.g. `rust-junit-1`, `rust-junit-2`) and run test-summary on each when present.

### Pass marker semantics

- **Key inputs:** same file set as today’s `verify-server-v2-*` (crates, proto, Cargo/Toasty, nextest config, justfile, workflow, CR index scripts). Bump the key prefix to `verify-server-v3-*` when the workflow shape changes so old markers cannot skip a differently structured verify.
- **Read:** gate (or each consumer job) restores the marker and skips work on hit.
- **Write:** only `verify-server-mark`, and only when the run was a **miss** and **lint + both shards succeeded**. Neither test shard writes the marker alone (avoids caching a pass while the sibling or lint still fails).
- On hit: keep today’s quirk — Postgres service containers may still start with matrix jobs that skip work. Do not block this design on eliminating that overhead.

### Aggregator

`verify-server` (or equivalently named) `needs` lint + the test matrix. It succeeds only if every needed job succeeded (or was correctly skipped on cache hit). This preserves a single server verify status for humans and required checks even though work is split.

### Local / justfile

- `just server-test *args` continues to run full `cargo nextest run --profile ci` with args passthrough (so `--partition count:1/2` works ad hoc without a new recipe).
- `just server-check` stays the single-runner local/CI-equivalent path: CR index + fmt + clippy + migrate + full nextest.
- CI does **not** call `just server-check` as one blob anymore; it composes the same steps across jobs.

### Docs

In the same implementation change, update [ci-and-release](2026-07-20-ci-and-release.md) Behavior / Implementation so it describes the split jobs, two-shard nextest, marker write rules, and aggregator — no migration/history prose.

## Error / degradation

| Condition | Behavior |
|---|---|
| Lint fails, tests pass | Aggregator fails; pass marker **not** written |
| One shard fails | Other shard still finishes (`fail-fast: false`); aggregator fails; marker not written |
| Cache hit | Lint/test work skipped; aggregator green |
| Cancelled matrix member | Aggregator fails (no false pass marker) |

## Verification (when implementing)

1. PR that touches `crates/**` → cache miss → lint + two shards run in parallel → aggregator green → both JUnit artifacts present.
2. Follow-up commit that does not change server hash inputs → pass marker hit → shards/lint skip → aggregator green.
3. Locally: `just server-check` still runs the full unsharded suite.

## Out of Scope

- Maelstrom or other alternate runners
- More than two shards
- Larger / self-hosted runners
- Client Vitest sharding
- Per-test slow-suite tuning
- Docker / semantic-release / terraform job changes
