#!/usr/bin/env sh
# Sync the vendored Card-Forge/forge script tree at .repos/forge from upstream.
# Sparse paths: cardsfolder + tokenscripts only. No nested .git — committed
# into mtgfr like .repos/effect.

set -eu

repo_dir=".repos/forge"
repo_url="https://github.com/Card-Forge/forge.git"
sparse_paths="forge-gui/res/cardsfolder forge-gui/res/tokenscripts"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/mtgfr-forge.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

git clone --depth 1 --filter=blob:none --sparse "$repo_url" "$tmp_dir/forge"
# shellcheck disable=SC2086
git -C "$tmp_dir/forge" sparse-checkout set $sparse_paths

upstream_sha="$(git -C "$tmp_dir/forge" rev-parse HEAD)"
printf '%s\n' "$upstream_sha" >"$tmp_dir/forge/VENDOR_REVISION"

# Cone sparse-checkout also materializes parent junk (pom, icons, …). Keep
# scripts + LICENSE(+README) + revision stamp only.
keep_root="LICENSE README.md VENDOR_REVISION"
for path in "$tmp_dir/forge"/* "$tmp_dir/forge"/.[!.]*; do
  [ -e "$path" ] || continue
  base="$(basename "$path")"
  case " $keep_root " in
    *" $base "*) continue ;;
  esac
  if [ "$base" = "forge-gui" ]; then
    continue
  fi
  rm -rf "$path"
done
# Drop forge-gui everything except res/cardsfolder + res/tokenscripts.
if [ -d "$tmp_dir/forge/forge-gui" ]; then
  for path in "$tmp_dir/forge/forge-gui"/* "$tmp_dir/forge/forge-gui"/.[!.]*; do
    [ -e "$path" ] || continue
    base="$(basename "$path")"
    if [ "$base" = "res" ]; then
      continue
    fi
    rm -rf "$path"
  done
  for path in "$tmp_dir/forge/forge-gui/res"/*; do
    [ -e "$path" ] || continue
    base="$(basename "$path")"
    case "$base" in
      cardsfolder|tokenscripts) continue ;;
      *) rm -rf "$path" ;;
    esac
  done
fi

rm -rf "$tmp_dir/forge/.git"

mkdir -p ".repos"
# Atomic-ish replace: promote new tree, then remove the previous one.
rm -rf "${repo_dir}.new"
mv "$tmp_dir/forge" "${repo_dir}.new"
rm -rf "${repo_dir}.old"
if [ -e "$repo_dir" ]; then
  mv "$repo_dir" "${repo_dir}.old"
fi
mv "${repo_dir}.new" "$repo_dir"
rm -rf "${repo_dir}.old"

echo "forge: vendored tree ready at $repo_dir (cardsfolder + tokenscripts @ $upstream_sha)."
