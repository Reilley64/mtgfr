#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
real_find=$(command -v find)

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

make_fake_cargo() {
  local bin_dir="$1"
  mkdir -p "$bin_dir"
  cat >"$bin_dir/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$CARGO_TARGET_DIR" >>"$FAKE_CARGO_TARGET_LOG"
if [[ " $* " == *" --release "* ]]; then
  mkdir -p "$CARGO_TARGET_DIR/release/build/server-fixture/out"
  printf 'ordinary release server\n' >"$CARGO_TARGET_DIR/release/server"
  printf 'ordinary production descriptor\n' >"$CARGO_TARGET_DIR/release/build/server-fixture/out/mtgfr_descriptor.bin"
else
  mkdir -p "$CARGO_TARGET_DIR/debug"
  printf '%s\n' \
    MTGFR_DEBUG_IMPLEMENTATION_MARKER_V1 \
    mtgfr.debug.v1.DebugService \
    /mtgfr.debug.v1.DebugService/ >"$CARGO_TARGET_DIR/debug/server"
fi
sleep 0.1
EOF
  chmod +x "$bin_dir/cargo"
}

# The release gate must resolve its checkout through an external symlink, give
# concurrent runs distinct target directories, and remove both on exit.
release_bin="$tmp/release-bin"
make_fake_cargo "$release_bin"
external="$tmp/external"
mkdir -p "$external"
ln -s "$root/scripts/check_debug_release_absence.sh" "$external/check-release"
export FAKE_CARGO_TARGET_LOG="$tmp/cargo-targets"
PATH="$release_bin:$PATH" "$external/check-release" >"$tmp/release-1.log" 2>&1 &
pid_one=$!
PATH="$release_bin:$PATH" "$external/check-release" >"$tmp/release-2.log" 2>&1 &
pid_two=$!
wait "$pid_one" || { cat "$tmp/release-1.log" >&2; fail 'first controlled release gate failed'; }
wait "$pid_two" || { cat "$tmp/release-2.log" >&2; fail 'second controlled release gate failed'; }
python3 - "$root" "$FAKE_CARGO_TARGET_LOG" <<'PY'
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
targets = [pathlib.Path(line) for line in pathlib.Path(sys.argv[2]).read_text().splitlines()]
assert len(targets) == 4, targets
run_roots = {target.parent for target in targets}
assert len(run_roots) == 2, targets
for run_root in run_roots:
    assert run_root.parent == root / "target", (run_root, root)
    assert run_root.name.startswith("debug-release-isolation."), run_root
    assert not run_root.exists(), run_root
PY

# A descriptor traversal that emits a plausible result and then errors must fail.
find_bin="$tmp/find-bin"
make_fake_cargo "$find_bin"
cat >"$find_bin/find" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
"$REAL_FIND" "$@"
exit 2
EOF
chmod +x "$find_bin/find"
: >"$tmp/find-error-targets"
if REAL_FIND="$real_find" FAKE_CARGO_TARGET_LOG="$tmp/find-error-targets" \
  PATH="$find_bin:$PATH" "$root/scripts/check_debug_release_absence.sh" >"$tmp/find-error.log" 2>&1; then
  fail 'release gate accepted a partial find traversal error'
fi

# The controlled-leak hook remains additive and must still reject its artifact.
printf 'MTGFR_DEBUG_IMPLEMENTATION_MARKER_V1\n' >"$tmp/controlled-leak"
: >"$tmp/hook-targets"
if MTGFR_EXTRA_RELEASE_BINARY="$tmp/controlled-leak" \
  FAKE_CARGO_TARGET_LOG="$tmp/hook-targets" PATH="$release_bin:$PATH" \
  "$root/scripts/check_debug_release_absence.sh" >"$tmp/hook.log" 2>&1; then
  fail 'release gate accepted its controlled leak test artifact'
fi

# Exercise the TypeScript exclusion gate in a disposable repository fixture.
ts_root="$tmp/ts-repo"
mkdir -p "$ts_root/scripts" "$ts_root/client/app/domain/wire/generated" "$tmp/ts-bin"
cp "$root/scripts/check_debug_ts_exclusion.sh" "$ts_root/scripts/"
cat >"$tmp/ts-bin/bun" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
chmod +x "$tmp/ts-bin/bun"
printf 'export const ordinary = true\n' >"$ts_root/client/app/domain/wire/generated/game.ts"
PATH="$tmp/ts-bin:$PATH" "$ts_root/scripts/check_debug_ts_exclusion.sh"
printf 'export const ordinary = true\n' >"$ts_root/client/app/domain/wire/generated/debug_service.ts"
if PATH="$tmp/ts-bin:$PATH" "$ts_root/scripts/check_debug_ts_exclusion.sh" >"$tmp/ts-path-match.log" 2>&1; then
  fail 'TypeScript exclusion gate accepted a debug generated path match'
fi
rm "$ts_root/client/app/domain/wire/generated/debug_service.ts"
printf 'export const DebugService = true\n' >"$ts_root/client/app/domain/wire/generated/game.ts"
if PATH="$tmp/ts-bin:$PATH" "$ts_root/scripts/check_debug_ts_exclusion.sh" >"$tmp/ts-symbol-match.log" 2>&1; then
  fail 'TypeScript exclusion gate accepted a debug symbol match'
fi
printf 'export const ordinary = true\n' >"$ts_root/client/app/domain/wire/generated/game.ts"
rm -rf "$ts_root/client/app/domain/wire/generated"
if PATH="$tmp/ts-bin:$PATH" "$ts_root/scripts/check_debug_ts_exclusion.sh" >"$tmp/ts-missing.log" 2>&1; then
  fail 'TypeScript exclusion gate accepted a missing generated root'
fi
mkdir -p "$ts_root/client/app/domain/wire/generated"
printf 'export const ordinary = true\n' >"$ts_root/client/app/domain/wire/generated/game.ts"
cat >"$tmp/ts-bin/find" <<'EOF'
#!/usr/bin/env bash
exit 2
EOF
chmod +x "$tmp/ts-bin/find"
if PATH="$tmp/ts-bin:$PATH" "$ts_root/scripts/check_debug_ts_exclusion.sh" >"$tmp/ts-find-error.log" 2>&1; then
  fail 'TypeScript exclusion gate accepted find status 2'
fi
rm "$tmp/ts-bin/find"
cat >"$tmp/ts-bin/grep" <<'EOF'
#!/usr/bin/env bash
exit 2
EOF
chmod +x "$tmp/ts-bin/grep"
if PATH="$tmp/ts-bin:$PATH" "$ts_root/scripts/check_debug_ts_exclusion.sh" >"$tmp/ts-grep-error.log" 2>&1; then
  fail 'TypeScript exclusion gate accepted grep status 2'
fi

printf 'debug shell gate regressions passed\n'
