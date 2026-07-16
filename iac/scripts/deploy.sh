#!/usr/bin/env bash
# Nested API rolls from a single desired server_image (+ web_image).
# Peers live in ConfigMap edh-api-peers; cutover: stage → drain → flip → GC → web.
# Cap wait: API_CAP_WAIT_SECONDS (default 6h). TF output failures fail closed.
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=tf-common.sh
source ./scripts/tf-common.sh

CAP_WAIT_SECONDS="${API_CAP_WAIT_SECONDS:-21600}"

if [ -n "${SERVER_IMAGE:-}" ]; then
  desired_server="$SERVER_IMAGE"
elif desired_server="$(read_tfvar server_image 2>/dev/null)"; then
  :
else
  die "set SERVER_IMAGE or server_image in terraform.tfvars"
fi

if [ -n "${WEB_IMAGE:-}" ]; then
  desired_web="$WEB_IMAGE"
elif desired_web="$(read_tfvar web_image 2>/dev/null)"; then
  :
else
  die "set WEB_IMAGE or web_image in terraform.tfvars"
fi

is_greenfield() {
  if ! terraform output -raw server_image >/tmp/mtgfr-server-image.txt 2>/dev/null; then
    return 0
  fi
  if ! terraform output -raw web_image >/tmp/mtgfr-web-image.txt 2>/dev/null; then
    return 0
  fi
  return 1
}

wait_for_cap_slot() {
  local peers_json="$1"
  local count start_time elapsed
  count="$(python3 -c 'import json,sys; print(len(json.loads(sys.argv[1])))' "$peers_json")"
  start_time=$(date +%s)
  while [ "$((count + 1))" -ge "$MAX_INSTANCES" ]; do
    elapsed=$(($(date +%s) - start_time))
    if [ "$elapsed" -ge "$CAP_WAIT_SECONDS" ]; then
      die "still at api_max_instances=$MAX_INSTANCES after ${CAP_WAIT_SECONDS}s — free a drain peer or raise the cap"
    fi
    echo "deploy: at api_max_instances=$MAX_INSTANCES — waiting for a drain peer to empty (${elapsed}s/${CAP_WAIT_SECONDS}s)…" >&2
    ./scripts/wait-drain.sh --gc-one
    peers_json="$(read_peer_images_cm)"
    count="$(python3 -c 'import json,sys; print(len(json.loads(sys.argv[1])))' "$peers_json")"
  done
}

if is_greenfield; then
  if terraform state list >/tmp/mtgfr-state-list.txt 2>/dev/null \
    && grep -q 'kubernetes_deployment_v1.edh_api' /tmp/mtgfr-state-list.txt; then
    die "api outputs missing but edh_api Deployments exist in state — fix outputs/state before rolling"
  fi
  new_id="$(instance_id_from_image "$desired_server")"
  echo "deploy: no prior API — applying $desired_server / $desired_web as $new_id." >&2
  apply_with_peers "$desired_server" "$desired_web" "{}"
  exit 0
fi

old_web_image="$(require_output_raw web_image)"
old_server_image="$(require_output_raw server_image)"
old_active_id="$(require_output_raw api_active_instance_id)"
peers_json="$(read_peer_images_cm)"
new_id="$(instance_id_from_image "$desired_server")"

if [ "$old_server_image" = "$desired_server" ]; then
  drain_ids="$(require_output_json api_drain_instance_ids)"
  if [ "$old_web_image" = "$desired_web" ] && [ "$drain_ids" = "[]" ]; then
    echo "deploy: server_image and web_image unchanged, no drain peers — nothing to roll." >&2
    exit 0
  fi
  if [ "$drain_ids" != "[]" ]; then
    echo "deploy: active API unchanged — GC drain peers before considering web bump…" >&2
    ./scripts/wait-drain.sh --gc-empty
    peers_json="$(read_peer_images_cm)"
    drain_ids="$(require_output_json api_drain_instance_ids)"
  fi
  if [ "$drain_ids" = "[]" ] && [ "$old_web_image" != "$desired_web" ]; then
    echo "deploy: bumping edh-web only ($old_web_image -> $desired_web)…" >&2
    apply_with_peers "$desired_server" "$desired_web" "$peers_json"
    exit 0
  fi
  if [ "$drain_ids" != "[]" ]; then
    echo "deploy: drain peers still have tables — holding edh-web on $old_web_image." >&2
  fi
  exit 0
fi

wait_for_cap_slot "$peers_json"
peers_json="$(read_peer_images_cm)"

if python3 -c 'import json,sys; raise SystemExit(0 if sys.argv[1] in json.loads(sys.argv[2]) else 1)' \
  "$new_id" "$peers_json"; then
  die "instance $new_id already in peer map — refusing to overwrite its image (would restart pods)"
fi
if [ "$new_id" = "$old_active_id" ]; then
  die "derived instance id $new_id matches current active — use a distinct image tag"
fi

echo "deploy: staging $new_id ($desired_server) as peer; active stays $old_active_id; holding web on $old_web_image…" >&2
staged_peers="$(python3 -c '
import json, sys
m = json.loads(sys.argv[1])
m[sys.argv[2]] = sys.argv[3]
print(json.dumps(m, separators=(",", ":")))
' "$peers_json" "$new_id" "$desired_server")"
apply_with_peers "$old_server_image" "$old_web_image" "$staged_peers"

echo "deploy: marking previous active $old_active_id draining (live toggle)…" >&2
./scripts/wait-drain.sh --mark-only "$old_active_id"

echo "deploy: flipping active $old_active_id -> $new_id…" >&2
flipped_peers="$(python3 -c '
import json, sys
m = json.loads(sys.argv[1])
m.pop(sys.argv[2], None)
m[sys.argv[3]] = sys.argv[4]
print(json.dumps(m, separators=(",", ":")))
' "$staged_peers" "$new_id" "$old_active_id" "$old_server_image")"
apply_with_peers "$desired_server" "$old_web_image" "$flipped_peers"

echo "deploy: GC empty drain peers…" >&2
./scripts/wait-drain.sh --gc-empty

peers_json="$(read_peer_images_cm)"
drain_ids="$(require_output_json api_drain_instance_ids)"
if [ "$drain_ids" = "[]" ]; then
  echo "deploy: no drain peers left — bumping edh-web to $desired_web…" >&2
  apply_with_peers "$desired_server" "$desired_web" "$peers_json"
else
  echo "deploy: drain peers remain ($drain_ids) — holding edh-web on $old_web_image (nested roll OK)." >&2
fi
