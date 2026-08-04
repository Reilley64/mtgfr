// Pile (graveyard / exile) expand overlay — art grid with a Close button.
// Mirrors Solid board-overlays.tsx PileOverlay.

import type { Html, HtmlBuilder } from "foldkit/html";
import { button } from "~/ui/button";
import { cardArt } from "~/ui/card-art";
import type { ObjectView, VisibleState } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import { type Message, PileCardClicked, PileOverlayClosed } from "../messages";

/** Cards that belong to the expanded pile: objects in the given zone owned by the given seat. */
export function pileCards(state: VisibleState, zone: number, owner: number): ObjectView[] {
  return state.objects.filter((o) => o.zone === zone && o.owner === owner);
}

/** Pile zone display name for the heading. */
function zoneName(zone: number, count: number): string {
  const base =
    zone === ZONE.Graveyard
      ? "Graveyard"
      : zone === ZONE.Exile
        ? "Exile"
        : zone === ZONE.Hand
          ? "Hand"
          : // Field of Dreams reveals the top card of a library, and the deck slot opens here.
            zone === ZONE.Library
            ? "Library"
            : "Pile";
  return `${base} (${count})`;
}

function cardThumb(card: ObjectView, selectable: boolean, selected: boolean, h: HtmlBuilder<Message>): Html {
  const face = card.print
    ? cardArt(h, {
        print: card.print,
        size: "display",
        alt: card.name,
        className: "block w-[90px] rounded-md",
      })
    : h.div(
        [
          h.Class(
            "flex h-[126px] w-[90px] items-center justify-center rounded-md bg-forest-surface text-caption text-lichen",
          ),
        ],
        [card.name],
      );
  if (!selectable) {
    return h.div([h.Class("relative"), h.Attribute("title", card.name)], [face]);
  }
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", `pile-card-${card.id}`),
      h.DataAttribute("selected", selected ? "true" : "false"),
      h.DataAttribute("selectable", "true"),
      h.Attribute("title", card.name),
      h.OnClick(PileCardClicked({ id: card.id })),
      h.Class(
        [
          "group/pile-card relative rounded-md ring-2",
          "data-[selected=true]:ring-priority-gold",
          "data-[selected=false]:ring-dashed data-[selected=false]:ring-island-blue",
        ].join(" "),
      ),
    ],
    [face],
  );
}

export type PileOverlayOptions = {
  selectableIds?: ReadonlySet<number> | null;
  selectedIds?: ReadonlyArray<number> | null;
};

/**
 * Pile expand overlay. Returns null when pileExpand is null.
 *
 * Backdrop click and the Close button both fire PileOverlayClosed.
 * When `selectableIds` is set, those cards emit `PileCardClicked` on click.
 */
export function pileOverlayView(
  expand: { zone: number; owner: number } | null,
  state: VisibleState,
  options: PileOverlayOptions = {},
  h: HtmlBuilder<Message>,
): Html | null {
  if (expand == null) return null;

  const cards = pileCards(state, expand.zone, expand.owner);
  const title = zoneName(expand.zone, cards.length);
  const selectable = options.selectableIds ?? null;
  const selected = new Set(options.selectedIds ?? []);

  const cardList = cards.map((card) => cardThumb(card, selectable?.has(card.id) ?? false, selected.has(card.id), h));

  const modal = h.div(
    [
      h.Class(
        "pointer-events-auto fixed top-[45%] left-1/2 z-30 max-w-[520px] w-full -translate-x-1/2 -translate-y-1/2 rounded-panel border border-vine bg-forest-surface p-lg shadow-hud",
      ),
      // Stop clicks inside the modal from bubbling to the backdrop.
      h.Attribute("data-pile-modal", "true"),
    ],
    [
      h.div(
        [h.DataAttribute("testid", "pile-overlay-title"), h.Class("mb-sm font-semibold text-body text-snow")],
        [title],
      ),
      h.div([h.Class("flex flex-wrap gap-xs")], cardList),
      h.div(
        [h.Class("mt-sm flex justify-end")],
        [button(h, { testId: "pile-overlay-close", onClick: PileOverlayClosed(), variant: "ghost" }, ["Close"])],
      ),
    ],
  );

  return h.div(
    [
      h.DataAttribute("testid", "pile-overlay"),
      h.Class("fixed inset-0 z-29 bg-black/50"),
      h.OnClick(PileOverlayClosed()),
    ],
    [modal],
  );
}
