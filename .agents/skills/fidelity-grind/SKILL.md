---
name: fidelity-grind
description: Given an Archidekt deck link, make every card in that deck faithful — deck intake, fidelity report checklist, observability re-audit, pure-authoring pass, engine grind loop, client catch-up, full verify, ending in an open PR watched through CI and review to merge. Use when the user provides a deck URL and wants the pool to support it faithfully.
---

# Fidelity Grind

Turn an Archidekt deck into a fully faithful slice of the card pool, end to end. This skill
encodes the process that took the first 429-card pool from ~60% to 99.3% faithful (waves 1–142,
2026-07). Read `card-dsl` (authoring bar) and use the `tdd` skill throughout.

**Inputs:** an Archidekt deck URL (`https://archidekt.com/decks/<id>/<slug>`).
**Output:** every deck card scripted in `crates/cards/data/`, faithful or carrying a precise
`approximates` residual; engine + client green; an open PR against the default branch with
the wrap-up report as its body.

## Phase 0 — Setup (isolation)

- Work in a dedicated git worktree so the user's checkout stays untouched:
  `git worktree add ../mtgfr-grind-<slug> -b fidelity-<slug> master`
- Commit every green wave on that branch. **Never merge to master until the grind is done**
  and the user confirms. Periodically sync-merge master *into* the branch (delegate to an
  agent; full workspace tests both sides after resolving).
- All agent briefs must name the worktree root and forbid touching the main checkout.

## Phase 1 — Deck intake

- Deck id from the URL; fetch `https://archidekt.com/api/decks/<id>/` (public JSON).
  Each entry: `card.oracleCard.name`, quantity, categories. Ignore basic-land quantities;
  dedupe by name. Cross-check oracle text against Scryfall when authoring (Archidekt's
  `oracleCard.text` can lag) — the TOML `oracle` field must be current Scryfall text.
- Card identity is the TOML `name` field (not the filename). Match deck names against
  `grep -h '^name = ' crates/cards/data/*.toml`.

## Phase 2 — Fidelity report (the checklist)

Write `docs/fidelity/<slug>.md` — a checkbox per deck card, in four sections:

