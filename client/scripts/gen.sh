#!/usr/bin/env sh
# Regenerate Effect-gRPC + protobuf-es clients from `proto/` into gitignored
# `client/src/wire/generated/`. Biome excludes that tree; tsc does not need patches —
# unused/import-type enforcement for hand-written code lives in biome.json.
set -e
cd "$(dirname "$0")/.."
PROTO_DIR=../proto
if [ ! -f "$PROTO_DIR/mtgfr/v1/mtgfr.proto" ]; then
  echo "gen.sh: $PROTO_DIR/mtgfr/v1/mtgfr.proto not found" >&2
  exit 1
fi
PATH="$(pwd)/node_modules/.bin:$PATH"
export PATH
(cd "$PROTO_DIR" && buf generate)
echo "gen.sh: regenerated client/src/wire/generated from proto."
