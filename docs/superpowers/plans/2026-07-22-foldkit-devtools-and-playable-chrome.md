# Foldkit DevTools + Playable Chrome Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land Foldkit DevTools MCP + vendored skills first, then fix inspect, HUD stack, prompt visibility, activate rejects, permanent badges/P/T, radial/selection, and Arena playable/zone outline chrome on PR #74.

**Architecture:** Tooling-first so `foldkit_*` MCP can observe live Model/Message history while fixing board bugs. Board work is pure Foldkit update/view + bitmap paint; payment stays engine-side (`take_action` + `settle_payment`). Wire still broadcasts redacted `pending_choice` to all seats — UI gates interactive prompts to `pending_choice.player === viewer`.

**Tech Stack:** Foldkit (`@foldkit/vite-plugin`, `@foldkit/devtools-mcp`), Vitest/Scene tests, Canvas bitmap paint, Tailwind HUD, Effect Schema messages, Bun.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-22-foldkit-devtools.md`, `docs/superpowers/specs/2026-07-20-battlefield.md`, `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md`
- Branch: `cursor/foldkit-migration-design-1ef0` (PR #74)
- Delivery order is mandatory: tooling → MCP-assisted investigations → chrome
- No Playwright CI matrix; no Foldkit git subtree; no restoring under-card name labels
- No new `ObjectView` counter kinds beyond `plus_counters` + `loyalty`
- Outcome tests in product language; Angular commit subjects; focused commits
- Interaction / UI PR — run verify Interaction checklist before claiming done
- Illegal printed activates with no wire list: select via `tapsForMana` or current legal activates; disabled wedges for tap-for-mana when unavailable; do not invent a client-only ability catalog

## File map

| File | Role |
|---|---|
| `client/package.json` | Add `@foldkit/devtools-mcp` devDependency |
| `client/vite.config.ts` | `foldkit({ devToolsMcpPort: 9988 })` |
| `.cursor/mcp.json` | Register `foldkit-devtools` MCP alongside `scryfall` |
| `AGENTS.md` | Note MCP needs open browser tab + port 9988 |
| `.agents/skills/{foldkit,generate-program,audit-program}/` | Vendored Foldkit skills (paths retargeted) |
| `client/app/board/html/discoverability.ts` | Remove nested `fixed` from toolbar children |
| `client/app/board/html/overlays.ts` | Single top-left toolbar; prompt gate |
| `client/app/board/html/prompts.ts` | Awaited-seat-only pending-choice formulators |
| `client/app/board/bitmap/mount.ts` | Resting layer paints full chrome (`paintCard`) |
| `client/app/board/bitmap/paint-cards.ts` | Keep badge/P/T/loyalty paint; drop dim-for-unplayable usage |
| `client/app/board/html/inspect.ts` / `keyboard-mount.ts` / `submodel.ts` | Live inspect fix |
| `client/app/board/geometry/radial.ts` / `html/activation-radial.ts` | Center + disabled wedges |
| `client/app/board/geometry/interaction.ts` | Select only activatable / tap-for-mana |
| `client/app/board/action/execution.ts` / `lib/wire/protoMap.ts` | Activate reject root-cause fix |
| `client/app/board/html/hand.ts` | GY purple / exile green playable outlines |
| `client/app/board/canvas/scene.ts` | Strip always-on seat strokes if still painted |
| `DESIGN.md` / `client/styles/global.css` | `graveyard-outline` / `exile-outline` tokens |
| `docs/client-canvas-map.md` | Note permanent chrome on bitmap layer |
| Spec design file | Status → Done at end |

---

### Task 1: Foldkit DevTools MCP + Vite port

**Files:**
- Modify: `client/package.json`
- Modify: `client/vite.config.ts`
- Modify: `.cursor/mcp.json`
- Modify: `AGENTS.md`
- Verify: `client/app/entry.ts` already has `devTools: { Message }`

**Interfaces:**
- Consumes: `@foldkit/vite-plugin` `foldkit({ devToolsMcpPort })`
- Produces: MCP server `foldkit-devtools` on relay port `9988`

- [ ] **Step 1: Install the MCP package**

```bash
cd client && bun add -d @foldkit/devtools-mcp
```

- [ ] **Step 2: Wire Vite + Cursor MCP**

In `client/vite.config.ts`, change `foldkit()` to:

```ts
foldkit({ devToolsMcpPort: 9988 }),
```

Merge into `.cursor/mcp.json` (keep `scryfall`):

```json
{
  "mcpServers": {
    "scryfall": {
      "type": "stdio",
      "command": "npx",
      "args": ["scryfall-mcp-server"],
      "env": {}
    },
    "foldkit-devtools": {
      "type": "stdio",
      "command": "npx",
      "args": ["@foldkit/devtools-mcp"],
      "env": {}
    }
  }
}
```

(If `npx @foldkit/devtools-mcp init` writes a different shape, prefer its recipe but keep `scryfall` and land the entry under `.cursor/mcp.json` for this repo.)

- [ ] **Step 3: Document in AGENTS.md**

Under Cursor Cloud / client commands, add: Foldkit DevTools MCP uses Vite relay port `9988`; `foldkit_list_runtimes` only sees a runtime while a browser tab has the app open (`devTools: { Message }` in `client/app/entry.ts`).

- [ ] **Step 4: Smoke (manual/agent)**

```bash
just dev
# open client tab, then from agent MCP:
# foldkit_list_runtimes → at least one runtime
```

Expected: runtime listed. If MCP client needs restart after config change, note that in the commit message body.

- [ ] **Step 5: Commit**

```bash
git add client/package.json client/bun.lock client/vite.config.ts .cursor/mcp.json AGENTS.md
git commit -m "build: wire Foldkit DevTools MCP on port 9988"
```

---

### Task 2: Vendor Foldkit agent skills

**Files:**
- Create: `.agents/skills/foldkit/SKILL.md`
- Create: `.agents/skills/generate-program/` (`SKILL.md`, `architecture.md`, `checklist.md`, `conventions.md`)
- Create: `.agents/skills/audit-program/SKILL.md`

**Interfaces:**
- Consumes: upstream https://github.com/foldkit/foldkit `skills/{foldkit,generate-program,audit-program}/`
- Produces: local skills that point at `client/node_modules/foldkit` (not `repos/foldkit/`)

- [ ] **Step 1: Copy upstream skills**

```bash
# from a sparse clone or curl of foldkit/foldkit@main skills/
mkdir -p .agents/skills
cp -R /tmp/foldkit-skills-probe/skills/foldkit .agents/skills/
cp -R /tmp/foldkit-skills-probe/skills/generate-program .agents/skills/
cp -R /tmp/foldkit-skills-probe/skills/audit-program .agents/skills/
```

(Re-clone if `/tmp` probe is gone — same sparse checkout as in design exploration.)

- [ ] **Step 2: Retarget “Where to look”**

In each vendored `SKILL.md`, replace guidance that requires `repos/foldkit/` with:

- Framework source / types: `client/node_modules/foldkit`
- Project Foldkit app: `client/app/`, `client/vite.config.ts`
- Do **not** instruct agents to `git subtree add` Foldkit into this repo

Keep architecture/conventions content that is still accurate.

- [ ] **Step 3: Commit**

```bash
git add .agents/skills/foldkit .agents/skills/generate-program .agents/skills/audit-program
git commit -m "docs(skills): vendor Foldkit foldkit/generate-program/audit-program"
```

Do **not** add entries to `skills-lock.json` (that file pins remote marketplace skills with hashes; these are vendored locally).

---

### Task 3: Top-left HUD toolbar (unstack legend + sound)

**Files:**
- Modify: `client/app/board/html/discoverability.ts`
- Modify: `client/app/board/html/overlays.ts` (toolbar already wraps children — keep one owner)
- Modify: `client/app/board/html/concede.ts` (confirm `top-md right-md`)
- Test: `client/app/board/html/discoverability.test.ts` and/or `surfaces.test.ts`

**Interfaces:**
- Consumes: `boardOverlays` toolbar `fixed top-md left-md flex …`
- Produces: legend toggle + sound as in-flow flex siblings (no nested `fixed` on the toggle)

- [ ] **Step 1: Failing test — legend toggle is not independently fixed**

Assert the Scene/HTML for `board-legend-toggle` does **not** carry `fixed top-md left-md` on its own wrapper when composed via `boardOverlays`, and that `board-sound-toggle` and `board-legend-toggle` both exist as siblings under the toolbar. Assert `concede` (or concede test id) uses right placement classes, not `left-md`.

- [ ] **Step 2: Run — expect FAIL**

```bash
cd client && bunx vitest run app/board/html/discoverability.test.ts app/board/html/surfaces.test.ts
```

- [ ] **Step 3: Fix discoverability**

In `discoverabilityView`, when rendering only the `?` button (and when rendering the button row with legend open), **remove** `fixed top-md left-md z-25` from the button wrapper — return an in-flow `pointer-events-none`/`auto` fragment the parent toolbar positions. Keep the legend **panel** as `fixed top-12 left-md` (dropdown under the toolbar). Keep the hint strip as its own bottom-left fixed band (not in the top toolbar).

`overlays.ts` already owns:

```ts
h.div(
  [h.Class("pointer-events-none fixed top-md left-md z-25 flex items-center gap-xs")],
  [discoverabilityView(board, state), soundToggleView(board)].filter((v): v is Html => v !== null),
),
```

Keep that as the single top-left cluster.

- [ ] **Step 4: Tests pass + commit**

```bash
cd client && bunx vitest run app/board/html/discoverability.test.ts app/board/html/surfaces.test.ts
git add client/app/board/html/discoverability.ts client/app/board/html/overlays.ts client/app/board/html/*.test.ts
git commit -m "fix(client): unstack top-left legend and sound toolbar"
```

---

### Task 4: Pending-choice prompts only for the awaited seat

**Files:**
- Modify: `client/app/board/html/overlays.ts`
- Modify: `client/app/board/html/prompts.ts` (guard inside `promptsView` / `pendingChoicePrompt`)
- Test: `client/app/board/scene.test.ts` and/or `html/surfaces.test.ts`

**Interfaces:**
- Consumes: `PendingChoiceView.player`, `VisibleState.viewer`
- Produces: interactive formulator DOM only when `pending.player === viewer`

- [ ] **Step 1: Failing Scene tests**

```ts
test("may_yes_no prompt mounts only for the awaited seat", () => {
  const pending = { kind: "may_yes_no", label: "Cast?", player: 0, source: 1 };
  // viewer 0 → prompt-yes exists
  // viewer 1 (still playing) → prompt-yes absent
  // spectator viewer 255 → prompt-yes absent
});
```

Use existing board Scene helpers from `scene.test.ts` / `surfaces.test.ts`.

- [ ] **Step 2: Run — expect FAIL (viewer 1 currently sees prompt)**

```bash
cd client && bunx vitest run app/board/scene.test.ts app/board/html/surfaces.test.ts
```

- [ ] **Step 3: Gate rendering**

In `overlays.ts` / `promptsView`:

```ts
function shouldShowPendingChoice(state: VisibleState): boolean {
  const pc = state.pending_choice;
  if (pc == null) return false;
  if (!isActivePlayer(state.players, state.viewer)) return false;
  return pc.player === state.viewer;
}
```

Call pending-choice formulators only when `shouldShowPendingChoice`. Keep client-local prompts (`xPrompt`, modal, cost picks, staged) for the local board model regardless of `pending_choice.player` (they are not shared state).

- [ ] **Step 4: Pass + commit**

```bash
cd client && bunx vitest run app/board/scene.test.ts app/board/html/surfaces.test.ts
git add client/app/board/html/overlays.ts client/app/board/html/prompts.ts client/app/board/scene.test.ts client/app/board/html/surfaces.test.ts
git commit -m "fix(client): show pending-choice prompts only to awaited seat"
```

---

### Task 5: Restore battlefield permanent chrome on bitmap layer

**Files:**
- Modify: `client/app/board/bitmap/mount.ts`
- Modify: `client/app/board/bitmap/paint-cards.ts` (only if `paintCard` needs split for highlight layering)
- Test: `client/app/board/bitmap/mount.test.ts`, `paint-cards.test.ts`
- Optional note: `docs/client-canvas-map.md`

**Interfaces:**
- Consumes: `paintCard(ctx, cam, card, cache, viewer)` (full chrome)
- Produces: resting layer with badges / P/T / loyalty / counters / damage

- [ ] **Step 1: Failing bitmap tests**

Assert `paintBitmapLayer` (or `paintCard` via mount) draws:

- P/T text `"2/2"` (or fillText spy) for a creature with `pt: "2/2"`
- Loyalty string for planeswalker `pt: "4"`
- Summoning-sick chip path when `summoningSick && !hasHaste`
- `+1` counter badge when `counters > 0`
- No resting name label under the card

Spy `fillText` / `fill` on a mock 2d context (existing mount test pattern).

- [ ] **Step 2: Run — expect FAIL (mount uses `paintCardArt`)**

```bash
cd client && bunx vitest run app/board/bitmap/mount.test.ts app/board/bitmap/paint-cards.test.ts
```

- [ ] **Step 3: Switch resting paint to full chrome**

In `paintBitmapLayer`:

```ts
import { paintAutoTapPreview, paintCard, paintCardTargetHighlight } from "./paint-cards";
// ...
paintCard(ctx, frame.camera, card, cache, frame.viewer);
```

If target highlight / auto-tap must stay above chrome, keep calling them after `paintCard` (current order). Ensure `paintCard` does not reintroduce resting name captions (names only in art-missing fallback inside the card face — acceptable).

- [ ] **Step 4: Pass + commit**

```bash
cd client && bunx vitest run app/board/bitmap/mount.test.ts app/board/bitmap/paint-cards.test.ts
git add client/app/board/bitmap/mount.ts client/app/board/bitmap/*.test.ts docs/client-canvas-map.md
git commit -m "fix(client): paint battlefield badges and P/T on resting layer"
```

---

### Task 6: Live Alt/Option inspect (MCP-assisted)

**Files:**
- Likely: `client/app/board/html/keyboard-mount.ts`, `inspect.ts`, `submodel.ts`, `overlays.ts`, `lib/deck-builder/card-hover-preview.ts`
- Test: `client/app/board/inspect-pile-concede.test.ts` + new Scene coverage for dock DOM

**Interfaces:**
- Consumes: `AltDown` / pin / `FetchInspectCard` / dock `cardHoverPreviewView({ mode: "dock" })`
- Produces: live hold-Alt → left dock + backdrop + oracle; release/Esc clears

- [ ] **Step 1: Reproduce with MCP**

With `just dev` + open game tab:

1. `foldkit_list_runtimes`
2. Hold Alt over a face-up permanent (or `foldkit_dispatch_message` `AltDown` if pointer hard)
3. `foldkit_list_messages` / `foldkit_get_model` path `game.board` — check `altDown`, `inspectPin`, `inspectCard`
4. Identify break: no AltDown → keyboard; pin null → hits; pin set but no fetch → command; fetch ok but no DOM → view/z-index/pointer-events

- [ ] **Step 2: Write failing regression for the failure class found**

Example if dock never mounts when pin set:

```ts
test("inspect dock renders when inspectPin and inspectCard are set", () => {
  // Scene.expect(testid inspect dock / backdrop).toExist()
});
```

If keyboard: extend `keyboard-mount` tests. If fetch: assert `FetchInspectCard` command from `AltDown` + pin.

- [ ] **Step 3: Minimal fix for the broken layer**

Fix only the failing layer; keep dock mode contract from remaining-bugs design (left art, right oracle/effects, backdrop, topmost).

- [ ] **Step 4: Pass unit/Scene + live Alt hold**

```bash
cd client && bunx vitest run app/board/inspect-pile-concede.test.ts app/board/html/keyboard-mount.test.ts
```

Live: Alt over board/hand/stack → dock; release/Esc → clear.

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(client): restore live Alt inspect dock"
```

---

### Task 7: Activate “That ability isn't available” (MCP-assisted)

**Files:**
- Likely: `client/lib/wire/protoMap.ts`, `client/app/board/action/execution.ts`, `client/app/board/submodel.ts`, possibly server/schema if id round-trip broken
- Test: `client/lib/wire/protoMap` tests (create if needed), `execution.test.ts`, Scene radial commit test

**Interfaces:**
- Consumes: `buildTakeActionIntent` / `intentEnvelopeToProto` / engine `TakeAction`
- Produces: legal listed activate commits without `CannotActivate` toast when payable

- [ ] **Step 1: Capture reject with MCP**

In a live game with a legal activate (e.g. untapped mana dork or Viscera Seer with a creature):

1. Open radial, commit wedge (or dispatch `RadialOptionPicked`)
2. Read `IntentRejected` reason via messages / board.reject
3. Classify: `CannotActivate` vs `UnknownAction` vs client-local `planCostPipeline`

- [ ] **Step 2: Failing regression for the classified bug**

**If UnknownAction / id:**

```ts
test("take_action intent coerces ActionView.id to BigInt for proto", () => {
  const env = intentEnvelopeToProto({
    table_id: "t",
    client_seq: 1,
    intent: buildTakeActionIntent(0, 42, null, 0, [], emptyCostPicks()),
  });
  // assert nested takeAction value.id === 42n
});
```

**If CannotActivate from payment:** add/extend client or engine test that `take_action` activate with auto_tap lands succeeds (engine already has coverage — find client gap that skips take_action or sends wrong fields).

**If client-local sacrifice `[]`:**

```ts
test("planCostPipeline ignores absent sacrifice_choices", () => {
  const action = { /* activate without sacrifice_choices key */ };
  expect(planCostPipeline(action, null, emptyCostPicks()).kind).toBe("run");
});
```

- [ ] **Step 3: Fix root cause minimally**

Do not pre-tap lands on the client. Prefer restoring correct `take_action` id / optional field presence / payment path.

- [ ] **Step 4: Pass + commit**

```bash
cd client && bunx vitest run app/board/action/execution.test.ts app/board/scene.test.ts
git commit -m "fix(client): allow legal radial activates to commit"
```

---

### Task 8: Radial centering on selected card

**Files:**
- Modify: `client/app/board/html/activation-radial.ts`
- Modify: `client/app/board/geometry/radial.ts` (radius helpers if needed)
- Test: `client/app/board/geometry/radial.test.ts` and/or Scene test

**Interfaces:**
- Consumes: `worldToScreen(camera, card.x + CARD_W/2, card.y + CARD_H/2)`
- Produces: SVG center equals selected card screen center (cluster/tapped layout card)

- [ ] **Step 1: Failing test for center**

Export a pure helper if needed:

```ts
export function radialScreenCenter(
  camera: Camera,
  card: Pick<RenderCard, "x" | "y" | "w" | "h">,
): { x: number; y: number } {
  return worldToScreen(camera, card.x + card.w / 2, card.y + card.h / 2);
}
```

Assert known camera + card → expected screen center. Scene: radial wrapper style `left`/`top` match (parse from view if exposed via test ids / data attributes).

- [ ] **Step 2: Align view**

In `activationRadialView`, set the SVG/container position from `radialScreenCenter` (not top-left of card). Account for `size = outer * 2 + 8` so the donut center is the card center (`left: center.x - size/2`, etc.).

- [ ] **Step 3: Pass + commit**

```bash
cd client && bunx vitest run app/board/geometry/radial.test.ts app/board/scene.test.ts
git commit -m "fix(client): center activation radial on selected card"
```

---

### Task 9: Selection + disabled radial wedges

**Files:**
- Modify: `client/app/board/geometry/interaction.ts`
- Modify: `client/app/board/geometry/radial.ts`
- Modify: `client/app/board/html/activation-radial.ts`
- Modify: `client/app/board/submodel.ts` (`commitRadialIndex` ignores disabled)
- Test: `interaction.test.ts`, `radial.test.ts`, `scene.test.ts`

**Interfaces:**
- Consumes: `ActionView[]`, `RenderCard.tapsForMana`
- Produces: `canSelectPermanent`; `RadialOption` with `disabled: boolean`

- [ ] **Step 1: Failing tests**

```ts
// selection
expect(resolveClick(..., vanillaLandNoTap, ctx)).toEqual({ kind: "none" }); // no activates, no tapsForMana
expect(resolveClick(..., tapLand, ctx)).toEqual({ kind: "select", id });
expect(resolveClick(..., creatureWithLegalActivate, ctx)).toEqual({ kind: "select", id });

