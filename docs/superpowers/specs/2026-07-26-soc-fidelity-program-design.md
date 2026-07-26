# Secrets of Strixhaven fidelity program (design)

**Status:** Approved design input (2026-07-26). Living behavior stays in indexed surface specs; each grind wave that changes engine/client/wire updates those specs in the same change. Per-deck checklists and increments live under `docs/fidelity/`.
**Surfaces:** `card-dsl-and-card-pool`, `engine-core-and-event-model`, `choices-actions-and-resolution`, `prompts-and-pending-choices`, `combat-and-commander-rules`, `turn-priority-and-stack`, `wire-protocol-and-visibility`, client board prompts / combat as demanded by each deck; fidelity process via `.agents/skills/fidelity-grind/`.

---

## Problem Statement

The product north star names the five Secrets of Strixhaven (`soc`, 2026) Commander precons (~389 unique cards) as the first faithful decks. Today those decks already have frozen lists (`docs/decklists/`), server fixtures, legality acceptance tests, and nearly complete pool coverage (~385/386 unique nonbasics in `crates/cards/data/`, including Quintorius, History Chaser). What they lack is the **formal fidelity bar** that closed Enchantress Rubinia, Deathdancer Xira, and Mirror Mastery: per-deck fidelity reports, observability re-audits, engine increments for remaining gaps, client catch-up, and live multiplayer smoke.

Pool presence is not fidelity. Decklist “Implementation risks” and residual `ponytail:` / `approximates` notes still mark real work (layer Auras, stacked dies triggers, copy/magecraft, X-modification, planeswalker commander + leaves-graveyard, etc.).

---

## Goal & success criteria

**Goal:** Finish all five SoC precons at the same fidelity bar as the closed C2011/MTGO grinds — friends can play these decks with rules that resolve correctly without manual bookkeeping.

**A deck is done when:**

1. Every non-basic card is in `crates/cards/data/`, frame-audited against live Scryfall (mana cost, P/T, type, legendary flag, verbatim `oracle`).
2. No silent ability drops: every remaining gap is a precise `approximates` and/or `# ponytail:` named in the fidelity report (user has seen the residual list).
3. `docs/fidelity/<slug>.md` and `docs/fidelity/<slug>-increments.md` exist; section D increments are LANDED or deliberately residual-flagged.
4. Client catch-up is green (`just client-check`) for any new wire / prompt / combat surfaces.
5. Precon fixture + legality acceptance remain green (fixtures already exist; fix if the grind drifts oracle/print ids).
6. A live 4-seat smoke game drives that precon over the real HTTP/SSE surface (project `verify` skill), with coverage logged and no unexplained rejects.

**Program is done when:** all five decks meet the above on one branch, and one mega-PR is ready to merge.

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Fidelity bar | Same as Rubinia / Xira / Mirror Mastery (full grind, not “playable enough”) |
| Deck order | Silverquill → Witherbloom → Prismari → Quandrix → Lorehold |
| Delivery | One long-lived branch + one mega-PR; merge only when all five are green |
| Parallelism | Start SoC without waiting for Heavenly Inferno `#214`; sync-merge `main` periodically |
| Smoke cadence | Live 4-seat smoke after **each** deck closes on the branch |
| Intake source | Frozen Wizards lists in `docs/decklists/` + Scryfall (not Archidekt) |
| Residual policy | Deliberate, named residuals only (absent subsystem / unobservable / dead variant); flag-don't-force |
| Engine growth | Card-driven via each deck’s increments file; no speculative cross-deck primitive program first |

---

## Approaches considered

1. **Sequential fidelity-grind on one mega-branch (chosen).** One worktree/branch; run the existing `fidelity-grind` loop per deck in order; smoke after each; one growing draft PR.
2. **Shared hard-primitives first, then decks.** Up-front copy / X-doubling / magecraft / leaves-graveyard engines before checklists. Rejected: anticipates cards, violates card-driven DSL growth, delays proven decks.
3. **Five mini-branches squashed at the end.** Rejected: extra sync overhead and conflicts with the locked one-branch / one-PR delivery.

---

## Scope

### In scope

The five frozen SoC lists:

1. `silverquill_influence` — Silverquill Influence (Killian)
2. `witherbloom_pestilence` — Witherbloom Pestilence (Dina)
3. `prismari_artistry` — Prismari Artistry (Rootha)
4. `quandrix_unlimited` — Quandrix Unlimited (Zimone)
5. `lorehold_spirit` — Lorehold Spirit (Quintorius, History Chaser)

### Out of scope

- Heavenly Inferno and other non-SoC grinds (land via their own PRs; absorb via sync-merge)
- Full Comprehensive Rules completion beyond what these decks demand
- Cosmetic board FX unrelated to new prompts / combat / wire surfaces from the grind
- Premature DSL generalization not demanded by a card in the active deck

---

## Branch, PR, and sync

- **Branch / worktree:** one long-lived isolation (e.g. `fidelity-soc` / `cursor/…`), created per `using-git-worktrees` before edits. Commit every green wave. Never merge to the default branch until the program is done and the user confirms.
- **PR:** open a draft mega-PR early; update the body as each deck closes (checklist summary + smoke evidence). Squash title on merge must be release-worthy (`feat:…`) because semantic-release reads the PR title.
- **Sync-merge:** periodically merge `main` into the SoC branch (including Inferno when `#214` lands). After each sync, re-run the **current** deck’s server + client verify bar before continuing waves.
- **Precons:** all five fixtures and legality tests already live under `crates/server/fixtures/decks/` and `soc_deck_tests`; re-validate rather than re-register unless ids/prints drift.

