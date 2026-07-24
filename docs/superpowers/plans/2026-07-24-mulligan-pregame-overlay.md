# Mulligan Pre-game Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the mulligan action-bar strip with an Arena-like full-screen overlay (large hand faces + Keep/Mulligan) while undecided, then dismiss to the board with a light waiting banner after Keep.

**Architecture:** New `mulligan-overlay.ts` owns undecided chrome; `mulliganChrome` stays the pure copy/enablement source. `overlays.ts` branches: undecided → overlay (no hand bar); kept+mulliganing → hand bar + waiting banner; else unchanged. Full-viewport `pointer-events-auto` overlay (z-40) hard-locks the board; bump concede button above it.

**Tech Stack:** Foldkit Html/Scene (Vitest), existing `cardArt`, Tailwind forest HUD tokens.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-24-mulligan-pregame-overlay-design.md`
- Undecided: `data-testid="mulligan-overlay"`; Keep=`mulligan-keep`; Mulligan=`mulligan-take`; hide `hand-bar`
- After Keep: no overlay; `mulligan-waiting` banner; `hand-bar` returns
- Hard lock while undecided (no pan/hand/priority under overlay); Concede stays available
- No London / timers / spectator mulligan UI / overlay inspect
- No wire/engine changes — snapshot fields only
- TDD; Angular commits (`feat(client):`, `test(client):`, `docs:`)
- Branch: `cursor/mulligan-overlay-b23c`

## File map

| File | Responsibility |
|------|----------------|
| `client/app/board/html/mulligan-overlay.ts` | Undecided full overlay + post-keep waiting banner |
| `client/app/board/html/mulligan-bar.ts` | Retire undecided bar (delete or re-export thin wrappers) |
| `client/app/board/html/overlays.ts` | Compose overlay vs hand bar vs waiting |
| `client/app/board/html/concede.ts` | Raise concede `z` above overlay |
| `client/app/board/html/chrome.test.ts` | Scene coverage for overlay / waiting / no hand-bar |
| `client/lib/mulligan.ts` | Unchanged API (reuse) |
| `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md` | Behavior truth |

---

### Task 1: Undecided mulligan overlay (Scene + view)

**Files:**
- Create: `client/app/board/html/mulligan-overlay.ts`
- Modify: `client/app/board/html/overlays.ts`
- Modify: `client/app/board/html/concede.ts` (z-index)
- Modify: `client/app/board/html/chrome.test.ts`
- Delete or empty: `client/app/board/html/mulligan-bar.ts` (after imports move)

**Interfaces:**
- Consumes: `mulliganChrome` from `~/mulligan`; `VisibleState`; `KeepHandClicked` / `MulliganClicked`; `cardArt` from `~/ui/card-art`; `ZONE` from geometry
- Produces: `mulliganOverlayView(state): Html | null` — undecided full overlay; `mulliganWaitingView(state): Html | null` — post-keep banner (may land in Task 2; Task 1 may stub waiting as null)

- [ ] **Step 1: Write failing Scene tests**

In `client/app/board/html/chrome.test.ts`, replace/update the undecided mulligan test:

```ts
test("mulliganing undecided seat sees overlay and hides hand bar", () => {
  const state = gameState({
    mulliganing: true,
    objects: [
      // reuse existing card helper if available in this file; otherwise inline ObjectView
      // with zone: ZONE.Hand, owner: 0, id: 1, name: "Forest", print: "forest-print"
    ],
    players: [
      { ...player(0), hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
      { ...player(1), hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
    ],
  });
  // Scene with overlayModel / resolveBoardOverlayMounts (+ card art mounts if needed)
  Scene.expect(Scene.testId("mulligan-overlay")).toExist(),
  Scene.expect(Scene.testId("mulligan-keep")).toExist(),
  Scene.expect(Scene.testId("mulligan-take")).toExist(),
  Scene.expect(Scene.testId("mulligan-face-1")).toExist(), // face per hand object id
  Scene.expect(Scene.testId("hand-bar")).not.toExist(),
  Scene.expect(Scene.testId("mulligan-bar")).not.toExist(),
  Scene.expect(Scene.testId("board-primary")).not.toExist(),
  Scene.expect(Scene.testId("board-concede")).toExist(),
});
```

If this file lacks a `card()` helper, copy the small `ObjectView` factory from `surfaces.test.ts` or import shared fixtures. Use `resolveBoardCardArtMounts()` when faces use `cardArt`.

Update the waiting test title temporarily to still expect old bar **or** skip rewriting waiting until Task 2 — for Task 1, change the kept-seat test to expect `mulligan-waiting` only if you implement waiting in the same task. Prefer implementing waiting in Task 2: for Task 1, leave the kept-seat chrome test asserting `mulligan-bar` absent and **no** overlay when kept (waiting may be missing until Task 2).

```ts
test("mulliganing kept seat does not show decision overlay", () => {
  // hand_kept: true local
  Scene.expect(Scene.testId("mulligan-overlay")).not.toExist(),
  Scene.expect(Scene.testId("mulligan-keep")).not.toExist(),
});
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cd client && bunx vitest run app/board/html/chrome.test.ts -t "mulligan"`

Expected: FAIL — `mulligan-overlay` missing; `hand-bar` still present.

- [ ] **Step 3: Implement `mulliganOverlayView`**

Create `client/app/board/html/mulligan-overlay.ts`:

```ts
import { type Html, html } from "foldkit/html";
import { mulliganChrome } from "~/mulligan";
import { cardArt } from "~/ui/card-art";
import { gameButtonClass } from "~/ui/buttonClass";
import type { VisibleState } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import { KeepHandClicked, type Message, MulliganClicked } from "../messages";

const h = html<Message>();

export function mulliganOverlayView(state: VisibleState): Html | null {
  const chrome = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  });
  if (!chrome.show || !chrome.showControls) return null;

  const hand = state.objects.filter(
    (o) => Number(o.zone) === ZONE.Hand && Number(o.owner) === Number(state.viewer),
  );

  return h.div(
    [
      h.DataAttribute("testid", "mulligan-overlay"),
      h.Class(
        "pointer-events-auto fixed inset-0 z-40 flex flex-col items-center justify-center gap-md bg-black/70 px-md py-lg text-snow",
      ),
    ],
    [
      h.div(
        [
          h.Class(
            "flex max-h-[min(70vh,640px)] w-full max-w-[min(96vw,1100px)] flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-md shadow-hud",
          ),
        ],
        [
          h.div([h.Class("text-label uppercase tracking-[0.08em] text-mist")], [chrome.title]),
          h.div([h.Class("text-caption text-snow-mint")], [chrome.status]),
          h.div(
            [
              h.DataAttribute("testid", "mulligan-hand"),
              h.Class("flex w-full flex-wrap justify-center gap-3 overflow-y-auto py-sm"),
            ],
            hand.map((obj) =>
              h.div(
                [
                  h.DataAttribute("testid", `mulligan-face-${obj.id}`),
                  h.Class("pointer-events-none shrink-0"),
                ],
                [
                  obj.print
                    ? cardArt(h, {
                        print: obj.print,
                        size: "large",
                        alt: obj.name,
                        className:
                          "block aspect-[150/209] w-[min(22vw,160px)] rounded-[9px] bg-morph-slate shadow-hand",
                      })
                    : h.div(
                        [
                          h.Class(
                            "flex aspect-[150/209] w-[min(22vw,160px)] items-center justify-center rounded-[9px] bg-morph-slate px-2 text-center text-caption text-snow",
                          ),
                        ],
                        [obj.name],
                      ),
                ],
              ),
            ),
          ),
          h.div(
            [h.Class("flex flex-wrap justify-center gap-sm")],
            [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "mulligan-keep"),
                  h.OnClick(KeepHandClicked()),
                  h.Class(gameButtonClass("game")),
                ],
                [chrome.keepLabel],
              ),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "mulligan-take"),
                  h.Disabled(!chrome.canMulligan),
                  h.OnClick(MulliganClicked()),
                  h.Class(gameButtonClass("game-quiet")),
                ],
                [chrome.mulliganLabel],
              ),
            ],
          ),
        ],
      ),
    ],
  );
}
```

- [ ] **Step 4: Wire overlays + concede z + retire bar**

In `overlays.ts`:

```ts
const chrome = mulliganChrome({
  mulliganing: state.mulliganing,
  localSeat: state.viewer,
  players: state.players,
});
const undecidedMulligan = chrome.show && chrome.showControls;

