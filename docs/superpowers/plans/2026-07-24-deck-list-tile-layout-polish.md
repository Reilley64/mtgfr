# Deck List Tile Layout Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align Your decks chrome to one 960px column, enlarge art_crop tiles, prefer CDN art_crop with Scryfall fallback on miss, and remove the deck-list hover preview.

**Architecture:** Client-only. Extract a pure `buildImageUrl(printId, size, face, cdnBase)` so CDN `art_crop` vs `large` paths are unit-testable without stubbing `import.meta.env`. `cardArt` attaches a Scryfall `data-art-fallback` when the primary URL is a CDN art_crop; `ImageCache` records load failures and notifies subscribers so `syncCardArtHost` can swap once. Deck list view drops hover mounts/state and widens tiles.

**Tech Stack:** Foldkit (Html / Messages / Mount / Scene), Effect Schema, Vitest, TypeScript, Tailwind utility classes.

**Spec:** [deck-list-tile-layout-polish-design](../specs/2026-07-24-deck-list-tile-layout-polish-design.md)  
**Current-behavior update:** [client-shell-deck-builder-and-observability](../specs/2026-07-20-client-shell-deck-builder-and-observability.md)

## Global Constraints

- No `.proto`, BFF, or `DeckSummary` schema changes.
- Lobby deck `<select>` / Bring strip unchanged.
- Scryfall fallback only for missing CDN **`art_crop`** — never for `large` / other sizes.
- Guard-return-first; imports at top of file; exhaustive message matching.
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages on branch `cursor/deck-list-tile-polish-2c79`.
- Scene/outcome tests assert product behavior, not migration/parity.
- Keep Play href, search, ordering, Precon chip, and Edit/Delete context menu behavior.

---

## File map

| File | Responsibility |
|------|----------------|
| `client/lib/deck-builder/scryfall.ts` | `buildImageUrl`, CDN `art_crop` folder, `artCropFallbackUrl`, wrap `imageUrlByPrint` |
| `client/lib/deck-builder/scryfall.test.ts` | Unit tests for URL builders (inject `cdnBase`) |
| `client/lib/image-cache.ts` | Track failed URLs; notify subscribers on load error |
| `client/lib/image-cache.test.ts` | Failure + notify coverage |
| `client/lib/ui/card-art.ts` | Set `data-art-fallback` for CDN art_crop; swap on primary failure |
| `client/lib/ui/card-art.test.ts` | Fallback swap after primary `onerror` |
| `client/app/shell/decks/list/view.ts` | 960 column, larger tiles, crop aspect; remove hover |
| `client/app/shell/decks/list/{hover,messages,submodel,update}.ts` | Delete hover types/messages/state/arms |
| `client/app/shell/decks/messages.ts` | Stop re-exporting hover messages |
| `client/app/update.ts` | Drop hover message arms |
| `client/app/shell/decks/list/story.test.ts` | No hover preview; layout/mount expectations |
| `client/app/shell/surfaces.test.ts` | Drop `BindDeckListCommanderHover` resolves |
| `client/app/smoke.test.ts` | Drop hover mount resolves |
| `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md` | Current behavior: layout + art_crop CDN→Scryfall |
| `docs/superpowers/specs/2026-07-24-deck-list-tile-layout-polish-design.md` | Status → Implemented |

---

### Task 1: Pure art URL helpers (CDN art_crop path)

**Files:**
- Modify: `client/lib/deck-builder/scryfall.ts`
- Modify: `client/lib/deck-builder/scryfall.test.ts`

**Interfaces:**
- Consumes: existing `ImageSize`, `ImageFace`, module `CDN` from `VITE_CARD_CDN`
- Produces:
  - `buildImageUrl(printId: string, size: ImageSize, face: ImageFace, cdnBase: string): string`
  - `scryfallImageUrl(printId: string, size: ImageSize, face?: ImageFace): string` — `buildImageUrl(..., "")`
  - `artCropFallbackUrl(printId: string, face?: ImageFace): string | null` — Scryfall art_crop when module CDN is non-empty and `printId` non-empty; else `null`
  - `imageUrlByPrint(printId, size?, face?)` — `buildImageUrl(printId, size, face, CDN)`

