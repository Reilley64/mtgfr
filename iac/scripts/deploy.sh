#!/usr/bin/env bash
# Nested API rolls: versioned instances (1 active + many drainers), SolidStart BFF sticky.
# Requires SERVER_IMAGE and WEB_IMAGE. Recovers prior map from `terraform output -json`.
#
# Cutover order (no dual-accept window):
#   1) Add the new Deployment while keeping the *previous* id as api_active_instance_id
#   2) POST /admin/drain on the previous active
#   3) Flip api_active_instance_id to the new id
# Cap waits fail after API_CAP_WAIT_SECONDS (default 6h) rather than blocking forever.
set -euo pipefail

cd "$(dirname "$0")/.."

: "${SERVER_IMAGE:?SERVER_IMAGE must be set to the new mtgfr-server image tag}"
: "${WEB_IMAGE:?WEB_IMAGE must be set to the new mtgfr-web image tag}"

MAX_INSTANCES="${API_MAX_INSTANCES:-4}"
CAP_WAIT_SECONDS="${API_CAP_WAIT_SECONDS:-21600}"

die() {
  echo "deploy: error: $*" >&2
  exit 1
}

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

# Fail closed: never treat a TF output failure as "empty fleet".
require_output_json() {
  local name="$1"
  local file
  file="$(mktemp)"
  if ! terraform output -json "$name" >"$file" 2>/tmp/mtgfr-tf-output.err; then
    rm -f "$file"
    die "terraform output -json $name failed (refusing to assume empty state). $(tr '\n' ' ' </tmp/mtgfr-tf-output.err)"
  fi
  cat "$file"
  rm -f "$file"
}

require_output_raw() {
  local name="$1"
  local value
  if ! value="$(terraform output -raw "$name" 2>/tmp/mtgfr-tf-output.err)"; then
    die "terraform output -raw $name failed (refusing to assume empty state). $(tr '\n' ' ' </tmp/mtgfr-tf-output.err)"
  fi
  if [ -z "$value" ]; then
    die "terraform output -raw $name returned empty"
  fi
  printf '%s' "$value"
}

# True when state has never been applied with the N-instance outputs (legacy / brand-new).
is_greenfield() {
  if ! terraform output -json api_instances >/tmp/mtgfr-api-instances.json 2>/dev/null; then
    return 0
  fi
  if ! terraform output -raw api_active_instance_id >/tmp/mtgfr-api-active.txt 2>/dev/null; then
    return 0
  fi
  if ! terraform output -raw web_image >/tmp/mtgfr-web-image.txt 2>/dev/null; then
    return 0
  fi
  local instances active web
  instances="$(cat /tmp/mtgfr-api-instances.json)"
  active="$(cat /tmp/mtgfr-api-active.txt)"
  web="$(cat /tmp/mtgfr-web-image.txt)"
  if [ "$instances" = "{}" ] || [ -z "$active" ] || [ -z "$web" ]; then
    return 0
  fi
  return 1
}

wait_for_cap_slot() {
  local count start_time elapsed
  count="$(python3 -c 'import json,sys; print(len(json.loads(sys.argv[1])))' "$1")"
  start_time=$(date +%s)
  while [ "$count" -ge "$MAX_INSTANCES" ]; do
    elapsed=$(($(date +%s) - start_time))
    if [ "$elapsed" -ge "$CAP_WAIT_SECONDS" ]; then
      die "still at api_max_instances=$MAX_INSTANCES after ${CAP_WAIT_SECONDS}s — free a drain peer or raise the cap"
    fi
    echo "deploy: at api_max_instances=$MAX_INSTANCES — waiting for a drain peer to empty (${elapsed}s/${CAP_WAIT_SECONDS}s)…" >&2
    ./scripts/wait-drain.sh --gc-one
    instances_json="$(require_output_json api_instances)"
    count="$(python3 -c 'import json,sys; print(len(json.loads(sys.argv[1])))' "$instances_json")"
  done
}

