---
name: fidelity-grind
description: Given an Archidekt deck link or frozen decklist, make every card in that deck faithful — deck intake, fidelity report checklist, observability re-audit, pure-authoring pass, engine grind loop, client catch-up, full verify + skill retrospective, then an open PR watched through CI and review to merge. Use when the user provides a deck source and wants the pool to support it faithfully.
---

# Fidelity Grind

Turn an Archidekt deck into a fully faithful slice of the card pool, end to end. This skill
encodes the process that took the first 429-card pool from ~60% to 99.3% faithful (waves 1–142,
2026-07). Read `card-dsl` (authoring bar). For sticky ability encoding, use the `forge` skill
(vendored `.repos/forge`). Drive implementation with the
`test-driven-development` skill (red → green) throughout. Use other superpowers plugin skills
named below for isolation, planning, verification, review, and branch finish.

**Inputs:** either an Archidekt deck URL (`https://archidekt.com/decks/<id>/<slug>`) or a frozen local decklist (for example `docs/decklists/<slug>.md` sourced from an official Wizards list).
**Output:** every deck card scripted in `crates/cards/data/`, faithful or carrying a precise
`approximates` residual; engine + client green; an open PR against the default branch with
the wrap-up report as its body.

## Superpowers plugin skills this grind uses

| Phase | Skill | Role |
|-------|-------|------|
| 0 | `using-git-worktrees` | Isolated worktree before any edits |
| 0 | `forge` | Vendored `.repos/forge` script reference (`just forge` to sync) |
| 3–4 | `test-driven-development` | Failing engine test before production code |
| 4 (plan) | `writing-plans` / `dispatching-parallel-agents` | Wave planning + brief writing (via `wave-workflow.js`) |
| 4 (verify) | `verification-before-completion` | Evidence before claiming a wave green |
| 4 (verify) | `requesting-code-review` | Adversarial diff review of the wave |
| 4 (bugs) | `systematic-debugging` | Root-cause before fixes when a wave goes red |
| 6 | `verification-before-completion` + project `verify` | Final bar + live smoke |
| 7 | `finishing-a-development-branch` | PR / merge options after the grind is green |

## Phase 0 — Setup (isolation)

- Follow **`using-git-worktrees`** to put the grind in an isolated workspace. Prefer the
  platform-native worktree flow; fall back to:
  `git worktree add ../mtgfr-grind-<slug> -b fidelity-<slug> main`
- If the platform flow or a nested `git worktree add` fails but you are already in an isolated
  checkout, treat the current checkout as the grind root and stamp that absolute path into every
  brief/script token. The SoC program used `/workspace` this way; do not invent a second nested
  worktree just to satisfy the ritual.
- Commit every green wave on that branch. **Never merge to the default branch until the grind is done**
  and the user confirms. Periodically sync-merge the default branch *into* the grind branch (delegate to an
  agent; full workspace tests both sides after resolving). If the default branch was
  force-pushed with rewritten history mid-grind ("refusing to merge unrelated histories"),
  graft the old root under the new one (`git replace --graft <new-root> <old-root>`), do the
  normal 3-way merge, then `git replace -d <new-root>` — one real conflict instead of
  hundreds of add/add ones. Before grafting, find the rewritten commit equivalent to your
  branch's base and prove the graft point with `git diff <old> <new>` — an empty diff means
  the 3-way will contain only genuinely-new work from each side.
- **`bun install --frozen-lockfile` in `<worktree>/client` before anything else.** A fresh
  worktree has no `client/node_modules`, and the miss only surfaces in Phase 5 when
  `just server-codegen` and `npx tsc --noEmit` both fail for a reason that looks like real
  wire drift.
- All agent briefs must name the worktree root and forbid touching the main checkout.

## Phase 1 — Deck intake