- [ ] **Step 1: Write the failing tests**

Append to `client/lib/deck-builder/scryfall.test.ts` (keep existing `searchPrints` User-Agent test):

```ts
import { afterEach, describe, expect, it, vi } from "vitest";
import { artCropFallbackUrl, buildImageUrl, imageUrlByPrint, scryfallImageUrl, searchPrints } from "./scryfall";

// ... existing afterEach + searchPrints describe ...

describe("buildImageUrl", () => {
  const id = "abcd1234-5678-90ab-cdef-000000000001";

  it("uses Scryfall version=art_crop when cdnBase is empty", () => {
    expect(buildImageUrl(id, "art_crop", "front", "")).toBe(
      `https://api.scryfall.com/cards/${id}?format=image&version=art_crop`,
    );
  });

  it("uses CDN art_crop folder when cdnBase is set", () => {
    expect(buildImageUrl(id, "art_crop", "front", "https://cards.example.com")).toBe(
      `https://cards.example.com/art_crop/front/a/b/${id}.webp`,
    );
  });

  it("maps non-art_crop sizes to CDN large folder when cdnBase is set", () => {
    expect(buildImageUrl(id, "large", "front", "https://cards.example.com")).toBe(
      `https://cards.example.com/large/front/a/b/${id}.webp`,
    );
    expect(buildImageUrl(id, "small", "back", "https://cards.example.com/")).toBe(
      `https://cards.example.com/large/back/a/b/${id}.webp`,
    );
  });

  it("adds face=back on Scryfall URLs", () => {
    expect(buildImageUrl(id, "art_crop", "back", "")).toBe(
      `https://api.scryfall.com/cards/${id}?format=image&version=art_crop&face=back`,
    );
  });

  it("returns empty string for empty print id", () => {
    expect(buildImageUrl("", "art_crop", "front", "https://cards.example.com")).toBe("");
  });
});

describe("scryfallImageUrl", () => {
  it("ignores CDN and always builds Scryfall", () => {
    const id = "ffff0000-0000-0000-0000-000000000001";
    expect(scryfallImageUrl(id, "art_crop")).toContain("version=art_crop");
    expect(scryfallImageUrl(id, "art_crop")).toContain("api.scryfall.com");
  });
});
```

Also add a note in the same file for `artCropFallbackUrl`: this helper depends on the module-level `CDN` from `import.meta.env.VITE_CARD_CDN`. In Vitest that is typically empty, so assert:

```ts
describe("artCropFallbackUrl", () => {
  it("returns null when module CDN is unset (default vitest)", () => {
    expect(artCropFallbackUrl("abcd1234-5678-90ab-cdef-000000000001")).toBeNull();
  });
});
```

If the Cloud/CI image bakes `VITE_CARD_CDN`, skip or assert non-null Scryfall URL instead — prefer making `artCropFallbackUrl` take an optional `cdnBase` override for tests:

```ts
export function artCropFallbackUrl(
  printId: string,
  face: ImageFace = "front",
  cdnBase: string = CDN,
): string | null {
  if (!printId || !cdnBase.replace(/\/$/, "")) return null;
  return scryfallImageUrl(printId, "art_crop", face);
}
```

Then test:

```ts
it("returns Scryfall art_crop when a cdnBase is provided", () => {
  const id = "abcd1234-5678-90ab-cdef-000000000001";
  expect(artCropFallbackUrl(id, "front", "https://cards.example.com")).toBe(
    `https://api.scryfall.com/cards/${id}?format=image&version=art_crop`,
  );
});

