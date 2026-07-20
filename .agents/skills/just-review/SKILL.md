---
name: just-review
description: >-
  Inspect a pull request (or the current branch vs main) and automatically choose
  which review(s) to run — Bugbot, Security Review, and/or PR review canvas —
  based on changed paths and change kind. Use when the user says "just-review",
  "/just-review", "review this PR", "review the PR", or wants review routing
  without manually choosing Bugbot vs Security.
disable-model-invocation: true
---

# Just review

Decide which review skill(s) to run from PR contents, then run them. Do **not** ask the user to pick Bugbot vs Security unless signals are tied or the user already named a specific review.

## 1. Resolve the target

1. If the user gave a PR URL or number, use that.
2. Else use the open PR for the current branch: `gh pr view --json number,url,title,files,additions,deletions,changedFiles,baseRefName,headRefName`.
3. If there is no PR yet, treat `origin/<base>...HEAD` (default base `main`) as the review target.

Gather a **file list with status** (path + added/removed/modified):

```bash
gh pr diff <n> --name-only
# or
git diff --name-status origin/main...HEAD
```

Optionally skim `gh pr diff <n> --patch` / `git diff origin/main...HEAD` only enough to confirm signals (auth strings, visibility filters) — do not dump the whole diff into context.

If the user pointed at a specific PR/branch and Bugbot or Security will run, follow the checkout rules in those skills (check out the head branch before launching those subagents).

## 2. Score signals

Compute boolean signals from **paths and light content cues** (file names + a few greps over the diff). Paths win over vibes.

### Security (run Security Review when **any** strong signal, or **≥2** weak)

**Strong**

- Auth / session / cookies / passwords / tokens / JWT / OAuth
- Visibility / private hand / library order / spectator / per-player filter
- Permission / authorize / ACL / CSRF
- Crypto / secret / credential / `.env` / vault
- Wire identity or trust: gRPC metadata session headers, BFF cookie→metadata bridging

**Weak (path heuristics)**

- `**/auth/**`, `**/session**`, `**/*password*`, `**/*credential*`
- Server request guards, middleware, `Reject::`, intent validation at trust boundary
- Proto/schema changes that add fields carrying player-private or auth material
- Client stores that decide what another seat can see

### Correctness / Bugbot (default for real code)

Run Bugbot when **any** of:

- Engine, server, schema, cards, or client **logic** changes (`.rs`, `.ts`, `.tsx` excluding pure docs/generated lockfiles)
- Proto / wire contract changes
- Tests that encode behavior

Skip Bugbot only when the PR is **docs-only**, **chore-only** (lockfiles, formatting, CI yaml with no app code), or the user explicitly asked for canvas/security alone.

### PR review canvas (human walkthrough)

Run canvas when **any** of:

- User asked for a walkthrough, summary, or “review this PR” **with a GitHub URL** and did not ask for Bugbot/Security by name
- Large surface: `changedFiles >= 25` **or** `(additions + deletions) >= 800` of non-generated code
- Mixed domains in one PR (e.g. engine + client + proto) where a guided tour helps before deep review

Canvas is **additive** for large/mixed PRs: still run Bugbot (and Security if signaled) unless the user only wanted a walkthrough.

### Do not auto-run

- `/ponytail-review`, `/thermo-nuclear-code-quality-review` — only if the user asked for those modes
- `/review-and-ship` — shipping/fixing is a different intent

## 3. Decide the plan

Build an ordered list:

1. **Canvas** first (if selected) — orients the human; cheap relative to subagents when they asked for a walkthrough.
2. **Bugbot** (if selected).
3. **Security** (if selected).

Announce in one short line before running, e.g.:

> Routing: Bugbot + Security (session cookie → gRPC metadata; visibility paths). Skipping canvas (14 files, focused client/engine).

If **no** signals match (empty or docs-only), say so and offer canvas summary or stop — do not invent a Bugbot run on markdown-only PRs.

## 4. Execute

Follow the existing skills **verbatim** for each selected review:

| Selected | Follow |
|----------|--------|
| Bugbot | `review-bugbot` skill (Task `subagent_type: "bugbot"`, fixed prompt shape) |
| Security | `review-security` skill (Task `subagent_type: "security-review"`, fixed prompt shape) |
| Canvas | `pr-review-canvas` skill (gh API + HTML walkthrough) |

Run Bugbot and Security **sequentially** (each skill says launch exactly one of that type; do not parallelize two of the same). Canvas may run before them.

Default Diff for Bugbot/Security: `branch changes`. Use `uncommitted changes` only if the user said local/uncommitted/dirty.

## 5. Report

After selected reviews finish:

1. Restate what was routed and why (paths/signals, not a novel).
2. Merge findings: critical → warnings → notes; mark which reviewer produced each.
3. If Security was skipped but a weak signal existed, mention it once so the user can request `/review-security`.

## Examples

- Client animation + ADR + `TokenCreated.creator` on proto → **Bugbot**; Security only if visibility/auth touched (usually not).
- BFF cookie / `x-session-token` / hand privacy filtering → **Bugbot + Security**.
- 40-file grind with engine + cards + client → **Canvas + Bugbot**.
- Docs/ADR only → **no Bugbot**; optional Canvas or stop.
