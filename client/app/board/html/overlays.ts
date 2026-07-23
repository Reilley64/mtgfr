// Board HTML overlays: pins together hand, priority bar, turn chrome, stack, prompts,
// pile, concede button + dialog, result overlay, and inspect dock (topmost).

import { type Html, html } from "foldkit/html";
import { mulliganChrome } from "~/mulligan";
import { isActivePlayer, SPECTATOR_VIEWER } from "~/spectator";
import type { VisibleState } from "~/wire/types";
import type { LogLine } from "../../game/fold";
import {
  gyExileCostObjectIds,
  pendingExilePickIds,
  pendingGraveyardPickIds,
  pendingHandPickIds,
} from "../action/targeting";
import type { Message } from "../messages";
import type { BoardModel } from "../submodel";
import { activationRadialView } from "./activation-radial";
import { concedeButtonView, concedeDialogView } from "./concede";
import { discoverabilityView } from "./discoverability";
import { handView } from "./hand";
import { inspectView } from "./inspect";
import { logPanelView } from "./log-panel";
import { mulliganBarView } from "./mulligan-bar";
import { pendingChoiceWaitingView } from "./pending-choice-waiting";
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
  const seatedViewer = isActivePlayer(state.players, state.viewer);
  const spectating = state.viewer === SPECTATOR_VIEWER;
  const mulliganing = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  }).show;

  // Live object for the inspect pin's modifiers (battlefield objects only).
  const inspectObject =
    board.inspectPin?.objectId != null
      ? (state.objects.find((o) => o.id === board.inspectPin?.objectId) ?? null)
      : null;

  const layers: Array<Html | null> = [
    turnChromeView(board, state),
    spectating ? spectatingBadgeView() : null,
    pendingChoiceWaitingView(state),
    h.div(
      [h.Class("pointer-events-none fixed top-md left-md z-25 flex items-center gap-xs")],
      [discoverabilityView(board, state), soundToggleView(board)].filter((v): v is Html => v !== null),
    ),
    // Battlefield mana tray is composed in view.ts between vector canvas and bitmap
    // (DOM order under resting permanents) — not here inside overlays.
    stackView(board, state),
    logPanelView(log),
    seatedViewer
      ? handView({
          state,
          hiddenId: stagedCardId,
          flyingIds: board.hideCardIds,
          hiddenIds,
          handDrag: board.handDrag,
          discardCostIds: (() => {
            if (board.discardPick != null) return new Set(board.discardPick.action.discard_choices ?? []);
            const pending = pendingHandPickIds(state.pending_choice, state);
            return pending != null ? pending : null;
          })(),
        })
      : null,
    seatedViewer && mulliganing ? mulliganBarView(state) : null,
    seatedViewer && !mulliganing ? priorityBarView(board, state) : null,
    seatedViewer && !mulliganing ? promptsView(board, state, tableId) : null,
    seatedViewer && !mulliganing ? activationRadialView(board, state) : null,
    seatedViewer ? concedeButtonView() : null,
    concedeDialogView(board.confirmConcede),
    pileOverlayView(board.pileExpand, state, {
      selectableIds: (() => {
        if (board.gyExilePick != null) {
          return gyExileCostObjectIds(board.gyExilePick.action.graveyard_exile_choices, state);
        }
        return pendingGraveyardPickIds(state.pending_choice, state) ?? pendingExilePickIds(state.pending_choice, state);
      })(),
      selectedIds: (() => {
        if (board.gyExilePick != null) return board.gyExilePick.picks.graveyard_exile;
        if (board.promptDraft?.kind === "card-pick") return board.promptDraft.picked;
        return null;
      })(),
    }),
    resultOverlayView(state, board.resultSeen),
    // Inspect dock is topmost (layer 10) — above pile, concede dialog, and result.
    inspectView(board.inspectPin, board.inspectCard, board.inspectFace, inspectObject, state.players, state.objects),
  ];

  return h.div(
    [h.Class("pointer-events-none absolute inset-0")],
    layers.filter((v): v is Html => v !== null),
  );
}
