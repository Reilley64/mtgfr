#!/usr/bin/env bash
# Assert CI Wave 1 wiring (concurrency cancel, terraform path-skip, cheap gates).
set -euo pipefail

ci=".github/workflows/ci.yml"
verify=".github/workflows/verify-jobs.yml"
justfile="justfile"

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

need_file "$ci"
need_file "$verify"
need_file "$justfile"
need_file "scripts/check-docker-workflow-cache.sh"

# PR concurrency cancel
need_grep "$ci" 'concurrency:' "ci concurrency"
need_grep "$ci" 'group:[[:space:]]*ci-\$\{\{[[:space:]]*github\.ref[[:space:]]*\}\}' "ci concurrency group"
need_grep "$ci" 'cancel-in-progress:[[:space:]]*true' "ci cancel-in-progress"

# paths-filter changes job + terraform gate
need_grep "$ci" 'dorny/paths-filter@v3' "paths-filter action"
need_grep "$ci" "iac/\*\*" "iac path filter"
need_grep "$ci" '\.github/workflows/ci\.yml' "ci.yml path filter"
need_grep "$ci" 'needs:[[:space:]]*changes' "terraform needs changes"
need_grep "$ci" "needs\.changes\.outputs\.iac[[:space:]]*==[[:space:]]*'true'" "terraform if iac"

# Always-on cheap guards
need_grep "$ci" 'check-docker-workflow-cache\.sh' "docker cache guard step"
need_grep "$ci" 'check-ci-wave1\.sh' "wave1 guard step"

# just recipes
need_grep "$justfile" '^server-check:.*engine-cr-index-check' "server-check includes engine-cr-index-check"
need_grep "$justfile" '^client-check:.*client-mana-oracle-check' "client-check includes client-mana-oracle-check"
need_grep "$justfile" '^check:.*engine-cr-index-check' "check includes engine-cr-index-check"
need_grep "$justfile" '^check:.*client-mana-oracle-check' "check includes client-mana-oracle-check"

# server pass-marker includes CR index inputs + v2 key
need_grep "$verify" 'verify-server-v2-' "server pass marker v2"
need_grep "$verify" "docs/CR_INDEX\.md" "server hashFiles CR_INDEX"
need_grep "$verify" "scripts/gen_cr_index\.py" "server hashFiles gen_cr_index"

./scripts/check-docker-workflow-cache.sh

echo "ok: CI Wave 1 wiring present"
