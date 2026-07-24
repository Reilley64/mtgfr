# Hand Discard Select → Confirm Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hand discard (local cost + engine `discard` / `may_discard`) selects with raise + Llanowar chrome, toggles off on re-click, and only commits on Confirm.

**Architecture:** Reuse GY-exile accumulate for local `discardPick.picks.discard_cost` + new `DiscardCostConfirmed`. Engine discard drops one-click (`pendingHandPickOneClick` + `submitPendingHandPick`) so all counts toggle `promptDraft` card-pick and Confirm via existing `trySubmitReadyPendingDraft`. Hand tiles gain `discardSelected` paint separate from island-blue legal ring.

**Tech Stack:** Foldkit Scene/unit tests (Vitest), board `Message` constructors, Tailwind design tokens (`ring-llanowar` / `border-llanowar`).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-24-hand-discard-select-confirm-design.md`
- TDD: failing test before production code for each behavior change
- Put-from-hand / face-down / put-on-top stay one-click where they already are
- Off-board `discard-pick-aim` button strip stays one-shot buttons
- Angular commit subjects (`feat(client):`, `test(client):`, `docs:`)
- Branch: `cursor/discard-confirm-select-b23c`

## File map

| File | Responsibility |
|------|----------------|
| `client/app/board/action/targeting.ts` | `pendingHandPickOneClick` excludes discard kinds |
| `client/app/board/action/targeting.test.ts` | Unit coverage for one-click false on discard |
| `client/app/board/messages.ts` | Add `DiscardCostConfirmed` |
| `client/app/board/submodel.ts` | Toggle local discard; confirm settles; engine discard always toggles |
| `client/app/board/scene.test.ts` | Outcome tests for toggle / confirm / no one-click |
| `client/app/board/html/prompts.ts` | `discard-cost-aim` Confirm + count; discard aims always show Confirm |
| `client/app/board/html/surfaces.test.ts` | Scene: Confirm disabled/enabled, selected chrome |
| `client/app/board/html/overlays.ts` | Pass `discardSelectedIds` into hand |
| `client/app/board/html/hand.ts` | `discardSelected` raise + Llanowar ring |
| `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md` | Behavior truth |

---

### Task 1: Engine discard never one-clicks

**Files:**
- Modify: `client/app/board/action/targeting.ts` (`pendingHandPickOneClick`)
- Modify: `client/app/board/submodel.ts` (`submitPendingHandPick`)
- Test: `client/app/board/action/targeting.test.ts`
- Test: `client/app/board/scene.test.ts` (update existing one-click discard test)

**Interfaces:**
- Consumes: `PendingChoiceView` discard / may_discard
- Produces: `pendingHandPickOneClick` returns `false` for those kinds; `submitPendingHandPick` toggles draft for discard instead of submitting

- [ ] **Step 1: Write failing unit test**

In `client/app/board/action/targeting.test.ts`, import `pendingHandPickOneClick` and add:

```ts
describe("pendingHandPickOneClick", () => {
  it("is false for discard and may_discard at any count", () => {
    expect(
      pendingHandPickOneClick({
        kind: "discard",
        player: 0,
        count: 1,
        items: [{ id: 1, label: "A" }],
      }),
    ).toBe(false);
    expect(
      pendingHandPickOneClick({
        kind: "may_discard",
        player: 0,
        items: [{ id: 1, label: "A" }],
      }),
    ).toBe(false);
  });

  it("stays true for put_land_from_hand", () => {
    expect(
      pendingHandPickOneClick({
        kind: "put_land_from_hand",
        player: 0,
        items: [{ id: 1, label: "Forest" }],
      }),
    ).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/board/action/targeting.test.ts -t "pendingHandPickOneClick"`

Expected: FAIL — discard count 1 currently returns `true`.

- [ ] **Step 3: Minimal implementation in targeting.ts**

Change `pendingHandPickOneClick` so discard kinds never one-click:

```ts
export function pendingHandPickOneClick(pc: PendingChoiceView | null | undefined): boolean {
  if (pc == null || !isPendingHandPick(pc)) return false;
  if (pc.kind === "discard" || pc.kind === "may_discard") return false;
  if (
    pc.kind === "put_land_from_hand" ||
    pc.kind === "put_creature_from_hand" ||
    pc.kind === "cast_creature_face_down"
  ) {
    return true;
  }
  if (pc.kind === "put_from_hand_on_top") return pc.count === 1;
  return false;
}
```

- [ ] **Step 4: Update scene test that expects one-click discard submit**

Replace `test("HandActionActivated during pending discard submits discard intent", …)` in `scene.test.ts` with toggle behavior:

```ts
test("HandActionActivated during pending discard toggles card-pick draft", () => {
  // same fixtures as before (a, b, pending count 1, fodderAction)
  const [next, commands] = updateBoard(
    initialBoardModel(),
    HandActionActivated({ action: fodderAction, x: 400, y: 200 }),
    gameFold,
    "T1",
  );
  expect(commands).toHaveLength(0);
  expect(next.promptDraft).toEqual({ kind: "card-pick", picked: [11], filter: "" });
});

test("HandActionActivated during pending discard toggles selection off", () => {
  // same fixtures
  const board = {
    ...initialBoardModel(),
    promptDraft: { kind: "card-pick" as const, picked: [11], filter: "" },
  };
  const [next, commands] = updateBoard(
    board,
    HandActionActivated({ action: fodderAction, x: 400, y: 200 }),
    gameFold,
    "T1",
  );
  expect(commands).toHaveLength(0);
  expect(next.promptDraft).toEqual({ kind: "card-pick", picked: [], filter: "" });
});
```

- [ ] **Step 5: Run scene tests — expect FAIL on toggle (still submits)**

Run: `cd client && bunx vitest run app/board/scene.test.ts -t "pending discard"`

Expected: FAIL — still emits discard intent / clears draft incorrectly.

- [ ] **Step 6: Fix submitPendingHandPick**

In `submodel.ts`, delete the early return:

```ts
if (pc.kind === "discard" && pc.count === 1) {
  return [
    { ...idle, promptDraft: null, pendingChoiceKey: null },
    boardIntentSubmit(tableId, choiceIntent(pc, { kind: "discard", cards: [objectId] })),
  ];
}
```

so discard falls through to `togglePendingObjectAimPick`.

- [ ] **Step 7: Run tests — expect PASS**

Run:

```bash
cd client && bunx vitest run app/board/action/targeting.test.ts -t "pendingHandPickOneClick" app/board/scene.test.ts -t "pending discard"
```

Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add client/app/board/action/targeting.ts client/app/board/action/targeting.test.ts client/app/board/submodel.ts client/app/board/scene.test.ts
git commit -m "feat(client): make engine hand discard accumulate before confirm"
```

---

### Task 2: Local discard cost toggles then confirms

**Files:**
- Modify: `client/app/board/messages.ts`
- Modify: `client/app/board/submodel.ts` (`HandActionActivated` discardPick branch, `DiscardChosen`, new `DiscardCostConfirmed`)
- Modify: `client/app/board/html/prompts.ts` (`discard-cost-aim` HUD)
- Test: `client/app/board/scene.test.ts`
- Test: `client/app/board/html/surfaces.test.ts`

**Interfaces:**
- Consumes: `CostPickState.picks.discard_cost`
- Produces: `DiscardCostConfirmed` message; toggle updates `discard_cost` without settling; confirm calls `continueAfterCostPick` with `discard_settled: true`

- [ ] **Step 1: Write failing scene tests**

Update `HandActionActivated during discardPick settles the discard cost` → toggle:

```ts
test("HandActionActivated during discardPick toggles discard_cost selection", () => {
  // same discardPick fixtures as existing test
  const [next, commands] = updateBoard(
    board,
    HandActionActivated({ action: fodderAction, x: 400, y: 200 }),
    gameFold,
    "T1",
  );
  expect(commands).toHaveLength(0);
  expect(next.discardPick).not.toBeNull();
  expect(next.discardPick?.picks.discard_cost).toEqual([11]);
  expect(next.discardPick?.picks.discard_settled).toBe(false);
});

test("DiscardCostConfirmed settles local discard cost when one card selected", () => {
  const board: BoardModel = {
    ...initialBoardModel(),
    discardPick: {
      action: castAction,
      card: caster,
      dropSeed: { x: 0, y: 0 },
      screenOrigin: { x: 0, y: 0 },
      picks: { ...emptyCostPicks(), discard_cost: [11] },
    },
  };
  const [next, commands] = updateBoard(board, DiscardCostConfirmed(), gameFold, "T1");
  expect(next.discardPick).toBeNull();
  expect(commands).toHaveLength(1);
  expect(intentFromCommand(commands[0])).toMatchObject({
    kind: "take_action",
    id: 50,
    discard_cost: [11],
  });
});
```

Import `DiscardCostConfirmed` once added (test will fail to compile until Step 3 — write the test assuming the export).

Add Scene assertion in `surfaces.test.ts` for the existing in-hand discard cost case:

```ts
Scene.expect(Scene.testId("discard-cost-aim")).toExist(),
Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
Scene.expect(Scene.testId("discard-cost-count")).toHaveText("0 / 1 selected"),
```

And a second Scene with `picks: { ...emptyCostPicks(), discard_cost: [11] }`:

```ts
Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
Scene.expect(Scene.testId("discard-cost-count")).toHaveText("1 / 1 selected"),
```

- [ ] **Step 2: Run tests — expect FAIL**

Run:

```bash
cd client && bunx vitest run app/board/scene.test.ts -t "discardPick|DiscardCostConfirmed" app/board/html/surfaces.test.ts -t "discard cost"
```

Expected: FAIL (settles on click / no Confirm / no count).

- [ ] **Step 3: Add message**

In `messages.ts` next to `GyExileConfirmed`:

```ts
/** Confirm local discard-cost draft (`discardPick.picks.discard_cost`). */
export const DiscardCostConfirmed = m("DiscardCostConfirmed");
```

Add to the board message union / export list where `GyExileConfirmed` is listed.

- [ ] **Step 4: Toggle + confirm in submodel**

Replace discardPick settle in the hand-drop / `HandActionActivated` path (~1315) with toggle:

```ts
if (model.discardPick != null) {
  const choices = model.discardPick.action.discard_choices ?? [];
  const objectId = action.object;
  if (objectId == null || !choices.includes(objectId)) {
    return [{ ...model, handDrag: null, hoverActionId: null }, []];
  }
  const current = model.discardPick.picks.discard_cost;
  const next = current.includes(objectId)
    ? current.filter((id) => id !== objectId)
    : current.length >= 1
      ? current
      : [...current, objectId];
  return [
    {
      ...model,
      handDrag: null,
      hoverActionId: null,
      discardPick: {
        ...model.discardPick,
        picks: { ...model.discardPick.picks, discard_cost: next, discard_settled: false },
      },
    },
    [],
  ];
}
```

In `DiscardChosen` when `discardPick != null`, toggle the first id the same way (do not settle).

Add case:

```ts
case "DiscardCostConfirmed": {
  const pick = model.discardPick;
  if (pick == null) return [model, []];
  const selected = pick.picks.discard_cost;
  if (selected.length !== 1) return [model, []];
  const picks: CostPicks = { ...pick.picks, discard_cost: selected, discard_settled: true };
  return continueAfterCostPick(
    { ...model, discardPick: null },
    fold,
    tableId,
    pick.action,
    pick.card,
    picks,
    pick.dropSeed,
    pick.screenOrigin,
  );
}
```

Wire `DiscardCostConfirmed` through `update.ts` / board message fold the same way as `GyExileConfirmed` if that file lists messages explicitly.

- [ ] **Step 5: Dock Confirm on discard-cost-aim**

In `prompts.ts` discard-cost branch (~2821), replace coach-only HUD with:

```ts
const selected = board.discardPick.picks.discard_cost;
const ready = selected.length === 1;
return h.div(
  [
    h.DataAttribute("testid", "discard-cost-aim"),
    h.Style({ bottom: `${HAND_BAR_H + 12}px` }),
    h.Class(
      "pointer-events-auto fixed left-1/2 z-30 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
    ),
  ],
  [
    h.div([h.Class("pointer-events-none")], ["Click a card in your hand to discard"]),
    h.div(
      [h.DataAttribute("testid", "discard-cost-count"), h.Class("pointer-events-none text-caption text-mist")],
      [`${selected.length} / 1 selected`],
    ),
    h.div(
      [h.Class("flex flex-wrap justify-center gap-2")],
      [
        h.button(
          [
            h.Type("button"),
            h.DataAttribute("testid", "prompt-submit"),
            h.OnClick(DiscardCostConfirmed()),
            h.Disabled(!ready),
            h.Class(
              ready
                ? "cursor-pointer rounded-hud bg-llanowar px-3 py-1 text-body text-snow hover:bg-llanowar/90"
                : "cursor-not-allowed rounded-hud bg-glass px-3 py-1 text-body text-mist",
            ),
          ],
          ["Confirm"],
        ),
        cancelButton(),
      ],
    ),
  ],
);
```

Import `DiscardCostConfirmed` from messages.

- [ ] **Step 6: Run tests — expect PASS**

Run:

```bash
cd client && bunx vitest run app/board/scene.test.ts -t "discardPick|DiscardCostConfirmed" app/board/html/surfaces.test.ts -t "discard cost"
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add client/app/board/messages.ts client/app/board/submodel.ts client/app/update.ts client/app/board/html/prompts.ts client/app/board/scene.test.ts client/app/board/html/surfaces.test.ts
git commit -m "feat(client): confirm local discard cost after hand selection"
```

---

### Task 3: Hand selected chrome (raise + Llanowar)

**Files:**
- Modify: `client/app/board/html/hand.ts`
- Modify: `client/app/board/html/overlays.ts`
- Test: `client/app/board/html/surfaces.test.ts` (or a focused hand Scene test)

**Interfaces:**
- Consumes: `discardSelectedIds: ReadonlySet<number> | null` on `handView` inputs
- Produces: tile arg `discardSelected: boolean` → raised face + `ring-2 ring-llanowar`

- [ ] **Step 1: Write failing Scene test for selected chrome**

Add test that renders overlays with `discardPick.picks.discard_cost: [11]` and legal choice in hand, then assert the face/container class includes `ring-llanowar` (use existing `className` helper in `surfaces.test.ts` on `hand-card-face-11` parent, or assert a `data-discard-selected="1"` attribute if class inspection is awkward — prefer attribute for stability):

```ts
test("selected discard-cost hand card paints Llanowar selected chrome", () => {
  // fodder id 11 in hand, discardPick with discard_cost: [11]
  overlayScene(
    overlayModel(...),
    resolveBoardCardArtMounts(), // if art mounts present
    Scene.expect(Scene.testId("hand-card-11")).toExist(),
    // After Step 3, face carries data-discard-selected="1"
    Scene.expect(Scene.testId("hand-card-face-11")).toExist(),
  );
  // Plus a direct class/attr check via find + className helper:
  // expect(className(face)).toContain("ring-llanowar");
});
```

If Scene cannot reach the face class cleanly, assert `data-discard-selected="1"` on `hand-card-face-${id}` in the test after adding that attribute in Step 3.

- [ ] **Step 2: Run — expect FAIL**

Run: `cd client && bunx vitest run app/board/html/surfaces.test.ts -t "Llanowar selected"`

Expected: FAIL

- [ ] **Step 3: Implement hand paint + wire selected ids**

In `overlays.ts` `handView` call:

```ts
discardCostIds: (() => {
  if (board.discardPick != null) return new Set(board.discardPick.action.discard_choices ?? []);
  const pending = pendingHandPickIds(state.pending_choice, state);
  return pending != null ? pending : null;
})(),
discardSelectedIds: (() => {
  if (board.discardPick != null) return new Set(board.discardPick.picks.discard_cost);
  if (
    state.pending_choice != null &&
    (state.pending_choice.kind === "discard" || state.pending_choice.kind === "may_discard") &&
    board.promptDraft?.kind === "card-pick"
  ) {
    return new Set(board.promptDraft.picked);
  }
  return null;
})(),
```

In `hand.ts`:

- Add `discardSelectedIds?: ReadonlySet<number> | null` to inputs (default null).
- Add `discardSelected?: boolean` to `tile` args.
- Face chrome:

```ts
const faceChromeClass = [
  "relative origin-bottom rounded-game",
  discardSelected
    ? "ring-2 ring-llanowar shadow-[0_0_12px_rgba(47,125,70,0.55)]"
    : discardSelectable
      ? "ring-2 ring-island-blue shadow-[0_0_12px_rgba(74,158,255,0.45)]"
      : barZoneAura(zone, playable),
].filter((v) => v !== "").join(" ");
```

- Raise when selected (always apply raise transform, not only hover):

```ts
const faceClass = [
  "pointer-events-none absolute top-0 right-0 transition-transform duration-[120ms] ease-state",
  discardSelected
    ? "z-30 [transform:translateY(var(--raise-y))]"
    : "group-hover/hand-tile:z-30 group-hover/hand-tile:[transform:translateY(var(--raise-y))]",
].join(" ");
```

- Hit height when selected uses raised hit height at rest (set height style to raised when selected, or keep CSS group trick via a selected class on the group).

- On face: `h.DataAttribute("discard-selected", discardSelected ? "1" : "0")` when `discardSelectable || discardSelected`.

- Slot mapping: `discardSelected: discardSelectedIds?.has(c.id) ?? false`.

- [ ] **Step 4: Run Scene test — expect PASS**

Run: `cd client && bunx vitest run app/board/html/surfaces.test.ts -t "Llanowar selected|discard cost"`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/hand.ts client/app/board/html/overlays.ts client/app/board/html/surfaces.test.ts
git commit -m "feat(client): raise and green-ring selected discard hand cards"
```

---

### Task 4: Engine discard Scene + prompts coach + behavior spec

**Files:**
- Modify: `client/app/board/html/surfaces.test.ts`
- Modify: `client/app/board/html/prompts.ts` (coach copy for discard always accumulate — `oneClick` already false after Task 1)
- Modify: `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md`
- Optional: `client/app/board/html/prompts.test.ts` if discard submit path needs a confirm click test

**Interfaces:**
- Consumes: Task 1 one-click false
- Produces: Scene coverage for count-1 Confirm; updated prompts spec

- [ ] **Step 1: Write failing Scene for engine discard Confirm**

```ts
test("pending discard aim shows Confirm and count for single-card discard", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [card(11, { zone: ZONE.Hand, name: "A" })],
        pending_choice: {
          kind: "discard",
          player: 0,
          count: 1,
          items: [{ id: 11, label: "A" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-discard-aim")).toExist(),
    Scene.expect(Scene.testId("pending-discard-count")).toHaveText("0 / 1 selected"),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
    Scene.expect(Scene.testId("prompt-submit")).toHaveText("Discard"),
  );
});
```

- [ ] **Step 2: Run — expect FAIL if count line missing for ready==false with required**

After Task 1, `oneClick` is false so count + Confirm should already render. If PASS immediately, keep the test as regression. If FAIL, fix prompts coach/count (no special-case for discard count 1).

- [ ] **Step 3: Update prompts behavior spec**

In `2026-07-20-prompts-and-pending-choices.md`, replace the discard bullet with:

```markdown
- Engine `discard` / `may_discard` with every item in hand use `pending-discard-aim`: click toggles hand selection (raised face + Llanowar ring); Confirm / Continue submits when ready (including count 1). Local `discardPick` with choices in hand uses the same select → Confirm pattern on `discard-cost-aim` (`discard-cost-count`, `DiscardCostConfirmed`). `put_land_from_hand` / `put_creature_from_hand` / `put_from_hand_on_top` / `cast_creature_face_down` keep their existing hand-aim rules.
```

Add testing bullet for Scene/unit coverage of toggle + Confirm.

- [ ] **Step 4: Full client check for touched suites**

Run:

```bash
cd client && bunx vitest run app/board/action/targeting.test.ts app/board/scene.test.ts app/board/html/surfaces.test.ts app/board/html/prompts.test.ts
bunx tsc --noEmit -p tsconfig.json
bunx biome check --write app/board/action/targeting.ts app/board/action/targeting.test.ts app/board/messages.ts app/board/submodel.ts app/board/scene.test.ts app/board/html/prompts.ts app/board/html/surfaces.test.ts app/board/html/hand.ts app/board/html/overlays.ts
```

Expected: all PASS / clean.

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/surfaces.test.ts client/app/board/html/prompts.ts docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md
git commit -m "docs(client): document hand discard select-then-confirm"
```

- [ ] **Step 6: Push + open/update PR**

```bash
git push -u origin cursor/discard-confirm-select-b23c
```

PR title: `feat(client): select then confirm hand discard`

---

## Spec coverage check

| Spec requirement | Task |
|------------------|------|
| Engine discard never one-click | Task 1 |
| Toggle off on second click (engine) | Task 1 |
| Local discard toggle in `discard_cost` | Task 2 |
| Confirm settles local cost | Task 2 |
| Confirm / count on `discard-cost-aim` | Task 2 |
| Legal blue ring unchanged | Task 3 (keeps island-blue when not selected) |
| Selected raise + Llanowar | Task 3 |
| Engine discard Confirm Scene | Task 4 |
| Prompts behavior spec update | Task 4 |
| Put-from-hand unchanged | Task 1 keeps their one-click branches |

## Placeholder scan

None intentional. Local discard max is hard-coded to 1 (matches today’s wire — single discard cost).
