# Foldkit PR Debt — Prompts + CardArt Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close PR #74 remaining debt — Solid-parity DOM `cardArt` over `sharedImageCache`, and a faithful formulator for every `PendingChoiceView` kind (no production stub).

**Architecture:** `cardArt` is an imperative Foldkit `Mount` host (skeleton → `<img>`) sharing `sharedImageCache` with the bitmap layer. Pending choices map exhaustively through formulators that only collect `AnswerInput`; `choiceIntent` remains the sole intent builder.

**Tech Stack:** Foldkit Html/Mount, existing `ImageCache`, `client/lib/choice.ts`, Vitest.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md`, `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`, `docs/superpowers/specs/2026-07-20-card-inspect.md`
- Same PR branch: `cursor/foldkit-migration-design-1ef0`
- Two commits: (1) CardArt, (2) formulators — subjects lower-case, header ≤72, no `feat!:`
- No production path may render `Limited UI` or call `stubPendingChoice`
- Intents only via `choiceIntent(pc, answer)` — no view-level WireIntent construction
- Bitmap `paintCardArt` unchanged
- Guard-return-first; TDD for new pure helpers and formulators

## File map

| File | Role |
|---|---|
| `client/lib/ui/card-art.ts` | **Create** — `cardArt()`, `BindCardArt` Mount |
| `client/lib/ui/card-art.test.ts` | **Create** — skeleton→ready, URL selection |
| `client/lib/image-cache.ts` | Expose readiness helper if needed (`isReady(url)`) |
| DOM call sites | hand, stack, prompts, pile, inspect, decks list/builder, card-hover-preview |
| `client/lib/choice.ts` | Extend drafts/answers/readiness; formulator registry; remove `FAITHFUL_PROMPT_KINDS` |
| `client/lib/choice.test.ts` | Exhaustiveness + answer builders |
| `client/app/board/html/prompts.ts` | Exhaustive formulators; delete stub |
| `client/app/board/html/prompts.test.ts` | Expand coverage |

### Kind → formulator registry (65 kinds)

| Formulator | Kinds |
|---|---|
| `orderTriggers` | `order_triggers` |
| `damageAssign` | `assign_combat_damage` |
| `yesNo` | `may_yes_no` |
| `modeList` | `choose_mode`, `choose_trigger_modes` |
| `payCost` | `pay_cost`, `pay_or_counter`, `pay_or_controller_draws`, `pay_echo_or_sacrifice`, `pay_recover_or_exile`, `sacrifice_unless_pay`, `pay_cumulative_upkeep_or_sacrifice` |
| `playerPick` | `choose_target_players`, `choose_splitting_opponent` |
| `divideTotal` | `divide_spell_damage`, `divide_counters` |
| `pilePick` | `opponent_chooses_pile`, `choose_pile_for_hand` |
| `partition` | `partition_revealed`, `distribute_top` |
| `colorPick` | `choose_color`, `choose_mana_color` |
| `stringPick` | `choose_creature_type` |
| `numberPick` | `may_draw_up_to`, `trade_secrets_caster_draw`, `trade_secrets_repeat` |
| `destinationPick` | `choose_countered_spell_destination`, `revealed_card_to_battlefield_or_hand`, `choose_top_or_bottom` (if present as destination-style) |
| `cardPick` | all remaining item-list kinds: `choose_target`, `choose_spell_targets`, `choose_ability_targets`, `choose_activation_cost_targets`, `decline_untap`, `sacrifice_unless_return_land`, `scry`, `surveil`, `search_library`, `select_from_top`, `shuffle_from_graveyard`, `sacrifice_edict`, `proliferate`, `phase_out`, `may_sacrifice`, `choose_own_sacrifices`, `devour`, `exile_from_graveyard`, `caster_keep_permanents`, `choose_counter_target_for_player`, `may_return_from_graveyard`, `may_discard`, `discard`, `put_land_from_hand`, `put_creature_from_hand`, `choose_dredge`, `cast_creature_face_down`, `choose_exiled_with_card`, `choose_exiled_with_card_to_cast`, `choose_exiled_dig_to_cast_free`, `dance_exile_more`, `opponent_chooses_exiled_nonland`, `choose_exiled_to_cast_free`, `choose_copy_target`, `choose_attach_host`, `put_from_hand_on_top`, `opponent_chooses_revealed_to_graveyard` |

If a kind’s shape does not fit `cardPick`, move it to the specialized formulator — registry must remain 1:1 exhaustive.

---

### Task 1: `cardArt` helper over `sharedImageCache`

**Files:**
- Create: `client/lib/ui/card-art.ts`
- Create: `client/lib/ui/card-art.test.ts`
- Modify: `client/lib/image-cache.ts` (add `isReady(url: string): boolean` if not already expressible)

**Interfaces:**
- Produces: `cardArt(opts): Html` for any Message type via generic `html` builder **or** a host-div Mount pattern that does not require Message plumbing
- Consumes: `sharedImageCache`, `imageUrlByPrint`, `cardBackUrl`

- [ ] **Step 1: Add readiness query on ImageCache**

In `client/lib/image-cache.ts`:

```ts
isReady(url: string): boolean {
  return this.ready.has(url);
}
```

- [ ] **Step 2: Write failing CardArt tests**

`client/lib/ui/card-art.test.ts` — test pure URL resolution helper (extract `cardArtUrl(print, size, face)`) and cache readiness with a fake `makeImage`:

```ts
import { describe, expect, it, vi } from "vitest";
import { ImageCache } from "../image-cache";
import { cardArtUrl } from "./card-art";

