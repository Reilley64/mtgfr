# GHA server verify sharding (design)

**Status:** Approved design input (2026-07-26).
**Surfaces:** `ci-and-release` (`.github/workflows/verify-jobs.yml`, `.config/nextest.toml`, `justfile` server recipes).

Related living spec: [ci-and-release](2026-07-20-ci-and-release.md) — update Behavior / Implementation in the same change when this ships.

---

## Problem Statement

Cold PR / main `verify-server` wall-clock is dominated by nextest (~5–6 minutes for ~2.4k tests on a single `ubuntu-latest` runner). Pass-marker caching already skips unchanged server trees; engine/card PRs still wait on the full sequential suite. Client verify (~1 minute cold) is not the bottleneck.

## Goal

Cut cold **server verify wall-clock** with a **modest** parallelization budget: **three** nextest shards. Best effort — no hard time target. Prefer GHA minutes spent on a small shard matrix over introducing a new test runner (Maelstrom) or large matrices.

## Locked decisions

| Decision | Choice |
|---|---|
| Optimize for | Cold `verify-server` wall-clock |
| Parallelism | 3 nextest shards (`count:1/3`, `count:2/3`, `count:3/3`) |
| Runner | Stay on `ubuntu-latest` (no larger / self-hosted runners in this pass) |
| Local commands | `just server-test` / `just server-check` remain unsharded |
| Pass marker | Same content-hash idea; **restore-only gate + save-only mark after lint + both shards succeed** |
| Cargo cache | Lint + both test shards share one `Swatinem/rust-cache` `shared-key` |
| Required-check shape | Keep a single aggregator job named like today’s server gate |
| Client / docker / release | Unchanged |

## Approaches considered

1. **Three-shard nextest matrix + separate lint + mark aggregator (chosen)** — Targets the nextest wall; modest minutes bump; stays on nextest/JUnit. Started at two shards, then bumped to three after confirming public-repo Actions minutes are free and 2→3 still beats setup overhead.
2. Lint job parallel to one nextest job — Simpler; only recovers ~clippy overlap (~30–60s); leaves the 5–6 min suite intact.
3. Larger single runner — Minimal workflow shape change; billed-minute / plan dependent; weaker fit for a small shard matrix.

## Design

### Job graph (`verify-jobs.yml`)

```text
verify-server-gate          (cache/restore only → outputs.cache-hit; never save)
        |
        +-- miss --> verify-server-lint     (CR index + fmt + clippy; rust-cache shared-key)
        |              verify-server-test   (matrix 1/3, 2/3, 3/3; Postgres + migrate + nextest shard;
        |                                   rust-cache shared-key)
        |                    |
        +--------------------+--> verify-server-mark  (cache/save only after lint + both shards OK)
        |
        +-- hit  --> lint / test / mark jobs skipped via if:

verify-server               (aggregator; green on hit OR full miss success)
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
    partition: [1, 2, 3]
```

On cache miss, each shard:

1. Postgres 16 service (same as today’s `verify-server`)
2. Toolchain + rust-cache + nextest + protoc + just (same setup as today)
3. `cargo run -p server -- migration apply`
4. `cargo nextest run --profile ci --partition count:${{ matrix.partition }}/3`

JUnit: keep the `ci` profile path; upload per-shard artifacts (e.g. `rust-junit-1` … `rust-junit-3`) and run test-summary on each when present.

### Caching (must keep working)

Two independent caches. Both must stay correct under the split; neither may false-pass.

#### 1. Pass marker (skip unchanged server trees)

Today one job restores `.ci-pass`, runs checks, then the cache **post-step** saves because the marker file was written in that same job. After the split, **restore and save must be separate jobs** so a green shard cannot publish a marker while lint or the other shard still fails.

| Job | Cache API | Behavior |
|---|---|---|
| `verify-server-gate` | `actions/cache/restore` only | Clean checkout → same `hashFiles(...)` key → emit `outputs.cache-hit`. **Never** write `.ci-pass`. **Never** save. |
| `verify-server-lint` / `verify-server-test` | none for pass marker | `if: needs.verify-server-gate.outputs.cache-hit != 'true'`. On hit these jobs are **skipped** (not “run empty”). |
| `verify-server-mark` | `actions/cache/save` only | Runs only when gate was a **miss** and lint + **all** matrix shards succeeded. Checkout (so `hashFiles` matches) → `mkdir .ci-pass && echo ok > .ci-pass/marker` → save with the **identical** key expression as restore. |

