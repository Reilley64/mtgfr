---
name: fidelity-grind
description: Given an Archidekt deck link, make every card in that deck faithful — deck intake, fidelity report checklist, observability re-audit, pure-authoring pass, engine grind loop, client catch-up, full verify, an open PR watched through CI and review to merge, then a skill retrospective folding the grind's lessons back into this skill. Use when the user provides a deck URL and wants the pool to support it faithfully.
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
  agent; full workspace tests both sides after resolving). If the default branch was
  force-pushed with rewritten history mid-grind ("refusing to merge unrelated histories"),
  graft the old root under the new one (`git replace --graft <new-root> <old-root>`), do the
  normal 3-way merge, then `git replace -d <new-root>` — one real conflict instead of
  hundreds of add/add ones.
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

**Frame audit (mandatory, after every authoring or grind wave that adds cards):** agents
hallucinate frames — the first grind shipped 8/66 cards with wrong mana costs, P/T, or
phantom keywords (Rubinia herself was {2}{W}{U}{U} 2/4 *with flying*), invisible to
ability-level tests but fatal to deck legality. Script a diff of every new card's mana cost,
P/T, type, legendary flag, and verbatim `oracle` field against a fresh Scryfall
`cards/collection` fetch; fix every mismatch (some are behavioral — a wrong activation cost
or counter count changes play) and keep the two-sided check until it reports zero.

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
  not backlog prose — "still blocked" lists go stale as riders land cards. A deck card not
  yet on disk is NOT a reason to skip: the increment authors that card (TOML + tests) as its
  own work — "card absent from the pool" never disqualifies an increment in a deck grind.
- **XL completion:** once every staged slice of an XL is built and only a documented,
  deliberate flag-don't-force residual remains, mark it LANDED with the residual named in the
  heading — an XL left "note-only" forever makes every later planner waste its mandatory XL
  slot re-confirming it.
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

## Phase 5.5 — Ship the deck as a precon

Every grind deck ends as a read-only in-app precon, so players get the deck, not just the
pool that supports it. After client catch-up (the wire is settled by then):

- Write `decklists/<snake_slug>.md` — the frozen target list (commander + grouped tables,
  totals summing to 99), sourced from the Archidekt fetch.
- Generate `crates/server/fixtures/decks/<snake_slug>.json` from that list: `commander` /
  `commander_print` (the commander card's `id` / `default_print` from its TOML) and one
  `{id, count, print}` entry per non-commander card (basics carry their count), mapped
  through the pool TOMLs. Do not use `tooling/rewrite-precon-fixtures.mjs` for non-soc
  decks — it prefers `soc` prints; the TOMLs' `default_print` is already right.
- Register it in `crates/server/src/precons.rs`: one `Source` entry with the **next
  negative id** (grow the `SOURCES` array length), name it after the deck.
- Add the fixture to `FIXTURES` and an `<slug>_is_a_legal_commander_deck` acceptance test
  in `crates/server/src/decks.rs` — deck legality is itself a frame gate.
- The client needs nothing: precons flow through the same deck-list wire.

## Phase 6 — Final verify + PR

1. Re-run the deck checklist: every card checked, or carrying a precise residual note the
   user has seen. Every remaining `approximates` in the pool must name *why* (absent
   subsystem, unobservable, dead variant) — never a silently dropped ability.
2. **Live smoke game (do not skip):** boot the real server + client from the worktree (own
   ports — never kill or reuse another session's dev servers) and drive a multiplayer game
   with the actual decklist over the HTTP/SSE surface, per the project `verify` skill. Saving
   the deck exercises deck legality (itself a frame gate — this is what exposed the
   hallucinated frames); the drive loop should answer every pending-choice kind it meets and
   log which new kinds fired live. Fix what it finds with regression tests at the lowest
   layer.
3. Sync-merge the default branch into the grind branch one last time; full bar both sides
   (`just check`-equivalent: workspace tests, fmt, clippy-no-new, tsc, client tests).
4. **End with an open PR, not a direct merge:** push the branch and open it with
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

## Phase 8 — Skill retrospective (after the merge)

The grind isn't done when the PR merges — it's done when this skill has absorbed what the
grind taught. Review the whole run against the skill and fold every lesson back in, as its
own small docs PR (the grind PR is already merged, so it can't carry them):

- Walk the run phase by phase and ask, for each surprise, rework loop, red wave, planner
  misread, or verification gap: *would the skill as written have prevented it?* If not, the
  fix belongs here — a new mandate, a sharpened rule, a planner-prompt patch in
  [`wave-workflow.js`](wave-workflow.js), or a convention in
  [`shared-context-template.md`](shared-context-template.md). (This is how the frame audit,
  the live smoke game, and the eligibility rule got here — each paid for itself the same
  grind it was learned in.)
- Also harvest the inverse: steps the skill mandates that contributed nothing this run.
  Don't delete on one data point, but note it in the step ("unexercised in the <slug>
  grind") so two dead runs justify removal.
- Check the assets still match reality: file paths, recipe names, API/lobby flows, and the
  project `verify` skill drift between grinds — fix stale handles now, while you know what
  the fresh ones are.
- Open the edits as a `docs(skills):` PR against the default branch and watch it to merge
  (Phase 7 rules apply). Push skill edits BEFORE the user merges the grind PR when possible —
  a lesson pushed seconds after the squash cut misses the train and needs exactly this
  follow-up PR.

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
