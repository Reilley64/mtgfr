Status: Implemented generated `.agents/skills/card-dsl/DSL_REFERENCE.md` from the CardToml schemars/rustdoc surface.
Commits: `feat(cards): generate card-dsl DSL_REFERENCE from TOML surface`.
Tests: Red test failed before `gen_dsl_reference` existed; focused generator checks passed; `just server-check` passed; `just check` passed after `just client-migrate`.
Concerns: `just check` reformatted unrelated client files; restored that formatter churn before commit.
Report path: `/workspace/.superpowers/sdd/task-9-report.md`
