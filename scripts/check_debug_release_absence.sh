#!/usr/bin/env bash
set -euo pipefail

script_path="$(python3 - "$0" <<'PY'
import pathlib
import sys

print(pathlib.Path(sys.argv[1]).resolve())
PY
)"
root="$(cd "$(dirname "$script_path")/.." && pwd)"
mkdir -p "$root/target"
run_target="$(mktemp -d "$root/target/debug-release-isolation.XXXXXX")"
debug_target="$run_target/debug"
release_target="$run_target/release"
cleanup() {
  rm -rf "$run_target"
}
trap cleanup EXIT
debug_binary="$debug_target/debug/server"
release_binary="$release_target/release/server"
needles=(
  'MTGFR_DEBUG_IMPLEMENTATION_MARKER_V1'
  'MTGFR_DEBUG_STACK_EDITOR_MARKER_V1'
  'mtgfr.debug.v1.DebugService'
  '/mtgfr.debug.v1.DebugService/'
)

require_bytes() {
  local file="$1"
  local needle="$2"
  local status

  if grep -aFq -- "$needle" "$file"; then
    printf 'debug control contains expected bytes: %s\n' "$needle"
    return 0
  else
    status=$?
  fi

  if [[ "$status" -eq 1 ]]; then
    printf 'debug control is missing expected bytes: %s\n' "$needle" >&2
  else
    printf 'failed to inspect debug control %s for: %s\n' "$file" "$needle" >&2
  fi
  return 1
}

reject_bytes() {
  local file="$1"
  local label="$2"
  local needle="$3"
  local status

  if grep -aFq -- "$needle" "$file"; then
    printf '%s contains forbidden debug bytes: %s\n' "$label" "$needle" >&2
    return 1
  else
    status=$?
  fi

  if [[ "$status" -ne 1 ]]; then
    printf 'failed to inspect %s %s for: %s\n' "$label" "$file" "$needle" >&2
    return "$status"
  fi
}

CARGO_TARGET_DIR="$debug_target" cargo build -p server --bin server
for needle in "${needles[@]}"; do
  require_bytes "$debug_binary" "$needle"
done

CARGO_TARGET_DIR="$release_target" cargo build -p server --release --bin server
for needle in "${needles[@]}"; do
  reject_bytes "$release_binary" 'release server' "$needle"
done

# Test hook: inspect an additional artifact without replacing or skipping the exact
# release binary above. This is used to prove the gate rejects a controlled leak.
if [[ -n "${MTGFR_EXTRA_RELEASE_BINARY:-}" ]]; then
  for needle in "${needles[@]}"; do
    reject_bytes "$MTGFR_EXTRA_RELEASE_BINARY" 'additional release test artifact' "$needle"
  done
fi

descriptor_list="$run_target/production-descriptors"
if ! find "$release_target/release/build" -type f -path '*/out/mtgfr_descriptor.bin' -print0 >"$descriptor_list"; then
  printf 'failed to enumerate production descriptors under %s\n' "$release_target/release/build" >&2
  exit 1
fi

descriptor_count=0
while IFS= read -r -d '' descriptor; do
  descriptor_count=$((descriptor_count + 1))
  reject_bytes "$descriptor" 'production descriptor' 'mtgfr.debug.v1'
done <"$descriptor_list"

if [[ "$descriptor_count" -ne 1 ]]; then
  printf 'expected exactly one production descriptor, inspected %d\n' "$descriptor_count" >&2
  exit 1
fi

# Docker uses this opt-in output so the runtime receives the exact binary scanned above.
if [[ -n "${MTGFR_RELEASE_BINARY_OUTPUT:-}" ]]; then
  install -m 0755 "$release_binary" "$MTGFR_RELEASE_BINARY_OUTPUT"
fi

printf 'release isolation passed: one server binary and one production descriptor contain no debug API bytes\n'