it("returns null when cdnBase is empty", () => {
  expect(artCropFallbackUrl("abcd1234-5678-90ab-cdef-000000000001", "front", "")).toBeNull();
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bunx vitest run lib/deck-builder/scryfall.test.ts`

Expected: FAIL — `buildImageUrl` / `artCropFallbackUrl` / `scryfallImageUrl` not exported (or CDN path still always `large`).

- [ ] **Step 3: Implement URL helpers**

Replace CDN helpers in `client/lib/deck-builder/scryfall.ts` with:

```ts
import { Schema as S } from "effect";

export type ImageSize = "small" | "normal" | "large" | "png" | "art_crop";
export type ImageFace = "front" | "back";

const CDN = String(import.meta.env.VITE_CARD_CDN ?? "").replace(/\/$/, "");

export function cardBackUrl(): string {
  return "/card-back.webp";
}

export function buildImageUrl(
  printId: string,
  size: ImageSize,
  face: ImageFace,
  cdnBase: string,
): string {
  if (!printId) return "";
  const base = cdnBase.replace(/\/$/, "");
  if (base) {
    const a = printId[0];
    const b = printId[1];
    const folder = size === "art_crop" ? "art_crop" : "large";
    return `${base}/${folder}/${face}/${a}/${b}/${printId}.webp`;
  }
  const faceParam = face === "back" ? "&face=back" : "";
  return `https://api.scryfall.com/cards/${printId}?format=image&version=${size}${faceParam}`;
}

export function scryfallImageUrl(printId: string, size: ImageSize, face: ImageFace = "front"): string {
  return buildImageUrl(printId, size, face, "");
}

export function artCropFallbackUrl(
  printId: string,
  face: ImageFace = "front",
  cdnBase: string = CDN,
): string | null {
  if (!printId) return null;
  if (!cdnBase.replace(/\/$/, "")) return null;
  return scryfallImageUrl(printId, "art_crop", face);
}

export function imageUrlByPrint(printId: string, size: ImageSize = "large", face: ImageFace = "front"): string {
  return buildImageUrl(printId, size, face, CDN);
}

// ... keep ScryfallPrintSchema + searchPrints unchanged ...
```

Delete the old private `cdnUrl` that always used `large`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run lib/deck-builder/scryfall.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/lib/deck-builder/scryfall.ts client/lib/deck-builder/scryfall.test.ts
git commit -m "feat(client): resolve CDN art_crop URLs separately from large"
```

---

### Task 2: ImageCache failure notify + cardArt Scryfall fallback

**Files:**
- Modify: `client/lib/image-cache.ts`
- Create: `client/lib/image-cache.test.ts`
- Modify: `client/lib/ui/card-art.ts`
- Modify: `client/lib/ui/card-art.test.ts`

**Interfaces:**
- Consumes: `artCropFallbackUrl`, `imageUrlByPrint` / `cardArtUrl`
- Produces:
  - `ImageCache.isFailed(url: string): boolean`
  - Subscriber notify on load **failure** as well as success
  - `cardArt` sets `data-art-fallback` when `opts.size === "art_crop"` and `artCropFallbackUrl(print)` is non-null
  - `syncCardArtHost`: if primary URL `isFailed` and fallback present, promote fallback to `data-art-url`, clear `data-art-fallback`, repaint

- [ ] **Step 1: Write the failing ImageCache test**

Create `client/lib/image-cache.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { ImageCache } from "./image-cache";

async function waitUntil(predicate: () => boolean, timeoutMs = 1000): Promise<void> {
  const start = Date.now();
  while (!predicate()) {
    if (Date.now() - start > timeoutMs) throw new Error("waitUntil timed out");
    await new Promise((r) => setTimeout(r, 5));
  }
}

describe("ImageCache failures", () => {
  it("marks url failed and notifies subscribers on onerror", async () => {
    let img!: { src: string; onload: (() => void) | null; onerror: (() => void) | null };
    let ticks = 0;
    const cache = new ImageCache(
      () => {
        ticks += 1;
      },
      () => {
        img = { src: "", onload: null, onerror: null };
        return img;
      },
    );
    let subTicks = 0;
    cache.subscribe(() => {
      subTicks += 1;
    });

    cache.get("https://example.test/missing.webp");
    expect(cache.isFailed("https://example.test/missing.webp")).toBe(false);
    img.onerror?.();
    await waitUntil(() => cache.isFailed("https://example.test/missing.webp"));
    expect(cache.isReady("https://example.test/missing.webp")).toBe(false);
    expect(ticks).toBeGreaterThan(0);
    expect(subTicks).toBeGreaterThan(0);
  });
});
```

- [ ] **Step 2: Run ImageCache test to verify it fails**

Run: `cd client && bunx vitest run lib/image-cache.test.ts`

Expected: FAIL — `isFailed` missing and/or no notify on error.

- [ ] **Step 3: Implement ImageCache failure tracking**

In `client/lib/image-cache.ts`, add a `failed` set and notify on error:

```ts
export class ImageCache {
  private images = new Map<string, ImageLike>();
  private ready = new Set<string>();
  private failed = new Set<string>();
  private listeners = new Set<() => void>();

  // ... constructor unchanged ...

  isReady(url: string): boolean {
    return this.ready.has(url);
  }

  isFailed(url: string): boolean {
    return this.failed.has(url);
  }

  get(url: string): HTMLImageElement | undefined {
    const existing = this.images.get(url);
    if (existing) return this.ready.has(url) ? (existing as HTMLImageElement) : undefined;

    const img = this.makeImage();
    this.images.set(url, img);
    this.failed.delete(url);

    const settled = new Promise<boolean>((resolve) => {
      img.onload = () => resolve(true);
      img.onerror = () => resolve(false);
    });
    img.src = url;

    const load = Effect.promise(() => settled).pipe(
      Effect.tap((success) =>
        Effect.sync(() => {
          if (success) {
            this.failed.delete(url);
            this.ready.add(url);
          } else {
            this.ready.delete(url);
            this.failed.add(url);
          }
          this.notifyLoaded();
        }),
      ),
    );
    Effect.runFork(load);

    return undefined;
  }

  // notifyLoaded unchanged
}
```

- [ ] **Step 4: Run ImageCache test to verify it passes**

Run: `cd client && bunx vitest run lib/image-cache.test.ts`

Expected: PASS

- [ ] **Step 5: Write the failing cardArt fallback test**

Append to `client/lib/ui/card-art.test.ts`:

```ts
describe("syncCardArtHost art_crop CDN fallback", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("swaps to data-art-fallback after primary load failure", async () => {
    let lastImg!: { src: string; onload: (() => void) | null; onerror: (() => void) | null };
    const cache = new ImageCache(
      () => {},
      () => {
        lastImg = { src: "", onload: null, onerror: null };
        return lastImg;
      },
    );

    const host = document.createElement("div");
    host.dataset.artUrl = "https://cards.example.com/art_crop/front/a/b/abcd.webp";
    host.dataset.artFallback = "https://api.scryfall.com/cards/abcd?format=image&version=art_crop";
    host.dataset.artAlt = "Commander";
    host.dataset.artClass = "art";
    document.body.append(host);

    syncCardArtHost(host, cache);
    expect(host.querySelector("[aria-hidden='true']")).toBeTruthy();
    lastImg.onerror?.();
    await waitUntil(() => cache.isFailed(host.dataset.artUrl ?? "nope") || host.dataset.artUrl?.includes("scryfall") === true);
    syncCardArtHost(host, cache);
    // After failure, host should promote fallback
    expect(host.dataset.artUrl).toContain("api.scryfall.com");
    expect(host.dataset.artFallback ?? "").toBe("");
    lastImg.onload?.();
    await waitUntil(() => cache.isReady(host.dataset.artUrl ?? ""));
    syncCardArtHost(host, cache);
    expect(host.querySelector("img")?.getAttribute("src")).toContain("api.scryfall.com");
  });
});
```

Adjust the wait/promote sequence to match the exact `syncCardArtHost` control flow you implement (subscribe-driven repaint may call sync automatically via BindCardArt; this unit test calls sync manually after onerror settles).

Also extend `cardArtUrl` / a small unit if useful:

```ts
it("cardArtUrl art_crop contains art_crop when no CDN (vitest default)", () => {
  expect(cardArtUrl("abcd1234-5678-90ab-cdef-000000000001", "art_crop")).toContain("version=art_crop");
});
```

- [ ] **Step 6: Run card-art test to verify fallback fails**

Run: `cd client && bunx vitest run lib/ui/card-art.test.ts`

Expected: FAIL — no fallback swap yet.

- [ ] **Step 7: Implement cardArt fallback wiring**

In `client/lib/ui/card-art.ts`:

1. Import `artCropFallbackUrl`.
2. In `cardArt`, when `opts.size === "art_crop"`, compute `fallback = artCropFallbackUrl(opts.print, opts.face ?? "front")` and set `h.DataAttribute("art-fallback", fallback)` when non-null (omit attribute when null).
3. Update MutationObserver `attributeFilter` to include `"data-art-fallback"`.
4. In `syncCardArtHost`:

```ts
export function syncCardArtHost(element: HTMLElement, cache: ImageCache = sharedImageCache): void {
  let url = element.dataset.artUrl ?? "";
  const fallback = element.dataset.artFallback ?? "";
  const alt = element.dataset.artAlt ?? "";
  const className = element.dataset.artClass ?? "";

  if (url && cache.isFailed(url) && fallback) {
    element.dataset.artUrl = fallback;
    delete element.dataset.artFallback;
    url = fallback;
  }

  element.replaceChildren();
  if (!url) return;

  if (cache.isReady(url)) {
    const img = document.createElement("img");
    img.src = url;
    img.alt = alt;
    img.draggable = false;
    img.className = className;
    element.append(img);
    return;
  }

  if (cache.isFailed(url)) {
    // Primary already failed and no (remaining) fallback — leave empty / broken intentionally for non-art_crop.
    return;
  }

  cache.get(url);
  const sk = document.createElement("div");
  sk.className = `${className} animate-skeleton bg-white/8`;
  sk.setAttribute("aria-hidden", "true");
  element.append(sk);
}
```

Ensure `cardArt` still defaults `size` to `"large"` so board/builder behavior is unchanged (no fallback attribute).

- [ ] **Step 8: Run card-art + image-cache tests**

Run: `cd client && bunx vitest run lib/image-cache.test.ts lib/ui/card-art.test.ts lib/deck-builder/scryfall.test.ts`

Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add client/lib/image-cache.ts client/lib/image-cache.test.ts client/lib/ui/card-art.ts client/lib/ui/card-art.test.ts
git commit -m "feat(client): fall back to Scryfall when CDN art_crop misses"
```

---

### Task 3: Remove deck-list hover preview

**Files:**
- Delete: `client/app/shell/decks/list/hover.ts`
- Modify: `client/app/shell/decks/list/messages.ts`
- Modify: `client/app/shell/decks/list/submodel.ts`
- Modify: `client/app/shell/decks/list/update.ts`
- Modify: `client/app/shell/decks/list/view.ts` (hover mounts/preview only — layout in Task 4)
- Modify: `client/app/shell/decks/messages.ts`
- Modify: `client/app/update.ts`
- Modify: `client/app/shell/decks/list/story.test.ts`
- Modify: `client/app/shell/surfaces.test.ts`
- Modify: `client/app/smoke.test.ts`

**Interfaces:**
- Consumes: remaining list messages (search, menu, delete, load)
- Produces: deck list with no hover submodel field and no `deck-list-hover-preview` node

- [ ] **Step 1: Write the failing Scene assertion (replace hover test)**

In `client/app/shell/decks/list/story.test.ts`, **replace** the test `"commander hover preview renders when model carries hover state"` with:

```ts
test("deck list does not render a hover preview", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      knownCommanders: {
        atraxa: card({
          id: "atraxa",
          name: "Atraxa, Praetors' Voice",
          color_identity: [2, 4, 5],
          default_print: "atraxa-print",
          legendary: true,
          kind: { kind: "creature", power: 4, toughness: 4 },
        }),
      },
      decks: [{ commander: "atraxa", commander_print: "atraxa-print", id: 1, name: "Superfriends" }],
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-hover-preview"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolveAll([BindCardArt, CardArtTick()], [BindCardArt, CardArtTick()]),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    Scene.Mount.expectEnded(BindCardArt),
  );
});
```

Remove `ClearedDeckListHover` / `BindDeckListCommanderHover` imports and all `Scene.Mount.resolve(..., ClearedDeckListHover())` / `BindDeckListCommanderHover` entries from other story tests, `surfaces.test.ts`, and `smoke.test.ts`. Update `isDeckListMessage` to drop hover tags.

- [ ] **Step 2: Run story test to see current hover still present / mounts required**

Run: `cd client && bunx vitest run app/shell/decks/list/story.test.ts`

Expected: FAIL or mount mismatch — hover mounts still required by view, or hover preview still exists if model somehow carries hover (after Step 1 the not.toExist may pass while other tests fail on unresolved `BindDeckListCommanderHover`).

- [ ] **Step 3: Remove hover from model, messages, update, view, wiring**

1. Delete `client/app/shell/decks/list/hover.ts`.
2. Remove `MovedDeckListHover` / `ClearedDeckListHover` from `messages.ts` (defs + union).
3. Remove `hover` from `DeckListSubmodel` and `initialDeckListSubmodel`.
4. Remove hover arms from `list/update.ts`.
5. In `view.ts`: remove `cardHoverPreviewView` import, `BindDeckListCommanderHover`, `hoverPreview()`, `OnMount(BindDeckListCommanderHover...)`, and `hoverPreview(model)` from the tree.
6. Stop re-exporting hover messages from `decks/messages.ts`.
7. Remove `MovedDeckListHover` / `ClearedDeckListHover` arms from `client/app/update.ts` `tagsExhaustive`.

- [ ] **Step 4: Run affected tests**

Run: `cd client && bunx vitest run app/shell/decks/list/story.test.ts app/shell/surfaces.test.ts app/smoke.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/list client/app/shell/decks/messages.ts client/app/update.ts client/app/shell/surfaces.test.ts client/app/smoke.test.ts
git commit -m "feat(client): remove Your decks tile hover preview"
```

---

### Task 4: Align column + enlarge art_crop tiles

**Files:**
- Modify: `client/app/shell/decks/list/view.ts`
- Modify: `client/app/shell/decks/list/story.test.ts`
- Modify: `client/app/shell/surfaces.test.ts` (optional class assertions)

**Interfaces:**
- Consumes: existing `view(model, username, apiVersion)`, `cardArt(..., size: "art_crop")`
- Produces: shared `max-w-[960px]` chrome; grid `minmax(220px, 1fr)`; art `aspect-[137/100]` (≈1.37:1) full width

- [ ] **Step 1: Write failing layout outcome assertions**

Add to `story.test.ts` (or extend an existing tile test):

```ts
test("deck list chrome and tiles share the wide column classes", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      decks: [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }],
      knownCommanders: { atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice", default_print: "atraxa-print" }) },
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-search"]')).toHaveClass("max-w-[960px]"),
    Scene.expect(Scene.selector('[data-testid="deck-list-grid"]')).toHaveClass("max-w-[960px]"),
    Scene.expect(Scene.selector('[data-testid="deck-list-grid"]')).toHaveClass(
      "grid-cols-[repeat(auto-fill,minmax(220px,1fr))]",
    ),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
  );
});
```

Also add `data-testid="deck-list-header"` on the header row with `max-w-[960px]`, and `data-testid="deck-list-grid"` on the grid container (required for the assertions above).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/shell/decks/list/story.test.ts`

