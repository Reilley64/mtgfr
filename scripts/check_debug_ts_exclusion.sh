#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
generated="$root/client/app/domain/wire/generated"
pattern='debug_service|DebugService|mtgfr\.debug\.v1'

(cd "$root/client" && bun run gen:wire)

if find "$generated" -type f -print | grep -E "$pattern"; then
  echo "debug protobuf generated file paths leaked into browser output" >&2
  exit 1
fi

if grep -REn "$pattern" "$generated"; then
  echo "debug protobuf symbols leaked into browser output" >&2
  exit 1
fi
