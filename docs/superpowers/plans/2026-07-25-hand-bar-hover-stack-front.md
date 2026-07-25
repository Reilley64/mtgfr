# Hand Bar Hover Bring-to-Front Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the pointer hovers a hand/action-bar tile, that tile’s root stacks above every other bar tile (Arena bring-to-front); discard-selected does not elevate z.

**Architecture:** Elevate z on the tile root via CSS `:hover` (`hover:z-50`) so sibling slots share one stacking context. Remove face-only hover `z-30`. Discard-selected keeps raise/ring without elevated z. Resting `z-index: index + 1` stays.

**Tech Stack:** TypeScript, Vitest, Foldkit Html (existing `handView` / `hand.test.ts` class helpers).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`
- Hover elevate on tile **root** (whole bar); use `hover:z-50` (parent `:hover` fires while the hit strip is hovered)
- Discard-selected: raise + green ring only — **no** elevated z from selection
- Remove face-only hover `z-30`; keep face `translateY` raise on hover and discard-selected
- Resting order unchanged (`z-index: index + 1` on root)
- No keyboard-focus bring-to-front; no geometry / priority / mulligan changes
- TDD; Angular commits (`feat(client):`, `test(client):`, `docs:`)
- Branch: `cursor/hand-hover-stack-b23c`

## File map

| File | Responsibility |
|------|----------------|
| `client/app/board/html/hand.ts` | Tile root hover z; face raise without face z elevate |
| `client/app/board/html/hand.test.ts` | Class-contract tests for root elevate / discard-selected |
| `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md` | Behavior truth |

---

### Task 1: Slot hover z (TDD)

**Files:**
- Modify: `client/app/board/html/hand.ts`
- Modify: `client/app/board/html/hand.test.ts`

**Interfaces:**
- Consumes: existing `handView` / `tile` (no new messages)
- Produces: tile root with `hover:z-50` when `objectId` present; `data-testid="hand-tile-{id}"` on root for tests

- [ ] **Step 1: Write failing tests**

Add `data-testid` lookup helpers are already in `hand.test.ts` (`findTestId`, `className`). Add:

```ts
describe("handView hover stacking", () => {
  it("puts hover elevate on the tile root, not the face", () => {
    const a = object(42, { name: "Lightning Bolt" });
    const b = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [a, b], actions: [action(7, { object: 42 })] }));

    const root = findTestId(tree, "hand-tile-42");
    expect(root).not.toBeNull();
    expect(className(root)).toContain("hover:z-50");
    expect(className(root)).toContain("group/hand-tile");

    const face = findTestId(tree, "hand-card-face-42");
    expect(className(face)).not.toContain("group-hover/hand-tile:z-30");
    // Face may still live under a wrapper — assert the tree no longer uses face hover z-30:
    expect(treeHasClass(tree, "group-hover/hand-tile:z-30")).toBe(false);
  });

  it("does not elevate z for discard-selected without relying on selection z", () => {
    const a = object(42, { name: "Lightning Bolt" });
    const tree = handView({
      state: state({ objects: [a], actions: [] }),
      hiddenId: null,
      flyingIds: new Set(),
      hiddenIds: new Set(),
      handDrag: null,
      discardCostIds: new Set([42]),
      discardSelectedIds: new Set([42]),
    });
    const root = findTestId(tree, "hand-tile-42");
    expect(root).not.toBeNull();
    // Root still has hover elevate available, but selection alone must not add a selected z class:
    expect(className(root)).toContain("hover:z-50");
    expect(className(root)).not.toContain("z-30");
    expect(className(root)).not.toContain("z-50"); // bare z-50 without hover: prefix
    const face = findTestId(tree, "hand-card-face-42");
    expect(className(face)).toContain("ring-llanowar");
    // Face raise for selection must not use elevated z-30:
    expect(treeHasClass(findTestId(tree, "hand-tile-42"), "z-30")).toBe(false);
  });
});
```

Also add on the tile root in implementation (Step 3) — tests fail until `hand-tile-{id}` exists.

- [ ] **Step 2: Run — expect FAIL**

Run: `cd client && bunx vitest run app/board/html/hand.test.ts -t "hover stacking"`

Expected: FAIL — `hand-tile-42` missing and/or no `hover:z-50`.

- [ ] **Step 3: Implement**

In `tile()` in `hand.ts`:

1. Tile root class — append `hover:z-50`:

```ts
h.Class(
  "group/hand-tile pointer-events-none relative shrink-0 origin-bottom overflow-visible hover:z-50",
),
```

2. When `objectId != null`, add:

```ts
h.DataAttribute("testid", `hand-tile-${objectId}`),
```

3. Face class — raise only, no face z elevate:

```ts
const faceClass = [
  "pointer-events-none absolute top-0 right-0 transition-transform duration-[120ms] ease-state",
  discardSelected
    ? "[transform:translateY(var(--raise-y))]"
    : "group-hover/hand-tile:[transform:translateY(var(--raise-y))]",
].join(" ");
```

Keep resting inline `"z-index": String(index + 1)`. Do not add elevated z for `discardSelected`.

Note: use `hover:z-50` on the root (not `group-hover/hand-tile:z-50` on the same node). CSS `:hover` on the root matches while the pointer is over the pointer-events hit strip child.

- [ ] **Step 4: Run — expect PASS**

Run: `cd client && bunx vitest run app/board/html/hand.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/hand.ts client/app/board/html/hand.test.ts
git commit -m "feat(client): bring hovered hand bar tile to front"
```

---

### Task 2: Module spec + verify

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`

**Interfaces:** none new

- [ ] **Step 1: Update hand-and-zone-bar spec**

After the Arena-forward geometry bullet under **Behavior**, add:

```markdown
- Hovering a bar tile elevates that tile’s root above all other action-bar tiles (`hover:z-50` on the slot). Discard-selected raises and rings but does not elevate z. See [hand-and-zone-bar](2026-07-20-hand-and-zone-bar.md).
```

Under **Testing Decisions**, add:

```markdown
- `hand.test.ts` locks hover elevate on `hand-tile-{id}` and asserts discard-selected does not add selection z elevate.
```

- [ ] **Step 2: Focused verify**

```bash
cd client && bunx vitest run app/board/html/hand.test.ts app/board/hand-drag.test.ts
bunx tsc --noEmit -p tsconfig.json
bunx biome check --write app/board/html/hand.ts app/board/html/hand.test.ts
```

Expected: PASS / clean.

- [ ] **Step 3: Commit + push**

```bash
git add docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md
git commit -m "docs(client): document hand bar hover bring-to-front"
git push -u origin cursor/hand-hover-stack-b23c
```

PR title: `feat(client): hand bar hover bring-to-front`

---

## Spec coverage check

| Spec requirement | Task |
|------------------|------|
| Hover root stacks above all bar tiles | Task 1 |
| Discard-selected no elevated z | Task 1 |
| Remove face hover z-30 | Task 1 |
| Keep raise translate | Task 1 |
| Update hand-and-zone-bar.md | Task 2 |
| No keyboard / geometry / priority changes | All (out of scope) |

## Placeholder scan

None. Plan uses `hover:z-50` on the root (correct CSS) rather than `group-hover` on the same node as `group/hand-tile`.
