# Client Interaction Test Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the client interaction test policy — product-behavior unit/Scene coverage for known gaps, plus AGENTS/`verify`/PR-template docs so flagged interaction PRs run a live checklist.

**Architecture:** Docs-discipline process (no new E2E framework). Unit/Scene tests assert user-visible outcomes in product language. Live Interaction checklist lives in the `verify` skill and is required only when a PR marks Interaction / UI.

**Tech Stack:** Vitest + Foldkit Scene, Markdown policy in `AGENTS.md` / `.agents/skills/verify/SKILL.md`, optional GitHub PR template.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md`
- Branch: `cursor/foldkit-migration-design-1ef0` (same Foldkit cutover PR)
- Product language only — no new “Solid parity” framing in test names or docs
- No Playwright / new CI browser matrix
- Lobby Bring UX redesign is out of scope (binding assertion only)
- Angular commit subjects; keep commits focused (`docs:`, `test:`)

## File map

| File | Role |
|---|---|
| `client/app/board/inspect-pile-concede.test.ts` | Rename migration-framed Alt comments/titles |
| `client/app/board/hand-drag.test.ts` | Rename migration-framed drag-play test titles |
| `client/app/shell/lobby/entry.test.ts` | Add ≥2-deck selected-option binding Scene test |
| `AGENTS.md` | Behavior-not-presence rule + Interaction / UI flag note |
| `.agents/skills/verify/SKILL.md` | Interaction checklist section |
| `.github/PULL_REQUEST_TEMPLATE.md` | Create — Interaction / UI checkbox |
| `docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md` | Status → Done after landing |

---

### Task 1: Rename migration-framed board test wording

**Files:**
- Modify: `client/app/board/inspect-pile-concede.test.ts`
- Modify: `client/app/board/hand-drag.test.ts`
- Test: same files (rename only; behavior unchanged)

**Interfaces:**
- Consumes: existing tests
- Produces: product-language titles/comments only

- [ ] **Step 1: Update inspect Alt section header and any “Solid” wording**

In `client/app/board/inspect-pile-concede.test.ts`, replace:

```ts
// ── AltDown / AltUp (Solid parity: hold Alt over a card to pin; release clears) ─
```

with:

```ts
// ── AltDown / AltUp (hold Alt over a card to pin; release clears) ─
```

Search the file for `Solid` / `seedDrop` / `parity` and remove migration framing from comments only. Do not change assertions.

- [ ] **Step 2: Rename hand-drag test titles**

In `client/app/board/hand-drag.test.ts`, rename:

```ts
it("hides the hand card and seeds a flight when drag-play commits (Solid seedDrop)", () => {
```

to:

```ts
it("hides the hand card and seeds a flight when drag-play commits", () => {
```

Search the file for `Solid` / `seedDrop` in strings/comments and rewrite to product language if present.

- [ ] **Step 3: Run the suites**

```bash
cd client && bunx vitest run app/board/inspect-pile-concede.test.ts app/board/hand-drag.test.ts
```

Expected: all tests PASS (rename-only).

- [ ] **Step 4: Commit**

```bash
git add client/app/board/inspect-pile-concede.test.ts client/app/board/hand-drag.test.ts
git commit -m "test(client): use product language in interaction tests"
```

---

### Task 2: Lobby selected-deck binding Scene test

**Files:**
- Modify: `client/app/shell/lobby/entry.test.ts`
- Modify (only if test fails): `client/app/shell/lobby/view.ts` (`deckPicker`)
- Test: `client/app/shell/lobby/entry.test.ts`

**Interfaces:**
- Consumes: `lobbyView` / `deckPicker` — `h.Value` + per-option `h.Selected(deck.id === selected)` where `selected = model.selectedDeckId ?? decks[0]?.id`
- Produces: Scene assertion that among ≥2 decks, the pre-picked non-first deck’s `<option>` is selected

- [ ] **Step 1: Write the failing (or contract) test**

Add to `client/app/shell/lobby/entry.test.ts` after the existing `"shows deck picker once decks resolve"` test:

```ts
test("entry select shows the pre-picked deck among multiple decks", () => {
  const other = {
    id: 9,
    name: "Tokens",
    commander: "rhys",
    commander_print: undefined as string | undefined,
  };
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        lobby: { ...initialLobbySlice(), selectedDeckId: 9 },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck, other] },
        },
      }),
    ),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"] option[value="9"][selected]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"] option[value="7"][selected]')).toBeAbsent(),
  );
});
```

`deck` is already `{ id: 7, name: "Superfriends", ... }` at the top of the file — keep using it as the first list entry so a wrong default to `decks[0]` would select `7`.

- [ ] **Step 2: Run the new test**

```bash
cd client && bunx vitest run app/shell/lobby/entry.test.ts -t "pre-picked deck among multiple"
```

Expected:
- PASS if `deckPicker` already binds `Selected` correctly — keep the test (encodes the contract).
- FAIL if the wrong option is selected — continue to Step 3.

- [ ] **Step 3: Fix `deckPicker` only if Step 2 failed**

In `client/app/shell/lobby/view.ts`, `deckPicker` must:

```ts
const selected = model.selectedDeckId ?? decks[0]?.id ?? "";
// ...
h.Value(String(selected)),
// each option:
h.Selected(deck.id === selected),
```

Do not redesign Bring UX. Only fix binding so the selected option matches `selectedDeckId` when set.

Re-run Step 2 until PASS.

- [ ] **Step 4: Run the full lobby entry suite**

```bash
cd client && bunx vitest run app/shell/lobby/entry.test.ts
```

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/lobby/entry.test.ts
# include view.ts only if Step 3 changed it
git add client/app/shell/lobby/view.ts 2>/dev/null || true
git commit -m "test(client): assert lobby select matches pre-picked deck"
```

