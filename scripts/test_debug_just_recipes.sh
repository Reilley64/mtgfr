#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
marker="$tmp/injected"
old_marker="$root/should-not-run"
rm -rf "$old_marker"
trap 'rm -rf "$tmp" "$old_marker"' EXIT
mkdir "$tmp/bin"
cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
printf '%s\0' "$@" >"$DEBUG_JUST_ARGS"
EOF
chmod +x "$tmp/bin/cargo"
export PATH="$tmp/bin:$PATH" DEBUG_JUST_ARGS="$tmp/args"

assert_clean() {
    test ! -e "$marker"
    test ! -e "$old_marker"
}

assert_args() {
    python3 - "$DEBUG_JUST_ARGS" "$@" <<'PY'
import pathlib, sys
actual=pathlib.Path(sys.argv[1]).read_bytes().split(b'\0')[:-1]
expected=[arg.encode() for arg in sys.argv[2:]]
assert actual == expected, (actual,expected)
PY
    assert_clean
}

prefix=(run -p server --bin mtgfr-debug --)
# Construct the command substitution as data: do not let this test's shell evaluate it.
printf -v payload '$(touch %s)' "$marker"
printf -v table 'table space "quote"; semi; %s' "$payload"
printf -v name 'base space "quote"; semi; %s' "$payload"
printf -v out '%s/output space "quote"; semi; %s.json' "$tmp" "$payload"
printf -v endpoint 'http://localhost/space "quote"; semi; %s' "$payload"
printf -v request '%s/request space "quote"; semi; %s.json' "$tmp" "$payload"
printf -v replace 'replace space "quote"; semi; %s' "$payload"
printf -v expected_table_seq '42 space "quote"; semi; %s' "$payload"
printf -v expected_debug_revision '7 space "quote"; semi; %s' "$payload"

# Tables: default endpoint omission and populated endpoint preservation.
(cd "$root" && just --quiet debug-tables)
assert_args "${prefix[@]}" tables
(cd "$root" && just --quiet debug-tables "$endpoint")
assert_args "${prefix[@]}" tables --endpoint "$endpoint"

# Inspect: default optionals and both populated optionals.
(cd "$root" && just --quiet debug-inspect "$table")
assert_args "${prefix[@]}" inspect "$table"
(cd "$root" && just --quiet debug-inspect "$table" "$out" "$endpoint")
assert_args "${prefix[@]}" inspect "$table" --out "$out" --endpoint "$endpoint"

# Mutate: defaults, the original three-position request/out/endpoint form, and the new guard.
(cd "$root" && just --quiet debug-mutate "$request")
assert_args "${prefix[@]}" mutate "$request"
(cd "$root" && just --quiet debug-mutate "$request" "$out" "$endpoint")
assert_args "${prefix[@]}" mutate "$request" --out "$out" --endpoint "$endpoint"
(cd "$root" && just --quiet debug-mutate "$request" "$out" "$endpoint" "$expected_table_seq")
assert_args "${prefix[@]}" mutate "$request" --out "$out" --endpoint "$endpoint" --expected-table-seq "$expected_table_seq"

# Checkpoint: empty replace omits --replace; any nonempty value adds it.
(cd "$root" && just --quiet debug-checkpoint "$table" "$name")
assert_args "${prefix[@]}" checkpoint "$table" "$name"
(cd "$root" && just --quiet debug-checkpoint "$table" "$name" "$replace" "$expected_table_seq" "$out" "$endpoint")
assert_args "${prefix[@]}" checkpoint "$table" "$name" --replace --expected-table-seq "$expected_table_seq" --out "$out" --endpoint "$endpoint"

# Restore: exercise the independent presence/absence matrix for both guards.
(cd "$root" && just --quiet debug-restore "$table" "$name")
assert_args "${prefix[@]}" restore "$table" "$name"
(cd "$root" && just --quiet debug-restore "$table" "$name" "$expected_debug_revision")
assert_args "${prefix[@]}" restore "$table" "$name" --expected-debug-revision "$expected_debug_revision"
(cd "$root" && just --quiet debug-restore "$table" "$name" '' "$expected_table_seq")
assert_args "${prefix[@]}" restore "$table" "$name" --expected-table-seq "$expected_table_seq"
(cd "$root" && just --quiet debug-restore "$table" "$name" "$expected_debug_revision" "$expected_table_seq" "$out" "$endpoint")
assert_args "${prefix[@]}" restore "$table" "$name" --expected-debug-revision "$expected_debug_revision" --expected-table-seq "$expected_table_seq" --out "$out" --endpoint "$endpoint"

# Checked stack fixture: default endpoint omission and populated endpoint preservation.
(cd "$root" && just --quiet debug-stack-seven "$table")
assert_args "${prefix[@]}" stack-fixture "$table"
(cd "$root" && just --quiet debug-stack-seven "$table" "$endpoint")
assert_args "${prefix[@]}" stack-fixture "$table" --endpoint "$endpoint"

# Journal: default and populated optionals.
(cd "$root" && just --quiet debug-journal "$table")
assert_args "${prefix[@]}" journal "$table"
(cd "$root" && just --quiet debug-journal "$table" "$out" "$endpoint")
assert_args "${prefix[@]}" journal "$table" --out "$out" --endpoint "$endpoint"

assert_clean