// radial: tap-for-mana present but disabled when tapped or !canAct
const opts = radialOptions(id, actions, true, true, true);
expect(opts.find(o => o.kind === "tap_for_mana")?.disabled).toBe(true);

// commit ignores disabled
// RadialOptionPicked on disabled index → no SubmitIntent
```

- [ ] **Step 2: Implement select gate**

```ts
export function canSelectPermanent(
  objectId: number,
  tapsForMana: boolean,
  actions: ActionView[] | undefined,
): boolean {
  if (tapsForMana) return true;
  return (actions ?? []).some(
    (a) => a.section === "battlefield" && a.object === objectId && a.kind === "activate",
  );
}
```

Use in `resolveClick` before `{ kind: "select" }`. Without a wire list of illegal printed activates, permanents with only currently illegal activates are not selectable this wave (document in commit body / design Further Notes).

- [ ] **Step 3: Disabled wedges**

Extend `RadialOption`:

```ts
| { kind: "tap_for_mana"; label: string; disabled: boolean }
| { kind: "action"; action: ActionView; label: string; disabled: boolean }
```

Always include tap-for-mana when `tapsForMana`; `disabled: !canAct || tapped`. Legal activate actions: `disabled: false`. Paint disabled wedges muted; `commitRadialIndex` no-ops when `opt.disabled`.

- [ ] **Step 4: Pass + commit**

```bash
cd client && bunx vitest run app/board/geometry/interaction.test.ts app/board/geometry/radial.test.ts app/board/scene.test.ts
git commit -m "feat(client): gate permanent select and disable illegal radial wedges"
```

---

### Task 10: Arena playable borders + zone outlines

**Files:**
- Modify: `DESIGN.md`, `client/styles/global.css`
- Modify: `client/app/board/bitmap/paint-cards.ts` / callers that pass `dim` / seat stroke
- Modify: `client/app/board/canvas/scene.ts` (remove always-on seat borders if present)
- Modify: `client/app/board/html/hand.ts` (hand / GY / exile tile outlines)
- Modify: `client/app/board/html/discoverability.ts` legend items if colors change
- Test: hand / paint / scene tests

**Interfaces:**
- Consumes: playable action lists; `isCommander`; zone section
- Produces: playable border; commander gold outline; GY purple; exile green; no dim veil for unplayable

- [ ] **Step 1: Tokens**

In `DESIGN.md` + CSS variables:

```md
graveyard-outline: "#7B5CFF"   # pick distinct purple; sync legend
exile-outline: "#3DDC97"       # pick distinct green; sync legend
```

Document: playable border ≠ commander gold ≠ zone outlines; dim-for-unplayable retired.

- [ ] **Step 2: Failing tests**

- Castable hand tile has playable border class/style; uncastable hand tile does not
- Battlefield permanent with non-tap activate gets playable stroke; tap-only land does not
- Commander still gold outline (`#E9B84A` / `commander-gold`)
- GY bar playable → purple outline; exile playable → green
- Resting paint does not apply `DIM_CARD_VEIL` for “unplayable”
- No default controller seat stroke on every permanent (owner-strip for control change may remain per existing `drawStatusBadges`)

