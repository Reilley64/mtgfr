import { Effect } from "effect";
import type { html as createHtml, Html } from "foldkit/html";
import { m } from "foldkit/message";
import * as Mount from "foldkit/mount";
import { cardBackUrl, type ImageFace, type ImageSize, imageUrlByPrint } from "../deck-builder/scryfall";
import { type ImageCache, sharedImageCache } from "../image-cache";

export function cardArtUrl(print: string, size: ImageSize = "large", face: ImageFace = "front"): string {
  if (!print) return cardBackUrl();
  return imageUrlByPrint(print, size, face);
}

/** Dispatched when card art mounts — handled as a no-op by the app update (see messages.ts). */
export const CardArtTick = m("CardArtTick");

/**
 * Paint a BindCardArt host from its `data-art-*` attributes.
 * Safe to call again when the URL/alt/class change (hover preview print swaps).
 */
export function syncCardArtHost(element: HTMLElement, cache: ImageCache = sharedImageCache): void {
  const url = element.dataset.artUrl ?? "";
  const alt = element.dataset.artAlt ?? "";
  const className = element.dataset.artClass ?? "";

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

  cache.get(url);
  const sk = document.createElement("div");
  sk.className = `${className} animate-skeleton bg-white/8`;
  sk.setAttribute("aria-hidden", "true");
  element.append(sk);
}

/** Mount: host is a sized box; paints skeleton then img when sharedImageCache is ready. */
export const BindCardArt = Mount.define(
  "BindCardArt",
  CardArtTick,
)((element) =>
  Effect.gen(function* () {
    yield* Effect.acquireRelease(
      Effect.sync(() => {
        if (!(element instanceof HTMLElement)) return null;

        const paint = () => syncCardArtHost(element);
        paint();
        const unsub = sharedImageCache.subscribe(paint);
        // Foldkit patches `data-art-url` in place on hover card changes — remount does not run.
        const observer = new MutationObserver(paint);
        observer.observe(element, {
          attributes: true,
          attributeFilter: ["data-art-url", "data-art-alt", "data-art-class"],
        });
        return { unsub, observer };
      }),
      (handle) =>
        Effect.sync(() => {
          handle?.unsub();
          handle?.observer.disconnect();
        }),
    );
    return CardArtTick();
  }),
);

export function cardArt<M>(
  h: ReturnType<typeof createHtml<M>>,
  opts: {
    print: string;
    size?: ImageSize;
    face?: ImageFace;
    alt: string;
    className: string;
    style?: Record<string, string>;
    testId?: string;
  },
): Html {
  const url = cardArtUrl(opts.print, opts.size ?? "large", opts.face ?? "front");
  return h.div(
    [
      h.Class(`${opts.className} relative overflow-hidden`),
      h.DataAttribute("art-url", url),
      h.DataAttribute("art-alt", opts.alt),
      h.DataAttribute("art-class", opts.className),
      h.OnMount(BindCardArt() as never),
      ...(opts.style ? [h.Style(opts.style)] : []),
      ...(opts.testId ? [h.DataAttribute("testid", opts.testId)] : []),
    ],
    [],
  );
}