Expected: FAIL — search still `max-w-[720px]`; no `deck-list-grid` testid / still `minmax(140px,…)`.

- [ ] **Step 3: Implement layout in `view.ts`**

Concrete class changes:

1. Header wrapper: `mx-auto mb-5 flex max-w-[960px] …` + `h.DataAttribute("testid", "deck-list-header")`.
2. Section stays `mx-auto max-w-[960px]`.
3. Search: `fieldClass("mb-md w-full max-w-[960px]")` (or omit inner max-w and rely on section — either way search must not be 720). Prefer `fieldClass("mb-md w-full")` inside the 960 section.
4. Grid:

```ts
h.div(
  [
    h.DataAttribute("testid", "deck-list-grid"),
    h.Class("mx-auto grid max-w-[960px] grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-md"),
  ],
  visible.map((deck) => { /* ... */ }),
)
```

5. Art host / placeholder: replace `h-[110px] w-full …` with `aspect-[137/100] w-full object-cover` (placeholder: `aspect-[137/100] w-full bg-glass`). Keep `cardArt(..., size: "art_crop", ...)`.
6. Label block: keep single-line `truncate` on name and commander; can reduce `min-h-[86px]` slightly if art is taller — keep readable padding (`p-md`).

Tune `220` → `240` only if Scene/manual check still clips badly; default plan value is **220**.

