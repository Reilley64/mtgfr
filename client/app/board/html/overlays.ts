// Board HTML overlays: pins together hand, priority bar, turn chrome, stack, prompts,
// inspect, pile, concede button + dialog, result overlay.

import { type Html, html } from "foldkit/html";
import { isActivePlayer, SPECTATOR_VIEWER } from "~/spectator";
import type { VisibleState } from "~/wire/types";
import type { LogLine } from "../../game/fold";
import type { Message } from "../messages";
import type { BoardModel } from "../submodel";
import { activationRadialView } from "./activation-radial";
import { concedeButtonView, concedeDialogView } from "./concede";
import { discoverabilityView } from "./discoverability";
import { handView } from "./hand";
import { inspectView } from "./inspect";
import { logPanelView } from "./log-panel";
import { manaTrayView } from "./mana-tray";
import { pileOverlayView } from "./pile-overlay";
import { priorityBarView } from "./priority-bar";
import { promptsView } from "./prompts";
import { resultOverlayView } from "./result-overlay";
import { soundToggleView } from "./sound-chrome";
import { stackView } from "./stack";
import { turnChromeView } from "./turn-chrome";

const h = html<Message>();

function spectatingBadgeView(): Html {
  return h.div(
    [
      h.DataAttribute("testid", "board-spectating"),
      h.Class(
        "pointer-events-none fixed top-md left-1/2 z-20 -translate-x-1/2 rounded-control bg-llanowar px-md py-xs font-semibold text-label text-snow-mint tracking-[0.04em]",
      ),
    ],
    ["Spectating"],
  );
}

export function boardOverlays(
  board: BoardModel,
  state: VisibleState,
  tableId: string | null,
  log: ReadonlyArray<LogLine> = [],
): Html {
  const stagedCardId = board.staged?.card.id ?? null;
  const hiddenIds = new Set<number>([...board.handHidden, ...board.hideCardIds]);
  const active = isActivePlayer(state.players, state.viewer);
  const spectating = state.viewer === SPECTATOR_VIEWER;

  // Live object for the inspect pin's modifiers (battlefield objects only).
  const inspectObject =
    board.inspectPin?.objectId != null
      ? (state.objects.find((o) => o.id === board.inspectPin?.objectId) ?? null)
      : null;

  const layers: Array<Html | null> = [
    turnChromeView(board, state),
    spectating ? spectatingBadgeView() : null,
    h.div(
      [h.Class("pointer-events-none fixed top-md left-md z-25 flex items-center gap-xs")],
      [discoverabilityView(board, state), soundToggleView(board)].filter((v): v is Html => v !== null),
    ),
    manaTrayView(board, state),
    stackView(board, state),
    logPanelView(log),
    active
      ? handView({ state, hiddenId: stagedCardId, flyingIds: board.hideCardIds, hiddenIds, handDrag: board.handDrag })
      : null,
    active ? priorityBarView(board, state) : null,
    active ? promptsView(board, state, tableId) : null,
    active ? activationRadialView(board, state) : null,
    active ? concedeButtonView() : null,
    concedeDialogView(board.confirmConcede),
    inspectView(board.inspectPin, board.inspectCard, board.inspectFace, inspectObject),
    pileOverlayView(board.pileExpand, state),
    resultOverlayView(state, board.resultSeen),
  ];

  return h.div(
    [h.Class("pointer-events-none absolute inset-0")],
    layers.filter((v): v is Html => v !== null),
  );
}
