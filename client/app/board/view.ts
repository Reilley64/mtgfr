import { Canvas, Submodel } from "foldkit";
import type { Html, HtmlBuilder } from "foldkit/html";
import { boardStatusSummary } from "~/boardStatus";
import { colors } from "~/design-tokens.generated";
import { isActivePlayer } from "~/spectator";
import type { VisibleState } from "~/wire/types";
import type { CardArtTick } from "../domain/ui/card-art";
import type { GameFoldState } from "../game/fold";
import {
  pendingDamageAssignOverlay,
  pendingDivideSpellOverlay,
  pendingPlayerAimOverlay,
  pendingTargetingOverlay,
  pickedPlayersFromDraft,
  sacrificeCostOverlay,
  stagingOverlay,
} from "./action/targeting";
import { MountBitmapLayer, MountFlightLayer, publishBitmapFrame } from "./bitmap/mount";
import { sceneShapes } from "./canvas/scene";
import { engagedIds } from "./engagement";
import { worldToScreen } from "./geometry/camera";
import { layout, STEP } from "./geometry/layout";
import { stackPresentation } from "./geometry/stackLayout";
import { autoTapPreviewIds, paymentPreviewAction } from "./html/actions";
import { MountBoardAudio, MountHintAutoHide } from "./html/audio-mount";
import { MountBoardCameraGesture } from "./html/camera-gesture-mount";
import { handMetrics } from "./html/hand";
import { MountBoardKeyboard } from "./html/keyboard-mount";
import { manaTrayView } from "./html/mana-tray";
import { boardOverlays } from "./html/overlays";
import { BoardPointerDown, BoardPointerMove, BoardPointerUp, type Message } from "./messages";
import { dragGhostFromHandDrag } from "./motion/screen-motion";
import type { BoardModel } from "./submodel";

/** Board TEA messages plus shell ticks emitted by shared mounts (e.g. `cardArt`). */
export type ViewMessage = Message | typeof CardArtTick.Type;

export type BoardViewModel = {
  board: BoardModel;
  fold: GameFoldState;
  tableId: string | null;
  connected: boolean;
};

function connectingBoard(h: HtmlBuilder<ViewMessage>): Html {
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

function boardAudioAttrs(model: BoardViewModel, state: VisibleState, h: HtmlBuilder<ViewMessage>) {
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
    h.DataAttribute("feel-destroy", feel.destroy ? "1" : "0"),
    h.DataAttribute("feel-exile", feel.exile ? "1" : "0"),
  ];
}

function hintVisible(board: BoardModel): boolean {
  return !board.hintDismissed && !board.hintAutoHidden;
}

function reconnectBannerText(model: BoardViewModel): string | null {
  if (model.connected) return null;
  return model.fold.reject ?? "Connection lost — reconnecting…";
}

