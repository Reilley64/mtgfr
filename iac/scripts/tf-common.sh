#!/usr/bin/env bash
# Shared helpers for deploy / wait-drain. Sourced, not executed.
# shellcheck shell=bash

MAX_INSTANCES="${API_MAX_INSTANCES:-4}"
NAMESPACE="${MTGFR_NAMESPACE:-edh}"
PEERS_CM="${MTGFR_PEERS_CM:-edh-api-peers}"

die() {
  echo "${0##*/}: error: $*" >&2
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

# Read a simple string assignment from terraform.tfvars / *.auto.tfvars (operator knobs).
read_tfvar() {
  local name="$1"
  python3 -c '
import pathlib, re, sys
name = sys.argv[1]
root = pathlib.Path(".")
paths = sorted(root.glob("*.auto.tfvars")) + [root / "terraform.tfvars"]
pat = re.compile(rf"^\s*{re.escape(name)}\s*=\s*\"([^\"]*)\"", re.M)
for path in paths:
    if not path.is_file():
        continue
    m = pat.search(path.read_text())
    if m:
        print(m.group(1))
        raise SystemExit(0)
raise SystemExit(1)
' "$name"
}

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

# Drain peers from ConfigMap edh-api-peers (fallback {} if missing).
read_peer_images_cm() {
  if ! kubectl get configmap "$PEERS_CM" -n "$NAMESPACE" >/dev/null 2>&1; then
    echo "{}"
    return 0
  fi
  kubectl get configmap "$PEERS_CM" -n "$NAMESPACE" -o json | python3 -c '
import json, sys
doc = json.load(sys.stdin)
print(json.dumps(doc.get("data") or {}, separators=(",", ":")))
'
}

# Replace ConfigMap data with JSON object { instance_id: image, ... } (full replace, drops stale keys).
write_peer_images_cm() {
  local peers_json="$1"
  NAMESPACE="$NAMESPACE" PEERS_CM="$PEERS_CM" python3 -c '
import json, os, subprocess, sys

peers = json.loads(sys.argv[1])
ns = os.environ["NAMESPACE"]
name = os.environ["PEERS_CM"]
doc = {
    "apiVersion": "v1",
    "kind": "ConfigMap",
    "metadata": {
        "name": name,
        "namespace": ns,
        "labels": {"app": "edh-api-peers"},
    },
    "data": peers,
}
payload = json.dumps(doc).encode()
# replace requires the object to exist; create on first use.
if subprocess.run(["kubectl", "get", "configmap", name, "-n", ns], capture_output=True).returncode != 0:
    subprocess.run(["kubectl", "create", "-f", "-"], input=payload, check=True)
else:
    subprocess.run(["kubectl", "replace", "-f", "-"], input=payload, check=True)
' "$peers_json"
}

# terraform apply; peers come from ConfigMap refresh (ignore_changes on data).
tf_apply_images() {
  local server="$1"
  local web="$2"
  terraform apply -auto-approve \
    -var="server_image=$server" \
    -var="web_image=$web" \
    -var="api_max_instances=$MAX_INSTANCES"
}

# Write peers then apply so for_each matches the ConfigMap.
apply_with_peers() {
  local server="$1"
  local web="$2"
  local peers_json="$3"
  write_peer_images_cm "$peers_json"
  tf_apply_images "$server" "$web"
}
