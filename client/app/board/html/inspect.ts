// Alt-pin card inspect overlay — shows card art + oracle/approximates text + DFC flip.
//
// State lives in BoardModel (inspectPin, inspectCard, inspectFace). The catalog fetch is
// a Command (FetchInspectCard → InspectCardFetched) fired from updateBoard on pin change.

import { type Html, html } from "foldkit/html";
import { cn } from "~/cn";
import { cardArt } from "~/ui/card-art";
import { type InspectFace, type InspectPin, shownName } from "~/inspect";
import { splitOracleText } from "~/oracleText";
import { buttonClass } from "~/ui/buttonClass";
import type { CatalogCard, ObjectView } from "~/wire/types";
import { InspectDismissed, InspectFlipFace, type Message } from "../messages";

const h = html<Message>();

const CARD_W = 240;
const CARD_H = Math.round(CARD_W / 0.716);
const PANEL_W = 280;

/** Render oracle text with mana-font symbol glyphs and reminder text in italics. */
function oracleNode(text: string): Html {
  const parts = splitOracleText(text);
  const children: Array<Html | string> = parts.map((part) => {
    if (part.kind === "text") {
      return part.reminder ? h.em([h.Class("italic")], [part.text]) : (part.text as unknown as Html);
    }
    return h.span(
      [
        h.Class(cn("ms ms-cost ms-oracle", `ms-${part.ms}`, part.reminder && "italic")),
        h.Attribute("aria-label", `{${part.code}}`),
        h.Role("img"),
      ],
      [],
    );
  });
  return h.span([], children as Html[]);
}

function textPanel(oracle: string | null | undefined, approximates: string | null | undefined): Html | null {
  if (!oracle && !approximates) return null;
  return h.div(
    [
      h.Class(
        `w-[${PANEL_W}px] shrink-0 overflow-y-auto rounded-panel border border-vine bg-forest-surface px-xl py-lg text-preview-ash text-body leading-[1.4]`,
      ),
      h.Style({ maxHeight: `${CARD_H}px` }),
    ],
    [
      oracle ? h.div([h.Class("whitespace-pre-wrap")], [oracleNode(oracle)]) : null,
      approximates
        ? h.div(
            [h.Class(cn("text-label text-note-gold italic", oracle ? "mt-3 border-vine/40 border-t pt-3" : ""))],
            [h.span([h.Class("font-semibold not-italic")], ["Approximation: "]), oracleNode(approximates)],
          )
        : null,
    ].filter((v): v is Html => v !== null),
  );
}

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

function artImg(print: string, face: InspectFace, alt: string): Html {
  return cardArt(h, {
    print,
    size: "large",
    face,
    alt,
    className: "block rounded-[14px] shadow-table object-cover",
    style: { width: `${CARD_W}px`, height: `${CARD_H}px` },
  });
}

function loadingCard(): Html {
  return h.div(
    [h.Class("animate-skeleton rounded-[14px] bg-white/10"), h.Style({ width: `${CARD_W}px`, height: `${CARD_H}px` })],
    [],
  );
}

/**
 * Full-screen inspect overlay. Returns null when there is no active pin.
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

  const artEl: Html = catalogReady ? artImg(artPrint, displayFace, displayName) : loadingCard();

  const textPanelEl = textPanel(oracle, approximates);
  const modsEl = modifierLedger(modifiers);

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

  const content = h.div(
    [
      h.Class(
        "pointer-events-auto relative m-lg flex max-h-[min(90vh,720px)] w-full max-w-[calc(100vw-2*var(--spacing-lg))] flex-row items-start gap-3 pt-11",
      ),
    ],
    [
      // Card column: flip button above the art, dismiss anchor below.
      h.div(
        [h.Class("relative flex shrink-0 flex-col items-center")],
        [
          flipButton != null
            ? h.div(
                [h.Class("absolute top-0 right-0 left-0 flex -translate-y-full items-center justify-end pb-2")],
                [flipButton],
              )
            : null,
          artEl,
          h.button(
            [h.Type("button"), h.OnClick(InspectDismissed()), h.Class(buttonClass("game-quiet", "mt-3"))],
            ["Close"],
          ),
        ].filter((v): v is Html => v !== null),
      ),
      // Text + modifier panels, if any.
      textPanelEl != null || modsEl != null
        ? h.div(
            [h.Class("flex min-w-0 flex-1 flex-col flex-wrap content-start gap-3 overflow-x-auto")],
            [textPanelEl, modsEl].filter((v): v is Html => v !== null),
          )
        : null,
    ].filter((v): v is Html => v !== null),
  );

  return h.div(
    [
      h.DataAttribute("testid", "inspect-overlay"),
      h.Class("fixed inset-0 z-30 flex items-center bg-black/55"),
      h.OnClick(InspectDismissed()),
    ],
    [content],
  );
}
