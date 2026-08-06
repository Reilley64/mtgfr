import { Effect } from "effect";
import type { Html, HtmlBuilder } from "foldkit/html";
import { m } from "foldkit/message";
import * as Mount from "foldkit/mount";
import { type CardFaceCache, sharedFaceCache } from "../card-render/cache";
import type { FaceData, FaceVariant } from "../card-render/frame";

/** Dispatched when a card face mounts — handled as a no-op by the app update (see messages.ts). */
export const CardFaceTick = m("CardFaceTick");

type Faces = Pick<CardFaceCache, "get" | "request">;

/**
 * Paint a BindCardFace host from its `data-face*` attributes: the rendered card face, drawn by
 * `card-render`, blitted into a canvas the size of the host's box.
 *
 * The face is a bitmap the cache draws once at canonical size; the canvas here is the on-screen
 * copy. Its bitmap is in device pixels so a retina hand bar draws a sharp card rather than a
 * doubled-up blur.
 */
export function syncCardFaceHost(element: HTMLElement, faces: Faces = sharedFaceCache, dpr?: number): void {
  const raw = element.dataset.face;
  if (raw == null) return;
  const face = JSON.parse(raw) as FaceData;
  const variant = (element.dataset.faceVariant ?? "full") as FaceVariant;
  const w = Number(element.dataset.faceW);
  const h = Number(element.dataset.faceH);
  const className = element.dataset.faceClass ?? "";

  // `request` draws on the spot when the frame and art are already in the image cache — the usual
  // case once a card of that colour is on the board. Read again after it: the cache tells its
  // listeners it drew, and this host has not subscribed yet on its first paint, so trusting the
  // first read leaves the skeleton up for good.
  let drawn = faces.get(face, variant);
  if (drawn == null) {
    faces.request(face, variant);
    drawn = faces.get(face, variant);
  }
  if (drawn == null) {
    element.replaceChildren();
    const sk = document.createElement("div");
    // Same skeleton the printed-image host shows: the hand bar sizes its host by inline style, so
    // a class-only skeleton would collapse to nothing while the frame assets load.
    sk.className = `${className} absolute inset-0 animate-skeleton bg-white/8`;
    sk.setAttribute("aria-hidden", "true");
    element.append(sk);
    return;
  }

  const scale = dpr ?? (typeof devicePixelRatio === "number" ? devicePixelRatio : 1);
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(w * scale);
  canvas.height = Math.round(h * scale);
  canvas.style.width = `${w}px`;
  canvas.style.height = `${h}px`;
  canvas.className = className;
  canvas.setAttribute("role", "img");
  canvas.setAttribute("aria-label", face.name);
  const ctx = canvas.getContext("2d");
  ctx?.drawImage(drawn, 0, 0, canvas.width, canvas.height);
  element.replaceChildren(canvas);
}

/** Mount: host is a sized box; paints the rendered face, and again when the cache draws it. */
export const BindCardFace = Mount.define(
  "BindCardFace",
  CardFaceTick,
)((element) =>
  Effect.gen(function* () {
    yield* Effect.acquireRelease(
      Effect.sync(() => {
        if (!(element instanceof HTMLElement)) return null;

        const paint = () => syncCardFaceHost(element);
        paint();
        const unsub = sharedFaceCache.subscribe(paint);
        // Foldkit patches these in place as the hand changes — remount does not run. The box size
        // is in the filter too: the hand bar shrinks its tiles as it fills, and the canvas carries
        // its own inline width.
        const observer = new MutationObserver(paint);
        observer.observe(element, {
          attributes: true,
          attributeFilter: ["data-face", "data-face-variant", "data-face-w", "data-face-h", "data-face-class"],
        });
        return { unsub, observer };
      }),
      (handle) =>
        Effect.sync(() => {
          handle?.unsub();
          handle?.observer.disconnect();
        }),
    );
    return CardFaceTick();
  }),
);

/**
 * A card drawn from the vendored frame assets rather than the printed card image — the same face
 * the battlefield paints, in the printed card's shape. No mana cost: the pip tray owns cost.
 */
export function cardFace<M>(
  h: HtmlBuilder<M>,
  opts: {
    face: FaceData;
    variant?: FaceVariant;
    width: number;
    height: number;
    className: string;
    style?: Record<string, string>;
    testId?: string;
  },
): Html {
  return h.div(
    [
      h.Class(`${opts.className} relative overflow-hidden`),
      h.DataAttribute("face", JSON.stringify(opts.face)),
      h.DataAttribute("face-variant", opts.variant ?? "full"),
      h.DataAttribute("face-w", String(opts.width)),
      h.DataAttribute("face-h", String(opts.height)),
      h.DataAttribute("face-class", opts.className),
      h.OnMount(BindCardFace() as never),
      ...(opts.style ? [h.Style(opts.style)] : []),
      ...(opts.testId ? [h.DataAttribute("testid", opts.testId)] : []),
    ],
    [],
  );
}
