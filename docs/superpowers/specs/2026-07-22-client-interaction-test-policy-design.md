# Client interaction test policy

**Status:** Done  
**Date:** 2026-07-22

## Goal

Stop shipping interaction and local-env bugs that “panel exists” Scene tests miss, without adding a heavy browser E2E matrix.

Recent slips (Host/`WEB_DATABASE_URL`, Alt-hold inspect, hand hide on drag-play, builder hover tooltips/stale art, lobby select vs pre-picked deck) shared one pattern: coverage asserted presence or intent commands, not the user-visible outcome.

## Principles

1. **Behavior, not presence** — Prefer assertions on outcomes (pin set, tile hidden, selected deck matches, art URL swapped, default URL works) over “`data-testid` exists.”
2. **Product language only** — Name tests for the desired product behavior. Do not frame them as legacy-client parity checks.
3. **Docs discipline for process** — `AGENTS.md` and the `verify` skill describe when to run live checks. No new Playwright/CI browser job in this policy.
4. **Live is optional unless flagged** — PRs that change interaction/UI check an Interaction / UI box; then the `verify` Interaction checklist is required before claiming done.
5. **Encode known failure classes** as ordinary unit/Scene tests (many already landed; fill remaining gaps).

## Unit / Scene behavior catalog

| Behavior | Test location | Status |
|---|---|---|
| Default `WEB_DATABASE_URL` when unset | `client/server/db/url.test.ts` | Done |
| Alt-down pins card under cursor; Alt-up clears; hand aux preferred | `client/app/board/inspect-pile-concede.test.ts` | Done |
| Drag-play commit hides hand card + seeds flight; cancel restores | `client/app/board/hand-drag.test.ts` | Done |
| `handHidden` removes hand tile from view | `client/app/board/scene.test.ts` | Done |
| Pool cards have no native `title` tooltip | `client/app/shell/decks/builder/story.test.ts` | Done |
| Card art host repaints when `data-art-url` changes | `client/lib/ui/card-art.test.ts` | Done |
| Lobby: selected deck matches pre-pick / `?deck=` among ≥2 decks (not silently `decks[0]`) | `client/app/shell/lobby/entry.test.ts` | Done |
| Lobby entry Bring presentation (text/card, no misleading select) | `client/app/shell/lobby/entry.test.ts`, surfaces | Done |

### AGENTS.md rule (add alongside existing surface rule)

When changing client interaction (pointer, keyboard, hover, drag, Mount hosts, BFF env defaults), add or extend a unit/Scene test that asserts the **outcome**, not only that a `data-testid` exists. Existing surface-presence coverage in `client/app/shell/surfaces.test.ts` and `client/app/board/html/surfaces.test.ts` remains required for new panels.

## Process: PR flag + verify checklist

### PR flagging

Authors and agents mark a PR as interaction/UI when it changes pointer, keyboard, drag, hover, Mount hosts, lobby/host flow, or BFF env defaults.

Preferred affordance: a checkbox in `.github/PULL_REQUEST_TEMPLATE.md`:

```markdown
- [ ] Interaction / UI (requires verify Interaction checklist)
```

If the template is omitted, the same expectation is stated in `AGENTS.md`.

### When the box is checked

Before claiming done:

1. Run the relevant unit/Scene suites for touched areas (`verification-before-completion`).
2. Run the `verify` skill **Interaction checklist** and note what was exercised.

### `verify` skill — Interaction checklist

Always documented in `.agents/skills/verify/SKILL.md`. Required only when the PR is flagged Interaction / UI.

1. **Host a table** with local defaults after migrate — create succeeds; not a generic “Couldn't reach the table.”
2. **Alt-hold** over a face-up board or hand card — inspect opens; release Alt — inspect closes.
3. **Drag a playable hand card** above the bar — after commit the hand no longer shows a duplicate tile while the flight plays.
4. **Deck builder hover** — move across two pool cards; preview art changes; no native title tooltip.
5. **Lobby with a pre-picked deck** (`/play?deck=…`) — Bring text shows that deck, Back returns to `/`, and no deck select is shown.

## Out of scope

- New Playwright (or other) CI browser matrix
- Cold-env CI job that unsets `WEB_DATABASE_URL` on every PR
- Relitigating engine rules coverage

## Implementation deliverables

1. This spec (committed).
2. `AGENTS.md` — behavior-not-presence rule; pointer to Interaction checklist; PR flag note.
3. `.agents/skills/verify/SKILL.md` — Interaction checklist section.
4. `.github/PULL_REQUEST_TEMPLATE.md` — Interaction / UI checkbox (optional but preferred).
5. Unit test follow-ups in the same policy PR: keep the listed behavior rows covered in product language.

## Success criteria

- Agents and humans have a single written policy for when outcome tests and live interaction checks are required.
- The failure classes listed above are covered by named product-behavior unit/Scene tests (or explicitly deferred with a pointer).
- Flagged interaction PRs cannot honestly claim “done” without the verify Interaction checklist.