describe("cardArtUrl", () => {
  it("uses card back when print is empty", () => {
    expect(cardArtUrl("")).toMatch(/card-back/);
  });
  it("defaults to large front", () => {
    expect(cardArtUrl("abcd-print")).toContain("abcd-print");
  });
});

describe("ImageCache readiness for DOM art", () => {
  it("becomes ready after onload", async () => {
    let img!: { src: string; onload: (() => void) | null; onerror: (() => void) | null };
    const cache = new ImageCache(
      () => {},
      () => {
        img = { src: "", onload: null, onerror: null };
        return img;
      },
    );
    expect(cache.get("https://example.test/a.webp")).toBeUndefined();
    expect(cache.isReady("https://example.test/a.webp")).toBe(false);
    img.onload?.();
    await vi.waitFor(() => expect(cache.isReady("https://example.test/a.webp")).toBe(true));
  });
});
```

Adjust wait pattern if `Effect.runFork` settles synchronously in tests — use `await Promise.resolve()` loops or subscribe callback.

- [ ] **Step 3: Implement `cardArtUrl` + `BindCardArt` + `cardArt`**

Imperative Mount (no Message required in shell unions):

```ts
// client/lib/ui/card-art.ts
import { Effect } from "effect";
import type { html as createHtml, Html } from "foldkit/html";
import * as Mount from "foldkit/mount";
import { sharedImageCache } from "../image-cache";
import { cardBackUrl, type ImageFace, type ImageSize, imageUrlByPrint } from "../deck-builder/scryfall";

export function cardArtUrl(print: string, size: ImageSize = "large", face: ImageFace = "front"): string {
  if (!print) return cardBackUrl();
  return imageUrlByPrint(print, size, face);
}

