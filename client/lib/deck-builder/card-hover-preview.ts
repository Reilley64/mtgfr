// Shared card preview: cursor-follow (builder/list) and left-dock inspect (board).

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

export type CardPreviewMode = "follow" | "dock";

export type FollowPreviewArgs = {
  mode?: "follow";
  hover: CardHover;
  card: HoverPreviewCard | undefined;
  testId?: string;
};

export type DockPreviewArgs<M> = {
  mode: "dock";
  print: string;
  name: string;
  oracle?: string | null;
  approximates?: string | null;
  face?: "front" | "back";
  /** Extra right-column nodes (modifier ledger Html). */
  extras?: ReadonlyArray<Html>;
  /** Optional; board wires backdrop click via wrapper. */
  onDismiss?: M;
  testId?: string;
};

export type CardPreviewArgs<M> = FollowPreviewArgs | DockPreviewArgs<M>;

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

function artColumn<M>(
  h: HtmlFactory<M>,
  print: string,
  name: string,
  face: "front" | "back" | undefined,
): Html {
  return cardArt(h, {
    print,
    size: "large",
    face,
    alt: name,
    className: "w-(--w) flex-none rounded-[14px] shadow-table",
    style: { "--w": `${PREVIEW_W}px` },
  });
}

function followPreviewView<M>(h: HtmlFactory<M>, args: FollowPreviewArgs): Html {
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
      artColumn(h, print, card?.name ?? "", undefined),
      textPanel(h, oracle, approximates, `${PREVIEW_H}px`),
    ],
  );
}

function dockPreviewView<M>(h: HtmlFactory<M>, args: DockPreviewArgs<M>): Html {
  const {
    print,
    name,
    oracle,
    approximates,
    face,
    extras,
    onDismiss,
    testId = "card-hover-preview",
  } = args;

  const text = textPanel(h, oracle, approximates, `${PREVIEW_H}px`);
  const rightColumn =
    text != null || (extras != null && extras.length > 0)
      ? h.div(
          [h.Class("flex min-w-0 flex-col items-start gap-3")],
          [text, ...(extras ?? [])],
        )
      : null;

  const content = h.div(
    [h.Class("pointer-events-auto relative m-lg flex flex-row items-start gap-3")],
    [artColumn(h, print, name, face), rightColumn],
  );

  return h.div(
    [
      h.DataAttribute("testid", testId),
      h.Class("fixed inset-0 z-[100] flex items-start bg-black/55"),
      ...(onDismiss !== undefined ? [h.OnClick(onDismiss)] : []),
    ],
    [content],
  );
}

/** Large face + optional oracle panel — `follow` (cursor) or `dock` (left + backdrop). */
export function cardHoverPreviewView<M>(h: HtmlFactory<M>, args: CardPreviewArgs<M>): Html {
  if (args.mode === "dock") return dockPreviewView(h, args);
  return followPreviewView(h, args);
}
