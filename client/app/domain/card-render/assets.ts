/**
 * Vendored card-frame art and typefaces, under `client/public/`. One manifest so a rename is a
 * one-line change and `assets.test.ts` can prove every entry is on disk.
 *
 * Every image is normalised to the same 750x1050 full-card canvas — frame, P/T box, and legend
 * crown alike — so the renderer blits all three at one rect instead of carrying per-asset bounds.
 */

/** The frame a card draws in: one per colour, plus multicolour, colourless, and land. */
export type FrameKey = "w" | "u" | "b" | "r" | "g" | "m" | "c" | "land";

const FRAME_KEYS: FrameKey[] = ["w", "u", "b", "r", "g", "m", "c", "land"];

/** The canvas every vendored asset is drawn on. Matches `CANONICAL.full` closely enough to blit 1:1. */
export const ASSET_W = 750;
export const ASSET_H = 1050;

function m15Set(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const key of FRAME_KEYS) {
    out[`m15/${key}`] = `/card-frames/m15/${key}.webp`;
    // Lands have no printed P/T; legendary lands are common enough to want a crown.
    if (key !== "land") out[`m15/pt/${key}`] = `/card-frames/m15/pt/${key}.webp`;
    out[`m15/crown/${key}`] = `/card-frames/m15/crown/${key}.webp`;
  }
  return out;
}

export const FRAME_ASSETS: Record<string, string> = m15Set();

export function frameAssetUrl(name: string): string {
  const url = FRAME_ASSETS[name];
  if (url == null) throw new Error(`unknown card frame asset: ${name}`);
  return url;
}

export const TITLE_FONT = "Beleren";
export const BODY_FONT = "MPlantin";
/** mana-font, declared in `global.css` — the pips inside rules text are drawn from it. */
export const SYMBOL_FONT = "Mana";

let fontsReady: Promise<void> | null = null;

/** Loads the card typefaces into the document so canvas `ctx.font` can name them. Idempotent. */
export function loadCardFonts(): Promise<void> {
  if (fontsReady != null) return fontsReady;
  if (typeof FontFace !== "function" || typeof document === "undefined") {
    fontsReady = Promise.resolve();
    return fontsReady;
  }
  const faces = [
    new FontFace(TITLE_FONT, `url(/card-fonts/beleren-bold.ttf) format("truetype")`, { weight: "700" }),
    new FontFace(BODY_FONT, `url(/card-fonts/mplantin.ttf) format("truetype")`),
  ];
  fontsReady = Promise.all([
    ...faces.map((face) => face.load().then((loaded) => document.fonts.add(loaded))),
    // Mana is declared in `global.css` (the pip tray uses it too) — this only waits for the bytes,
    // so the first face drawn has its `{T}` rather than a blank box.
    document.fonts.load(`16px ${SYMBOL_FONT}`),
  ]).then(() => undefined);
  return fontsReady;
}