- Intake source can be an Archidekt deck JSON, a frozen local decklist, or a **set code**.
- For a set code (`2ed`), the "deck" is the whole set: fetch the Scryfall set list
  (`https://api.scryfall.com/cards/search?q=set:<code>&unique=prints`, paginate on `has_more`) and
  diff it against `grep -h '^name = ' crates/cards/data/*.toml`. The diff is two-sided — names
  missing from the pool, *and* pool cards already scripted from a later printing whose `sets` array
  is missing `<code>`. Everything downstream (report, increments file, waves) works unchanged with
  `<slug>` = the set code.
- For Archidekt, fetch `https://archidekt.com/api/decks/<id>/` (public JSON). Each entry:
  `card.oracleCard.name`, quantity, categories. Ignore basic-land quantities; dedupe by name.
- For a frozen local decklist (for example the SoC Wizards lists committed under
  `docs/decklists/*.md`), treat the checked-in list as the source of truth and run
  `python tooling/soc_fidelity_intake.py docs/decklists/<slug>.md` to baseline A/B/missing
  against `crates/cards/data/`. Use that output as the starting checklist only; the manual
  observability re-audit still decides what stays faithful vs. moves to B/D.
- Cross-check oracle text against Scryfall when authoring — Archidekt's `oracleCard.text` can lag,
  and local decklists omit oracle text entirely — so the TOML `oracle` field must be current
  Scryfall text.
- Card identity is the TOML `name` field (not the filename). Match deck names against
  `grep -h '^name = ' crates/cards/data/*.toml`.

## Phase 2 — Fidelity report (the checklist)

Write `docs/fidelity/<slug>.md` — a checkbox per deck card, in four sections:

- **A. In pool, faithful** — no `approximates`. No work; check off immediately.
- **B. In pool, approximated** — quote each card's current note verbatim.
- **C. New, expressible today** — cards the current DSL can script with no engine change
  (judge against `DSL_REFERENCE.md` + `de.rs`; when unsure, mark D — flag-don't-force).
- **D. New, needs engine work** — for these, write ranked increments to a per-deck file,
  `docs/fidelity/<slug>-increments.md`. That file is the **sole engine-capability backlog
  for this deck** (no global fidelity backlog). Format: numbered heading (numbering is
  **local to this deck file** — start at 1 for a new deck, or continue the highest number
  already in *this* file), effort (S/M/L/XL), `Depends on:` line, example cards, a *Sketch*
  of the intended design. XL increments get an explicit slice staging. Fold per-card
  exotics into the same file (don't invent a second ledger).

**Observability re-audit (do not skip):** many residuals are justified by pool absence —
notes saying "no pool card does X", "unobservable", "dead variant". Grep every
`approximates` and `ponytail:` in `crates/cards/data/` and `crates/engine/src/` for such
claims and re-test each against the incoming deck. Any claim the new cards falsify moves
that residual into section B/D as real work. (Example: "damage to planeswalkers is a dead
variant" died the moment a planeswalker entered the pool.) A set older than every subsystem the
residuals name falsifies little — the 2ed audit moved zero residuals — but it is a grep, so run it
and record the zero rather than skipping the step.

## Phase 3 — Pure authoring pass

Author all of section C in batched waves. Invoke **`test-driven-development`**: failing
engine test in `crates/engine/tests/game.rs` first, then the TOML. No engine edits allowed
in this phase — if a card turns out to need one, reclassify it to D and move on.

**Frame audit (mandatory, after every authoring or grind wave that adds cards):** agents
hallucinate frames — the first grind shipped 8/66 cards with wrong mana costs, P/T, or
phantom keywords (Rubinia herself was {2}{W}{U}{U} 2/4 *with flying*), invisible to
ability-level tests but fatal to deck legality. Script a diff of every new card's mana cost,
P/T, type, legendary flag, and verbatim `oracle` field against a fresh Scryfall
`cards/collection` fetch; fix every mismatch (some are behavioral — a wrong activation cost
or counter count changes play) and keep the two-sided check until it reports zero. Stale
oracle TEXT is the sneakiest frame bug: the Xira grind shipped a card modeled on old
mandatory wording where current text says "you may" — invisible to its own ability tests.
The wave verify stage runs this audit as a green-gate requirement (baked into
[`wave-workflow.js`](wave-workflow.js)); zero problems or the wave is red.

## Phase 4 — Engine grind loop

Run the wave loop until the planner declares done. Assets in this folder:

- [`wave-workflow.js`](wave-workflow.js) — the orchestration script (plan → implement
  sequentially on the shared tree → verify+reconcile). Copy it to scratch space, fill the
  `{{WORKTREE}}` / `{{BRANCH}}` / `{{BACKLOG_RANGE}}` / `{{BACKLOG_FILE}}` tokens, and run it via the Workflow
  tool, relaunching after each green wave. Without a workflow orchestrator, run the same
  three stages as sequential subagent dispatches (`dispatching-parallel-agents` only when
  rider increments are proven disjoint; default is sequential on the shared tree). The Plan
  stage should read like a `writing-plans` brief per increment.
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
- **Verify gate (every wave, opus, adversarial):** follow **`verification-before-completion`**
  — no green claim without fresh command evidence. Run full `cargo test --workspace` and
  **`just server-check`** — the whole recipe, not the clippy line out of it. `just server-lint` is
  *only* `cargo clippy --all-targets`; the pool validation is the separate `cards-toml-validate-pool`
  and `cards-schema-check` recipes that `server-check` (and CI) depend on, and that validation is
  the *only* thing that catches drift between the TOML surface's two hand-synced structs: the
  deserializer's private `Table` in `crates/cards/src/de.rs` and its schema mirror
  `AmountTableSchema` (and siblings) in `crates/cards/src/toml_surface/dsl_schema.rs`. Both are
  `deny_unknown_fields`, so a key added to one and not the other loads and plays perfectly and
  fails only at schema validation — `on_attached` shipped that way and sat broken until CI caught
  it a whole set later. Bare `cargo clippy --all-targets` is not the gate; expect zero NEW
  warnings from it vs a git-stash baseline, but do not mistake it for `server-check`. (Locally the
  gate's last step wants `DATABASE_URL`; a failure there and nowhere else is environment, not
  code.) then **`requesting-code-review`** for an adversarial diff review against the CR;
  reconcile every touched card's
  `approximates` note against the actual diff; update **this deck's**
  `docs/fidelity/<slug>-increments.md` LANDED marks (XL slices get
  dated progress notes, not LANDED, until all slices land); regenerate `docs/CR_INDEX.md`
  (`just engine-cr-index`). **Frame audit (mandatory):** script a diff of every new/touched
  card's mana cost, P/T, type, legendary flag, and verbatim `oracle` field against a fresh
  Scryfall fetch; fix every mismatch and keep the check until it reports zero. If Scryfall
  fetch fails (rate limit 403, timeout), use the Archidekt deck JSON as the id source of
  truth — every card has `card.oracleCard.id` (oracle id) and `card.uid` (print id). Never
  commit placeholder uuids; they break the frame audit and deck fixtures. Verify stages catch
  real rules bugs (~1 per wave in the mirror-mastery grind) — never skip.
- **Re-verify the backlog's own premise, not just the card.** The intake author paraphrases
  from memory and gets cards wrong; a brief that inherits the paraphrase builds the wrong
  card and its tests agree with it. Eight of ~36 mirror-mastery increments described a card
  that does not exist (a free cast that is really a conditional {6} reduction; "copies
  creature spells" for a card that copies instants and sorceries; an exile clause on a card
  with none; a forced-attack clause invented whole). Every implementer must open live
  Scryfall for its increment's *Cards:* line and, when the premise is wrong, **correct that
  backlog section in place** before writing code.
- **The *Sketch* is a hypothesis from the wave that filed it, not a spec.** The premise rule above
  covers the card; this covers the engineering. Two of the four wave-13 Legends sketches were
  substantively wrong in ways that would have shipped needless machinery: Takklemaggot's described a
  decision the card does not offer ("if they can't *or won't*" — the branch is reachable only on an
  empty candidate list) and proposed two permanent shapes selected by an ETB flag, when an existing
  pause's free `player` axis already covered it; Psychic Purge's quoted only the trigger and silently
  dropped "deals 1 damage to any target". Re-derive the design from the oracle text *and the current
  engine* before implementing, and correct the entry in place so the error dies with the wave.
- **"No single choke point exists" is a survey result, not a verdict.** Wave 12 surveyed the draw
  path, found the mint functions pure (so unable to raise a pause) and only one batch site able to,
  and deferred Chains of Mephistopheles as possibly unlandable. The answer was to *build* the choke
  point — one mutating funnel with a parked batch so a pause resumes mid-draw — and convert the
  bypassing sites onto it. A missing seam is work, not a blocker; a survey must say which of the two
  it found.
- **Reach for the free axis on an existing pause before minting a new one.** Three of four wave-13
  slices needed a new player-facing choice and *none* added a wire surface: one reused an existing
  `ChooseAttachHost` `player` field, one answered a new pause with the existing
  `Intent::ChooseCopyTarget` plus a wording flag, and one's new fields projected through the `..`
  already in `projection/choice.rs`. Twelve pre-allocated proto field numbers went unused. Budget
  them anyway — an unused reservation costs nothing and a collision costs a wave.
- **When the orchestrator stalls, rescue then abandon it.** A stalled implement stage
  ("no progress for 180000ms") leaves the shared tree RED mid-edit — a half-added `CardDef`
  field is 157 compile errors, a half-added enum variant is a non-exhaustive `match` in
  `crates/schema`. Recovery is always the same: dispatch one opus agent to finish *or revert*
  that one increment and then run the verify stage by hand. After the second stall, stop using
  the workflow orchestrator for this grind and dispatch the three stages as direct agents —
  in the mirror-mastery grind that converted three consecutive stalled waves into three clean
  ones with no other change.
- **Parallel slices share one cargo lock and one working tree — brief them accordingly.** Two rules,
  both learned the expensive way in the Legends grind. **(1) A slice runs only its own test binary**
  (`cargo test -p engine --test leg_w<N>_<slice>`), never the full suite: four concurrent
  `nextest --profile ci` runs serialize on the build-dir lock, so each slice waits on the others and
  reports "23m elapsed, three full suites sharing the machine" instead of finishing. Worse, they
  outlive the agent — five orphaned cargo processes held the lock with *zero* `rustc` running and
  stalled the orchestrator's own regeneration for 28 minutes. **At wave close, confirm no cargo
  process survives its slice before running central verification**, and check `pgrep -c rustc`: a
  cargo process with no compiler under it is a lock-holding zombie, not progress. **(2) No test may
  rewrite a checked-in file in place.** `gen_dsl_reference`'s check-mode test overwrote the real
  `DSL_REFERENCE.md` with `# stale` and restored it afterwards; with two suites overlapping, the
  second captured `# stale` as its "original" and restored *that*, destroying the file. Recovery is
  `git show HEAD:<path> > <path>` — never `git checkout --`, which a sibling session's edits share.
  Fixed by running it in a temp dir; audit for the pattern before fanning out.
- **Budget the stack, not just the wave.** `Effect` grows a card at a time and is now ~9 KiB
  (`Ability` ~11 KiB), so a test binding a hundred card locals overflows a test thread's 2 MiB stack
  and aborts as **SIGABRT with no assertion to point at** — and nextest's threads get *less* stack
  than `cargo test`'s, so a split test can pass under one and abort under the other. `.cargo/config.toml`
  sets `RUST_MIN_STACK` for every cargo-spawned process; keep per-test card counts modest anyway. The
  same growth surfaces as clippy `large_enum_variant` and `large_const_arrays` — the established
  answer is an `#[allow]` with a `ponytail:` naming the upgrade (box the widest variants), and
  `static` rather than `const` for large ability arrays.
- **A declined increment must take its tests with it.** A slice that writes tests, then decides the
  increment cannot land, leaves permanently-red tests that read as a broken wave. Deleting them is
  part of declining, not cleanup.
- When a wave goes red or a bug resists a first glance, use **`systematic-debugging`**
  (root cause before fix) rather than patching symptoms.
- **Commit per green wave**; on a red wave stop and surface it to the user.
- Consolidate before the XL tier if the codebase has grown fast (module splits, walker
  unification) — behavior-preserving only, full green bar.

## Phase 5 — Client catch-up

Engine waves accrue wire debt. After the grind (or mid-grind if large):
`just server-codegen`, then close the hand-written wire gap —

1. Add any new `PendingChoiceView` kinds to `client/app/domain/wire/types.ts` and register them in
   `FORMULATOR_FOR_KIND` (`client/app/domain/choice.ts`); reuse an existing formulator when the answer
   shape matches (prompts render via `client/app/board/html/prompts.ts`).
2. Add any new `VisibleEvent` kinds to `client/app/domain/wire/types.ts`, extend
   `VISIBLE_EVENT_KIND_PRESENCE` (`client/app/domain/wire/visibleEventKindPresence.ts`), and add
   exhaustive arms in `client/app/domain/event-fold.ts` (`extractProvenance` / `describe`).
3. New `MeaningfulAction`s surface via the existing generic tiles / activation menu.

Canvas tests the grind adds must assert the *invariant*, not the pixel. `expect(ys).toEqual([142,
156, 170])` for three stacked chips reads as a stacking assertion but is really a snapshot of where
the avatar block starts, and it goes red the first time the default branch nudges that block.
Assert what the feature promises — three rows, one line-height apart, in a named order.

Gate: **`just client-check`** (codegen + format + lint + typecheck + Vitest). That includes
`wire-case-coverage.test.ts`, which fails if generated proto oneofs drift from the hand unions.
`VISIBLE_EVENT_KIND_PRESENCE` is hand-written and exhaustive, so a new wire event kind that skips
step 2 above fails only in `just client-typecheck`, as a `Record` "missing the following
properties" error naming every kind you forgot.
`just client-test` alone is `bun run test` (vitest). Plain `bun test` runs Bun's own runner over
the same files and reports dozens of phantom failures — only the just recipes count as evidence.

## Phase 5.5 — Ship the deck as a precon

Every grind deck ends as a read-only in-app precon, so players get the deck, not just the
pool that supports it. After client catch-up (the wire is settled by then):

- Write `docs/decklists/<snake_slug>.md` — the frozen target list (commander + grouped tables for
  the other 99 cards, 100 total), sourced from the Archidekt fetch.
- Generate `crates/server/fixtures/decks/<snake_slug>.json` from that list: `commander` /
  `commander_print` and one `{id, count, print}` entry per non-commander card (basics carry
  their count). Map `id` through the pool TOMLs, but stamp each `print` from Archidekt
  `card.uid` (the precon printing — e.g. `cmd` / `td0`), not `CardDef.default_print`
  (Scryfall's preferred print from `/cards/named`, often a different set). For the three
  existing grind precons use `node tooling/rewrite-grind-precon-fixtures.mjs`. Do not use
  `tooling/rewrite-precon-fixtures.mjs` for non-soc decks — it prefers `soc` prints. Never
  commit placeholder oracle/print UUIDs; they 404 on Scryfall and break art. Pool TOMLs keep
  Scryfall's preferred `default_print`; only the fixture carries the Archidekt print.
- Register it in `crates/server/src/precons.rs`: one `Source` entry with the **next
  negative id** (grow the `SOURCES` array length), name it after the deck.
- Add the fixture to `FIXTURES` and an `<slug>_is_a_legal_commander_deck` acceptance test
  in `crates/server/src/decks.rs` — deck legality is itself a frame gate.
- The client needs nothing: precons flow through the same deck-list wire.

## Phase 6 — Final verify + skill retrospective

1. Re-run the deck checklist: every card checked, or carrying a precise residual note the
   user has seen. Every remaining `approximates` in the pool must name *why* (absent
   subsystem, unobservable, dead variant) — never a silently dropped ability.
   **Two blind spots make "the audit is green" weaker than it reads.** `tooling/fidelity_report_audit.py`
   excuses an unticked card whose name appears *anywhere* in a blocking increment's prose — a
   substring match, so Recall sat unticked for two waves because an unrelated increment mentioned it
   in passing, and the audit called the whole set agreed. And **"no `approximates`" is not a proxy
   for "faithful"**: frame-only stubs (front matter + oracle comment, no `[[abilities]]`) carry no
   note at all. So at wave close, read the unticked list by hand and confirm each remaining card is
   genuinely unsupportable, and spot-check that every newly-ticked card actually has abilities.
2. **Live smoke game (do not skip):** follow **`verification-before-completion`**, then drive
   the project **`verify`** skill — boot the real server + client from the worktree (own
   ports — never kill or reuse another session's dev servers) and drive a multiplayer game
   with the actual decklist over the HTTP/SSE surface. **A set grind has no decklist and may have
   no commander** — pre-Legends sets contain no legendary creature at all. Seat four out-of-set
   commanders whose color identities *union* to WUBRG (2ed used Kaalia WBR / Riku URG / Rubinia
   WUG / Xira BRG) and fill each 99 with in-identity cards from the set, so every colour of the
   grind's own work gets played. Saving the deck exercises deck
   legality (itself a frame gate — this is what exposed the hallucinated frames); the drive
   loop should answer every pending-choice kind it meets and log which new kinds fired live.
   **Bias the driver toward this grind's new surfaces and report coverage honestly** — a
   random-play driver reaches the common ones and misses the rest. Mirror mastery drove six
   games (one to game over, 42 turns) and still never saw retrace, storm, vanishing or an
   attack on a planeswalker, because no seat ever drew the mana for them; those stay
   engine-test-only, and the report says so rather than implying they were exercised. What
   the live drive *did* catch was invisible to every unit test: a mandatory two-target prompt
   the client could only ever answer with one id, wedging all four seats forever.
   Fix what it finds with regression tests at the lowest layer (`test-driven-development`).
   **Six traps have produced false wedges across the grinds so far — the `verify` skill carries
   every one:** the smoke stack may take its own HTTP/Vite/Postgres ports but *not* its own gRPC
   port (routed tables are pinned to `:50051`); a rejected intent acks HTTP 200, so the driver must
   branch on `Ack.accepted`; an action's legal targets already ride on `ActionView.targets`, so
   guessing from the battlefield buries the drive under thousands of `reject.illegal_target` acks;
   a modal cast needs the `modes` its own `modal` block describes (`reject.illegal_mode`); a seat
   must submit only while its snapshot still says it holds priority, *except* for the mulligan
   decision every seat makes at once; and a 99 with no lands is not a wedge at all — nothing is a
   meaningful action on an empty board, so the server correctly auto-passes entire turns.
   Read `logs/actions.<TABLE>.toon` (every intent, its accepted
   flag, reject reason, post-state and events) before blaming the engine.
   **Then read the surviving rejects per card — that is where the real bugs are.** Two clusters
   outlived the traps here: one was the driver skipping a cost the action itself carried
   (`sacrifice_choices` → `reject.cannot_activate`), the other a genuine product bug the whole
   unit suite missed — a spell whose targets are chosen *after* the cast (CR 601.2c) was
   advertised with `needs_target: true`, so the board staged a target click the cast gate could
   only reject. Rule: **an action the snapshot lists must be submittable exactly as listed** —
   and so must a pending choice. Fixing that cluster exposed the next (equip enumerating
   opponents' creatures its own gate rejects), which exposed the next (a `ChooseSpellTargets`
   raised with `min: 0, max: 0` and a full `legal` list). Expect this class to recur, and re-run
   the drive after each fix: a loud cluster hides the quieter ones behind it.
   If the local environment
   cannot boot server+client (missing DB, port conflicts, etc.), delegate the smoke game to a
   cloud/remote agent with a clean setup, or fix the environment before proceeding. Do not
   skip — this is the final integration gate before the PR.
3. Sync-merge the default branch into the grind branch one last time; full bar both sides
   (`just check`-equivalent: workspace tests, fmt, clippy-no-new, tsc, client tests).
   Two things a merge breaks that no test name warns you about: a wire field whose *type*
   changed on the default branch (heavenly-inferno merged an i18n hard-cut that turned
   `ActionView.label` from `string` into `MessageRef`) only fails in **`just client-typecheck`**,
   in this branch's own test fixtures — reach for the repo's fixture helper
   (`client/app/domain/i18n/testMessageRef.ts`) rather than hand-building the new shape; and
   `docs/CR_INDEX.md` goes stale from both sides' new citations, so re-run
   `just engine-cr-index` before `just engine-cr-index-check`.

   **The merge's real danger is not conflicts — it is silent unions and silent losses.** A long
   grind and a moving default branch reimplement the *same* subsystems, and git resolves that
   without ever showing a marker. Frank-Horrigan merged 74 commits and went 148 → 38 → 23 → 10
   → 0 failures, every step a variant of one of these:
   - **Silent union — behavior doubles.** Main had moved self-`Etb` and combat-damage trigger
     queuing into table dispatch (`queue_trigger_watch_table(ENTERS_BATTLEFIELD_TRIGGER_WATCHES,
     …)`); the grind still called `queue_self_trigger` / `queue_combat_damage_triggers`
     explicitly. Both survived, so every ETB fired twice. Symptom: a pending choice re-raised
     for a clause already answered. After resolving, grep for every dispatch/registration call
     the grind added and confirm the default branch hasn't grown a second path to it.
   - **Silent loss — an additive gate vanishes.** Where main's version of a file won wholesale,
     the grind's *additions inside* untouched-looking functions went with it: an emblem source
     chain in `matching_anthems`, a `Condition::SourceAttackedThisTurn` keyword arm, a
     `min_level` gate on a keyword anthem. Nothing conflicted; three tests just failed.
     **Before merging, snapshot every file the grind touched** (`git show HEAD:<f> >
     scratchpad/<f>.pre`) so restoring a dropped gate is a two-file diff instead of archaeology.
   - **Duplicate surface — one of them is now dead.** Main shipped
     `spell_multikicker_count` / `entered_multikicker_count` for the same card need the grind
     had solved as `times_kicked` / `entered_times_kicked`. Both compiled; one was written and
     never read. After the merge, walk the increments backlog's LANDED surface names and
     confirm each is still *read*, then delete the loser from code, `DSL_REFERENCE.md`, and the
     increments entry together.
   - **A default-branch test can encode a scenario the grind's new rule makes illegal.** A
     server cleanup-discard test relied on a decked-out player still discarding; the grind added
     the CR 800.4e "no turn-based actions for a player who has left the game" guard. Fix the
     test's *setup* to a legal scenario — never weaken the new rule to keep an old fixture green.
   - **`git checkout -- <file>` during resolution is unrecoverable.** Unstaged merge edits live
     nowhere else. `git add` each file the moment it compiles.
   - **Card TOML corrupts silently too** — a stray resolution dropped a green pip from Doubling
     Season's `{4}{G}{G}`, and the only thing that caught it was an *unrelated* cost-reduction
     test. Diff `crates/cards/data/` against the merge base for cards the grind never touched.
   - **Collided subsystem — pick the general one and delete the other outright.** Sharpest
     case yet: while Frank-Horrigan was opening its PR, main shipped a *narrow* poison surface
     (`Player::poison`, `Event::PlayerPoisonChanged`, `CountersEffect::EachOpponentGetsPoison` /
     `EachOpponentLosesAllCounters`) for two cards the grind had already covered with the general
     player-counter subsystem (`PlayerCounterKind`, `Event::PlayerCountersPlaced`,
     `put_counters_on_player` / `remove_all_player_counters`). Both compiled side by side, and
     *two storage paths for the same game fact is a bug that no test catches* — a counter placed
     through one is invisible to the other. Resolve by deleting the loser end to end in one pass:
     engine field + event + accessor + effect variants + `de.rs` parsing + `message.rs` keys +
     schema DTO + projection + gRPC map + client `types.ts` /
     `visibleEventKindPresence.ts` / `event-fold.ts` / i18n catalog + `rustKeys.json` +
     `DSL_REFERENCE.md` rows. Keep the loser's *tests* — retarget their assertions to the
     surviving accessor; they are free coverage of cards the grind never wrote.
   - **A shipped proto field number is burned, even when you delete the field.**
     `docs/WIRE_COMPAT.md` is expand-only, so when the merge deletes a default-branch event that
     already released, `reserved N;` its number (at message scope, not inside the `oneof`) and
     renumber the grind's own new fields above it. Never reuse the slot for a different type.
   - **A conflicting PR gets no CI at all.** GitHub cannot build the merge ref, so
     `pull_request` workflows never fire and the PR sits with an empty `statusCheckRollup` — it
     reads as "still queueing," not "blocked." Check
     `gh pr view N --json mergeStateStatus,mergeable` right after opening; `DIRTY` /
     `CONFLICTING` means merge again *now*. Re-merge default immediately before opening the PR,
     not an hour earlier.
   - Three mechanical ones: a moved-away generated tree from the default branch's own refactor
     (`client/lib/wire/generated/` after `client/lib/` → `client/app/domain/`) stays on disk and
     fails `just client-lint` until deleted; `bun install` must run in the *worktree* before
     `just client-typecheck` means anything; and commitlint caps the **PR title** at 72
     characters — check it with `printf '%s' '<title>' | ./node_modules/.bin/commitlint` from the
     main checkout before opening, since the worktree has no root `node_modules`.
4. **Skill retrospective (before opening the PR):** the grind isn't done until this skill has
   absorbed what the grind taught. Review the whole run against the skill and fold every
   lesson in *before* opening the PR, so the skill improvements ride in the same squash
   commit. Walk the run phase by phase and ask, for each surprise, rework loop, red wave,
   planner misread, or verification gap: *would the skill as written have prevented it?* If
   not, the fix belongs here — a new mandate, a sharpened rule, a planner-prompt patch in
   [`wave-workflow.js`](wave-workflow.js), or a convention in
   [`shared-context-template.md`](shared-context-template.md). (This is how the frame audit,
   the live smoke game, and the eligibility rule got here — each paid for itself the same
   grind it was learned in.) Also harvest the inverse: steps the skill mandates that
   contributed nothing this run. Don't delete on one data point, but note it in the step
   ("unexercised in the <slug> grind") so two dead runs justify removal. Check the assets
   still match reality: file paths, recipe names, API/lobby flows. Commit the skill edits to
   the grind branch so they ride the PR. Anything learned *during* Phase 7 PR watch (a CI
   failure mode, a review pattern) goes in a small `docs(skills):` follow-up PR.
5. **Commit hygiene for the PR:** keep verified wave history by default. This repo's CI
   lint-checks the **PR title** for release semantics, not every branch commit, so do **not**
   squash or rewrite the branch just for aesthetics. Only rewrite history if an actual CI /
   commitlint gate blocks on branch commit messages or merge subjects; tag the old head first,
   note the reason in the wrap-up report, and force-push only as the unblock.
6. **End with an open PR, not a direct merge:** follow **`finishing-a-development-branch`**
   (verify tests → present options → execute). Prefer opening a PR with `gh pr create`
   against the default branch. The PR body is the wrap-up report — the deck
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
  Phase 4/5 rules (`test-driven-development`, verify gate, conventions).
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
- `CardDef` is `Clone`, not `Copy`; keep skill/backlog/spec prose aligned with the current
  `Arc`-backed printed-definition storage.
- Every bug fix gets a regression test at the lowest layer that catches it.