/** Mount: host is a sized box; paints skeleton then img when sharedImageCache is ready. */
export const BindCardArt = Mount.define("BindCardArt")((element) =>
  Effect.gen(function* () {
    yield* Effect.acquireRelease(
      Effect.sync(() => {
        if (!(element instanceof HTMLElement)) return null;
        const url = element.dataset.artUrl ?? "";
        const alt = element.dataset.artAlt ?? "";
        const paint = () => {
          element.replaceChildren();
          if (!url) return;
          if (sharedImageCache.isReady(url) || sharedImageCache.get(url)) {
            // get() may still be undefined until onload; isReady is authoritative
          }
          if (sharedImageCache.isReady(url)) {
            const img = document.createElement("img");
            img.src = url;
            img.alt = alt;
            img.draggable = false;
            img.className = element.dataset.artClass ?? "";
            element.append(img);
            return;
          }
          sharedImageCache.get(url); // start load
          const sk = document.createElement("div");
          sk.className = `${element.dataset.artClass ?? ""} animate-skeleton bg-white/8`;
          sk.setAttribute("aria-hidden", "true");
          element.append(sk);
        };
        paint();
        const unsub = sharedImageCache.subscribe(paint);
        return { unsub };
      }),
      (handle) =>
        Effect.sync(() => {
          handle?.unsub();
        }),
    );
    return undefined as never; // Mount.define with no message — check foldkit API; if message required, emit a no-op shared ModalOpened-style message or reuse ArtLoaded only on board
  }),
);
```

**Important:** Inspect `Mount.define` overloads in `foldkit/mount`. If a message type is required, add a shared no-op `CardArtTick = m("CardArtTick")` and handle it as identity in app/board/shell updates (or only use Mount on board and pass `html<M>` with optional tick). Prefer a Mount that needs no message if the API allows (mirror patterns that only use `acquireRelease`).

If Mount always requires a message, define:

```ts
export const CardArtTick = m("CardArtTick");
```

and in each update that hosts card art, `case "CardArtTick": return [model, []]` (no-op). Board already has `ArtLoaded` — `BindCardArt` may dispatch `ArtLoaded` when used on the board host only; for shell, add `CardArtTick` to auth/decks/lobby message unions **or** keep shell mounts message-free via DOM-only Mount.

Simplest approach that always works: **DOM-only paint in Mount without dispatching** — if `Mount.define` requires a message factory, pass a dummy message that updates ignore.

```ts
export function cardArt<M>(
  h: ReturnType<typeof createHtml<M>>,
  opts: {
    print: string;
    size?: ImageSize;
    face?: ImageFace;
    alt: string;
    className: string;
    testId?: string;
  },
): Html {
  const url = cardArtUrl(opts.print, opts.size ?? "large", opts.face ?? "front");
  const attrs = [
    h.Class("relative overflow-hidden"),
    h.DataAttribute("art-url", url),
    h.DataAttribute("art-alt", opts.alt),
    h.DataAttribute("art-class", opts.className),
    h.OnMount(BindCardArt() as never),
  ];
  if (opts.testId) attrs.push(h.DataAttribute("testid", opts.testId));
  // Host sized by className on inner — put className on host when it includes aspect/size
  return h.div([h.Class(opts.className), ...attrs.slice(1)], []);
}
```

Refine so `className` sizes the host (skeleton fills host). Implementers should match existing tile aspect classes at call sites.

- [ ] **Step 4: Run tests**

`cd client && bunx vitest run lib/ui/card-art.test.ts lib/image-cache.ts` (or image-cache tests if added)

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add client/lib/ui/card-art.ts client/lib/ui/card-art.test.ts client/lib/image-cache.ts
git commit -m "feat(client): shared cardart over imagecache"
```

---

### Task 2: Swap all DOM card faces to `cardArt`

**Files:**
- Modify: `client/app/board/html/hand.ts`
- Modify: `client/app/board/html/stack.ts`
- Modify: `client/app/board/html/prompts.ts` (img sites only)
- Modify: `client/app/board/html/pile-overlay.ts`
- Modify: `client/app/board/html/inspect.ts`
- Modify: `client/app/shell/decks/list/view.ts`
- Modify: `client/app/shell/decks/builder/view.ts`
- Modify: `client/lib/deck-builder/card-hover-preview.ts`

**Interfaces:**
- Consumes: `cardArt(h, { print, size, face, alt, className, testId? })`
- Do **not** change bitmap paint files

- [ ] **Step 1: Replace each call site**

Examples:

**inspect.ts**

```ts
return cardArt(h, {
  print,
  size: "large",
  face,
  alt,
  className: "block rounded-[14px] shadow-table object-cover",
  // width/height: include in className or Style on host — preserve CARD_W/CARD_H
});
```

If `cardArt` host needs explicit pixel size, extend opts with `style?: Record<string, string>`.

**decks list** — `size: "art_crop"`.

**prompts / stack / pile** — use `size: "large"` (or `"normal"` if current code uses normal — prefer `large` per spec table; prompts currently use `"normal"` — **spec says prompts use `large`**; switch to `large`).

**hand** — `size: "large"` (default).

**card-hover-preview** — `size: "large"`.

Remove direct `h.img` + `imageUrlByPrint` for card faces. Keep non-card images if any.

- [ ] **Step 2: Grep gate**

```bash
rg -n 'h\.img\(' client/app client/lib/deck-builder --glob '!**/*.test.ts'
```

Expected: no card-face `h.img` left (or only non-card icons). Every former card face goes through `cardArt`.

- [ ] **Step 3: Run focused tests**

```bash
cd client && bunx vitest run lib/ui/card-art.test.ts lib/deck-builder/card-hover-preview.test.ts app/board/html/prompts.test.ts app/shell/decks
```

Expected: PASS (update any tests that queried `img` before mount — allow Mount microtask).

- [ ] **Step 4: Amend or second commit under CardArt wave**

Prefer **one** CardArt commit total — if Task 1 already committed, add:

```bash
git add -u client/
git commit -m "feat(client): use cardart for all dom card faces"
```

Or squash with Task 1 into the single CardArt commit from the design (`feat(client): shared cardart over imagecache`) via soft-reset if not pushed. Design allows one commit for CardArt wave — implementers should keep Tasks 1–2 as one commit if possible.

