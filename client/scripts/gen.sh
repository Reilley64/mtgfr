#!/usr/bin/env sh
# Regenerate Effect-gRPC + protobuf-es clients from `proto/mtgfr/v1/mtgfr.proto`.
# into `client/src/wire/generated/` (consumed by `~/wire/grpcClient`).
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

for GEN in src/wire/generated/mtgfr/v1/*_effect_grpc.ts; do
  [ -f "$GEN" ] || continue

  # protoc-gen-effect-grpc emits a value import for `GrpcMethodRegistry`, but with
  # `verbatimModuleSyntax` it is type-only — flip it so `tsc` accepts the generated file. Only
  # `mtgfr_effect_grpc.ts` defines the `*GrpcRegistry` maps today, but the sed is harmless (a
  # no-op) on files where the pattern doesn't match, so it's safe to run over all of them.
  sed -i.bak \
    -e 's/import { CodegenSupport, GrpcMethodRegistry, GrpcStatusError } from "@effect-grpc\/effect-grpc";/import { CodegenSupport, GrpcStatusError } from "@effect-grpc\/effect-grpc";\
import type { GrpcMethodRegistry } from "@effect-grpc\/effect-grpc";/' \
    "$GEN"

  # protoc-gen-effect-grpc also imports every cross-file message type it references (e.g.
  # `type WireCost`) alongside that message's schema/from/to helpers, even in files that only
  # ever use the schema (`WireCostSchema`) — never the bare type. `noUnusedLocals` fails the
  # build on those; `@ts-nocheck` is the least brittle fix for fully generated, DO-NOT-EDIT
  # output (a per-import allowlist would silently rot as the proto grows). It only suppresses
  # errors *inside* this file — consumers still get real types from its exports.
  if ! grep -q "^// @ts-nocheck" "$GEN"; then
    printf '// @ts-nocheck -- generated: some cross-file message-type imports are unused (see gen.sh).\n' | cat - "$GEN" > "$GEN.tmp"
    mv "$GEN.tmp" "$GEN"
  fi

  rm -f "$GEN.bak"
done

echo "gen.sh: regenerated client/src/wire/generated from proto."