// handView only when seatedViewer && !undecidedMulligan
seatedViewer && !undecidedMulligan
  ? handView({ ... })
  : null,

// Replace mulliganBarView:
seatedViewer ? mulliganOverlayView(state) : null,
```

Remove import of `mulliganBarView`. Delete `mulligan-bar.ts` once nothing imports it (grep first).

In `concede.ts`, change concede button class from `z-20` to `z-45` so it stays clickable above the overlay:

```ts
h.Class(cn("pointer-events-auto fixed top-md right-md z-45", buttonClass("ghost"))),
```

- [ ] **Step 5: Run tests — expect PASS for undecided**

Run: `cd client && bunx vitest run app/board/html/chrome.test.ts -t "mulligan"`

Expected: undecided overlay tests PASS. Fix any broken imports / art mounts.

- [ ] **Step 6: Commit**

```bash
git add client/app/board/html/mulligan-overlay.ts client/app/board/html/overlays.ts client/app/board/html/concede.ts client/app/board/html/chrome.test.ts
# and deletion of mulligan-bar.ts if removed
git commit -m "feat(client): show mulligan opening hand in pre-game overlay"
```

---

### Task 2: Post-keep waiting banner

**Files:**
- Modify: `client/app/board/html/mulligan-overlay.ts` (add `mulliganWaitingView`)
- Modify: `client/app/board/html/overlays.ts`
- Modify: `client/app/board/html/chrome.test.ts`

**Interfaces:**
- Consumes: `mulliganChrome` when `show && !showControls`
- Produces: `mulliganWaitingView(state): Html | null` with `data-testid="mulligan-waiting"`

- [ ] **Step 1: Write failing Scene test**

```ts
test("mulligan kept seat sees waiting banner and hand bar", () => {
  const state = gameState({
    mulliganing: true,
    players: [
      { ...player(0), hand_kept: true, can_mulligan: false, mulligans_taken: 0 },
      { ...player(1), hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
    ],
  });
  Scene.expect(Scene.testId("mulligan-overlay")).not.toExist(),
  Scene.expect(Scene.testId("mulligan-waiting")).toExist(),
  Scene.expect(Scene.testId("mulligan-waiting")).toContainText("Waiting for Bob to choose."),
  Scene.expect(Scene.testId("hand-bar")).toExist(),
  Scene.expect(Scene.testId("mulligan-keep")).not.toExist(),
});
```

Update/remove the old `mulligan bar waiting status…` test that asserted `mulligan-bar`.

- [ ] **Step 2: Run — expect FAIL**

Run: `cd client && bunx vitest run app/board/html/chrome.test.ts -t "waiting banner|mulligan kept"`

Expected: FAIL — `mulligan-waiting` absent.

- [ ] **Step 3: Implement waiting banner**

In `mulligan-overlay.ts`:

```ts
export function mulliganWaitingView(state: VisibleState): Html | null {
  const chrome = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  });
  if (!chrome.show || chrome.showControls) return null;
  return h.div(
    [
      h.DataAttribute("testid", "mulligan-waiting"),
      h.Class(
        "pointer-events-none fixed top-md left-1/2 z-30 max-w-[min(90vw,28rem)] -translate-x-1/2 rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-center text-chip text-seafoam shadow-hud",
      ),
    ],
    [chrome.status],
  );
}
```

In `overlays.ts`, after overlay:

```ts
seatedViewer ? mulliganWaitingView(state) : null,
```

- [ ] **Step 4: Run — expect PASS**

Run: `cd client && bunx vitest run app/board/html/chrome.test.ts -t "mulligan"`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/mulligan-overlay.ts client/app/board/html/overlays.ts client/app/board/html/chrome.test.ts
git commit -m "feat(client): show mulligan waiting banner after keep"
```