- [ ] **Step 4: Run story + surfaces tests**

Run: `cd client && bunx vitest run app/shell/decks/list/story.test.ts app/shell/surfaces.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/list/view.ts client/app/shell/decks/list/story.test.ts client/app/shell/surfaces.test.ts
git commit -m "feat(client): enlarge and align Your decks tile grid"
```

---

### Task 5: Shell current-behavior spec + design status + verify

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md`
- Modify: `docs/superpowers/specs/2026-07-24-deck-list-tile-layout-polish-design.md` (Status → Implemented)

**Interfaces:**
- Consumes: implemented behavior from Tasks 1–4
- Produces: shell spec matches shipped UI/CDN rules

- [ ] **Step 1: Update deck list + CDN paragraphs in the shell spec**

Replace the **Deck list** paragraph with:

```markdown
**Deck list** (`/`) shows saved decks from the deck list submodel as a compact tile grid.
Header, search, and grid share one `max-w-[960px]` column. Tiles use a raised
`minmax(220px, 1fr)` track, landscape commander `art_crop` (~1.37:1), deck name,
color-identity pips, and a Precon chip when `id < 0`. Names stay single-line truncate.
There is no cursor-follow card hover preview on this surface. The whole tile links to
`/play?deck={id}`. A **Search decks…** field filters by deck name and commander display
name (client-only). Display order: owned decks first (API relative order), then precons
by ascending id (newest release first). Right-click on an owned deck opens Edit
(`/decks/{id}`) and Delete (confirm dialog); precons do not open a context menu. A New
Deck button navigates to `/decks/new`.
```

Replace the **Card art CDN** `imageUrlByPrint` bullets with:

```markdown
Art is keyed by Scryfall **Printing** UUID. `imageUrlByPrint(printId, size, face)` returns:
- When `VITE_CARD_CDN` is set and `size === "art_crop"`: CDN
  `VITE_CARD_CDN/art_crop/{face}/{a}/{b}/{id}.webp`. If that asset fails to load, `cardArt`
  falls back once to Scryfall `version=art_crop` (deck-list tiles use this path).
