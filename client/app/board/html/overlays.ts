// Board HTML overlays: pins together hand, priority bar, turn chrome, stack, prompts,
// pile, concede button + dialog, result overlay, and inspect dock (topmost).

import type { Html, HtmlBuilder } from "foldkit/html";
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
import { activationMenuView } from "./activation-menu";
import { concedeButtonView, concedeDialogView } from "./concede";
import { discoverabilityView } from "./discoverability";
import { firstPlayerRevealView } from "./first-player-reveal";
import { handView } from "./hand";
import { inspectView } from "./inspect";
import { logPanelView } from "./log-panel";
import { mulliganOverlayView, mulliganWaitingView } from "./mulligan-overlay";
import { pendingChoiceWaitingView } from "./pending-choice-waiting";
import { pileOverlayView } from "./pile-overlay";
import { priorityBarView } from "./priority-bar";
import { promptsView } from "./prompts";
import { resultOverlayView } from "./result-overlay";
import { seenHandsView } from "./seen-hands";
import { soundToggleView } from "./sound-chrome";
import { stackView } from "./stack";
import { turnChromeView } from "./turn-chrome";

function spectatingBadgeView(h: HtmlBuilder<Message>): Html {
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
  h: HtmlBuilder<Message>,
): Html {
  const stagedCardId = board.staged?.card.id ?? null;
  const hiddenIds = new Set<number>([...board.handHidden, ...board.hideCardIds]);
  const seatedViewer = isActivePlayer(state.players, state.viewer);
  const spectating = state.viewer === SPECTATOR_VIEWER;
  const chrome = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  });
  const undecidedMulligan = chrome.show && chrome.showControls;

  // Live object for the inspect pin's modifiers (battlefield objects only).
  const inspectObject =
    board.inspectPin?.objectId != null
      ? (state.objects.find((o) => o.id === board.inspectPin?.objectId) ?? null)
      : null;

  const layers: Array<Html | null> = [
    turnChromeView(board, state, h),
    spectating ? spectatingBadgeView(h) : null,
    pendingChoiceWaitingView(state, h),
    h.div(
      [h.Class("pointer-events-none fixed top-md left-md z-25 flex items-center gap-xs")],
      [discoverabilityView(board, state, h), soundToggleView(board, h), seenHandsView(state, h)].filter(
        (v): v is Html => v !== null,
      ),
    ),
    // Battlefield mana tray is composed in view.ts between vector canvas and bitmap
    // (DOM order under resting permanents) — not here inside overlays.
    stackView(board, state, h),
    logPanelView(board, log, h),
    seatedViewer && !undecidedMulligan
      ? handView(
          {
            viewport: board.viewport,
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
            discardSelectedIds: (() => {
              if (board.discardPick != null) return new Set(board.discardPick.picks.discard_cost);
              // Same gate as discardCostIds: any on-hand pending hand pick (discard, put land/creature,
              // face-down cast, put on top, pay-cost discard) paints Llanowar on draft picks.
              if (pendingHandPickIds(state.pending_choice, state) != null && board.promptDraft?.kind === "card-pick") {
                return new Set(board.promptDraft.picked);
              }
              return null;
            })(),
          },
          h,
        )
      : null,
    seatedViewer ? mulliganOverlayView(state, h) : null,
    seatedViewer ? mulliganWaitingView(state, h) : null,
    seatedViewer && !chrome.show ? promptsView(board, state, tableId, h) : null,
    seatedViewer && !chrome.show ? activationMenuView(board, state, h) : null,
    seatedViewer ? concedeButtonView(h) : null,
    concedeDialogView(board.concedeDialog, h),
    pileOverlayView(
      board.pileExpand,
      state,
      {
        selectableIds: (() => {
          if (board.gyExilePick != null) {
            return gyExileCostObjectIds(board.gyExilePick.action.graveyard_exile_choices, state);
          }
          return (
            pendingGraveyardPickIds(state.pending_choice, state) ?? pendingExilePickIds(state.pending_choice, state)
          );
        })(),
        selectedIds: (() => {
          if (board.gyExilePick != null) return board.gyExilePick.picks.graveyard_exile;
          if (board.promptDraft?.kind === "card-pick") return board.promptDraft.picked;
          return null;
        })(),
      },
      h,
    ),
    // After pile/prompt backdrops so equal-z siblings still keep actions on top; simple prompts use z-45.
    seatedViewer && !chrome.show ? priorityBarView(board, state, tableId, h) : null,
    resultOverlayView(state, board.resultDialog, h),
    // Inspect stays off during undecided mulligans so the opening-hand overlay is a true hard lock.
    undecidedMulligan
      ? null
      : inspectView(
          board.inspectPin,
          board.inspectCard,
          board.inspectFace,
          inspectObject,
          state.players,
          state.objects,
          h,
        ),
    // CR 103.1 spotlight sits above everything, spectators included — no seatedViewer gate.
    firstPlayerRevealView(board.firstPlayerReveal, state, h),
  ];

  return h.div(
    [h.Class("pointer-events-none absolute inset-0")],
    layers.filter((v): v is Html => v !== null),
  );
}
