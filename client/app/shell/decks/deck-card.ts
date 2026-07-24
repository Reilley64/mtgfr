import type { html as createHtml, Html } from "foldkit/html";
import { deckCardViewTransitionName } from "../../deck-id";
import { manaFontClass } from "../../../lib/oracleText";
import { cardArt } from "../../../lib/ui/card-art";
import { listRowClass } from "../../../lib/ui/surfaces";
import { identityPipCodes } from "./list/visible";

export type DeckCardModel = {
  id: number;
  name: string;
  commander: string;
  commanderName: string;
  print: string;
  colorIdentity: readonly number[];
};

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

function renderPips<Msg>(h: HtmlFactory<Msg>, colorIdentity: readonly number[]): Html {
  const pips = identityPipCodes(colorIdentity);
  if (pips.length === 0) return null;

  return h.div(
    [h.Class("mt-auto flex gap-[3px] text-[14px] text-snow")],
    pips.map((code) => {
      const ms = manaFontClass(code);
      if (ms == null) return null;
      return h.i([h.Class(`ms ms-cost ms-${ms}`)], []);
    }),
  );
}

function renderDeckCardBody<Msg>(h: HtmlFactory<Msg>, card: DeckCardModel): Html {
  return h.div(
    [h.Class("flex flex-1 flex-col")],
    [
      card.print === ""
        ? h.div([h.Class("aspect-[137/100] w-full bg-glass")], [])
        : cardArt(h, {
            print: card.print,
            size: "art_crop",
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
          renderPips(h, card.colorIdentity),
        ],
      ),
    ],
  );
}

export function renderDeckCard<Msg>(
  h: HtmlFactory<Msg>,
  card: DeckCardModel,
  opts: {
    mode: "link" | "static";
    href?: string;
    testId: string;
  },
): Html {
  const attrs = [
    h.DataAttribute("testid", opts.testId),
    h.Class(listRowClass("relative flex flex-col overflow-hidden rounded-hud no-underline text-snow")),
    h.Style({ "view-transition-name": deckCardViewTransitionName(card.id) }),
  ];

  if (opts.mode === "static") {
    return h.div(attrs, [renderDeckCardBody(h, card)]);
  }

  return h.a([h.Href(opts.href ?? ""), ...attrs], [renderDeckCardBody(h, card)]);
}
