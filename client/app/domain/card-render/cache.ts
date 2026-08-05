import { imageUrlByPrint } from "../deck-builder/scryfall";
import { type ImageCache, sharedImageCache } from "../image-cache";
import { CANONICAL, type FaceData, type FaceVariant } from "./frame";
import { drawFace, faceAssetUrls } from "./render";

/**
 * Identifies a drawn face. **Every field of `FaceData` goes in**, because every field changes what
 * the draw puts in the bitmap — a creature that grows, a land animated into one, a permanent whose
 * colour was changed. Anything left out serves a stale face forever. `cache.test.ts` sweeps the
 * whole type to prove nothing is missing.
 */
export function faceKey(face: FaceData, variant: FaceVariant): string {
  const pt = `${face.power}/${face.toughness}/${face.loyalty}`;
  const flags = `${face.isLand ? "L" : ""}${face.isToken ? "t" : ""}${face.legendary ? "l" : ""}`;
  return `${variant}:${face.print}:${face.name}:${face.colors.join("")}:${pt}:${flags}:${face.typeLine}:${face.oracle}:${face.flavor}`;
}

type Images = Pick<ImageCache, "get" | "preload" | "subscribe" | "isFailed">;
type MakeCanvas = (w: number, h: number) => OffscreenCanvas;

/** How many drawn faces to keep. A four-seat board rarely shows more than a hundred at once. */
const MAX_FACES = 240;

/**
 * Drawn card faces, keyed by everything the draw reads.
 *
 * A face needs images that arrive at different times: the printing's art (the card CDN) and the
 * vendored frame assets. `request` preloads whatever is missing and returns; `get` serves the face
 * once the frame has landed, and subscribers repaint. Same posture as `ImageCache`, which the board
 * already drives this way.
 *
 * The card's *facts* need no fetch at all — `FaceData` is read straight off the `ObjectView` the
 * board already holds.
 */
export class CardFaceCache {
  private faces = new Map<string, OffscreenCanvas>();
  private pending = new Map<string, { face: FaceData; variant: FaceVariant }>();
  private listeners = new Set<() => void>();

  constructor(
    private images: Images,
    private makeCanvas: MakeCanvas = (w, h) => new OffscreenCanvas(w, h),
  ) {
    this.images.subscribe(() => this.drawReady());
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  get(face: FaceData, variant: FaceVariant): CanvasImageSource | undefined {
    return this.faces.get(faceKey(face, variant));
  }

  /**
   * Drops every drawn face so the next paint redraws them. The card typefaces load after the first
   * frames are already on screen, and a bitmap keeps whatever typeface drew it — without this the
   * board would show the fallback serif for the rest of the session.
   */
  clear(): void {
    this.faces.clear();
    this.pending.clear();
    for (const listener of this.listeners) listener();
  }

  /** Ensures this face is drawn, or waiting on an image. Cheap to call every frame. */
  request(face: FaceData, variant: FaceVariant): void {
    const key = faceKey(face, variant);
    if (this.faces.has(key) || this.pending.has(key)) return;
    this.pending.set(key, { face, variant });

    const urls = faceAssetUrls(face);
    const artUrl = artUrlFor(face);
    // One call: `preload` takes an iterable, and a bare string is an iterable of *characters*.
    this.images.preload([urls.frame, urls.pt, urls.crown, artUrl].filter((url) => url != null));

    this.drawReady();
  }

  /**
   * Draws every pending face whose frame asset has landed.
   *
   * The frame gates the draw; the art does not. A card with no art yet is still a readable card —
   * name, frame, P/T — and blocking on the CDN would leave the tile blank for the whole round trip.
   * But the face stays pending until the art settles, so it is redrawn when the art lands: the
   * frame is a local asset and always wins that race, so without the redraw every cold-loaded card
   * would keep the art-less face forever.
   */
  private drawReady(): void {
    let drew = false;
    for (const [key, { face, variant }] of [...this.pending]) {
      const urls = faceAssetUrls(face);
      const frameImage = this.images.get(urls.frame);
      if (frameImage == null) continue;
      // The plate and the crown are vendored alongside the frame and arrive on their own requests.
      // Only the art earns a redraw, so a face drawn while one was still in flight would keep the
      // hole for the rest of the session — wait for them, unless the load has failed outright.
      const pending = [urls.pt, urls.crown].some(
        (url) => url != null && this.images.get(url) == null && !this.images.isFailed(url),
      );
      if (pending) continue;

      const artUrl = artUrlFor(face);
      const art = artUrl == null ? null : (this.images.get(artUrl) ?? null);
      const waitingOnArt = artUrl != null && art == null && !this.images.isFailed(artUrl);

      if (this.faces.has(key)) {
        // Already drawn art-less. Only the art landing is worth a redraw; a failed url ends the wait.
        if (waitingOnArt) continue;
        if (art == null) {
          this.pending.delete(key);
          continue;
        }
      }

      const { w, h } = CANONICAL[variant];
      const canvas = this.makeCanvas(w, h);
      const ctx = canvas.getContext("2d");
      if (ctx == null) continue;

      drawFace(ctx as unknown as CanvasRenderingContext2D, {
        face,
        variant,
        art,
        frameImage,
        ptImage: urls.pt == null ? null : (this.images.get(urls.pt) ?? null),
        crownImage: urls.crown == null ? null : (this.images.get(urls.crown) ?? null),
      });

      this.faces.set(key, canvas);
      if (!waitingOnArt) this.pending.delete(key);
      this.evict();
      drew = true;
    }
    if (drew) for (const listener of this.listeners) listener();
  }

  // ponytail: insertion-order eviction, not true LRU — a Map's oldest entry is the oldest *drawn*
  // face, which on a board that only grows is the same thing. Track access time if a long game
  // starts thrashing.
  private evict(): void {
    while (this.faces.size > MAX_FACES) {
      const oldest = this.faces.keys().next();
      if (oldest.done) return;
      this.faces.delete(oldest.value);
    }
  }
}

/** The `art` size is the art box alone — exactly what the art window draws. Tokens have no print. */
function artUrlFor(face: FaceData): string | null {
  return face.print === "" ? null : imageUrlByPrint(face.print, "art");
}

/** The board's face cache. Mirrors `sharedImageCache`: one per document. */
export const sharedFaceCache = new CardFaceCache(sharedImageCache);
