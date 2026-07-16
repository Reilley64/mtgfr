#!/usr/bin/env bash
# Nested API rolls: versioned instances (1 active + many drainers), SolidStart BFF sticky.
# Requires SERVER_IMAGE and WEB_IMAGE. Recovers prior map from `terraform output -json`.
set -euo pipefail

cd "$(dirname "$0")/.."

: "${SERVER_IMAGE:?SERVER_IMAGE must be set to the new mtgfr-server image tag}"
: "${WEB_IMAGE:?WEB_IMAGE must be set to the new mtgfr-web image tag}"

MAX_INSTANCES="${API_MAX_INSTANCES:-4}"

# ghcr.io/owner/mtgfr-server:1.2.3 → edh-api-1-2-3
instance_id_from_image() {
  local image="$1"
  local tag="${image##*:}"
  local slug
  slug="$(printf '%s' "$tag" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//; s/-+/-/g')"
  printf 'edh-api-%s' "$slug"
}

# Build -var api_instances=… from a JSON object { id: image, … }
tf_instances_var() {
  local images_json="$1"
  python3 -c '
import json, sys
images = json.loads(sys.argv[1])
mapped = {k: {"image": v} for k, v in images.items()}
print(json.dumps(mapped, separators=(",", ":")))
' "$images_json"
}

apply_api() {
  local images_json="$1"
  local active_id="$2"
  local web="$3"
  terraform apply -auto-approve \
    -var="api_instances=$(tf_instances_var "$images_json")" \
    -var="api_active_instance_id=$active_id" \
    -var="api_max_instances=$MAX_INSTANCES" \
    -var="web_image=$web"
}

read_state() {
  if ! terraform output -json api_instances >/tmp/mtgfr-api-instances.json 2>/dev/null; then
    echo "{}"
    return
  fi
  # output is { "id": "image", ... } already from outputs.tf
  cat /tmp/mtgfr-api-instances.json
}

old_web_image="$(terraform output -raw web_image 2>/dev/null || true)"
old_active_id="$(terraform output -raw api_active_instance_id 2>/dev/null || true)"
old_server_image="$(terraform output -raw server_image 2>/dev/null || true)"
instances_json="$(read_state)"

new_id="$(instance_id_from_image "$SERVER_IMAGE")"

# First-ever apply — single active instance, bump web immediately.
if [ "$instances_json" = "{}" ] || [ -z "$old_active_id" ] || [ -z "$old_web_image" ]; then
  echo "deploy: no prior API instances — applying $SERVER_IMAGE / $WEB_IMAGE as $new_id." >&2
  apply_api "$(python3 -c 'import json,sys; print(json.dumps({sys.argv[1]: sys.argv[2]}))' "$new_id" "$SERVER_IMAGE")" \
    "$new_id" "$WEB_IMAGE"
  exit 0
fi

# Same active image — maybe web-only (only if no drain peers).
if [ "$old_server_image" = "$SERVER_IMAGE" ]; then
  drain_ids="$(terraform output -json api_drain_instance_ids 2>/dev/null || echo '[]')"
  if [ "$old_web_image" = "$WEB_IMAGE" ] && [ "$drain_ids" = "[]" ]; then
    echo "deploy: server_image and web_image unchanged, no drain peers — nothing to roll." >&2
    exit 0
  fi
  if [ "$drain_ids" != "[]" ]; then
    echo "deploy: active API unchanged — GC drain peers before considering web bump…" >&2
    ./scripts/wait-drain.sh --gc-empty
    instances_json="$(read_state)"
    drain_ids="$(terraform output -json api_drain_instance_ids 2>/dev/null || echo '[]')"
  fi
  if [ "$drain_ids" = "[]" ] && [ "$old_web_image" != "$WEB_IMAGE" ]; then
    echo "deploy: bumping edh-web only ($old_web_image -> $WEB_IMAGE)…" >&2
    apply_api "$instances_json" "$old_active_id" "$WEB_IMAGE"
    exit 0
  fi
  if [ "$drain_ids" != "[]" ]; then
    echo "deploy: drain peers still have tables — holding edh-web on $old_web_image." >&2
  fi
  exit 0
fi

# Cap: free a slot if needed (wait for at least one empty drainer and remove it).
count="$(python3 -c 'import json,sys; print(len(json.loads(sys.argv[1])))' "$instances_json")"
while [ "$count" -ge "$MAX_INSTANCES" ]; do
  echo "deploy: at api_max_instances=$MAX_INSTANCES — waiting for a drain peer to empty…" >&2
  ./scripts/wait-drain.sh --gc-one
  instances_json="$(read_state)"
  count="$(python3 -c 'import json,sys; print(len(json.loads(sys.argv[1])))' "$instances_json")"
done

if python3 -c 'import json,sys; raise SystemExit(0 if sys.argv[1] in json.loads(sys.argv[2]) else 1)' \
  "$new_id" "$instances_json"; then
  echo "deploy: instance $new_id already in map — refusing to overwrite its image (would restart pods)." >&2
  exit 1
fi

echo "deploy: rolling active $old_active_id ($old_server_image) -> $new_id ($SERVER_IMAGE); holding web on $old_web_image…" >&2
next_json="$(python3 -c '
import json, sys
m = json.loads(sys.argv[1])
m[sys.argv[2]] = sys.argv[3]
print(json.dumps(m))
' "$instances_json" "$new_id" "$SERVER_IMAGE")"

apply_api "$next_json" "$new_id" "$old_web_image"

echo "deploy: marking previous active $old_active_id draining (live toggle)…" >&2
./scripts/wait-drain.sh --mark-only "$old_active_id"

echo "deploy: GC empty drain peers…" >&2
./scripts/wait-drain.sh --gc-empty

instances_json="$(read_state)"
drain_ids="$(terraform output -json api_drain_instance_ids 2>/dev/null || echo '[]')"
if [ "$drain_ids" = "[]" ]; then
  echo "deploy: no drain peers left — bumping edh-web to $WEB_IMAGE…" >&2
  apply_api "$instances_json" "$new_id" "$WEB_IMAGE"
else
  echo "deploy: drain peers remain ($drain_ids) — holding edh-web on $old_web_image (nested roll OK)." >&2
fi
