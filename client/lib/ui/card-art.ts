import { Effect } from "effect";
import type { html as createHtml, Html } from "foldkit/html";
import { m } from "foldkit/message";
import * as Mount from "foldkit/mount";
import { cardBackUrl, type ImageFace, type ImageSize, imageUrlByPrint } from "../deck-builder/scryfall";
import { sharedImageCache } from "../image-cache";

export function cardArtUrl(print: string, size: ImageSize = "large", face: ImageFace = "front"): string {
  if (!print) return cardBackUrl();
  return imageUrlByPrint(print, size, face);
}

/** Dispatched when card art mounts — handled as a no-op by update. */
export const CardArtTick = m("CardArtTick");

/** Mount: host is a sized box; paints skeleton then img when sharedImageCache is ready. */
export const BindCardArt = Mount.define(
  "BindCardArt",
  CardArtTick,
)((element) =>
  Effect.gen(function* () {
    yield* Effect.acquireRelease(
      Effect.sync(() => {
        if (!(element instanceof HTMLElement)) return null;
        const url = element.dataset.artUrl ?? "";
        const alt = element.dataset.artAlt ?? "";
        const paint = () => {
          element.replaceChildren();
          if (!url) return;
          if (sharedImageCache.isReady(url)) {
            const img = document.createElement("img");
            img.src = url;
            img.alt = alt;
            img.draggable = false;
            img.className = element.dataset.artClass ?? "";
            element.append(img);
            return;
          }
          sharedImageCache.get(url);
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
  const attrs = [
    h.Class(`${opts.className} relative overflow-hidden`),
    h.DataAttribute("art-url", url),
    h.DataAttribute("art-alt", opts.alt),
    h.DataAttribute("art-class", opts.className),
    h.OnMount(BindCardArt() as never),
  ];
  if (opts.style) attrs.push(h.Style(opts.style));
  if (opts.testId) attrs.push(h.DataAttribute("testid", opts.testId));
  return h.div(attrs, []);
}
