# List available recipes
default:
    @just --list

# ── Server ───────────────────────────────────────────────────────────────────────────

[group('server')]
[doc("Format Rust code")]
server-format:
    cargo fmt

[group('server')]
[doc("Lint Rust code")]
server-lint:
    cargo clippy --all-targets

[group('server')]
[doc("Build the workspace")]
server-build:
    cargo build

[group('server')]
[doc("Run Rust tests via nextest (JUnit under GitHub Actions)")]
server-test *args:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo nextest run --profile ci "$@"

[group('server')]
[doc("Build the server for production")]
server-build-prod:
    cargo build -p server --release

[group('server')]
[doc("Run the server")]
server-run: server-build-prod
    cargo run -p server --release -- serve

[group('server')]
[doc("Regenerate openapi.json and the client wire client")]
server-codegen:
    cd client && bun run gen

# ── Client ───────────────────────────────────────────────────────────────────────────

[group('client')]
[doc("Format client code")]
client-format:
    cd client && bun run format

[group('client')]
[doc("Lint client code")]
client-lint:
    cd client && bun run lint

[group('client')]
[doc("Typecheck client code")]
client-typecheck:
    cd client && bun run typecheck

[group('client')]
[doc("Regenerate src/mana-oracle.css from mana-font")]
client-mana-oracle:
    cd client && node scripts/gen-mana-oracle.mjs

[group('client')]
[doc("Fail if src/mana-oracle.css is stale vs mana-font")]
client-mana-oracle-check:
    cd client && node scripts/gen-mana-oracle.mjs --check

[group('client')]
[doc("Run client tests")]
client-test:
    cd client && bun run test

[group('client')]
[doc("Build client for production")]
client-build:
    cd client && bun run build

[group('client')]
[doc("Run the client")]
client-run: client-build
    cd client && bun run preview

# ── Deploy / DB ──────────────────────────────────────────────────────────────────────

[doc("Apply Toasty migrations against DATABASE_URL (default: compose Postgres)")]
migrate:
    DATABASE_URL="${DATABASE_URL:-postgresql://mtgfr:mtgfr@localhost:5432/mtgfr}" cargo run -p server -- migration apply

[doc("Apply-machine deploy: roll to tfvars server_image/web_image (or SERVER_IMAGE/WEB_IMAGE env). See docs/prds/DEPLOYMENT.md.")]
deploy:
    ./iac/scripts/deploy.sh

# ── Workspace ────────────────────────────────────────────────────────────────────────

[doc("Format all code")]
format: server-format client-format

[doc("Lint all code")]
lint: server-lint client-lint

[doc("Typecheck all code")]
typecheck: client-typecheck

[doc("Run all tests")]
test *args:
    @just server-test {{ args }}
    @just client-test

[doc("Run all checks")]
check: server-codegen format lint typecheck test

[doc("Regenerate docs/CR_INDEX.md from engine CR citations")]
engine-cr-index:
    python3 scripts/gen_cr_index.py

[doc("Fail if docs/CR_INDEX.md is stale vs engine CR citations")]
engine-cr-index-check:
    python3 scripts/gen_cr_index.py --check

[doc("Scan engine for likely missing CR citations (advisory)")]
engine-cr-scan:
    python3 scripts/scan_missing_cr.py

[doc("Add CR cites to comments flagged by engine-cr-scan, then refresh CR_INDEX")]
engine-cr-fix:
    python3 scripts/fix_missing_cr.py --self-test
    python3 scripts/scan_missing_cr.py --self-test
    python3 scripts/fix_missing_cr.py
    @just engine-cr-index

[doc("Run server and client for development")]
dev:
    tmux new-session -d -s dev "bacon server"
    tmux split-window -t dev -h "cd client && bun run dev"
    tmux select-layout -t dev even-horizontal
    tmux set-option -t dev mouse on
    tmux set-option -t dev remain-on-exit on
    tmux attach -t dev