**Key**

- Same input set as today’s `verify-server-v2-*` (`crates/**`, `proto/**`, Cargo/Toasty, `.config/nextest.toml`, `justfile`, `.github/workflows/verify-jobs.yml`, `docs/CR_INDEX.md`, `scripts/gen_cr_index.py`).
- Bump prefix to `verify-server-v3-*` when this workflow ships so v2 markers cannot skip the new graph.
- Compute the key only on a clean checkout (same rule as today: do not re-`hashFiles` after mutating the tree).

**Invariants**

- Marker present ⇒ lint + full suite (all partitions) previously succeeded for that content hash.
- Partial success ⇒ no save.
- Cancelled / failed needed job ⇒ no save; aggregator red.
- Gate miss + empty `.ci-pass` must not produce a saved empty entry (restore-only gate + save-only mark).

**Cache hit path:** gate reports hit → lint/test/mark skipped → aggregator green. Postgres may still start only if a non-skipped job keeps a `services:` block; with `if:` skip on lint/test, hit path should not start Postgres. Document the actual Actions behavior in the living spec after implement.

#### 2. Cargo / rust-cache (compile reuse across shards)

Lint and both test shards each need a toolchain on miss. Job names change from today’s single `verify-server`, so **default** `Swatinem/rust-cache` keys (which incorporate job identity) would cold-miss after the refactor.

**Lock:** every miss-path Rust job (lint + all partitions) uses the same:

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: verify-server
```

So clippy and nextest shards share one Cargo cache namespace. Concurrent saves are acceptable (last writer wins); correctness does not depend on which shard saves last. Do **not** give each matrix cell a distinct cache key for this pass.

Public-repo standard hosted runners do not bill Actions minutes, so shard count is chosen for wall-clock vs setup overhead, not spend.

Client pass-marker / bun install caching stays as today.

### Aggregator

`verify-server` `needs: [verify-server-gate, verify-server-lint, verify-server-test]` (matrix collapses to one need). Use `if: always()` and succeed when:

- `gate.outputs.cache-hit == 'true'`, or
- gate miss **and** lint result `success` **and** test matrix result `success` (all three partitions)

Any other combination (failure, cancelled, unexpected skip on miss) → fail. This preserves a single server verify status for humans and required checks.

### Local / justfile

- `just server-test *args` continues to run full `cargo nextest run --profile ci` with args passthrough (so `--partition count:1/3` works ad hoc without a new recipe).
- `just server-check` stays the single-runner local/CI-equivalent path: CR index + fmt + clippy + migrate + full nextest.
- CI does **not** call `just server-check` as one blob anymore; it composes the same steps across jobs.

### Docs

In the same implementation change, update [ci-and-release](2026-07-20-ci-and-release.md) Behavior / Implementation so it describes the split jobs, three-shard nextest, marker write rules, and aggregator — no migration/history prose.

## Error / degradation

| Condition | Behavior |
|---|---|
| Lint fails, tests pass | Aggregator fails; pass marker **not** written |
| One shard fails | Other shards still finish (`fail-fast: false`); aggregator fails; marker not written |
| Cache hit | Lint/test work skipped; aggregator green |
| Cancelled matrix member | Aggregator fails (no false pass marker) |

## Verification (when implementing)

Task 3 records PR-run evidence in `.superpowers/sdd/2026-07-26-gha-server-verify-sharding/task-3-report.md`.

1. PR that touches `crates/**` → pass-marker **miss** → lint + three shards run → mark job saves → aggregator green → JUnit artifacts for partitions 1–3 present.
2. Immediate follow-up commit that does **not** change server hash inputs → gate **hit** → lint/test/mark skipped → aggregator green (proves pass-marker still short-circuits).
3. Forced failure: break one shard (or lint) on a miss run → aggregator red **and** mark job does not run / does not save (proves no false pass marker). Revert before merge.
4. On a miss run, confirm rust-cache restore reports the shared `verify-server` key on lint and all shards (not disjoint cold keys).
5. Locally: `just server-check` still runs the full unsharded suite.

## Out of Scope

- Maelstrom or other alternate runners
- More than three shards
- Larger / self-hosted runners
- Client Vitest sharding
- Per-test slow-suite tuning
- Docker / semantic-release / terraform job changes
