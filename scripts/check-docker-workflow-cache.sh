#!/usr/bin/env bash
# Assert docker.yml persists Buildx layers via type=gha (mode=max) per image scope.
set -euo pipefail

wf=".github/workflows/docker.yml"

if [[ ! -f "$wf" ]]; then
  echo "missing $wf" >&2
  exit 1
fi

need() {
  local pat=$1
  if ! grep -qE "$pat" "$wf"; then
    echo "docker.yml missing required pattern: $pat" >&2
    exit 1
  fi
}

need 'actions:[[:space:]]*write'
need 'cache-from:[[:space:]]*type=gha,scope=mtgfr-server'
need 'cache-to:[[:space:]]*type=gha,mode=max,scope=mtgfr-server'
need 'cache-from:[[:space:]]*type=gha,scope=mtgfr-web'
need 'cache-to:[[:space:]]*type=gha,mode=max,scope=mtgfr-web'

echo "ok: docker.yml Buildx GHA cache wiring present"
