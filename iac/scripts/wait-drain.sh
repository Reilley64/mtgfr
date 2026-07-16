#!/usr/bin/env bash
# Live-drain the edh-api-drain peer via kubectl port-forward (not the public tunnel).
set -euo pipefail

NAMESPACE="${MTGFR_NAMESPACE:-edh}"
SERVICE="edh-api-drain"
LOCAL_PORT="${WAIT_DRAIN_LOCAL_PORT:-18080}"
POLL_INTERVAL_SECONDS="${WAIT_DRAIN_POLL_INTERVAL_SECONDS:-5}"
LOUD_LOG_AFTER_SECONDS="${WAIT_DRAIN_LOUD_LOG_AFTER_SECONDS:-86400}"

AUTH_ARGS=()
if [ -n "${MTGFR_ADMIN_TOKEN:-}" ]; then
  AUTH_ARGS=(-H "Authorization: Bearer ${MTGFR_ADMIN_TOKEN}")
fi

if ! kubectl get service "$SERVICE" -n "$NAMESPACE" >/dev/null 2>&1; then
  echo "wait-drain: no $SERVICE Service in namespace $NAMESPACE — nothing to drain, skipping." >&2
  exit 0
fi

kubectl port-forward -n "$NAMESPACE" "service/$SERVICE" "$LOCAL_PORT:8080" \
  >/tmp/mtgfr-wait-drain-port-forward.log 2>&1 &
PORT_FORWARD_PID=$!
trap 'kill "$PORT_FORWARD_PID" 2>/dev/null || true' EXIT

# Give the port-forward a moment to establish before the first request.
ready=0
for _ in $(seq 1 10); do
  if curl -s -o /dev/null "${AUTH_ARGS[@]}" "http://127.0.0.1:${LOCAL_PORT}/health/drain"; then
    ready=1
    break
  fi
  sleep 1
done
if [ "$ready" -ne 1 ]; then
  echo "wait-drain: could not reach $SERVICE via port-forward on 127.0.0.1:${LOCAL_PORT}" >&2
  exit 1
fi

echo "wait-drain: marking $SERVICE draining (POST /admin/drain) — live toggle, does not restart the pod."
curl -sf -X POST "${AUTH_ARGS[@]}" "http://127.0.0.1:${LOCAL_PORT}/admin/drain" >/dev/null

start_time=$(date +%s)
while true; do
  response=$(curl -sf "${AUTH_ARGS[@]}" "http://127.0.0.1:${LOCAL_PORT}/health/drain") || {
    echo "wait-drain: /health/drain request failed — retrying in ${POLL_INTERVAL_SECONDS}s" >&2
    sleep "$POLL_INTERVAL_SECONDS"
    continue
  }

  active_tables=$(printf '%s' "$response" | python3 -c 'import json, sys; print(json.load(sys.stdin)["active_tables"])')

  if [ "$active_tables" -eq 0 ]; then
    echo "wait-drain: $SERVICE reports active_tables=0 — drain complete."
    break
  fi

  elapsed=$(($(date +%s) - start_time))
  if [ "$elapsed" -ge "$LOUD_LOG_AFTER_SECONDS" ]; then
    echo "wait-drain: WARNING — $SERVICE still draining after ${elapsed}s (active_tables=${active_tables}). Operator decides whether to keep waiting (no auto-kill for v1)." >&2
  else
    echo "wait-drain: $SERVICE active_tables=${active_tables}, waiting…"
  fi

  sleep "$POLL_INTERVAL_SECONDS"
done
