#!/usr/bin/env bash
# Shared helpers for deploy / wait-drain / tf-apply. Sourced, not executed.
# shellcheck shell=bash

MAX_INSTANCES="${API_MAX_INSTANCES:-4}"

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

# Current drain peers from state, or {} when outputs are unavailable (greenfield).
peer_images_json() {
  if terraform output -json api_peer_images >/tmp/mtgfr-api-peers.json 2>/dev/null; then
    cat /tmp/mtgfr-api-peers.json
  else
    echo "{}"
  fi
}

# terraform apply with operator images + script-owned peer map.
tf_apply_images() {
  local server="$1"
  local web="$2"
  local peers_json="$3"
  terraform apply -auto-approve \
    -var="server_image=$server" \
    -var="web_image=$web" \
    -var="api_peer_images=${peers_json}" \
    -var="api_max_instances=$MAX_INSTANCES"
}
