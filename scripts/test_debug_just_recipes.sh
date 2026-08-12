#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/bin"
cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
printf '%s\0' "$@" >"$DEBUG_JUST_ARGS"
EOF
chmod +x "$tmp/bin/cargo"
export PATH="$tmp/bin:$PATH" DEBUG_JUST_ARGS="$tmp/args"
marker="$tmp/injected"
table="table\"; touch $marker; #"
out="$tmp/output path; still one arg.json"
(cd "$root" && just --quiet debug-inspect "$table" "$out")
python3 - "$DEBUG_JUST_ARGS" "$table" "$out" <<'PY'
import pathlib, sys
actual=pathlib.Path(sys.argv[1]).read_bytes().split(b'\0')[:-1]
expected=[b'run',b'-p',b'server',b'--bin',b'mtgfr-debug',b'--',b'inspect',sys.argv[2].encode(),b'--out',sys.argv[3].encode()]
assert actual == expected, (actual,expected)
PY
test ! -e "$marker"
request="$tmp/request path; \$(touch $marker).json"
endpoint='http://localhost/?x=$(touch should-not-run)'
(cd "$root" && just --quiet debug-mutate "$request" '' "$endpoint")
python3 - "$DEBUG_JUST_ARGS" "$request" "$endpoint" <<'PY'
import pathlib, sys
actual=pathlib.Path(sys.argv[1]).read_bytes().split(b'\0')[:-1]
expected=[b'run',b'-p',b'server',b'--bin',b'mtgfr-debug',b'--',b'mutate',sys.argv[2].encode(),b'--endpoint',sys.argv[3].encode()]
assert actual == expected, (actual,expected)
PY
test ! -e "$marker"