---

### Task 3: Pure choice helpers for all kinds

**Files:**
- Modify: `client/lib/choice.ts`
- Create or modify: `client/lib/choice.test.ts`

**Interfaces:**
- Produces: `FORMULATOR_FOR_KIND: Record<PendingChoiceView["kind"], FormulatorId>`
- Produces: `buildAnswerFromDraft` / `answerFromSimple` covering every kind
- Produces: `assertAllKindsRegistered()` used by tests

- [ ] **Step 1: Failing exhaustiveness test**

```ts
import { describe, expect, it } from "vitest";
import { FORMULATOR_FOR_KIND } from "./choice";
import type { PendingChoiceView } from "~/wire/types";

// Maintain a const array of all kinds — or derive via satisfies
const ALL_KINDS = [ /* paste all 65 kind strings */ ] as const satisfies readonly PendingChoiceView["kind"][];

describe("FORMULATOR_FOR_KIND", () => {
  it("registers every PendingChoiceView kind", () => {
    for (const kind of ALL_KINDS) {
      expect(FORMULATOR_FOR_KIND[kind], kind).toBeDefined();
    }
    expect(Object.keys(FORMULATOR_FOR_KIND).sort()).toEqual([...ALL_KINDS].sort());
  });
});
```

- [ ] **Step 2: Add registry + extend drafts**

```ts
export type FormulatorId =
  | "cardPick"
  | "orderTriggers"
  | "damageAssign"
  | "yesNo"
  | "modeList"
  | "payCost"
  | "playerPick"
  | "divideTotal"
  | "pilePick"
  | "partition"
  | "colorPick"
  | "stringPick"
  | "numberPick"
  | "destinationPick";

export const FORMULATOR_FOR_KIND: { [K in PendingChoiceView["kind"]]: FormulatorId } = {
  order_triggers: "orderTriggers",
  assign_combat_damage: "damageAssign",
  may_yes_no: "yesNo",
  // ... every kind from the registry table above
};
```

Extend `PromptDraft`:

```ts
export type PromptDraft =
  | { kind: "card-pick"; picked: number[] }
  | { kind: "order"; order: number[] }
  | { kind: "damage"; amounts: Record<number, number> }
  | { kind: "divide"; amounts: Record<number, number> }
  | { kind: "partition"; buckets: Record<string, number[]> }
  | { kind: "pile"; pile: 0 | 1 }
  | { kind: "number"; count: number }
  | { kind: "string"; value: string }
  | { kind: "color"; color: number }
  | { kind: "mode"; mode: number }
  | { kind: "modes"; modes: WireModeChoice[] }
  | { kind: "pay"; pay: boolean }
  | { kind: "may"; yes: boolean }
  | { kind: "destination"; choice: number | null | boolean };
```

Extend `buildAnswerFromDraft` (or add `answerFromDraft(pc, draft): AnswerInput | null`) so each kind maps to the existing `AnswerInput` variants consumed by `choiceIntent`. Reuse `cardPick` mappings for all item-list kinds (sacrifice ids, search choice, arrange top/bottom, etc.).

Remove `FAITHFUL_PROMPT_KINDS` / `isFaithfulPromptKind` once formulators land (Task 4) — in this task, keep them but stop relying on them.

- [ ] **Step 3: Tests for answer builders**

For each formulator id, one representative kind:

```ts
it("builds pay intent for pay_cost", () => {
  const pc = { kind: "pay_cost", player: 0, source: 1, label: "Pay", cost: emptyCost } as PendingChoiceView;
  const answer = answerFromDraft(pc, { kind: "pay", pay: true });
  expect(choiceIntent(pc, answer!)).toMatchObject({ kind: "pay_optional_cost", pay: true });
});
```

Cover cardPick representatives (scry arrange, discard, search).

- [ ] **Step 4: Run tests — PASS, commit with formulators wave (or hold commit until Task 4)**

Design wants one formulators commit — hold commit until Task 4 completes.

---

### Task 4: Exhaustive formulators in `prompts.ts`

**Files:**
- Modify: `client/app/board/html/prompts.ts`
- Modify: `client/app/board/html/prompts.test.ts`
- Modify: `client/lib/choice.ts` (remove FAITHFUL_*)

**Interfaces:**
- Consumes: `FORMULATOR_FOR_KIND`, `answerFromDraft`, `cardArt`
- Produces: `pendingChoicePrompt` exhaustive; no `stubPendingChoice`

- [ ] **Step 1: Implement formulators**

