# Hand Bar Arena-Forward Spacing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply hand-tuned Arena-forward geometry to the bottom hand/zone bar (larger face, taller visible strip, wider peeks) so resting hands feel less cramped.

**Architecture:** Bump the shared constants (`HAND_FACE_W`, `HAND_BAR_PEEK`, `HAND_VISIBLE_H`, pip row, bar padding). Hit/raise/drag/priority-aim offsets keep deriving from those constants — no parallel magic numbers. Update module spec truth in the same change.

**Tech Stack:** TypeScript, Vitest, Foldkit Html/Scene (existing hand bar).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`
- Targets: `HAND_FACE_W=208`, `HAND_BAR_PEEK=92`, `HAND_VISIBLE_H=178`, pip row `24`, bar padding `16`, derived `HAND_BAR_H=218`
- Section gap stays `gap-xl`; fan / left-peek hit / playable borders unchanged in policy
- Out of scope: priority chrome restyle, responsive clamps, mulligan overlay faces, canvas hand bar
- `HAND_PLAY_SLACK_PX` stays `96` unless a Scene drag test fails after the bump
- No wire/engine changes
- TDD; Angular commits (`feat(client):`, `test(client):`, `docs:`)
- Branch: `cursor/hand-bar-arena-spacing-b23c`

## File map

| File | Responsibility |
|------|----------------|
| `client/app/board/motion/flights.ts` | `HAND_FACE_W` |
| `client/app/board/geometry/handBarHit.ts` | `HAND_BAR_PEEK` |
| `client/app/board/html/hand.ts` | `HAND_VISIBLE_H`, `HAND_PIP_ROW_H`, `HAND_BAR_H` padding term |
| `client/app/board/geometry/handBarHit.test.ts` | Hit/raise unit coverage + geometry lock |
| `client/app/board/html/hand.test.ts` | Optional constant lock if Scene-friendly; else geometry test owns it |
| `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md` | Behavior truth for geometry |

---

### Task 1: Geometry constants (TDD)

**Files:**
- Modify: `client/app/board/motion/flights.ts`
- Modify: `client/app/board/geometry/handBarHit.ts`
- Modify: `client/app/board/html/hand.ts`
- Modify: `client/app/board/geometry/handBarHit.test.ts`

**Interfaces:**
- Consumes: existing `handBarHitHeight` / `handBarRaiseTranslateY` / `hitHandBarSlot` APIs (signatures unchanged)
- Produces: exported constants at design targets; `HAND_BAR_H === 218`

- [ ] **Step 1: Write failing geometry lock tests**

In `client/app/board/geometry/handBarHit.test.ts`, add (or replace the hardcoded `VISIBLE = 130` block) a describe that imports bar constants and locks the table:

```ts
import { HAND_BAR_H, HAND_VISIBLE_H } from "../html/hand";

describe("Arena-forward hand bar geometry", () => {
  it("locks face, peek, visible height, and derived bar height", () => {
    expect(HAND_FACE_W).toBe(208);
    expect(HAND_BAR_PEEK).toBe(92);
    expect(HAND_VISIBLE_H).toBe(178);
    expect(HAND_BAR_H).toBe(218);
  });
});
```

In the existing raise describe, replace `const VISIBLE = 130` with `const VISIBLE = HAND_VISIBLE_H` (import from `../html/hand`) so vertical thrash tests track the real constant.

Update the comment above `TWO` if it still says `x=164` — peeks are now 92 apart (`FIRST_PEEK_LEFT + PEEK`).

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cd client && bunx vitest run app/board/geometry/handBarHit.test.ts`

Expected: FAIL — `HAND_FACE_W` is 180 / peek 64 / visible 130 / bar 162 (or import errors until constants are exported as used).

- [ ] **Step 3: Apply target constants**

In `client/app/board/motion/flights.ts`:

```ts
export const HAND_FACE_W = 208;
```