- [ ] **Step 3: Implement**

Stop passing `dim: true` for unplayable resting cards. Apply playable outline via `paintCard` `options.outline` / hand tile classes. Keep target highlights + combat arrows.

- [ ] **Step 4: Pass + commit**

```bash
cd client && bunx vitest run app/board/html/hand.ts app/board/bitmap/paint-cards.test.ts app/board/canvas/scene.test.ts
git commit -m "feat(client): Arena playable borders and GY/exile outlines"
```

---

### Task 11: Spec Done + Interaction checklist

**Files:**
- Modify: `docs/superpowers/specs/2026-07-22-foldkit-devtools.md`, `docs/superpowers/specs/2026-07-20-battlefield.md`, `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md` (Status/current behavior notes)
- Verify: `.agents/skills/verify/SKILL.md` Interaction checklist items relevant to this PR

**Interfaces:**
- Consumes: all prior tasks green
- Produces: design marked Done; live checklist notes for PR

- [ ] **Step 1: Mark design Done**

Set **Status:** Done. Add a one-line note if illegal printed activates remain unselectable without wire (Task 9).

- [ ] **Step 2: Run client verification**

```bash
just client-check
```

Expected: format/lint/typecheck/tests pass (or project’s `just client-check` equivalent).

- [ ] **Step 3: Interaction checklist (live)**