---

### Task 3: Spec + disabled Mulligan + verification

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md`
- Modify: `client/app/board/html/chrome.test.ts` (disabled mulligan control if not already covered)
- Optional: surfaces test if mulligan appears only in chrome.test — keep chrome as source of Scene truth

**Interfaces:** none new

- [ ] **Step 1: Write failing Scene for disabled Mulligan**

```ts
test("mulligan take is disabled when can_mulligan is false", () => {
  // undecided, can_mulligan: false, mulligans_taken: 6
  Scene.expect(Scene.testId("mulligan-overlay")).toExist(),
  Scene.expect(Scene.testId("mulligan-take")).toBeDisabled(),
});
```

- [ ] **Step 2: Run — expect PASS if already wired via `chrome.canMulligan`; else FAIL and fix button Disabled binding**

- [ ] **Step 3: Update turn-and-priority-chrome spec**

Replace the mulligan bullets with:

```markdown
- While `VisibleState.mulliganing` is true and the local seated viewer has not kept (`!hand_kept`), `mulliganOverlayView` shows full-viewport `mulligan-overlay` (dimmed hard-lock backdrop, large opening-hand faces, Keep / Mulligan). The normal `hand-bar` and priority bar are hidden. Space and Enter stay inert; Concede remains available above the overlay.
- After the local seat keeps while others are still deciding, the overlay dismisses, `hand-bar` returns, and `mulligan-waiting` shows waiting copy that names undecided living seats (username, or `P{seat}` when empty). Lost seats are omitted. When every living seat has kept, status is “All players kept. Starting game…”.
```

Update **Module:** line to list `mulligan-overlay.ts` instead of `mulligan-bar.ts`.

Add testing bullet for overlay / waiting Scene coverage. Cross-link `2026-07-24-mulligan-pregame-overlay-design.md`.

- [ ] **Step 4: Full focused verify**

```bash
cd client && bunx vitest run app/board/html/chrome.test.ts app/board/inspect-pile-concede.test.ts client/lib/mulligan.test.ts
# from client/: also
bunx vitest run ../client/lib/mulligan.test.ts
bunx tsc --noEmit -p tsconfig.json
bunx biome check --write app/board/html/mulligan-overlay.ts app/board/html/overlays.ts app/board/html/concede.ts app/board/html/chrome.test.ts
```

Fix paths: `bunx vitest run app/board/html/chrome.test.ts app/board/inspect-pile-concede.test.ts` and `bunx vitest run lib/mulligan.test.ts` from `client/` (mulligan tests live under `client/lib`).

Expected: all PASS.

- [ ] **Step 5: Commit + push**

```bash
git add docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md client/app/board/html/chrome.test.ts
git commit -m "docs(client): document mulligan pre-game overlay chrome"
git push -u origin cursor/mulligan-overlay-b23c
```

PR title: `feat(client): Arena-like mulligan pre-game overlay`

---

## Spec coverage check

| Spec requirement | Task |
|------------------|------|
| Full overlay + large faces while undecided | Task 1 |
| Hide hand-bar while undecided | Task 1 |
| Keep / Mulligan labels + disable | Task 1 / 3 |
| Hard lock via full-screen pointer-events + omit hand/priority | Task 1 |
| Concede above overlay | Task 1 |
| Dismiss after Keep + waiting banner | Task 2 |
| Hand bar returns after Keep | Task 2 |
| Spec doc update | Task 3 |
| No London / wire changes | All (out of scope) |

## Placeholder scan

None. Face width uses `min(22vw,160px)` as a concrete starting size; adjust only if Scene/layout review fails readability.