Keep existing `cardPickPrompt`, `orderPrompt`, `damageAssignPrompt`. Add:

```ts
function yesNoPrompt(pending: Extract<PendingChoiceView, { kind: "may_yes_no" }>, tableId: string | null): Html
function modeListPrompt(pending: PendingChoiceView, board: BoardModel, tableId: string | null): Html
function payCostPrompt(pending: PendingChoiceView, tableId: string | null): Html
function playerPickPrompt(pending: PendingChoiceView, state: VisibleState, tableId: string | null): Html
function divideTotalPrompt(pending: PendingChoiceView, board: BoardModel, state: VisibleState): Html
function pilePickPrompt(pending: PendingChoiceView, tableId: string | null): Html
function partitionPrompt(pending: PendingChoiceView, board: BoardModel, state: VisibleState): Html
function colorPickPrompt(pending: PendingChoiceView, tableId: string | null): Html
function stringPickPrompt(pending: PendingChoiceView, tableId: string | null): Html
function numberPickPrompt(pending: PendingChoiceView, tableId: string | null): Html
function destinationPickPrompt(pending: PendingChoiceView, tableId: string | null): Html
```

Each submits via `PendingChoiceAnswered({ intent: choiceIntent(pending, answer) })` when ready.

Card faces in formulators use `cardArt(h, …)`.

- [ ] **Step 2: Exhaustive dispatcher**

```ts
function pendingChoicePrompt(...): Html {
  const id = FORMULATOR_FOR_KIND[pending.kind];
  switch (id) {
    case "cardPick":
      return cardPickForKind(pending, state, board);
    case "orderTriggers":
      return orderPrompt(pending as Extract<..., "order_triggers">, board);
    // ...
    default: {
      const _x: never = id;
      return _x;
    }
  }
}
```

Delete `stubPendingChoice`, `faithfulPendingChoice`, `isFaithfulPromptKind` usage, and the Limited UI caption.

`cardPickForKind` wraps existing `cardPickPrompt` with titles/hints per kind (switch on `pending.kind` for copy only).

- [ ] **Step 3: Tests**

- Assert `rg "Limited UI" client/app` is empty.
- Story/unit: one test per formulator id builds intent.
- Existing faithful tests still pass.

```bash
cd client && bunx vitest run lib/choice.test.ts app/board/html/prompts.test.ts
rg -n 'Limited UI|stubPendingChoice|isFaithfulPromptKind|FAITHFUL_PROMPT' client
```

Expected: tests PASS; grep empty (or only comments in design/plan docs).

- [ ] **Step 4: Commit formulators wave**

```bash
git add client/lib/choice.ts client/lib/choice.test.ts client/app/board/html/prompts.ts client/app/board/html/prompts.test.ts
git commit -m "feat(client): pending_choice formulators for all kinds"
```

---

### Task 5: Verify + PR debt status

**Files:**
- Modify: current prompt, hand/zone, and inspect specs → record landed behavior

- [ ] **Step 1: Full client check**

```bash
just client-check
```

Expected: format, lint (0 warnings), typecheck, all tests pass.

- [ ] **Step 2: Mark design Done** (include in formulators commit or tiny `docs:` commit — prefer amend formulators commit if not pushed)

- [ ] **Step 3: Push**

```bash
git push -u origin cursor/foldkit-migration-design-1ef0
```

- [ ] **Step 4: PR body** (owner may need to edit if bot lacks write)

Remove:

```markdown
## Remaining debt (non-page chrome)
- ~47 lower-traffic ...
- Shared DOM CardArt ...
```

Keep BREAKING CHANGE footer.

---

## Spec coverage

| Spec item | Task |
|---|---|
| `cardArt` API + shared cache + skeleton | Task 1 |
| All DOM call sites | Task 2 |
| Formulators + exhaustive kinds | Tasks 3–4 |
| Delete stub / Limited UI | Task 4 |
| `choiceIntent` only | Tasks 3–4 |
| Two commits | Tasks 1–2 (CardArt), Tasks 3–4 (formulators) |
| `just client-check` | Task 5 |
| PR body debt removal | Task 5 |

## Risks

- **Mount message plumbing:** Resolve `Mount.define` signature early in Task 1; prefer DOM-only BindCardArt without per-route Message unions.
- **`buildAnswerFromDraft` gaps:** Some kinds need optional null choice (search fail-to-find) — preserve existing decline labels.
- **divide/partition UX:** Keep minimal but correct (inputs summing to `total`; bucket toggles) — not a full MTGA clone, but not a stub banner.
