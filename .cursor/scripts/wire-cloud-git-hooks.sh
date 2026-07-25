#!/usr/bin/env bash
# After `npm clean-install` / husky prepare, restore Cursor Cloud agent hooksPath and
# chain repo Husky hooks through the dispatcher so commit-msg still runs commitlint.
#
# Cursor sets core.hooksPath to ~/.cursor/agent-hooks/<id> and runs
# $ORIGINAL_HOOKS_PATH/$HOOK_NAME before its own *.cursor hooks. Husky's prepare
# overwrites core.hooksPath to .husky/_ — this script puts Cursor back in front
# and points ORIGINAL_HOOKS_PATH at .husky.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

agent_hooks=""
for d in "${HOME}/.cursor/agent-hooks"/*/; do
  if [[ -x "${d}.dispatcher" ]]; then
    agent_hooks="${d%/}"
    break
  fi
done

if [[ -z "$agent_hooks" ]]; then
  # Non-cloud (or hooks not installed yet): leave Husky's core.hooksPath alone.
  exit 0
fi

if [[ ! -f .husky/commit-msg ]]; then
  echo "wire-cloud-git-hooks: missing .husky/commit-msg" >&2
  exit 1
fi

chmod +x .husky/commit-msg

printf '%s\n' "${repo_root}/.husky" >"${agent_hooks}/.cursor-original-hooks-path"
git config core.hooksPath "${agent_hooks}"

echo "wire-cloud-git-hooks: Cursor hooks at ${agent_hooks}; original → ${repo_root}/.husky"
