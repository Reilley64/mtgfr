#!/usr/bin/env bash
# Drain helpers for versioned API Services (port-forward, never the public tunnel).
#
# Usage:
#   wait-drain.sh --mark-only <instance_id>   # POST /admin/drain only
#   wait-drain.sh --wait <instance_id>        # mark + poll until active_tables=0
#   wait-drain.sh --gc-empty                  # remove drain peers with active_tables=0
#   wait-drain.sh --gc-one                    # wait until one drain peer empties, then remove it
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=tf-common.sh
source ./scripts/tf-common.sh

NAMESPACE="${MTGFR_NAMESPACE:-edh}"
LOCAL_PORT="${WAIT_DRAIN_LOCAL_PORT:-18080}"
POLL_INTERVAL_SECONDS="${WAIT_DRAIN_POLL_INTERVAL_SECONDS:-5}"
LOUD_LOG_AFTER_SECONDS="${WAIT_DRAIN_LOUD_LOG_AFTER_SECONDS:-86400}"

AUTH_ARGS=()
if [ -n "${MTGFR_ADMIN_TOKEN:-}" ]; then
  AUTH_ARGS=(-H "Authorization: Bearer ${MTGFR_ADMIN_TOKEN}")
fi

MODE="${1:-}"
ARG2="${2:-}"

apply_map() {
  local peers_json="$1"
  local server web
  server="$(require_output_raw server_image)"
  web="$(require_output_raw web_image)"
  tf_apply_images "$server" "$web" "$peers_json"
}

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
  local peers_json id tables changed
  peers_json="$(require_output_json api_peer_images)"
  changed=0

  for id in $(python3 -c 'import json,sys; print(" ".join(json.loads(sys.argv[1])))' "$peers_json"); do
    tables="$(read_tables "$id")"
    if [ "$tables" = "unreachable" ]; then
      echo "wait-drain: keep $id (unreachable — refusing GC)." >&2
    elif [ "$tables" = "0" ]; then
      echo "wait-drain: GC $id (tables=0)." >&2
      peers_json="$(python3 -c 'import json,sys; m=json.loads(sys.argv[1]); m.pop(sys.argv[2], None); print(json.dumps(m, separators=(",", ":")))' "$peers_json" "$id")"
      changed=1
    else
      echo "wait-drain: keep $id (active_tables=$tables)." >&2
    fi
  done

  if [ "$changed" -eq 1 ]; then
    apply_map "$peers_json"
  fi
}

gc_one() {
  local drain_ids id tables peers_json pf_pid start_time elapsed
  local max_wait="${API_CAP_WAIT_SECONDS:-21600}"
  drain_ids="$(require_output_json api_drain_instance_ids)"
  if [ "$drain_ids" = "[]" ]; then
    echo "wait-drain: no drain peers to GC." >&2
    return 0
  fi

  start_time=$(date +%s)
  while true; do
    elapsed=$(($(date +%s) - start_time))
    if [ "$elapsed" -ge "$max_wait" ]; then
      echo "wait-drain: error: no empty drain peer after ${max_wait}s" >&2
      exit 1
    fi
    for id in $(python3 -c 'import json,sys; print(" ".join(json.loads(sys.argv[1])))' "$drain_ids"); do
      pf_pid="$(start_pf "$id")" || continue
      if wait_pf_ready; then
        post_admin_drain || true
        tables="$(active_tables_from_json "$(curl_drain_json)")"
        kill "$pf_pid" 2>/dev/null || true
        if [ "$tables" = "0" ]; then
          peers_json="$(require_output_json api_peer_images)"
          peers_json="$(python3 -c 'import json,sys; m=json.loads(sys.argv[1]); m.pop(sys.argv[2], None); print(json.dumps(m, separators=(",", ":")))' "$peers_json" "$id")"
          echo "wait-drain: removing emptied peer $id." >&2
          apply_map "$peers_json"
          return 0
        fi
        echo "wait-drain: $id still has active_tables=$tables." >&2
      else
        kill "$pf_pid" 2>/dev/null || true
        echo "wait-drain: keep $id (unreachable — refusing GC)." >&2
      fi
    done
    echo "wait-drain: no empty drain peer yet — sleeping ${POLL_INTERVAL_SECONDS}s (${elapsed}s/${max_wait}s)…" >&2
    sleep "$POLL_INTERVAL_SECONDS"
    drain_ids="$(require_output_json api_drain_instance_ids)"
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
