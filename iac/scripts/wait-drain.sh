#!/usr/bin/env bash
# Drain helpers for versioned API Services (port-forward, never the public tunnel).
#
# Usage:
#   wait-drain.sh --mark-only <instance_id>   # POST /admin/drain only
#   wait-drain.sh --wait <instance_id>        # mark + poll until active_tables=0
#   wait-drain.sh --gc-empty                  # remove drain peers with active_tables=0 from TF map
#   wait-drain.sh --gc-one                    # wait until one drain peer empties, then remove it
set -euo pipefail

cd "$(dirname "$0")/.."

NAMESPACE="${MTGFR_NAMESPACE:-edh}"
LOCAL_PORT="${WAIT_DRAIN_LOCAL_PORT:-18080}"
POLL_INTERVAL_SECONDS="${WAIT_DRAIN_POLL_INTERVAL_SECONDS:-5}"
LOUD_LOG_AFTER_SECONDS="${WAIT_DRAIN_LOUD_LOG_AFTER_SECONDS:-86400}"
MAX_INSTANCES="${API_MAX_INSTANCES:-4}"

AUTH_ARGS=()
if [ -n "${MTGFR_ADMIN_TOKEN:-}" ]; then
  AUTH_ARGS=(-H "Authorization: Bearer ${MTGFR_ADMIN_TOKEN}")
fi

MODE="${1:-}"
ARG2="${2:-}"

tf_instances_var() {
  python3 -c '
import json, sys
images = json.loads(sys.argv[1])
mapped = {k: {"image": v} for k, v in images.items()}
print(json.dumps(mapped, separators=(",", ":")))
' "$1"
}

apply_map() {
  local images_json="$1"
  local active_id="$2"
  local web
  web="$(terraform output -raw web_image)"
  terraform apply -auto-approve \
    -var="api_instances=$(tf_instances_var "$images_json")" \
    -var="api_active_instance_id=$active_id" \
    -var="api_max_instances=$MAX_INSTANCES" \
    -var="web_image=$web"
}

# Start port-forward; echo pid. Caller must kill.
start_pf() {
  local service="$1"
  if ! kubectl get service "$service" -n "$NAMESPACE" >/dev/null 2>&1; then
    return 1
  fi
  kubectl port-forward -n "$NAMESPACE" "service/$service" "$LOCAL_PORT:8080" \
    >/tmp/mtgfr-wait-drain-port-forward.log 2>&1 &
  echo $!
}

