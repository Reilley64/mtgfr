// Alt-pin card/player inspect overlay — shared dock preview + modifier / commander-damage extras.
//
// State lives in BoardModel (inspectPin, inspectCard, inspectFace). The catalog fetch is
// a Command (FetchInspectCard → InspectCardFetched) fired from updateBoard on pin change.

import { type Html, html } from "foldkit/html";
import { cardHoverPreviewView } from "~/deck-builder/card-hover-preview";
import { commanderDamageBreakdown, type InspectFace, type InspectPin, shownName } from "~/inspect";
import { buttonClass } from "~/ui/buttonClass";
import type { CatalogCard, ObjectView, PlayerView } from "~/wire/types";
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

function panelClass(): string {
  return `w-[${PANEL_W}px] shrink-0 rounded-panel border border-vine bg-forest-surface px-xl py-lg text-label text-preview-ash/80`;
}

function commanderDamagePanel(
  player: PlayerView,
  players: ReadonlyArray<PlayerView>,
  objects: ReadonlyArray<ObjectView>,
): Html | null {
  const rows = commanderDamageBreakdown(player, players, objects);
  if (rows.length === 0) return null;
  return h.div(
    [h.DataAttribute("testid", "inspect-commander-damage"), h.Class(panelClass())],
    [
      h.div([h.Class("font-semibold text-seafoam")], ["Commander damage"]),
      ...rows.map((row) =>
        h.div([h.DataAttribute("testid", `inspect-commander-damage-${row.fromSeat}`), h.Class("mt-0.5")], [row.text]),
      ),
    ],
  );
}

function playerInspectView(
  pin: InspectPin,
  players: ReadonlyArray<PlayerView>,
  objects: ReadonlyArray<ObjectView>,
): Html {
  const seat = pin.playerSeat;
  const player = seat != null ? (players.find((p) => p.player === seat) ?? null) : null;
  const life = player?.life ?? null;
  const lifeEl =
    life != null
      ? h.div([h.DataAttribute("testid", "inspect-player-life"), h.Class(panelClass())], [`Life: ${life}`])
      : null;
  const damageEl = player != null ? commanderDamagePanel(player, players, objects) : null;
  const panels = [
    h.div(
      [
        h.Class(
          `w-[${PANEL_W}px] shrink-0 rounded-panel border border-vine bg-forest-surface px-xl py-lg text-title font-semibold text-preview-ash`,
        ),
      ],
      [pin.name],
    ),
    lifeEl,
    damageEl,
  ].filter((v): v is Html => v !== null);

  const content = h.div(
    [
      h.DataAttribute("testid", "inspect-overlay-content"),
      h.Class("pointer-events-auto relative z-10 m-lg flex flex-row items-start gap-3"),
    ],
    [h.div([h.Class("flex min-w-0 flex-col items-start gap-3")], panels)],
  );
  const backdrop = h.div(
    [
      h.DataAttribute("testid", "inspect-overlay-backdrop"),
      h.Class("pointer-events-auto absolute inset-0"),
      h.OnClick(InspectDismissed()),
    ],
    [],
  );

  return h.div(
    [h.DataAttribute("testid", "inspect-overlay"), h.Class("fixed inset-0 z-[100] flex items-center bg-black/55")],
    [backdrop, content],
  );
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
  /** Live ObjectView for the pinned object, when on battlefield — modifiers and marked damage. */
  liveObject?: ObjectView | null,
  players: ReadonlyArray<PlayerView> = [],
  objects: ReadonlyArray<ObjectView> = [],
): Html | null {
  if (pin == null) return null;
  if (pin.playerSeat != null) return playerInspectView(pin, players, objects);

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
  const markedDamage = liveObject?.marked_damage ?? 0;
  const markedDamageEl =
    markedDamage > 0
      ? h.div(
          [h.DataAttribute("testid", "inspect-marked-damage"), h.Class(panelClass())],
          [`Marked damage: ${markedDamage}`],
        )
      : null;

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
    markedDamageEl,
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
