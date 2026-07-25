#!/usr/bin/env bash
# Assert CI Wave 2 wiring (server cache gate, client hash tighten, vitest junit, Node-24 actions).
set -euo pipefail

verify=".github/workflows/verify-jobs.yml"
ci=".github/workflows/ci.yml"
vitest="client/vitest.config.ts"

need_file() {
  local f=$1
  if [[ ! -f "$f" ]]; then
    echo "missing $f" >&2
    exit 1
  fi
}

need_grep() {
  local f=$1
  local pat=$2
  local label=$3
  if ! grep -qE "$pat" "$f"; then
    echo "$label: missing pattern in $f: $pat" >&2
    exit 1
  fi
}

need_file "$verify"
need_file "$ci"
need_file "$vitest"

# Server split: gate job without postgres service; full job has postgres + if miss
need_grep "$verify" 'verify-server-gate:' "server gate job"
need_grep "$verify" 'lookup-only:[[:space:]]*true' "cache lookup-only"
need_grep "$verify" 'needs:[[:space:]]*verify-server-gate' "full server needs gate"
need_grep "$verify" "needs\.verify-server-gate\.outputs\.cache-hit[[:space:]]*!=[[:space:]]*'true'" "full server if miss"
need_grep "$verify" 'image:[[:space:]]*postgres:16' "postgres only on full job"

# Gate job must not declare services:
if awk '/^  verify-server-gate:/,/^  [a-z]/{print}' "$verify" | grep -q '^    services:'; then
  echo "verify-server-gate must not declare services:" >&2
  exit 1
fi

# Client hash no longer includes crates/** or Cargo.*
if grep -E "verify-client-v[0-9]+-.*hashFiles\([^)]*crates/\*\*" "$verify"; then
  echo "client hashFiles still includes crates/**" >&2
  exit 1
fi
need_grep "$verify" 'verify-client-v3-' "client pass marker v3"
need_grep "$verify" "hashFiles\('client/\*\*', 'proto/\*\*'" "client hashFiles proto+client"

# Client job must not install Rust toolchain
if awk '/^  verify-client:/,/^  [a-z]/{print}' "$verify" | grep -q 'dtolnay/rust-toolchain'; then
  echo "verify-client must not use dtolnay/rust-toolchain" >&2
  exit 1
fi

# Vitest JUnit
need_grep "$vitest" 'junit' "vitest junit reporter"
need_grep "$verify" 'client-junit|junit\.xml' "client junit path in workflow"
need_grep "$verify" 'test-summary/action@v2' "test-summary for client"

# Node-24 action majors (checkout@v5+, cache@v5+, setup-node@v5+, upload-artifact@v5+)
need_grep "$ci" 'actions/checkout@v[5-9]' "ci checkout major >=5"
need_grep "$verify" 'actions/checkout@v[5-9]' "verify checkout major >=5"
need_grep "$verify" 'actions/cache@v[5-9]' "verify cache major >=5"
need_grep "$ci" 'actions/setup-node@v[5-9]' "ci setup-node major >=5"
need_grep "$verify" 'actions/upload-artifact@v[5-9]' "upload-artifact major >=5"

# Wave 2 guard job
need_grep "$ci" 'check-ci-wave2\.sh' "wave2 guard step"

./scripts/check-ci-wave1.sh

echo "ok: CI Wave 2 wiring present"