if is_greenfield; then
  # Distinguish "outputs missing because never applied" from "backend error after prior applies".
  if terraform state list >/tmp/mtgfr-state-list.txt 2>/dev/null \
    && grep -q 'kubernetes_deployment_v1.edh_api' /tmp/mtgfr-state-list.txt; then
    die "api instance outputs missing/empty but edh_api Deployments exist in state — fix outputs/state before rolling"
  fi
  new_id="$(instance_id_from_image "$SERVER_IMAGE")"
  echo "deploy: no prior API instances — applying $SERVER_IMAGE / $WEB_IMAGE as $new_id." >&2
  apply_api "$(python3 -c 'import json,sys; print(json.dumps({sys.argv[1]: sys.argv[2]}))' "$new_id" "$SERVER_IMAGE")" \
    "$new_id" "$WEB_IMAGE"
  exit 0
fi

old_web_image="$(require_output_raw web_image)"
old_active_id="$(require_output_raw api_active_instance_id)"
old_server_image="$(require_output_raw server_image)"
instances_json="$(require_output_json api_instances)"
new_id="$(instance_id_from_image "$SERVER_IMAGE")"

# Same active image — maybe web-only (only if no drain peers).
if [ "$old_server_image" = "$SERVER_IMAGE" ]; then
  drain_ids="$(require_output_json api_drain_instance_ids)"
  if [ "$old_web_image" = "$WEB_IMAGE" ] && [ "$drain_ids" = "[]" ]; then
    echo "deploy: server_image and web_image unchanged, no drain peers — nothing to roll." >&2
    exit 0
  fi
  if [ "$drain_ids" != "[]" ]; then
    echo "deploy: active API unchanged — GC drain peers before considering web bump…" >&2
    ./scripts/wait-drain.sh --gc-empty
    instances_json="$(require_output_json api_instances)"
    drain_ids="$(require_output_json api_drain_instance_ids)"
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

wait_for_cap_slot "$instances_json"
instances_json="$(require_output_json api_instances)"

if python3 -c 'import json,sys; raise SystemExit(0 if sys.argv[1] in json.loads(sys.argv[2]) else 1)' \
  "$new_id" "$instances_json"; then
  die "instance $new_id already in map — refusing to overwrite its image (would restart pods)"
fi

echo "deploy: adding $new_id ($SERVER_IMAGE) while keeping active=$old_active_id; holding web on $old_web_image…" >&2
next_json="$(python3 -c '
import json, sys
m = json.loads(sys.argv[1])
m[sys.argv[2]] = sys.argv[3]
print(json.dumps(m))
' "$instances_json" "$new_id" "$SERVER_IMAGE")"

# Step 1: new peer up, old still active (BFF still sends cookieless traffic to old).
apply_api "$next_json" "$old_active_id" "$old_web_image"

# Step 2: stop new tables on old *before* flipping active — no dual-accept window.
echo "deploy: marking previous active $old_active_id draining (live toggle)…" >&2
./scripts/wait-drain.sh --mark-only "$old_active_id"

# Step 3: cookieless / new tables → new instance.
echo "deploy: flipping active $old_active_id -> $new_id…" >&2
apply_api "$next_json" "$new_id" "$old_web_image"

echo "deploy: GC empty drain peers…" >&2
./scripts/wait-drain.sh --gc-empty

instances_json="$(require_output_json api_instances)"
drain_ids="$(require_output_json api_drain_instance_ids)"
if [ "$drain_ids" = "[]" ]; then
  echo "deploy: no drain peers left — bumping edh-web to $WEB_IMAGE…" >&2
  apply_api "$instances_json" "$new_id" "$WEB_IMAGE"
else
  echo "deploy: drain peers remain ($drain_ids) — holding edh-web on $old_web_image (nested roll OK)." >&2
fi
