#!/usr/bin/env sh
# Sync the vendored Card-Forge/forge script tree at .repos/forge from upstream.
# Sparse paths only (cardsfolder + tokenscripts + effects). No nested .git —
# files are committed into mtgfr like .repos/effect.

set -eu

repo_dir=".repos/forge"
repo_url="https://github.com/Card-Forge/forge.git"
sparse_paths="forge-gui/res/cardsfolder forge-gui/res/tokenscripts forge-gui/res/effects"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/mtgfr-forge.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

git clone --depth 1 --filter=blob:none --sparse "$repo_url" "$tmp_dir/forge"
# shellcheck disable=SC2086
git -C "$tmp_dir/forge" sparse-checkout set $sparse_paths
rm -rf "$tmp_dir/forge/.git"

mkdir -p ".repos"
rm -rf "$repo_dir"
mv "$tmp_dir/forge" "$repo_dir"

echo "forge: vendored tree ready at $repo_dir (cardsfolder + tokenscripts + effects)."