---

## Per-deck process

Run `.agents/skills/fidelity-grind/` with these SoC adaptations:

### 1. Intake

Parse `docs/decklists/<snake_slug>.md` (commander + 99). Resolve oracle/print ids via Scryfall `cards/collection`. Deduplicate by name; ignore basic-land quantity noise. Card identity is the TOML `name` field.

### 2. Fidelity report

Write `docs/fidelity/<kebab-slug>.md` with sections A–D (in-pool faithful / approximated / expressible / needs engine). Write ranked increments to `docs/fidelity/<kebab-slug>-increments.md` (numbering local to that file). Expect A/B-heavy intake because most cards are already authored; reclassify anything whose `ponytail:` or decklist risk note is still real into B/D.

### 3. Observability re-audit (mandatory)

Grep pool + engine `approximates` / `ponytail:` claims justified by absence or “unobservable.” Any claim this deck falsifies becomes real B/D work.

### 4. Pure authoring

Author true section-C gaps only (likely thin). No engine edits in this phase; misclassified cards move to D. Frame-audit every new/touched card against Scryfall until zero mismatches.

### 5. Engine grind loop

Wave loop from the skill (`wave-workflow.js` or sequential agents): TDD; batch S/M; while any XL remains, every wave carries exactly one XL slice last; commit per green wave; verify gate includes nextest/fmt/clippy, adversarial review, frame audit, increments LANDED marks, `just engine-cr-index` when citations move. On red: stop and use `systematic-debugging`. Stalled orchestrator: rescue or revert one increment, then continue with direct agents.

### 6. Client catch-up

When waves add wire / `PendingChoice` / visible events: `just server-codegen`, close hand-written gaps (`types`, formulators, event-fold, presence), gate with `just client-check`. Scene / interaction tests for any new user-visible prompt surface.

### 7. Precon re-validate

Confirm fixture + `<slug>_is_a_legal_commander_deck` still pass. If prints/ids change, rewrite with the soc-aware precon tooling (not placeholder UUIDs).

### 8. Live smoke

Drive a multiplayer game with that precon over HTTP/SSE (`verify` skill). Bias toward new surfaces; log actions, pending-choice kinds, rejects. Update the mega-PR body with the evidence. Then sync-merge `main` and proceed to the next deck.

### Deck risk focus (drive increments; do not pre-build)

| Deck | Primary hard surfaces |
|------|------------------------|
| Silverquill | Layer Auras; goad/control; Prepare / copy-on-stack; constellation / conditional wipes |
| Witherbloom | Stacked dies/drain triggers; recursion; life-as-resource |
| Prismari | Copy / token-copy; magecraft trigger doubling |
| Quandrix | X on stack; cost reduction; X-doubling and copy interaction |
| Lorehold | Planeswalker commander loyalty; leaves-graveyard triggers; Hofri-style death replacement |

### Residual policy

Same as prior grinds: only deliberate, named residuals. Example allowed until a deck needs the subsystem: Final Act’s dropped battle / player-counter modes. An XL with only a documented residual is marked LANDED with the residual named — not left open forever.

---

## Verification

### Per-wave

- Failing engine test before production code (`test-driven-development`)
- Fresh `cargo nextest` / `cargo fmt` / `cargo clippy --all-targets` evidence (`verification-before-completion`)
- Adversarial diff review (`requesting-code-review`)
- Scryfall frame audit zero-mismatch for new/touched cards
- Update that deck’s increments file; regenerate `docs/CR_INDEX.md` when needed

### Per-deck

- Checklist complete (or residuals named)
- `just client-check` if client touched
- Precon legality green
- Live smoke evidence in the PR body

### After sync-merge from `main`

Re-run the current deck’s server + client verify (not necessarily the whole program) before the next wave.

### Program (before merge)

- All five fidelity reports closed
- Explicit residual inventory in the PR body
- All five SoC fixtures still validate
- Living surface specs updated for shipped behavior (no orphan design-only claims)
- Update `card-dsl-and-card-pool` Status / pool counts in the merge change
- Skill retrospective into `fidelity-grind` only if SoC teaches durable process changes (e.g. Wizards-list intake)

---

## Error handling

- **Red wave:** stop; root-cause with `systematic-debugging`; do not “push through.”
- **Flag-don't-force:** if a card needs an unready subsystem, name the residual in the card and increments file; do not contort the DSL or lie in tests.
- **Stalled orchestrator:** finish or revert the half-applied increment, verify by hand, then prefer direct agent stages (mirror-mastery rule).
- **Inferno / main conflicts:** resolve on sync-merge; never overwrite SoC fidelity reports with stale main copies without a three-way read.

---

## Docs map

| Artifact | Role |
|----------|------|
| This file | Program design input (order, delivery, SoC adaptations) |
| `docs/fidelity/<slug>.md` | Per-deck checklist (source of grind truth) |
| `docs/fidelity/<slug>-increments.md` | Sole engine-capability backlog for that deck |
| Indexed surface specs | Living Behavior / Implementation / Testing for shipped code |
| `.agents/skills/fidelity-grind/` | Operational process (wave loop, verify, precon, smoke) |

---

## Success criteria (program)

- Silverquill, Witherbloom, Prismari, Quandrix, and Lorehold each meet the per-deck done bar above.
- One mega-PR merges the program; default branch then treats the five SoC precons as fidelity-closed (residuals only where explicitly named).
- No speculative engine work that no SoC card in the active grind demanded.