export const view = Submodel.defineView<BoardViewModel, ViewMessage>((model, h) => {
  const state = model.fold.state;
  if (state == null) return connectingBoard(h);

  // ponytail: the frame's universe (ViewMessage) is a superset of the board's, but the phantom
  // marker on HtmlBuilder is invariant, so helpers that only build board Messages need the cast.
  const bh = h as unknown as HtmlBuilder<Message>;

  // Paint and hit-test must agree on which permanents are engaged, or a card paints where it
  // cannot be clicked — one set shared between `layout()` here and the `sceneShapes` call below.
  const engaged = engagedIds(state, model.board);
  const cards = layout(state, state.viewer, engaged);
  const stagedOverlay = stagingOverlay(model.board.staged, state, model.board.viewport, state.stack.length);
  const pendingOverlay = pendingTargetingOverlay(
    state.pending_choice,
    state,
    model.board.viewport,
    state.stack.length,
    model.board.promptDraft,
  );
  const damageOverlay = pendingDamageAssignOverlay(state.pending_choice, state);
  const divideOverlay = pendingDivideSpellOverlay(state.pending_choice, state);
  const playerOverlay = pendingPlayerAimOverlay(state.pending_choice, state);
  const sacrificeOverlay =
    model.board.sacrificePick != null
      ? sacrificeCostOverlay(model.board.sacrificePick.action.sacrifice_choices, state)
      : {
          aiming: false,
          targetObjects: new Set<number>(),
          targetPlayers: new Set<number>(),
          aimFrom: null,
        };
  const overlay = stagedOverlay.aiming
    ? stagedOverlay
    : sacrificeOverlay.aiming
      ? sacrificeOverlay
      : pendingOverlay.aiming
        ? pendingOverlay
        : damageOverlay.aiming
          ? damageOverlay
          : divideOverlay.aiming
            ? divideOverlay
            : playerOverlay;
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
  const dividePicked =
    overlay.aiming &&
    model.board.promptDraft?.kind === "divide" &&
    divideOverlay.aiming &&
    !stagedOverlay.aiming &&
    !pendingOverlay.aiming &&
    state.pending_choice?.kind === "divide_spell_damage"
      ? new Set(
          state.pending_choice.items.flatMap((item, index) => {
            const amount =
              model.board.promptDraft?.kind === "divide" ? (model.board.promptDraft.amounts[index] ?? 0) : 0;
            return amount > 0 ? [item.id] : [];
          }),
        )
      : null;
  const divideAssignAmounts =
    model.board.promptDraft?.kind === "divide" && state.pending_choice?.kind === "divide_spell_damage"
      ? new Map(
          state.pending_choice.items.flatMap((item, index) => {
            const amount =
              model.board.promptDraft?.kind === "divide" ? (model.board.promptDraft.amounts[index] ?? 0) : 0;
            return amount > 0 ? ([[item.id, amount]] as const) : [];
          }),
        )
      : null;
  const stackMode = stackPresentation({
    count: state.stack.length,
    expandedOpen: model.board.stackExpand,
    viewportW: model.board.viewport.width,
    viewportH: model.board.viewport.height,
  });
  publishBitmapFrame({
    width: model.board.viewport.width,
    height: model.board.viewport.height,
    dpr: model.board.dpr,
    camera: model.board.camera,
    cards,
    viewer: state.viewer,
    players: state.players,
    priority: state.priority,
    combat: state.combat,
    stagedAttackers: model.board.combatAttackers,
    stagedBlocks: model.board.combatBlocks,
    stack: state.stack,
    stackPresentation: stackMode,
    flights: [...(model.board.flights instanceof Map ? model.board.flights.values() : [])],
    dragGhost:
      model.board.handDrag == null
        ? null
        : dragGhostFromHandDrag(model.board.handDrag, model.board.camera.zoom, handMetrics(model.board.viewport).cardW),
    exitFx: [...(model.board.exitFx instanceof Map ? model.board.exitFx.values() : [])],
    hideCardIds: model.board.hideCardIds,
    targetObjects: overlay.targetObjects,
    pickedObjects:
      damagePicked ??
      dividePicked ??
      (overlay.aiming && model.board.promptDraft?.kind === "card-pick"
        ? new Set(model.board.promptDraft.picked)
        : new Set()),
    assignAmounts:
      divideAssignAmounts ??
      (model.board.promptDraft?.kind === "damage"
        ? new Map(Object.entries(model.board.promptDraft.amounts).map(([id, amount]) => [Number(id), amount]))
        : new Map()),
    targetPlayers: overlay.targetPlayers,
    pickedPlayers: pickedPlayersFromDraft(overlay.aiming, model.board.promptDraft),
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

  const bar = handMetrics(model.board.viewport);
  const dpr = model.board.dpr;
  const ariaSummary = boardStatusSummary(state, state.viewer);
  const reconnectText = reconnectBannerText(model);

  // Foldkit keeps only the last OnMount insert hook per element — never stack
  // MountBoardKeyboard / MountBoardAudio / MountHintAutoHide on the same node
  // (that silently dropped Alt inspect and could mute table audio).
  //
  // Every mount host here is keyed, and so is every conditional sibling. Unkeyed sibling `div`s are
  // interchangeable to snabbdom (`sameVnode` compares sel + key), so a conditional child vanishing
  // shifts the whole tail one slot: the hint auto-hiding after 12s used to patch the camera vnode
  // onto the hint's element, leaving the running gesture mount holding a `sr-only` node whose rect
  // rejects every wheel event — scroll zoom died silently. Keys also make connecting → live a real
  // insert rather than a reuse of the connecting div, so the keyboard mount starts on a cold load.
  const showHint = hintVisible(model.board);
  return h.main(
    [
      h.Class("fixed inset-0 select-none overflow-hidden bg-forest-floor text-snow"),
      h.DataAttribute("testid", "board-mount"),
      // Overlays anchored above the hand bar read `--hand-bar-h` rather than baking its height,
      // so they follow the bar when it rescales with the window.
      h.Style({ "--hand-bar-h": `${bar.barH}px` }),
    ],
    [
      h.keyed("div")(
        "board-keyboard-mount",
        [h.Class("hidden"), h.DataAttribute("testid", "board-keyboard-mount"), h.OnMount(MountBoardKeyboard())],
        [],
      ),
      h.keyed("div")(
        "board-audio-mount",
        [
          h.Class("hidden"),
          h.DataAttribute("testid", "board-audio-mount"),
          ...boardAudioAttrs(model, state, h),
          h.OnMount(MountBoardAudio()),
        ],
        [],
      ),
      showHint
        ? h.keyed("div")(
            "board-hint-mount",
            [
              h.Class("hidden"),
              h.DataAttribute("testid", "board-hint-mount"),
              h.DataAttribute("hint-visible", "1"),
              h.OnMount(MountHintAutoHide()),
            ],
            [],
          )
        : null,
      h.keyed("div")(
        "board-camera-gesture-mount",
        [
          h.Class("pointer-events-none absolute inset-0 z-10 touch-none"),
          h.DataAttribute("testid", "board-camera-gesture-mount"),
          h.DataAttribute("board-width", String(model.board.viewport.width)),
          h.DataAttribute("board-height", String(model.board.viewport.height)),
          h.OnMount(MountBoardCameraGesture()),
        ],
        [],
      ),
      h.div([h.Class("sr-only"), h.Attribute("aria-live", "polite")], [ariaSummary]),
      // Foldkit's Canvas sizes its backing store to `width`/`height` and hands pointer events back
      // in that space. Paint at device resolution and scale the whole scene by the DPR so the felt,
      // seat bands, and arrows stay sharp on retina; pointer coordinates divide back to CSS px.
      Canvas.view(
        {
          width: Math.round(model.board.viewport.width * dpr),
          height: Math.round(model.board.viewport.height * dpr),
          className: "block h-full w-full touch-none",
          shapes: [
            Canvas.Group({
              scale: { x: dpr, y: dpr },
              shapes: sceneShapes(state, {
                width: model.board.viewport.width,
                height: model.board.viewport.height,
                camera: model.board.camera,
                engaged,
                selectedId: model.board.selectedId,
                stagedAttackers: model.board.combatAttackers,
                stagedBlocks: model.board.combatBlocks,
                stagedTargeting,
                combatDrag: combatDragShapes,
                stackPresentation: stackMode,
              }),
            }),
          ],
          onPointerDown: ({ x, y }) => BoardPointerDown({ x: x / dpr, y: y / dpr }),
          onPointerMove: ({ x, y }) => BoardPointerMove({ x: x / dpr, y: y / dpr }),
          onPointerUp: ({ x, y }) => BoardPointerUp({ x: x / dpr, y: y / dpr }),
        },
        h,
      ),
      // Layer 2: in-play mana under resting permanents (DOM sibling before bitmap).
      manaTrayView(model.board, state, bh),
      h.keyed("canvas")(
        "board-bitmap-layer",
        [
          // Device-resolution backing store, CSS-pixel box: the Mount paints through a DPR
          // transform, and these attributes must match or the next vdom patch shrinks it back.
          h.Width(String(Math.round(model.board.viewport.width * dpr))),
          h.Height(String(Math.round(model.board.viewport.height * dpr))),
          h.Class("pointer-events-none absolute inset-0 block h-full w-full"),
          h.DataAttribute("testid", "board-bitmap-layer"),
          h.OnMount(MountBitmapLayer()),
        ],
        [],
      ),
      boardOverlays(model.board, state, model.tableId, model.fold.log, bh),
      // Layer 6: flights ride their own canvas above the hand/stack HTML (z-30) but below prompts.
      h.keyed("canvas")(
        "board-flight-layer",
        [
          h.Width(String(Math.round(model.board.viewport.width * dpr))),
          h.Height(String(Math.round(model.board.viewport.height * dpr))),
          h.Class("pointer-events-none absolute inset-0 z-30 block h-full w-full"),
          h.DataAttribute("testid", "board-flight-layer"),
          h.OnMount(MountFlightLayer()),
        ],
        [],
      ),
      reconnectText == null
        ? null
        : h.keyed("div")(
            "board-reconnecting",
            [
              h.DataAttribute("testid", "board-reconnecting"),
              h.Role("alert"),
              h.Class(
                "fixed top-0 right-0 left-0 z-40 bg-reconnect-rust p-sm text-center font-semibold text-label text-snow",
              ),
            ],
            [reconnectText],
          ),
    ],
  );
});
