#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
generated="$root/client/app/domain/wire/generated"
pattern='debug_service|DebugService|mtgfr\.debug\.v1'

(cd "$root/client" && bun run gen:wire)

if [[ ! -d "$generated" ]]; then
  printf 'browser wire generation did not create %s\n' "$generated" >&2
  exit 1
fi

path_list=$(mktemp)
trap 'rm -f "$path_list"' EXIT
if ! find "$generated" -type f -print >"$path_list"; then
  printf 'failed to enumerate browser protobuf output under %s\n' "$generated" >&2
  exit 1
fi

if grep -E "$pattern" "$path_list"; then
  printf 'debug protobuf generated file paths leaked into browser output\n' >&2
  exit 1
else
  status=$?
fi
if [[ "$status" -ne 1 ]]; then
  printf 'failed to inspect browser protobuf generated file paths\n' >&2
  exit 1
fi

if grep -REn "$pattern" "$generated"; then
  printf 'debug protobuf symbols leaked into browser output\n' >&2
  exit 1
else
  status=$?
fi
if [[ "$status" -ne 1 ]]; then
  printf 'failed to inspect browser protobuf symbols under %s\n' "$generated" >&2
  exit 1
fi
