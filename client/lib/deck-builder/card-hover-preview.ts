// Cursor-follow card preview shared by deck builder and deck list commander hover.

import type { Html, html } from "foldkit/html";
import { cn } from "../cn";
import { type OraclePart, splitOracleText } from "../oracleText";
import { cardArt } from "../ui/card-art";

export type CardHover = {
  id: string;
  print: string;
  x: number;
  y: number;
};

export type HoverPreviewCard = {
  approximates?: string | null;
  default_print?: string;
  name?: string;
  oracle?: string | null;
};

const PREVIEW_W = 320;
const PREVIEW_H = Math.round(PREVIEW_W / 0.716);
const PREVIEW_PANEL = 300;
const PREVIEW_GAP = 24;

const PANEL_CARD =
  "w-(--w) shrink-0 rounded-panel border border-vine bg-forest-surface px-xl py-lg text-preview-ash leading-[1.4]";

type HtmlFactory<M> = ReturnType<typeof html<M>>;

function oracleRichText<M>(h: HtmlFactory<M>, text: string): Array<Html | string> {
  return splitOracleText(text).map((part: OraclePart) => {
    if (part.kind === "symbol") {
      return h.span(
        [
          h.Role("img"),
          h.Attribute("aria-label", `{${part.code}}`),
          h.Class(cn("ms", "ms-cost", "ms-oracle", `ms-${part.ms}`, part.reminder && "italic")),
        ],
        [],
      );
    }
    if (part.reminder) return h.i([], [part.text]);
    return part.text;
  });
}

function textPanel<M>(
  h: HtmlFactory<M>,
  oracle: string | null | undefined,
  approximates: string | null | undefined,
  maxH: string,
): Html | null {
  if (!oracle && !approximates) return null;
  return h.div(
    [
      h.Class(cn(PANEL_CARD, "text-body", "max-h-(--max-h) overflow-y-auto")),
      h.Style({ "--w": `${PREVIEW_PANEL}px`, "--max-h": maxH }),
    ],
    [
      oracle ? h.div([h.Class("whitespace-pre-wrap")], [...oracleRichText(h, oracle)]) : null,
      approximates
        ? h.div(
            [h.Class(cn("text-label text-note-gold italic", oracle && "mt-3 border-vine/40 border-t pt-3"))],
            [h.span([h.Class("font-semibold not-italic")], ["Approximation: "]), ...oracleRichText(h, approximates)],
          )
        : null,
    ],
  );
}

/** Large face + optional oracle panel following the cursor (Solid HoverPreview / CardPreview). */
export function cardHoverPreviewView<M>(
  h: HtmlFactory<M>,
  args: {
    hover: CardHover;
    card: HoverPreviewCard | undefined;
    testId?: string;
  },
): Html {
  const { hover, card, testId = "card-hover-preview" } = args;
  const oracle = card?.oracle ?? null;
  const approximates = card?.approximates ?? null;
  const hasText = !!(oracle || approximates);
  const width = hasText ? PREVIEW_W + 12 + PREVIEW_PANEL : PREVIEW_W;
  const vw = typeof window !== "undefined" ? window.innerWidth : 1280;
  const vh = typeof window !== "undefined" ? window.innerHeight : 720;
  const flipped = hover.x + PREVIEW_GAP + width > vw;
  const left = flipped ? Math.max(PREVIEW_GAP, hover.x - PREVIEW_GAP - width) : hover.x + PREVIEW_GAP;
  const top = Math.min(Math.max(PREVIEW_GAP, hover.y - PREVIEW_H / 2), vh - PREVIEW_H - PREVIEW_GAP);
  const print = hover.print || card?.default_print || "";

  return h.div(
    [
      h.DataAttribute("testid", testId),
      h.Class(
        cn(
          "pointer-events-none fixed top-(--y) left-(--x) z-40 flex flex-row items-start gap-3",
          flipped && "flex-row-reverse",
        ),
      ),
      h.Style({ "--x": `${left}px`, "--y": `${top}px` }),
    ],
    [
      cardArt(h, {
        print,
        size: "large",
        alt: card?.name ?? "",
        className: "w-(--w) flex-none rounded-[14px] shadow-table",
        style: { "--w": `${PREVIEW_W}px` },
      }),
      textPanel(h, oracle, approximates, `${PREVIEW_H}px`),
    ],
  );
}