---

### Task 3: AGENTS.md behavior + flag rules

**Files:**
- Modify: `AGENTS.md` (Coding standards section, near the existing Client UI surface rule ~line 64)

**Interfaces:**
- Consumes: existing surface + verify bullets
- Produces: two new coding-standard bullets (behavior outcomes; Interaction / UI flag)

- [ ] **Step 1: Insert policy bullets after the Client UI surface rule**

In `AGENTS.md`, under **Coding standards**, immediately after the bullet that starts with `**Client UI: every surface gets a Scene test.**`, add:

```markdown
- **Client interaction: assert outcomes, not only presence.** When changing pointer, keyboard, hover, drag, Mount hosts, lobby/host flow, or BFF env defaults, add or extend a unit/Scene test for the user-visible result (pin set, tile hidden, selected deck matches, art URL swapped, default URL works). Do not frame tests as migration/"parity" checks — name the product behavior. See [`docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md`](docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md).
- **Interaction / UI PRs.** Check the PR template box when the change touches those surfaces. Before claiming done, run the Interaction checklist in `.agents/skills/verify/SKILL.md` (in addition to `verification-before-completion`).
```

Keep the existing **Verify before claiming done** bullet; do not delete it.

- [ ] **Step 2: Sanity-check the markdown renders as a list**

Open `AGENTS.md` and confirm the new bullets are siblings under Coding standards (same `- **...**` indentation as neighbors).

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "docs: require outcome tests for client interaction changes"
```

---

### Task 4: `verify` skill Interaction checklist

**Files:**
- Modify: `.agents/skills/verify/SKILL.md`

**Interfaces:**
- Consumes: existing verify seating / browser guidance
- Produces: new `## Interaction checklist` section required when PR is flagged

- [ ] **Step 1: Append the Interaction checklist section**

Add at the end of `.agents/skills/verify/SKILL.md` (after Gotchas):

```markdown
## Interaction checklist

Required before claiming done when the PR is flagged **Interaction / UI**
(PR template checkbox / AGENTS.md). Always available otherwise.

Drive via browser (`agent-browser`) and/or BFF curls against the running
`just dev` stack. Note which items you exercised in the PR or agent summary.

1. **Host a table** with local defaults after `just migrate` / `just client-migrate` — create succeeds; not a generic “Couldn't reach the table.”
2. **Alt-hold** over a face-up board or hand card — inspect opens; release Alt — inspect closes.
3. **Drag a playable hand card** above the bar — after commit the hand no longer shows a duplicate tile while the flight plays.
4. **Deck builder hover** — move across two pool cards; preview art changes; no native title tooltip.
5. **Lobby with a pre-picked deck** (`/play?deck=…`) — shown deck matches the pick (select value today; Bring text/card once that UX lands).
```

- [ ] **Step 2: Commit**

```bash
git add .agents/skills/verify/SKILL.md
git commit -m "docs(verify): add Interaction checklist for flagged PRs"
```

---

### Task 5: PR template checkbox

**Files:**
- Create: `.github/PULL_REQUEST_TEMPLATE.md`

**Interfaces:**
- Consumes: GitHub PR template convention
- Produces: visible Interaction / UI checkbox on new PRs

- [ ] **Step 1: Create the template**

Create `.github/PULL_REQUEST_TEMPLATE.md`:

```markdown
## Summary

<!-- What changed and why -->

## Checklist

- [ ] Interaction / UI (requires verify Interaction checklist — see `.agents/skills/verify/SKILL.md`)
- [ ] Client surfaces touched have Scene `data-testid` coverage (`AGENTS.md`)
- [ ] Outcome tests added/updated for interaction changes (not presence-only)

## Test plan

- [ ] …
```

Keep it short — do not invent a full review ritual beyond the policy.

- [ ] **Step 2: Commit**

```bash
git add .github/PULL_REQUEST_TEMPLATE.md
git commit -m "docs: add PR template Interaction / UI checkbox"
```

---

### Task 6: Mark design spec Done

**Files:**
- Modify: `docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md`

**Interfaces:**
- Consumes: landed Tasks 1–5
- Produces: `Status: Done`

- [ ] **Step 1: Flip status**

Change the header from:

```markdown
**Status:** Draft
```

to:

```markdown
**Status:** Done
```

- [ ] **Step 2: Commit and push**

```bash
git add docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md
git commit -m "docs: mark client interaction test policy design done"
git push -u origin cursor/foldkit-migration-design-1ef0
```

- [ ] **Step 3: Final verification**

```bash
cd client && bunx vitest run app/board/inspect-pile-concede.test.ts app/board/hand-drag.test.ts app/shell/lobby/entry.test.ts
```

Expected: all PASS.

---

## Spec coverage check (self-review)

| Spec requirement | Task |
|---|---|
| Product language (no migration framing) | Task 1 |
| Lobby ≥2-deck binding assertion | Task 2 |
| AGENTS behavior-not-presence + flag | Task 3 |
| verify Interaction checklist | Task 4 |
| PR template checkbox | Task 5 |
| Spec status Done | Task 6 |
| Out of scope: Playwright, cold-env CI, Bring UX redesign | Not tasked |

No placeholders remain in task steps.