wait_pf_ready() {
  local _
  for _ in $(seq 1 10); do
    if curl -s -o /dev/null "${AUTH_ARGS[@]}" "http://127.0.0.1:${LOCAL_PORT}/health/drain"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

curl_drain_json() {
  curl -sf "${AUTH_ARGS[@]}" "http://127.0.0.1:${LOCAL_PORT}/health/drain"
}

post_admin_drain() {
  curl -sf -X POST "${AUTH_ARGS[@]}" "http://127.0.0.1:${LOCAL_PORT}/admin/drain" >/dev/null
}

active_tables_from_json() {
  printf '%s' "$1" | python3 -c 'import json,sys; print(json.load(sys.stdin)["active_tables"])'
}

mark_only() {
  local service="$1"
  local pf_pid
  pf_pid="$(start_pf "$service")" || {
    echo "wait-drain: no Service $service — skip mark." >&2
    return 0
  }
  trap 'kill '"$pf_pid"' 2>/dev/null || true' EXIT
  wait_pf_ready || {
    echo "wait-drain: could not reach $service" >&2
    exit 1
  }
  echo "wait-drain: POST /admin/drain on $service…" >&2
  post_admin_drain
  kill "$pf_pid" 2>/dev/null || true
  trap - EXIT
}

wait_peer() {
  local service="$1"
  local pf_pid response tables start_time elapsed
  pf_pid="$(start_pf "$service")" || {
    echo "wait-drain: no Service $service — skip." >&2
    return 0
  }
  trap 'kill '"$pf_pid"' 2>/dev/null || true' EXIT
  wait_pf_ready || {
    echo "wait-drain: could not reach $service" >&2
    exit 1
  }
  echo "wait-drain: POST /admin/drain on $service…" >&2
  post_admin_drain
  start_time=$(date +%s)
  while true; do
    response="$(curl_drain_json)" || {
      echo "wait-drain: /health/drain failed — retrying…" >&2
      sleep "$POLL_INTERVAL_SECONDS"
      continue
    }
    tables="$(active_tables_from_json "$response")"
    if [ "$tables" -eq 0 ]; then
      echo "wait-drain: $service active_tables=0." >&2
      break
    fi
    elapsed=$(($(date +%s) - start_time))
    if [ "$elapsed" -ge "$LOUD_LOG_AFTER_SECONDS" ]; then
      echo "wait-drain: WARNING — $service still draining after ${elapsed}s (active_tables=${tables})." >&2
    else
      echo "wait-drain: $service active_tables=${tables}…" >&2
    fi
    sleep "$POLL_INTERVAL_SECONDS"
  done
  kill "$pf_pid" 2>/dev/null || true
  trap - EXIT
}

read_tables() {
  local service="$1"
  local pf_pid response
  pf_pid="$(start_pf "$service")" || {
    echo "unreachable"
    return 0
  }
  if ! wait_pf_ready; then
    kill "$pf_pid" 2>/dev/null || true
    echo "unreachable"
    return 0
  fi
  response="$(curl_drain_json)" || {
    kill "$pf_pid" 2>/dev/null || true
    echo "unreachable"
    return 0
  }
  active_tables_from_json "$response"
  kill "$pf_pid" 2>/dev/null || true
}

gc_empty() {
  local images_json active_id drain_ids id tables changed
  images_json="$(terraform output -json api_instances)"
  active_id="$(terraform output -raw api_active_instance_id)"
  drain_ids="$(terraform output -json api_drain_instance_ids)"
  changed=0

  for id in $(python3 -c 'import json,sys; print(" ".join(json.loads(sys.argv[1])))' "$drain_ids"); do
    tables="$(read_tables "$id")"
    if [ "$tables" = "0" ] || [ "$tables" = "unreachable" ]; then
      echo "wait-drain: GC $id (tables=$tables)." >&2
      images_json="$(python3 -c 'import json,sys; m=json.loads(sys.argv[1]); m.pop(sys.argv[2], None); print(json.dumps(m))' "$images_json" "$id")"
      changed=1
    else
      echo "wait-drain: keep $id (active_tables=$tables)." >&2
    fi
  done

  if [ "$changed" -eq 1 ]; then
    apply_map "$images_json" "$active_id"
  fi
}

gc_one() {
  local active_id drain_ids id tables images_json pf_pid
  active_id="$(terraform output -raw api_active_instance_id)"
  drain_ids="$(terraform output -json api_drain_instance_ids)"
  if [ "$drain_ids" = "[]" ]; then
    echo "wait-drain: no drain peers to GC." >&2
    return 0
  fi

  while true; do
    for id in $(python3 -c 'import json,sys; print(" ".join(json.loads(sys.argv[1])))' "$drain_ids"); do
      pf_pid="$(start_pf "$id")" || continue
      if wait_pf_ready; then
        post_admin_drain || true
        tables="$(active_tables_from_json "$(curl_drain_json)")"
        kill "$pf_pid" 2>/dev/null || true
        if [ "$tables" = "0" ]; then
          images_json="$(terraform output -json api_instances)"
          images_json="$(python3 -c 'import json,sys; m=json.loads(sys.argv[1]); m.pop(sys.argv[2], None); print(json.dumps(m))' "$images_json" "$id")"
          echo "wait-drain: removing emptied peer $id." >&2
          apply_map "$images_json" "$active_id"
          return 0
        fi
        echo "wait-drain: $id still has active_tables=$tables." >&2
      else
        kill "$pf_pid" 2>/dev/null || true
      fi
    done
    echo "wait-drain: no empty drain peer yet — sleeping ${POLL_INTERVAL_SECONDS}s…" >&2
    sleep "$POLL_INTERVAL_SECONDS"
    drain_ids="$(terraform output -json api_drain_instance_ids)"
  done
}

case "$MODE" in
  --mark-only)
    : "${ARG2:?instance id required}"
    mark_only "$ARG2"
    ;;
  --wait)
    : "${ARG2:?instance id required}"
    wait_peer "$ARG2"
    ;;
  --gc-empty)
    gc_empty
    ;;
  --gc-one)
    gc_one
    ;;
  *)
    echo "usage: $0 --mark-only|--wait <id> | --gc-empty | --gc-one" >&2
    exit 2
    ;;
esac
