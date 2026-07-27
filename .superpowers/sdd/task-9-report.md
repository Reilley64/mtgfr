Status: Implemented generated `.agents/skills/card-dsl/DSL_REFERENCE.md` from the CardToml schemars/rustdoc surface.
Commits: `feat(cards): generate card-dsl DSL_REFERENCE from TOML surface`.
Tests: Red test failed before `gen_dsl_reference` existed; focused generator checks passed; `just server-check` passed; `just check` passed after `just client-migrate`.
Concerns: `just check` reformatted unrelated client files; restored that formatter churn before commit.
Report path: `/workspace/.superpowers/sdd/task-9-report.md`

## Review follow-up (remaining Task 9 findings)

Status: Removed stale DSL_REFERENCE §10 "Unsupported" pointer from `card-dsl/SKILL.md`; aligned `ci-and-release.md` cache action versions with `verify-jobs.yml` (`@v6`).
Commits: `docs: fix card-dsl skill and ci spec review follow-ups`.
Tests: not run (doc-only).
Report path: `/workspace/.superpowers/sdd/task-9-report.md`

## Review follow-up (Important Task 9 findings)

Status: Wired `just cards-dsl-ref-check` into `verify-server-lint`; added `.agents/skills/card-dsl/DSL_REFERENCE.md` to `verify-server-v3-*` hash paths; removed stale hand-maintained / Wave B prose from `2026-07-20-card-dsl-and-card-pool.md`; updated `2026-07-20-ci-and-release.md` lint bullet for dsl-ref-check.
Commits: `ci: wire cards-dsl-ref-check into verify-server-lint`.
Tests: `just cards-dsl-ref-check`, `just cards-schema-check`.
Report path: `/workspace/.superpowers/sdd/task-9-report.md`

## Review follow-up (remaining Task 9 findings)

Status: Removed stale DSL_REFERENCE §10 "Unsupported" pointer from `card-dsl/SKILL.md`; aligned `ci-and-release.md` cache action versions with `verify-jobs.yml` (`@v6`).
Commits: `docs: fix card-dsl skill and ci spec review follow-ups`.
Tests: not run (doc-only).
Report path: `/workspace/.superpowers/sdd/task-9-report.md`