- When `VITE_CARD_CDN` is set and `size` is any other value: CDN
  `VITE_CARD_CDN/large/{face}/{a}/{b}/{id}.webp` — missing `large` does **not** fall back to Scryfall.
- When `VITE_CARD_CDN` is unset: Scryfall image API
  (`https://api.scryfall.com/cards/{id}?format=image&version={size}`) (local/dev).
```

Keep the existing sentence about DFC `face=back` / no Scryfall fallback for ordinary missing CDN art, but ensure it does not contradict the art_crop exception above (qualify “ordinary” / non-`art_crop`).

- [ ] **Step 2: Mark design doc Implemented**

In `docs/superpowers/specs/2026-07-24-deck-list-tile-layout-polish-design.md`, set `**Status:** Implemented`.

- [ ] **Step 3: Full client verification**

Run: `cd client && bun run typecheck && bunx vitest run`

Expected: typecheck clean; all tests PASS.

If format/lint are part of local habit: `just client-format` / `just client-lint` (or `just client-check`).

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md docs/superpowers/specs/2026-07-24-deck-list-tile-layout-polish-design.md
git commit -m "docs(client): record Your decks layout polish as current behavior"
```

---

## Spec coverage self-check

| Spec requirement | Task |
|------------------|------|
| Shared ~960px column | Task 4 |
| Larger tiles / less truncate clipping | Task 4 |
| Real art_crop (CDN prefer) | Tasks 1–2 |
| Scryfall fallback only for missing CDN art_crop | Task 2 |
| No hover preview | Task 3 |
| Play / search / order / context menu unchanged | Tasks 3–4 (preserve; existing tests stay) |
| Shell current-behavior update | Task 5 |
| Lobby / CDN ingest / large fallback non-goals | Not implemented (explicit) |