Exercise at least: Alt inspect; top-left `?` + Sound side-by-side; concede top-right; pending prompt only on deciding seat (2p); radial activate succeeds; badges/P/T visible; playable borders.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-07-22-foldkit-devtools.md docs/superpowers/specs/2026-07-20-battlefield.md docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md
git commit -m "docs: mark foldkit playable chrome design done"
```

---

## Self-review

**Spec coverage**

| Spec section | Task |
|---|---|
| DevTools MCP + Vite port | 1 |
| Vendored skills | 2 |
| Top-left HUD stack | 3 |
| Prompt visibility | 4 |
| Permanent chrome badges/P/T/counters/PW | 5 |
| Live inspect | 6 |
| Activate CannotActivate | 7 |
| Radial centering | 8 |
| Selection + disabled wedges | 9 |
| Arena borders + GY/exile outlines + DESIGN | 10 |
| Testing / success / Interaction checklist | 11 (+ per-task tests) |
| Out of scope (subtree, Playwright, name labels, new counters) | Global Constraints |

**Placeholders:** none intentional — Tasks 6–7 are investigation-shaped but require a classified regression before fix.

**Type consistency:** `RadialOption.disabled`, `canSelectPermanent`, `shouldShowPendingChoice`, `paintCard` on resting layer, `devToolsMcpPort: 9988` used consistently.
)
