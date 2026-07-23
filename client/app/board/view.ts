import { Canvas, Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import { boardStatusSummary } from "~/boardStatus";
import { colors } from "~/design-tokens.generated";
import { isActivePlayer } from "~/spectator";
import type { VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { pendingDamageAssignOverlay, pendingTargetingOverlay, stagingOverlay } from "./action/targeting";
import { MountBitmapLayer, MountFlightLayer, publishBitmapFrame } from "./bitmap/mount";
import { sceneShapes } from "./canvas/scene";
import { worldToScreen } from "./geometry/camera";
import { layout, STEP } from "./geometry/layout";
import { autoTapPreviewIds, paymentPreviewAction } from "./html/actions";
import { MountBoardAudio, MountHintAutoHide } from "./html/audio-mount";
import { MountBoardKeyboard } from "./html/keyboard-mount";
import { manaTrayView } from "./html/mana-tray";
import { boardOverlays } from "./html/overlays";
import { BoardPointerDown, BoardPointerMove, BoardPointerUp, type Message } from "./messages";
import type { BoardModel } from "./submodel";

const h = html<Message>();

export type BoardViewModel = {
  board: BoardModel;
  fold: GameFoldState;
  tableId: string | null;
  connected: boolean;
};

function connectingBoard(): Html {
  return h.main(
    [h.Class("fixed inset-0 select-none bg-forest-floor text-snow"), h.DataAttribute("testid", "board-mount")],
    [
      h.div(
        [h.Class("fixed inset-0 flex items-center justify-center"), h.DataAttribute("testid", "board-connecting")],
        [
          h.div(
            [
              h.Class(
                "animate-breathe rounded-hud bg-forest-hud px-xl py-lg text-center text-label text-seafoam shadow-hud",
              ),
            ],
            ["Connecting to the table…"],
          ),
        ],
      ),
    ],
  );
}

function boardAudioAttrs(model: BoardViewModel, state: VisibleState) {
  const me = state.players.find((p: (typeof state.players)[number]) => p.player === state.viewer);
  const canHearAttention = isActivePlayer(state.players, state.viewer) && me != null && !me.lost;
  const feel = model.fold.tableFeel;
  return [
    h.DataAttribute("game-seq", String(model.fold.seq)),
    h.DataAttribute("viewer", String(state.viewer)),
    h.DataAttribute("active-player", String(state.active_player)),
    h.DataAttribute("priority", String(state.priority)),
    h.DataAttribute("can-hear-attention", canHearAttention ? "1" : "0"),
    h.DataAttribute("feel-land", feel.land ? "1" : "0"),
    h.DataAttribute("feel-stack", feel.stack ? "1" : "0"),
    h.DataAttribute("feel-resolve", feel.resolve ? "1" : "0"),
    h.DataAttribute("feel-damage", feel.damage ? "1" : "0"),
  ];
}

function hintVisible(board: BoardModel): boolean {
  return !board.hintDismissed && !board.hintAutoHidden;
}

export const view = Submodel.defineView<BoardViewModel, Message>((model) => {
  const state = model.fold.state;
  if (state == null) return connectingBoard();

  const cards = layout(state, state.viewer);
  const stagedOverlay = stagingOverlay(model.board.staged, state, model.board.viewport, state.stack.length);
  const pendingOverlay = pendingTargetingOverlay(state.pending_choice, state, model.board.viewport, state.stack.length);
  const damageOverlay = pendingDamageAssignOverlay(state.pending_choice, state);
  const overlay = stagedOverlay.aiming ? stagedOverlay : pendingOverlay.aiming ? pendingOverlay : damageOverlay;
  const previewAction = paymentPreviewAction(model.board, state.actions);
  const paymentPreviewIds = autoTapPreviewIds(previewAction);
  const combatDrag =
    model.board.pointer.kind === "drag"
      ? {
          from: worldToScreen(
            model.board.camera,
            model.board.pointer.card.x + model.board.pointer.card.w / 2,
            model.board.pointer.card.y + model.board.pointer.card.h / 2,
          ),
          declaringBlock: state.step === STEP.DeclareBlockers && state.active_player !== state.viewer,
        }
      : null;
  const damagePicked =
    overlay.aiming &&
    model.board.promptDraft?.kind === "damage" &&
    damageOverlay.aiming &&
    !stagedOverlay.aiming &&
    !pendingOverlay.aiming
      ? new Set(
          Object.entries(model.board.promptDraft.amounts)
            .filter(([, amount]) => amount > 0)
            .map(([id]) => Number(id)),
        )
      : null;
  publishBitmapFrame({
    width: model.board.viewport.width,
    height: model.board.viewport.height,
    camera: model.board.camera,
    cards,
    viewer: state.viewer,
    players: state.players,
    priority: state.priority,
    combat: state.combat,
    stagedAttackers: model.board.combatAttackers,
    stagedBlocks: model.board.combatBlocks,
    flights: [...model.board.flights.values()],
    hideCardIds: model.board.hideCardIds,
    targetObjects: overlay.targetObjects,
    pickedObjects:
      damagePicked ??
      (overlay.aiming && model.board.promptDraft?.kind === "card-pick"
        ? new Set(model.board.promptDraft.picked)
        : new Set()),
    assignAmounts:
      model.board.promptDraft?.kind === "damage"
        ? new Map(Object.entries(model.board.promptDraft.amounts).map(([id, amount]) => [Number(id), amount]))
        : new Map(),
    targetPlayers: overlay.targetPlayers,
    aimFrom: overlay.aiming ? overlay.aimFrom : null,
    cursor: model.board.cursor,
    combatDragFrom: combatDrag?.from ?? null,
    // Attack drag stroke matches arrows.ts ATTACK_STROKE (not colors.mountainRed).
    combatDragStroke: combatDrag == null ? null : combatDrag.declaringBlock ? colors.wallGreen : "#ff6b6b",
    paymentPreviewIds,
    actions: state.actions,
  });

  const stagedTargeting =
    overlay.aiming && overlay.aimFrom != null
      ? {
          targetObjects: overlay.targetObjects,
          targetPlayers: overlay.targetPlayers,
          aimFrom: overlay.aimFrom,
          cursor: model.board.cursor,
        }
      : null;
  const combatDragShapes =
    combatDrag == null
      ? null
      : {
          from: combatDrag.from,
          to: model.board.cursor,
          declaringBlock: combatDrag.declaringBlock,
        };

  const ariaSummary = boardStatusSummary(state, state.viewer);

  // Foldkit keeps only the last OnMount insert hook per element — never stack
  // MountBoardKeyboard / MountBoardAudio / MountHintAutoHide on the same node
  // (that silently dropped Alt inspect and could mute table audio).
  const showHint = hintVisible(model.board);
  return h.main(
    [
      h.Class("fixed inset-0 select-none overflow-hidden bg-forest-floor text-snow"),
      h.DataAttribute("testid", "board-mount"),
    ],
    [
      h.div(
        [h.Class("hidden"), h.DataAttribute("testid", "board-keyboard-mount"), h.OnMount(MountBoardKeyboard())],
        [],
      ),
      h.div(
        [
          h.Class("hidden"),
          h.DataAttribute("testid", "board-audio-mount"),
          ...boardAudioAttrs(model, state),
          h.OnMount(MountBoardAudio()),
        ],
        [],
      ),
      showHint
        ? h.div(
            [
              h.Class("hidden"),
              h.DataAttribute("testid", "board-hint-mount"),
              h.DataAttribute("hint-visible", "1"),
              h.OnMount(MountHintAutoHide()),
            ],
            [],
          )
        : null,
      h.div([h.Class("sr-only"), h.Attribute("aria-live", "polite")], [ariaSummary]),
      Canvas.view<Message>({
        width: model.board.viewport.width,
        height: model.board.viewport.height,
        className: "block h-full w-full touch-none",
        shapes: sceneShapes(state, {
          width: model.board.viewport.width,
          height: model.board.viewport.height,
          camera: model.board.camera,
          selectedId: model.board.selectedId,
          stagedAttackers: model.board.combatAttackers,
          stagedBlocks: model.board.combatBlocks,
          stagedTargeting,
          combatDrag: combatDragShapes,
        }),
        onPointerDown: ({ x, y }) => BoardPointerDown({ x, y }),
        onPointerMove: ({ x, y }) => BoardPointerMove({ x, y }),
        onPointerUp: ({ x, y }) => BoardPointerUp({ x, y }),
      }),
      // Layer 2: in-play mana under resting permanents (DOM sibling before bitmap).
      manaTrayView(model.board, state),
      h.canvas(
        [
          h.Width(String(model.board.viewport.width)),
          h.Height(String(model.board.viewport.height)),
          h.Class("pointer-events-none absolute inset-0 block h-full w-full"),
          h.DataAttribute("testid", "board-bitmap-layer"),
          h.OnMount(MountBitmapLayer()),
        ],
        [],
      ),
      boardOverlays(model.board, state, model.tableId, model.fold.log),
      // Layer 6: flights ride their own canvas above the hand/stack HTML (z-30) but below prompts.
      h.canvas(
        [
          h.Width(String(model.board.viewport.width)),
          h.Height(String(model.board.viewport.height)),
          h.Class("pointer-events-none absolute inset-0 z-30 block h-full w-full"),
          h.DataAttribute("testid", "board-flight-layer"),
          h.OnMount(MountFlightLayer()),
        ],
        [],
      ),
      model.connected
        ? null
        : h.div(
            [
              h.DataAttribute("testid", "board-reconnecting"),
              h.Class(
                "fixed top-0 right-0 left-0 z-40 bg-reconnect-rust p-sm text-center font-semibold text-label text-snow",
              ),
            ],
            ["Reconnecting…"],
          ),
    ],
  );
});
