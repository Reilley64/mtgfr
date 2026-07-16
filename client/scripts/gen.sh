#!/usr/bin/env sh
# Regenerate the Effect wire client from the Rust schema. Run via `bun run gen` (also wired as
# predev/prebuild/pretest), which puts node_modules/.bin on PATH for `openapigen`.
#
# 1. Emit the OpenAPI contract from the `schema` crate (no DB needed) into the gitignored
#    ../openapi.json. 2. Generate a type-only Effect client from it (plain types, cast-not-validated
#    — matches the app's "decode JSON and trust" philosophy; keeps typed `MtgfrError` for declared
#    error bodies and the SSE `streamSse`). 3. Patch generator output: strip `readonly` for consumer
#    ergonomics, and fix two type-only quirks (a value used under `import type`; an `unknown` SSE
#    stream element).
set -e
mkdir -p src/api
cargo run -q -p server -- openapi > ../openapi.json
openapigen -s ../openapi.json -f httpclient-type-only -n Mtgfr --log-level error \
  | sed -e 's/readonly //g' \
        -e 's/ReadonlyArray/Array/g' \
        -e 's/import type \* as HttpClient/import * as HttpClient/' \
        -e 's/Stream\.Stream<unknown,/Stream.Stream<any,/' \
  > src/api/generated.ts
