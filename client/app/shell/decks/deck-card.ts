import type { Attribute, Html, HtmlBuilder } from "foldkit/html";
import { BindDeckCardFlip } from "../../deck-card-nav";
import { manaFontClass } from "../../domain/oracleText";
import { cardArt } from "../../domain/ui/card-art";
import { listRowClass } from "../../domain/ui/surfaces";
import { identityPipCodes } from "./list/visible";

export type DeckCardModel = {
  id: number;
  name: string;
  commander: string;
  commanderName: string;
  print: string;
  colorIdentity: readonly number[];
};

function renderPips<Msg>(h: HtmlBuilder<Msg>, colorIdentity: readonly number[]): Html {
  const pips = identityPipCodes(colorIdentity);
  if (pips.length === 0) return null;

  return h.div(
    [h.Class("flex gap-[3px] text-[14px] text-snow")],
    pips.map((code) => {
      const ms = manaFontClass(code);
      if (ms == null) return null;
      return h.i([h.Class(`ms ms-cost ms-${ms}`)], []);
    }),
  );
}

function renderDeckCardBody<Msg>(h: HtmlBuilder<Msg>, card: DeckCardModel, opts: { showPlayLabel: boolean }): Html {
  // Flip mounts on the inner chrome so the outer root can keep a separate OnMount
  // (e.g. deck-list context menu) — Foldkit allows one OnMount per element.
  return h.div(
    [
      h.Class("flex flex-1 flex-col"),
      h.DataAttribute("deck-card-flip", String(card.id)),
      // Same cast as cardArt's BindCardArt — Mount tick is not constrained to Msg.
      h.OnMount(BindDeckCardFlip({ deckId: card.id }) as never),
    ],
    [
      card.print === ""
        ? h.div([h.Class("aspect-[137/100] w-full bg-glass")], [])
        : cardArt(h, {
            print: card.print,
            size: "art",
            alt: "",
            className: "aspect-[137/100] w-full object-cover",
          }),
      h.div(
        [h.Class("flex min-h-[86px] flex-col gap-xs p-md")],
        [
          h.div(
            [h.Class("truncate text-label font-semibold")],
            [
              card.name,
              card.id < 0
                ? h.span(
                    [h.Class("ml-sm rounded-full bg-lichen/14 px-[7px] py-px align-middle text-chip text-lichen")],
                    ["Precon"],
                  )
                : null,
            ],
          ),
          h.div([h.Class("truncate text-chip text-lichen")], [card.commanderName]),
          h.div(
            [h.Class("mt-auto flex items-end justify-between gap-sm")],
            [
              renderPips(h, card.colorIdentity),
              opts.showPlayLabel
                ? h.span(
                    [
                      h.DataAttribute("testid", "deck-play-label"),
                      h.Class("shrink-0 text-chip font-semibold uppercase tracking-chip text-vine"),
                    ],
                    ["Play"],
                  )
                : null,
            ],
          ),
        ],
      ),
    ],
  );
}

export function renderDeckCard<Msg>(
  h: HtmlBuilder<Msg>,
  card: DeckCardModel,
  opts: {
    mode: "link" | "static";
    href?: string;
    rootAttrs?: ReadonlyArray<Attribute<Msg>>;
    testId: string;
  },
): Html {
  const attrs = [
    h.DataAttribute("testid", opts.testId),
    h.Class(listRowClass("relative flex flex-col overflow-hidden rounded-hud no-underline text-snow")),
    ...(opts.rootAttrs ?? []),
  ];
  const body = renderDeckCardBody(h, card, { showPlayLabel: opts.mode === "link" });

  if (opts.mode === "static") {
    return h.div(attrs, [body]);
  }

  return h.a([h.Href(opts.href ?? ""), ...attrs], [body]);
}