In `client/app/board/geometry/handBarHit.ts`:

```ts
export const HAND_BAR_PEEK = 92;
```

In `client/app/board/html/hand.ts`:

```ts
export const HAND_VISIBLE_H = 178;
/** Room above each face for cast-cost pips (reserved band outside the card). */
const HAND_PIP_ROW_H = 24;
/** Height of the bottom action bar — tuck + pip row + padding. */
export const HAND_BAR_H = HAND_VISIBLE_H + HAND_PIP_ROW_H + 16;
```

Leave `HAND_PLAY_SLACK_PX = 96`. Do not change fan / tile markup classes except values that already read these constants.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cd client && bunx vitest run app/board/geometry/handBarHit.test.ts app/board/html/hand.test.ts app/board/hand-drag.test.ts`

Expected: PASS. If `hand-drag.test.ts` or Scene drag suites fail only because of slack feel, bump `HAND_PLAY_SLACK_PX` minimally and re-run; otherwise leave at 96.

- [ ] **Step 5: Commit**

```bash
git add client/app/board/motion/flights.ts \
  client/app/board/geometry/handBarHit.ts \
  client/app/board/html/hand.ts \
  client/app/board/geometry/handBarHit.test.ts
git commit -m "feat(client): Arena-forward hand bar face and peek spacing"
```

---

### Task 2: Module spec + verify

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`

**Interfaces:** none new

- [ ] **Step 1: Update hand-and-zone-bar spec**

Set **Status** date to `2026-07-25`.

Under **Behavior**, replace the dense-overlap bullet with:

```markdown
- Hand tiles fan with Arena-forward resting geometry (`HAND_FACE_W` 208, `HAND_BAR_PEEK` 92, `HAND_VISIBLE_H` 178, derived `HAND_BAR_H` 218), hover raise, and cost pips above the card face. See [hand-and-zone-bar](2026-07-20-hand-and-zone-bar.md).
```

Under **Implementation Decisions**, add:

```markdown
- Resting bar spacing is hand-tuned Arena-forward constants (not a single global scale factor). Hit height, raise translate, sticky inspect band, and drag play threshold derive from those constants.
```

Under **Testing Decisions**, add:

```markdown
- Geometry lock in `handBarHit.test.ts` asserts face/peek/visible/`HAND_BAR_H` targets so a silent regress to the old dense values fails.
```

Do not rewrite drag/playable-border rules.

- [ ] **Step 2: Focused verify**

Run:

```bash
cd client && bunx vitest run \
  app/board/geometry/handBarHit.test.ts \
  app/board/html/hand.test.ts \
  app/board/hand-drag.test.ts \
  app/board/html/chrome.test.ts \
  app/board/html/surfaces.test.ts
bunx tsc --noEmit -p tsconfig.json
bunx biome check --write \
  app/board/motion/flights.ts \
  app/board/geometry/handBarHit.ts \
  app/board/html/hand.ts \
  app/board/geometry/handBarHit.test.ts
```

Expected: all PASS / clean.

- [ ] **Step 3: Commit + push**

```bash
git add docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md
git commit -m "docs(client): document Arena-forward hand bar geometry"
git push -u origin cursor/hand-bar-arena-spacing-b23c
```

PR title: `feat(client): Arena-forward hand bar spacing`

---

## Spec coverage check

| Spec requirement | Task |
|------------------|------|
| `HAND_FACE_W=208` | Task 1 |
| `HAND_BAR_PEEK=92` | Task 1 |
| `HAND_VISIBLE_H=178` | Task 1 |
| pip 24 + pad 16 → `HAND_BAR_H=218` | Task 1 |
| Hit/raise derive from constants | Task 1 |
| Geometry lock test | Task 1 |
| Update `hand-and-zone-bar.md` | Task 2 |
| No priority restyle / no clamp / no mulligan faces | All (out of scope) |

## Placeholder scan

None. Slack bump only if drag tests fail after the geometry change.