- **A. In pool, faithful** — no `approximates`. No work; check off immediately.
- **B. In pool, approximated** — quote each card's current note verbatim.
- **C. New, expressible today** — cards the current DSL can script with no engine change
  (judge against `DSL_REFERENCE.md` + `de.rs`; when unsure, mark D — flag-don't-force).
- **D. New, needs engine work** — for these, append ranked increments to
  `docs/FIDELITY_BACKLOG.md` in the existing format: numbered heading, effort (S/M/L/XL),
  `Depends on:` line, example cards, a *Sketch* of the intended design. XL increments get an
  explicit slice staging.

**Observability re-audit (do not skip):** many residuals are justified by pool absence —
notes saying "no pool card does X", "unobservable", "dead variant". Grep every
`approximates` and `ponytail:` in `crates/cards/data/` and `crates/engine/src/` for such
claims and re-test each against the incoming deck. Any claim the new cards falsify moves
that residual into section B/D as real work. (Example: "damage to planeswalkers is a dead
variant" died the moment a planeswalker entered the pool.)

## Phase 3 — Pure authoring pass

Author all of section C in batched waves (TDD: failing engine test in
`crates/engine/tests/game.rs` first, then the TOML). No engine edits allowed in this phase —
if a card turns out to need one, reclassify it to D and move on.

## Phase 4 — Engine grind loop

Run the wave loop until the planner declares done. Assets in this folder:

- [`wave-workflow.js`](wave-workflow.js) — the orchestration script (plan → implement
  sequentially on the shared tree → verify+reconcile). Copy it to scratch space, fill the
  `{{WORKTREE}}` / `{{BRANCH}}` / `{{BACKLOG_RANGE}}` tokens, and run it via the Workflow
  tool, relaunching after each green wave. Without a workflow orchestrator, run the same
  three stages as sequential subagent dispatches.
- [`shared-context-template.md`](shared-context-template.md) — the brief every wave agent
  reads first; fill the same tokens and keep it in scratch space next to the script.

Hard-won loop rules (already baked into the script — do not soften them):

- **Selection:** batch aggressively on count (up to 6 S/M or exotics per wave); an L may ride
  with up to 3 disjoint S/M. **While any XL remains unlanded, every wave must carry exactly
  one XL slice, placed last** — prefer finishing an in-progress XL over starting a new one.
  Left to a cheapest-first rule alone, XLs get deferred forever.
- **Eligibility:** an increment is eligible only if its deps are landed AND a real pool card
  becomes faithful or measurably closer (never add a dead variant). Verify against the TOMLs,
  not backlog prose — "still blocked" lists go stale as riders land cards.
- **Verify gate (every wave, opus, adversarial):** full `cargo test --workspace`,
  `cargo fmt --check`, `cargo clippy --all-targets` with zero NEW warnings vs a git-stash
  baseline; adversarial diff review against the CR; reconcile every touched card's
  `approximates` note against the actual diff; update backlog LANDED marks (XL slices get
  dated progress notes, not LANDED, until all slices land); regenerate `docs/CR_INDEX.md`
  (`just engine-cr-index`). Verify stages catch real rules bugs (~1 every 3 waves in
  practice) — never skip.
- **Commit per green wave**; on a red wave stop and surface it to the user.
- Consolidate before the XL tier if the codebase has grown fast (module splits, walker
  unification) — behavior-preserving only, full green bar.

## Phase 5 — Client catch-up

Engine waves accrue wire debt. After the grind (or mid-grind if large):
`just server-codegen`, then diff the regenerated wire types against the client registries —
every `PendingChoiceView` needs a form in `client/src/components/molecules/prompt-forms.tsx` (reuse an existing
form when the answer shape matches; the engine dispatches by pending-choice kind), every
`VisibleEvent` an arm in `client/src/store.ts` (effect/Match, exhaustive), and new
`MeaningfulAction`s surface via the existing generic tiles/radial. Gate:
`npx tsc --noEmit` clean + client tests green.

## Phase 6 — Final verify + PR

1. Re-run the deck checklist: every card checked, or carrying a precise residual note the
   user has seen. Every remaining `approximates` in the pool must name *why* (absent
   subsystem, unobservable, dead variant) — never a silently dropped ability.
2. Sync-merge the default branch into the grind branch one last time; full bar both sides
   (`just check`-equivalent: workspace tests, fmt, clippy-no-new, tsc, client tests).
3. **End with an open PR, not a direct merge:** push the branch and open it with
   `gh pr create` against the default branch. The PR body is the wrap-up report — the deck
   checklist summary (faithful counts before/after), engine capabilities landed, remaining
   residuals and their why, client surface added, and the test totals. The user reviews and
   merges. If the repo has no GitHub remote (yet), stop after the final verify and hand the
   user the merge command (usually `--ff-only`) instead — never merge into the default
   branch yourself.

## Phase 7 — PR watch (CI + review)

Stay on the PR until it merges; don't end the run at "PR opened".

- **CI:** `gh pr checks <num> --watch` (or poll `gh pr checks` on a timer — CI runs take
  minutes, so poll at ~4-minute intervals, not tight loops). On a failure, pull the log
  (`gh run view <id> --log-failed`), fix on the grind branch with the usual bar (regression
  test for any real bug), push, and watch again. If a failure is infrastructure flake or
  pre-existing on the default branch, say so on the PR instead of chasing it.
- **Comments:** poll review comments and threads (`gh pr view <num> --comments` and
  `gh api repos/{owner}/{repo}/pulls/<num>/comments`) for anything newer than your last
  reply. Address each: code change + push for real issues, or a short factual reply
  (`gh pr comment` / replying on the thread) when no change is warranted — never leave a
  comment unanswered or resolve a thread without responding. Requested changes reopen
  Phase 4/5 rules (TDD, verify gate, conventions).
- Ping the user only when CI is green and all threads are addressed, when a review asks
  for something out of scope (a design decision), or when the same check fails twice with
  no fix in sight. Remove the worktree after the PR merges.

## Conventions (enforced in every brief and verify stage)

- No faithful-asserting comments — silence means faithful; only `approximates` +
  `# ponytail:` gap notes.
- Every card file opens with the bare verbatim oracle text (no `Oracle:` prefix); every
  `[[abilities]]` and `[[abilities.effects]]` block (incl. `[[back.*]]`) carries the oracle
  sentence/clause it implements, quoted directly above. Comments wrap at 120 chars.
- Effects always use the `[[abilities.effects]]` array form.
- Any TOML-surface change updates `DSL_REFERENCE.md` in the same change.
- `CardDef: Copy` — `&'static` leaked slices, never `Vec`/`String` fields.
- Every bug fix gets a regression test at the lowest layer that catches it.
