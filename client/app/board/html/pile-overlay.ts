// Pile (graveyard / exile) expand overlay — art grid with a Close button.
// Mirrors Solid board-overlays.tsx PileOverlay.

import { type Html, html } from "foldkit/html";
import { buttonClass } from "~/ui/buttonClass";
import { cardArt } from "~/ui/card-art";
import type { ObjectView, VisibleState } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import { type Message, PileOverlayClosed } from "../messages";

const h = html<Message>();

/** Cards that belong to the expanded pile: objects in the given zone owned by the given seat. */
export function pileCards(state: VisibleState, zone: number, owner: number): ObjectView[] {
  return state.objects.filter((o) => o.zone === zone && o.owner === owner);
}

/** Pile zone display name for the heading. */
function zoneName(zone: number, count: number): string {
  const base = zone === ZONE.Graveyard ? "Graveyard" : zone === ZONE.Exile ? "Exile" : "Pile";
  return `${base} (${count})`;
}

function cardThumb(card: ObjectView): Html {
  return h.div(
    [h.Class("relative"), h.Attribute("title", card.name)],
    [
      card.print
        ? cardArt(h, {
            print: card.print,
            size: "large",
            alt: card.name,
            className: "block rounded-md",
            style: { width: "90px" },
          })
        : h.div(
            [
              h.Class("flex items-center justify-center rounded-md bg-forest-surface text-caption text-lichen"),
              h.Style({ width: "90px", height: "126px" }),
            ],
            [card.name],
          ),
    ],
  );
}

/**
 * Pile expand overlay. Returns null when pileExpand is null.
 *
 * Backdrop click and the Close button both fire PileOverlayClosed.
 */
export function pileOverlayView(expand: { zone: number; owner: number } | null, state: VisibleState): Html | null {
  if (expand == null) return null;

  const cards = pileCards(state, expand.zone, expand.owner);
  const title = zoneName(expand.zone, cards.length);

  const cardList = cards.map(cardThumb);

  const modal = h.div(
    [
      h.Class(
        "pointer-events-auto fixed top-[45%] left-1/2 z-30 max-w-[520px] w-full -translate-x-1/2 -translate-y-1/2 rounded-panel border border-vine bg-forest-surface p-lg shadow-hud",
      ),
      // Stop clicks inside the modal from bubbling to the backdrop.
      h.Attribute("data-pile-modal", "true"),
    ],
    [
      h.div([h.Class("mb-sm font-semibold text-body text-snow")], [title]),
      h.div([h.Class("flex flex-wrap gap-xs")], cardList),
      h.div(
        [h.Class("mt-sm flex justify-end")],
        [
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "pile-overlay-close"),
              h.OnClick(PileOverlayClosed()),
              h.Class(buttonClass("ghost")),
            ],
            ["Close"],
          ),
        ],
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
