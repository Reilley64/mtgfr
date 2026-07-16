#!/usr/bin/env bash
# terraform apply that preserves drain peers (api_peer_images from outputs).
# Use this instead of bare `terraform apply` whenever peers may exist.
# Extra args are forwarded (e.g. -target=...).
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=tf-common.sh
source ./scripts/tf-common.sh

if [ -n "${SERVER_IMAGE:-}" ]; then
  server="$SERVER_IMAGE"
elif server="$(read_tfvar server_image 2>/dev/null)"; then
  :
elif server="$(terraform output -raw server_image 2>/dev/null)"; then
  :
else
  die "set SERVER_IMAGE, server_image in tfvars, or apply once so outputs exist"
fi

if [ -n "${WEB_IMAGE:-}" ]; then
  web="$WEB_IMAGE"
elif web="$(read_tfvar web_image 2>/dev/null)"; then
  :
elif web="$(terraform output -raw web_image 2>/dev/null)"; then
  :
else
  die "set WEB_IMAGE, web_image in tfvars, or apply once so outputs exist"
fi

peers_json="$(peer_images_json)"

terraform apply \
  -var="server_image=$server" \
  -var="web_image=$web" \
  -var="api_peer_images=${peers_json}" \
  -var="api_max_instances=$MAX_INSTANCES" \
  "$@"
