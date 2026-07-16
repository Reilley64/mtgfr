#!/usr/bin/env bash
# Two-phase roll: API (+ drain peer, hold web) → wait-drain → bump web, tear down drain.
# Requires SERVER_IMAGE and WEB_IMAGE. Recovers prior tags from `terraform output`.
set -euo pipefail

cd "$(dirname "$0")/.."

: "${SERVER_IMAGE:?SERVER_IMAGE must be set to the new mtgfr-server image tag}"
: "${WEB_IMAGE:?WEB_IMAGE must be set to the new mtgfr-web image tag}"

old_server_image="$(terraform output -raw server_image 2>/dev/null || true)"
old_web_image="$(terraform output -raw web_image 2>/dev/null || true)"

# No prior state (first-ever apply) — there is no old binary to drain, so there's nothing to hold
# edh-web back from either. Apply both images directly, no drain peer.
if [ -z "$old_server_image" ] || [ -z "$old_web_image" ]; then
  echo "deploy: no prior terraform output (first deploy) — applying $SERVER_IMAGE / $WEB_IMAGE directly." >&2
  terraform apply -auto-approve \
    -var="server_image=$SERVER_IMAGE" \
    -var="web_image=$WEB_IMAGE" \
    -var="api_drain_enabled=false"
  exit 0
fi

if [ "$old_server_image" = "$SERVER_IMAGE" ]; then
  if [ "$old_web_image" = "$WEB_IMAGE" ]; then
    echo "deploy: server_image and web_image unchanged — nothing to roll." >&2
    exit 0
  fi
  # The API isn't rolling, so there's no drain peer needed to protect it — just bump web.
  echo "deploy: server_image unchanged — bumping edh-web only ($old_web_image -> $WEB_IMAGE)…" >&2
  terraform apply -auto-approve \
    -var="server_image=$SERVER_IMAGE" \
    -var="web_image=$WEB_IMAGE" \
    -var="api_drain_enabled=false"
  exit 0
fi

echo "deploy: rolling edh-api $old_server_image -> $SERVER_IMAGE (edh-web held on $old_web_image)…" >&2
terraform apply -auto-approve \
  -var="server_image=$SERVER_IMAGE" \
  -var="server_image_drain=$old_server_image" \
  -var="web_image=$old_web_image" \
  -var="api_drain_enabled=true"

echo "deploy: waiting for drain (active_tables=0) via kubectl port-forward…" >&2
./scripts/wait-drain.sh

echo "deploy: drain complete — bumping edh-web to $WEB_IMAGE and tearing down edh-api-drain…" >&2
terraform apply -auto-approve \
  -var="server_image=$SERVER_IMAGE" \
  -var="web_image=$WEB_IMAGE" \
  -var="api_drain_enabled=false"
