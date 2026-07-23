// Alt-pin card inspect overlay — shared dock preview + modifier ledger extras.
//
// State lives in BoardModel (inspectPin, inspectCard, inspectFace). The catalog fetch is
// a Command (FetchInspectCard → InspectCardFetched) fired from updateBoard on pin change.

import { type Html, html } from "foldkit/html";
import { cardHoverPreviewView } from "~/deck-builder/card-hover-preview";
import { type InspectFace, type InspectPin, shownName } from "~/inspect";
import { buttonClass } from "~/ui/buttonClass";
import type { CatalogCard, ObjectView } from "~/wire/types";
import { InspectDismissed, InspectFlipFace, type Message } from "../messages";

const h = html<Message>();

const PANEL_W = 280;

function modifierLedger(modifiers: NonNullable<ObjectView["modifiers"]>): Html | null {
  if (modifiers.length === 0) return null;
  const entries = modifiers.map((group) =>
    h.div(
      [h.Class(`w-[${PANEL_W}px] shrink-0 rounded-panel border border-vine bg-forest-surface px-xl py-lg text-label`)],
      [
        h.div([h.Class("font-semibold text-seafoam")], [group.source_name]),
        h.div([h.Class("mt-0.5 text-preview-ash/80")], [group.contributions.join(", ")]),
      ],
    ),
  );
  return h.div([h.Class("flex flex-col gap-3")], entries);
}

/**
 * Full-screen inspect dock. Returns null when there is no active pin.
 *
 * Backdrop click and Escape both fire InspectDismissed.
 */
export function inspectView(
  pin: InspectPin | null,
  card: CatalogCard | null | undefined,
  face: InspectFace,
  /** Live ObjectView for the pinned object, when on battlefield — provides current modifiers. */
  liveObject?: ObjectView | null,
): Html | null {
  if (pin == null) return null;

  const back = card?.back ?? null;
  const hasBack = !!back?.name;
  const currentFace = hasBack ? face : "front";

  const displayName = shownName(pin.name, back?.name, currentFace);
  const oracle = currentFace === "back" ? back?.oracle : card?.oracle;
  const approximates = currentFace === "back" ? back?.approximates : card?.approximates;

  const artPrint = pin.print ?? card?.default_print ?? "";
  const modifiers = liveObject?.modifiers ?? [];

  // While catalog is still in-flight, pin.prepared DFCs default to showing the back face
  // to avoid a front-face flash before catalog confirms the actual play-face.
  const catalogReady = card !== undefined;
  const displayFace: InspectFace = catalogReady ? currentFace : pin.prepared ? "back" : "front";

  const modsEl = modifierLedger(modifiers);

  // Dismiss via backdrop / Esc / Alt-up — no Close control (Solid parity).
  const flipButton: Html | null = hasBack
    ? h.button(
        [
          h.Type("button"),
          h.OnClick(InspectFlipFace()),
          h.Class(buttonClass("game-quiet")),
          h.Attribute("title", "Flip card face"),
        ],
        ["Flip"],
      )
    : null;

  const extras: Html[] = [
    modsEl,
    flipButton != null ? h.div([h.Class("flex flex-wrap items-center gap-2")], [flipButton]) : null,
  ].filter((v): v is Html => v !== null);

  return cardHoverPreviewView(h, {
    mode: "dock",
    print: artPrint,
    name: displayName,
    oracle,
    approximates,
    face: displayFace,
    extras,
    onDismiss: InspectDismissed(),
    testId: "inspect-overlay",
  });
}
