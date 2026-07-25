#!/usr/bin/env bash
# Assert CI Wave 3 wiring (parallel Docker jobs).
set -euo pipefail

docker=".github/workflows/docker.yml"
ci=".github/workflows/ci.yml"

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

need_file "$docker"
need_file "$ci"

need_grep "$docker" '^  docker-server:' "docker-server job"
need_grep "$docker" '^  docker-web:' "docker-web job"
need_grep "$docker" '^  docker-visibility:' "docker-visibility job"
need_grep "$docker" 'needs:[[:space:]]*\[docker-server,[[:space:]]*docker-web\]' "visibility needs both builds"
need_grep "$docker" 'file:[[:space:]]*docker/server/Dockerfile' "server Dockerfile"
need_grep "$docker" 'file:[[:space:]]*docker/web/Dockerfile' "web Dockerfile"
need_grep "$docker" 'scope=mtgfr-server' "server cache scope"
need_grep "$docker" 'scope=mtgfr-web' "web cache scope"

# Must not keep a monolithic sequential job named exactly `docker:` at jobs root
if grep -qE '^  docker:' "$docker"; then
  echo "docker.yml still has monolithic job 'docker:' — expected parallel jobs" >&2
  exit 1
fi

need_grep "$ci" 'check-ci-wave3\.sh' "wave3 guard step"

./scripts/check-docker-workflow-cache.sh
./scripts/check-ci-wave2.sh

echo "ok: CI Wave 3 wiring present"
