#!/usr/bin/env bash
# Ensure Node/npm matching .node-version are on PATH (image may preinstall them;
# otherwise bootstrap a user-local Node so root npm clean-install can run).
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
version_file="${repo_root}/.node-version"
want="$(tr -d '[:space:]' <"$version_file")"
want="${want#v}"

if command -v npm >/dev/null 2>&1 && command -v node >/dev/null 2>&1; then
  have="$(node -v | tr -d 'v')"
  if [[ "$have" == "$want"* ]] || [[ "$have" == "$want" ]]; then
    exit 0
  fi
fi

prefix="${HOME}/.local/node-v${want}"
if [[ ! -x "${prefix}/bin/npm" ]]; then
  mkdir -p "${prefix}"
  curl -fsSL "https://nodejs.org/dist/v${want}/node-v${want}-linux-x64.tar.xz" \
    | tar -xJ -C "${prefix}" --strip-components=1
fi

export PATH="${prefix}/bin:${PATH}"
# Persist for later steps in the same install script (sourced vs executed):
# environment.json runs this with `&&`, so also write an env file callers can source.
printf 'export PATH=%q\n' "${prefix}/bin:${PATH}" >"${HOME}/.cursor-node-env"
echo "ensure-node: using ${prefix} (node $(node -v), npm $(npm -v))"
