Status: Implemented `gen_card_schema`, committed card/token schemas, `--check` drift mode, and Just recipes.
Commits: `feat(cards): generate card/token JSON Schema from CardToml`.
Tests/commands: `cargo test -p cards --test gen_card_schema`; `just cards-schema && just cards-schema-check`; `just cards-schema-check && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo nextest run --profile ci -p cards`.
Concerns: None.
