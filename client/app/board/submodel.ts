import * as Combobox from "@foldkit/ui/combobox";
import * as Dialog from "@foldkit/ui/dialog";
import { Option } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import {
  answerFromBoardTarget,
  buildAnswerFromDraft,
  cardPickReady,
  cardPickRequiredCount,
  choiceDraftKey,
  choiceIntent,
  clickDamageAssign,
  damageAssignReady,
  declineAnswer,
  initPromptDraft,
  type PromptDraft,
} from "~/choice";
import { mulliganChrome } from "~/mulligan";
import { outcome } from "~/outcome";
import type {
  ActionView,
  CatalogCard,
  ObjectView,
  VisibleState,
  WireAttack,
  WireBlock,
  WireCost,
  WireIntent,
  WireModeChoice,
  WireTarget,
} from "~/wire/types";
import { clampX } from "~/xCost";
import { formatMessage } from "../domain/i18n/message";
import { type InspectPin, inspectPinChanged, pinFromCard, pinFromPlayer } from "../domain/inspect";
import { humanReason } from "../domain/reject";
import { isSoundEnabled, playUnmuteTick, setSoundEnabled, unlockTableAudio } from "../domain/tableAudio";
import type { GameFoldState } from "../game/fold";
import {
  FetchInspectCard,
  SearchCardNames,
  SetStackDwell,
  SetTurnYield,
  SetYield,
  SubmitIntent,
} from "../game/intents";
import type { Message as GameMessage } from "../game/messages";
import type { RpcClient } from "../resources";
import {
  buildTakeActionIntent,
  type CostPickState,
  type CostPicks,
  emptyCostPicks,
  findCastActionForObject,
  type ModalCast,
  type PlayModePick,
  planCostPipeline,
  planHandDrop,
  planHandPlay,
  planRunAction,
  reconcilePlayModeModes,
  type StagedAction,
  settleSacrificePick,
  usedCostPick,
  type XPromptState,
} from "./action/execution";
import { advance } from "./action/modal";
import {
  digCastNeedsHost,
  gyExileCostPile,
  pendingBoardTargetMode,
  pendingDamageAssignBlockers,
  pendingDigCastHostMode,
  pendingDivideSpellObjectIndexes,
  pendingExilePickIds,
  pendingExilePickOneClick,
  pendingGraveyardPickIds,
  pendingGraveyardPickOneClick,
  pendingHandPickIds,
  pendingHandPickOneClick,
  pendingPilePickPile,
  pendingPlayerAimOneClick,
  pendingPlayerAimSeats,
  pendingStackGhost,
  pendingTargetOneClick,
  sacrificeCostObjectIds,
  stagedPickTargets,
} from "./action/targeting";
import { CARD_NAME_COMBOBOX_ID, CardNameCombobox } from "./card-name-combobox";
import { engagedIds } from "./engagement";
import {
  markRevealSeen,
  prefersReducedMotion,
  REVEAL_HOLD_MS,
  REVEAL_HOLD_REDUCED_MS,
  RevealHoldTimer,
  RevealStepTimer,
  revealSeen,
  type SpotlightStep,
  spotlightSteps,
} from "./first-player-reveal";
import type { Camera, Vec2 } from "./geometry/camera";
import { panBy, screenToWorld, worldToScreen, zoomAt } from "./geometry/camera";
import {
  canArmEndTurn,
  combatStagingClearsOnStepChange,
  handleCombatDrop,
  stagedAttackersForDisplay,
} from "./geometry/combat-staging";
import { hitAvatar, hitTest } from "./geometry/hit-test";
import {
  canSelectPermanent,
  combatMode,
  declaresFor,
  fitCamera,
  type PointerPhase,
  pointerDown,
  pointerMove,
  pointerUp,
  primaryActionFor,
  resolveClick,
} from "./geometry/interaction";
import { avatarPos, CARD_H, CARD_W, landRowCenter, layout, type RenderCard, seatSlot, ZONE } from "./geometry/layout";
import { type RadialPress, radialPressDown, radialPressUp } from "./geometry/radial";
import {
  STACK_HOLD_MAX_MS,
  STACK_VERTICAL_RESERVED,
  shouldAutoCollapseStackExpand,
  stackFaceScreenOrigin,
  stackPeekFor,
  stackPresentation,
} from "./geometry/stackLayout";
import { modesForObject } from "./html/actions";
import { selectedRadialOptions } from "./html/activation-menu";
import { persistHintDismissed, readHintDismissed } from "./html/discoverability";
import { HAND_BAR_H, handMetrics } from "./html/hand";
import { CopyBoardLog } from "./log-commands";
import {
  CombatCancelAttacker,
  CombatCancelBlocker,
  GotCardNameComboboxMessage,
  GotConcedeDialogMessage,
  GotResultDialogMessage,
  GyExileChosen,
  type Message,
  PromptStringSet,
} from "./messages";
import { type ExitFx, spawnExitFx } from "./motion/exit-fx";
import {
  type CardFlight,
  flightOwnsId,
  flyingCardIds,
  handFlightScale,
  poseAtTarget,
  poseNearHandoff,
  rebindFlightId,
  remapFlightsForZoom,
  retargetFlight,
  spawnFlight,
  stackFlightScale,
  traceFlightSync,
} from "./motion/flights";

/** Fallback board size when there is no window to measure (SSR, tests). */
export const BOARD_VIEWPORT = { width: 1440, height: 900 } as const;

/** The board is `fixed inset-0`, so the window *is* the viewport. */
function measuredViewport(): { width: number; height: number } {
  if (typeof window === "undefined") return { ...BOARD_VIEWPORT };
  const width = window.innerWidth;
  const height = window.innerHeight;
  if (!(width > 0) || !(height > 0)) return { ...BOARD_VIEWPORT };
  return { width, height };
}

/** Device pixels per CSS pixel — canvas backing stores are sized by it so retina paints sharp. */
export function measuredDpr(): number {
  if (typeof window === "undefined") return 1;
  const dpr = window.devicePixelRatio;
  if (!(dpr > 0)) return 1;
  return Math.min(dpr, 3);
}

/** Bottom bar height at the design window — live boards use `handMetrics(viewport).barH`. */
export { HAND_BAR_H };

export type HandDragState = {
  action: ActionView;
  name: string;
  print: string;
  manaCost: WireCost;
  kind?: string;
  zone?: "hand" | "command" | "graveyard" | "exile";
  x: number;
  y: number;
};

type BattlefieldPose = {
  x: number;
  y: number;
  scale: number;
  print: string;
  name: string;
};

/** CR 103.1 spotlight: which seat won, the hop schedule, and where the spotlight sits. */
export type FirstPlayerReveal = { winner: number; steps: SpotlightStep[]; index: number };

export type BoardModel = {
  camera: Camera;
  cameraFitPlayers: number | null;
  /** True after the player pans/zooms — stops automatic fitCamera from fighting them. */
  cameraUserMoved: boolean;
  exitFx: Map<number, ExitFx>;
  flights: Map<number, CardFlight>;
  handHidden: Set<number>;
  hideCardIds: Set<number>;
  lastFlightFrame: number | null;
  lastBattlefieldPoses: Map<number, BattlefieldPose>;
  lastProvenanceSeq: number | null;
  ownedIds: Set<number>;
  pointer: PointerPhase;
  selectedId: number | null;
  /** Activation radial pointer arm (down on a wedge). */
  radialPress: RadialPress;
  /** Activation radial hover highlight index. */
  radialHover: number | null;
  viewport: { width: number; height: number };
  /** Device pixels per CSS pixel, clamped to 3. Canvas backing stores multiply by it. */
  dpr: number;
  cursor: Vec2;
  // Action session state (pre-submit chrome, cost pipeline, staging).
  staged: StagedAction | null;
  playModePick: PlayModePick | null;
  xPrompt: XPromptState | null;
  modalCast: ModalCast | null;
  sacrificePick: CostPickState | null;
  discardPick: CostPickState | null;
  gyExilePick: CostPickState | null;
  // Combat staging.
  combatAttackers: WireAttack[];
  combatBlocks: WireBlock[];
  attackersConfirmed: boolean;
  blockersConfirmed: boolean;
  priorStep: number | null;
  // Local reject text (mirrored from fold.reject on IntentRejected but kept separately for board-only rejects).
  reject: string | null;
  // Alt-pin inspect (Solid parity: Alt-down pins under cursor / aux hover; Alt-up dismisses).
  /** Alt key is currently held — also gates Alt+click pin as a secondary path. */
  altDown: boolean;
  /** The card pinned in the inspect overlay; null when no overlay is shown. */
  inspectPin: InspectPin | null;
  /** Catalog data for the current inspect pin. `undefined` = fetch in-flight; `null` = not found. */
  inspectCard: CatalogCard | null | undefined;
  /** Which face of a DFC to show in the inspect overlay. */
  inspectFace: "front" | "back";
  /** Hand-bar card under the pointer (DOM overlay above the canvas). */
  handInspectHover: InspectAuxCard | null;
  /** Stack overlay card under the pointer. */
  stackInspectHover: InspectAuxCard | null;
  // Pile (GY/exile) overlay.
  /** Non-null when the pile overlay is open. */
  pileExpand: { zone: number; owner: number } | null;
  // Stack overlay expand (magnifier / strip view).
  stackExpand: boolean;
  /** Peak hold-ms seen this countdown — bar denominator for `stack_hold_remaining_ms`. */
  stackHoldPeak: number;
  /** Board log panel shows the full fold buffer instead of the last 30 lines. */
  logExpanded: boolean;
  /** Last board log copy succeeded; reset by expand/collapse or a new copy attempt. */
  logCopied: boolean;
  /** Last board log copy failed; paired with `logCopied` for toolbar feedback. */
  logCopyFailed: boolean;
  // Concede.
  concedeDialog: Dialog.Model;
  // Game result.
  resultDialog: Dialog.Model;
  /** The result overlay has already been raised for this game — it is a one-shot, so dismissing it
   *  must not let the next fold put it straight back up. */
  resultRaised: boolean;
  // Discoverability chrome.
  hintDismissed: boolean;
  hintAutoHidden: boolean;
  legendOpen: boolean;
  soundOn: boolean;
  priorityElapsed: number;
  lastPriorityHolder: number | null;
  /** Key of the current `pending_choice` — resets `promptDraft` when it changes. */
  pendingChoiceKey: string | null;
  /** In-progress answer for interactive pending-choice forms. */
  promptDraft: PromptDraft | null;
  /**
   * True after a prompt answer was submitted while `pending_choice` still matches.
   * Keeps the draft painted (no re-init flash) and blocks double-submit / edits until
   * the choice key changes, a newer board seq arrives for an equivalent-looking re-raise
   * (CR 701.27b proliferate twice), or the intent is rejected.
   */
  promptSubmitInFlight: boolean;
  /** Board `seq` at the moment `promptSubmitInFlight` was set — used to detect re-raises. */
  promptSubmitSeq: number | null;
  /** Catalog name suggestions for `choose_card_name` (query must match current draft). */
  cardNameSuggestions: { query: string; names: ReadonlyArray<string> } | null;
  /** The `choose_card_name` typeahead. Owns the input text; the string draft mirrors it. */
  cardNameCombobox: Combobox.Model;
  /** Filter query for closed option prompts (creature types). */
  promptOptionFilter: string;
  /** Selected row while click-to-place reordering `order_triggers` (null when idle). */
  orderPickPos: number | null;
  /** Window-captured hand-bar drag ghost (null when idle). */
  handDrag: HandDragState | null;
  /** Hovered hand/radial action id — resolves `auto_tap` from the live action list. */
  hoverActionId: number | null;
  /** CR 103.1 one-shot starting-player spotlight; null once dismissed or already shown. */
  firstPlayerReveal: FirstPlayerReveal | null;
};

/** Document-unique id for the concede confirmation. Doubles as its `data-testid`. */
export const CONCEDE_DIALOG_ID = "concede-dialog";

/** Document-unique id for the game-result overlay. Doubles as its `data-testid`. */
export const RESULT_DIALOG_ID = "result-overlay";

export function initialBoardModel(): BoardModel {
  return {
    camera: { panX: 0, panY: 0, zoom: 1 },
    cameraFitPlayers: null,
    cameraUserMoved: false,
    exitFx: new Map(),
    flights: new Map(),
    handHidden: new Set(),
    hideCardIds: new Set(),
    lastFlightFrame: null,
    lastBattlefieldPoses: new Map(),
    lastProvenanceSeq: null,
    ownedIds: new Set(),
    pointer: { kind: "idle" },
    selectedId: null,
    radialPress: { armed: null },
    radialHover: null,
    viewport: measuredViewport(),
    dpr: measuredDpr(),
    cursor: { x: 0, y: 0 },
    staged: null,
    playModePick: null,
    xPrompt: null,
    modalCast: null,
    sacrificePick: null,
    discardPick: null,
    gyExilePick: null,
    combatAttackers: [],
    combatBlocks: [],
    attackersConfirmed: false,
    blockersConfirmed: false,
    priorStep: null,
    reject: null,
    altDown: false,
    inspectPin: null,
    inspectCard: undefined,
    inspectFace: "front",
    handInspectHover: null,
    stackInspectHover: null,
    pileExpand: null,
    stackExpand: false,
    stackHoldPeak: 0,
    logExpanded: false,
    logCopied: false,
    logCopyFailed: false,
    concedeDialog: Dialog.init({ id: CONCEDE_DIALOG_ID }),
    resultDialog: Dialog.init({ id: RESULT_DIALOG_ID }),
    resultRaised: false,
    hintDismissed: readHintDismissed(),
    hintAutoHidden: false,
    legendOpen: false,
    soundOn: isSoundEnabled(),
    priorityElapsed: 0,
    lastPriorityHolder: null,
    pendingChoiceKey: null,
    promptDraft: null,
    promptSubmitInFlight: false,
    promptSubmitSeq: null,
    cardNameSuggestions: null,
    cardNameCombobox: Combobox.init({ id: CARD_NAME_COMBOBOX_ID }),
    promptOptionFilter: "",
    orderPickPos: null,
    handDrag: null,
    hoverActionId: null,
    firstPlayerReveal: null,
  };
}

type BoardFold = Pick<GameFoldState, "provenance" | "seq" | "state">;

export function syncBoardWithGame(model: BoardModel, fold: BoardFold): BoardModel {
  if (fold.state == null) return model;

  let next = undecidedMulliganInspectLock(fold.state) ? clearInspectState(model) : model;
  next = syncCombatStaging(next, fold);
  next = syncPromptDraft(next, fold);
  if (next.lastPriorityHolder !== fold.state.priority) {
    next = { ...next, priorityElapsed: 0, lastPriorityHolder: fold.state.priority };
  }
  const playerCount = Math.max(1, fold.state.players.length);
  if (!next.cameraUserMoved && next.cameraFitPlayers !== playerCount) {
    const fitted = fitCamera(
      { x: next.viewport.width, y: next.viewport.height },
      playerCount,
      handMetrics(next.viewport).barH,
    );
    next = {
      ...next,
      flights: remapFlightsForZoom(next.flights, next.camera.zoom, fitted.zoom),
      camera: fitted,
      cameraFitPlayers: playerCount,
    };
  }

  // Drop radial selection when the permanent leaves the battlefield.
  if (next.selectedId != null) {
    const obj = fold.state.objects.find((o) => o.id === next.selectedId);
    if (!obj || obj.zone !== ZONE.Battlefield) {
      next = { ...next, selectedId: null, radialPress: { armed: null }, radialHover: null };
    }
  }

  if (next.lastProvenanceSeq !== fold.seq) {
    next = syncFlightsWithGame(next, fold);
  }
  next = syncPlayModePick(next, fold);
  return syncStackChrome(next, fold);
}

function syncPlayModePick(model: BoardModel, fold: BoardFold): BoardModel {
  const pick = model.playModePick;
  if (pick == null) return model;

  const modes = reconcilePlayModeModes(pick.modes, fold.state?.actions);
  if (modes.length === 0) {
    return { ...clearPlayOrigin(model, pick.card.id), playModePick: null };
  }
  return { ...model, playModePick: { ...pick, modes } };
}

function syncStackChrome(model: BoardModel, fold: BoardFold): BoardModel {
  const state = fold.state;
  if (state == null) return model;

  const holdMs = state.stack_hold_remaining_ms ?? 0;
  const stackHoldPeak = holdMs > 0 ? Math.min(STACK_HOLD_MAX_MS, Math.max(model.stackHoldPeak, holdMs)) : 0;

  const showStaged =
    (model.staged != null && stagedPickTargets(model.staged, state) === null) || pendingStackGhost(state) != null;
  const visualCount = state.stack.length + (showStaged ? 1 : 0);
  const peek = stackPeekFor(visualCount, model.viewport.height, STACK_VERTICAL_RESERVED);
  const stackExpand = shouldAutoCollapseStackExpand({
    expanded: model.stackExpand,
    count: visualCount,
    peek,
    staged: showStaged,
  })
    ? false
    : model.stackExpand;

  if (stackHoldPeak === model.stackHoldPeak && stackExpand === model.stackExpand) return model;
  return { ...model, stackHoldPeak, stackExpand };
}

function syncPromptDraft(model: BoardModel, fold: BoardFold): BoardModel {
  const gameState = fold.state;
  const pc = gameState?.pending_choice ?? null;
  const key = pc != null ? choiceDraftKey(pc) : null;
  if (key === model.pendingChoiceKey) {
    // Same draft identity — but a newer board seq after submit means the engine re-raised an
    // equivalent-looking choice (CR 701.27b "proliferate twice"). Unfreeze and reset so Confirm
    // is not a silent no-op on the next iteration.
    if (model.promptSubmitInFlight && model.promptSubmitSeq != null && fold.seq !== model.promptSubmitSeq) {
      return {
        ...model,
        promptDraft: pc != null && gameState != null ? initPromptDraft(pc, gameState) : null,
        promptSubmitInFlight: false,
        promptSubmitSeq: null,
        cardNameSuggestions: null,
        cardNameCombobox: Combobox.init({ id: CARD_NAME_COMBOBOX_ID }),
        promptOptionFilter: "",
        orderPickPos: null,
      };
    }
    return model;
  }
  const pile = pc != null && gameState != null ? pendingPilePickPile(pc, gameState) : null;
  return {
    ...model,
    pendingChoiceKey: key,
    promptDraft: pc != null && gameState != null ? initPromptDraft(pc, gameState) : null,
    promptSubmitInFlight: false,
    promptSubmitSeq: null,
    cardNameSuggestions: null,
    cardNameCombobox: Combobox.init({ id: CARD_NAME_COMBOBOX_ID }),
    promptOptionFilter: "",
    orderPickPos: null,
    pileExpand: pile != null ? pile : model.gyExilePick != null ? model.pileExpand : null,
  };
}

/** Freeze the current prompt draft after submitting an answer (avoids Bottom-lane re-init flash). */
function withPromptSubmitInFlight(model: BoardModel, seq: number, extras: Partial<BoardModel> = {}): BoardModel {
  return { ...model, ...extras, promptSubmitInFlight: true, promptSubmitSeq: seq };
}

function samePromptTarget(a: WireTarget | null | undefined, b: WireTarget | null | undefined): boolean {
  if (a == null || b == null) return a == null && b == null;
  if (a.kind !== b.kind) return false;
  if (a.kind === "player" && b.kind === "player") return a.player === b.player;
  if (a.kind === "object" && b.kind === "object") return a.id === b.id;
  return false;
}

function samePromptModeChoice(a: WireModeChoice, b: WireModeChoice): boolean {
  return a.index === b.index && samePromptTarget(a.target, b.target);
}

function partitionReady(
  pc: Extract<ActionlessPendingChoice, { kind: "partition_revealed" | "distribute_top" }>,
  draft: PromptDraft,
): boolean {
  if (draft.kind !== "partition") return false;
  if (pc.kind === "partition_revealed") return true;
  const toHand = draft.buckets.to_hand ?? [];
  const toBottom = draft.buckets.to_bottom ?? [];
  const toExile = draft.buckets.to_exile_may_play ?? [];
  return (
    toHand.length === pc.to_hand &&
    toBottom.length === pc.to_bottom &&
    toExile.length === pc.to_exile_may_play &&
    toHand.length + toBottom.length + toExile.length === pc.items.length
  );
}

type ActionlessPendingChoice = NonNullable<BoardFold["state"]>["pending_choice"];

function cardsFor(fold: GameFoldState, model: BoardModel): RenderCard[] {
  if (fold.state == null) return [];
  return layout(fold.state, fold.state.viewer, engagedIds(fold.state, model));
}

function cardAt(fold: GameFoldState, model: BoardModel, x: number, y: number): RenderCard | null {
  const cards = cardsFor(fold, model);
  const hitId = hitTest(model.camera, x, y, cards);
  if (hitId == null) return null;
  return cards.find((card) => card.id === hitId) ?? null;
}

/** The seats whose creatures this viewer may drag into a combat declaration right now — empty when
 * no declaration is theirs to make. Usually just themselves; a moved declaration (Master Warcraft)
 * hands them somebody else's creatures. */
function stageableSeats(fold: GameFoldState): number[] {
  const state = fold.state;
  if (state == null) return [];
  const mode = combatMode(state.actions, false, {
    attackersDeclared: state.combat.attackers_declared,
    blockersDeclared: state.combat.blockers_declared.includes(state.viewer),
  });
  return declaresFor(state.actions, mode);
}

/** Screen pose + scale for a stack flight so settle matches the resting HTML face. */
function stackFlightAim(
  model: BoardModel,
  opts: { count: number; row: number },
): { x: number; y: number; scale: number } {
  const count = Math.max(1, opts.count);
  const row = Math.max(0, Math.min(count - 1, opts.row));
  const presentation = stackPresentation({
    count,
    expandedOpen: model.stackExpand,
    viewportW: model.viewport.width,
    viewportH: model.viewport.height,
  });
  const origin = stackFaceScreenOrigin({
    presentation,
    viewportW: model.viewport.width,
    viewportH: model.viewport.height,
    count,
    row,
    peek: presentation === "pile" ? stackPeekFor(count, model.viewport.height) : undefined,
  });
  return { x: origin.x, y: origin.y, scale: stackFlightScale(model.camera.zoom) };
}

function stackFlightAimForSource(
  model: BoardModel,
  stack: ReadonlyArray<{ source: number }>,
  sourceId: number,
): { x: number; y: number; scale: number } {
  const count = Math.max(1, stack.length);
  const row = stack.findIndex((entry) => entry.source === sourceId);
  return stackFlightAim(model, { count, row: row >= 0 ? row : count - 1 });
}

function cardTarget(camera: Camera, card: RenderCard): Vec2 {
  return worldToScreen(camera, card.x + card.w / 2, card.y + card.h / 2);
}

function playerOrigin(model: BoardModel, fold: BoardFold, seat: number): Vec2 {
  if (fold.state == null) {
    const aim = stackFlightAim(model, { count: 1, row: 0 });
    return { x: aim.x, y: aim.y };
  }
  const count = Math.max(1, fold.state.players.length);
  const pos = avatarPos(seat, fold.state.viewer, count);
  return worldToScreen(model.camera, pos.x, pos.y);
}

function retargetFlightToCard(
  flight: CardFlight,
  model: BoardModel,
  card: RenderCard,
  opts?: { retainHold?: boolean; zone?: "stack" | "land" | "from-stack" | "battlefield"; note?: string },
): CardFlight {
  const target = cardTarget(model.camera, card);
  return retargetFlight(flight, { x: target.x, y: target.y, scale: 1 }, opts);
}

function hiddenCardIds(flights: ReadonlyMap<number, CardFlight>, exitFx: ReadonlyMap<number, ExitFx>): Set<number> {
  const hidden = flyingCardIds(flights);
  for (const id of exitFx.keys()) hidden.add(id);
  return hidden;
}

function battlefieldPoseFromCard(camera: Camera, card: RenderCard): BattlefieldPose {
  const target = cardTarget(camera, card);
  return {
    x: target.x,
    y: target.y,
    scale: 1,
    print: card.print,
    name: card.name,
  };
}

function battlefieldPoseFromFlight(flight: CardFlight): BattlefieldPose {
  return {
    x: flight.x,
    y: flight.y,
    scale: flight.scale,
    print: flight.print,
    name: flight.name,
  };
}

function currentBattlefieldPoses(model: BoardModel, cards: readonly RenderCard[]): Map<number, BattlefieldPose> {
  const poses = new Map<number, BattlefieldPose>();
  for (const card of cards) {
    if (card.zone !== ZONE.Battlefield) continue;
    poses.set(card.id, battlefieldPoseFromCard(model.camera, card));
  }
  return poses;
}

function syncFlightsWithGame(model: BoardModel, fold: BoardFold): BoardModel {
  const state = fold.state;
  if (state == null) return model;

  const cards = layout(state, state.viewer, engagedIds(state, model));
  const cardsById = new Map(cards.map((card) => [card.id, card]));
  const battlefieldExitIds = new Set(fold.provenance.battlefieldExits.keys());
  const exitFx = new Map(model.exitFx);
  const handHidden = new Set(model.handHidden);
  let flights = new Map(model.flights);

  for (const [id, flight] of flights) {
    // Local seeds hold until landPlayFrom / stackEntrances rebind them. Retargeting here would
    // clear hold and let a settle sync drop the flight before provenance arrives.
    if (flight.hold) continue;
    const card = cardsById.get(id);
    if (card != null) {
      flights.set(id, retargetFlightToCard(flight, model, card));
      continue;
    }
    if (flight.kind !== "stack") continue;
    const aim = stackFlightAimForSource(model, state.stack, id);
    flights.set(id, retargetFlight(flight, { x: aim.x, y: aim.y, scale: aim.scale }));
  }

  for (const [id, zone] of fold.provenance.battlefieldExits) {
    const from = fold.provenance.zoneMoves.get(id);
    const flightId = flights.has(id) ? id : from != null && flights.has(from) ? from : null;
    const flight = flightId == null ? undefined : flights.get(flightId);
    const pose =
      flight != null
        ? battlefieldPoseFromFlight(flight)
        : ((from != null ? model.lastBattlefieldPoses.get(from) : undefined) ?? model.lastBattlefieldPoses.get(id));
    if (flight != null && flightId != null) {
      flights.delete(flightId);
      if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
    }
    if (pose == null) continue;
    exitFx.set(
      id,
      spawnExitFx({
        id,
        print: pose.print,
        name: pose.name,
        kind: zone === "graveyard" ? "destroy" : "exile",
        x: pose.x,
        y: pose.y,
        scale: pose.scale,
        seed: id,
      }),
    );
  }

  for (const [permanent, from] of fold.provenance.landPlayFrom) {
    const card = cardsById.get(permanent);
    if (card == null) continue;

    if (!flights.has(permanent) && flights.has(from)) {
      flights = rebindFlightId(flights, from, permanent);
    }

    const existing = flights.get(permanent);
    if (existing != null) {
      const target = cardTarget(model.camera, card);
      const aim = { x: target.x, y: target.y, scale: 1 };
      // On/near the real slot — hand off when parked, or keep easing without a short retarget.
      // Settled far from the slot (provisional row) must still retarget with hold retained.
      if (existing.hold && poseAtTarget(existing, aim)) {
        flights.delete(permanent);
        handHidden.delete(from);
        continue;
      }
      if (existing.hold && poseNearHandoff(existing, aim)) {
        const remainingPx = Math.hypot(aim.x - existing.x, aim.y - existing.y);
        // Near the real slot — hand off now (flying or settled). Keeping a stale glide
        // toward the provisional aim then correcting later is the short second ease.
        traceFlightSync({
          op: "handoff",
          zone: "land",
          id: permanent,
          hold: true,
          phase: existing.phase,
          remainingPx,
        });
        flights.delete(permanent);
        handHidden.delete(from);
        continue;
      }
      flights.set(
        permanent,
        retargetFlightToCard({ ...existing, kind: "battlefield", fromCardId: from }, model, card, {
          retainHold: existing.hold === true,
          zone: "land",
          note: "landPlayFrom",
        }),
      );
      handHidden.add(from);
      continue;
    }

    const start = playerOrigin(model, fold, card.controller);
    const target = cardTarget(model.camera, card);
    flights.set(
      permanent,
      spawnFlight({
        id: permanent,
        print: card.print,
        name: card.name,
        x: start.x,
        y: start.y,
        scale: handFlightScale(model.camera.zoom, handMetrics(model.viewport).cardW),
        targetX: target.x,
        targetY: target.y,
        targetScale: 1,
        kind: "battlefield",
        fromCardId: from,
      }),
    );
    handHidden.add(from);
  }

  for (const [spell, meta] of fold.provenance.stackEntrances) {
    const aim = stackFlightAimForSource(model, state.stack, spell);
    if (!flights.has(spell) && flights.has(meta.from)) {
      flights = rebindFlightId(flights, meta.from, spell);
    }

    const existing = flights.get(spell);
    if (existing != null) {
      // At the face — hand off. Near but still flying: keep the current glide (do not retarget
      // the last inches — that is the short second ease). Far: retarget, retain hold.
      if (poseAtTarget(existing, aim)) {
        flights.delete(spell);
        if (meta.from != null) handHidden.delete(meta.from);
        continue;
      }
      if (existing.hold && poseNearHandoff(existing, aim)) {
        const remainingPx = Math.hypot(aim.x - existing.x, aim.y - existing.y);
        // Near the stack face — hand off now. Do not keep easing toward a stale seed aim
        // (that path later retargets and reads as a short second glide).
        traceFlightSync({
          op: "handoff",
          zone: "stack",
          id: spell,
          hold: true,
          phase: existing.phase,
          remainingPx,
        });
        flights.delete(spell);
        if (meta.from != null) handHidden.delete(meta.from);
        continue;
      }
      flights.set(
        spell,
        retargetFlight(
          { ...existing, kind: "stack", fromCardId: meta.from },
          {
            x: aim.x,
            y: aim.y,
            scale: aim.scale,
          },
          { retainHold: existing.hold === true, zone: "stack", note: "stackEntrances" },
        ),
      );
      handHidden.add(meta.from);
      continue;
    }

    const start = playerOrigin(model, fold, meta.controller);
    flights.set(
      spell,
      spawnFlight({
        id: spell,
        print: "",
        name: "",
        x: start.x,
        y: start.y,
        scale: handFlightScale(model.camera.zoom, handMetrics(model.viewport).cardW),
        targetX: aim.x,
        targetY: aim.y,
        targetScale: aim.scale,
        kind: "stack",
        fromCardId: meta.from,
      }),
    );
    handHidden.add(meta.from);
  }

  // After provenance folds, held seeds may still be easing. Hand off when parked; while far,
  // refresh aim without clearing hold (avoids the post-retarget short second ease).
  // From `state.objects`, not `cards` — the hand is HTML, so `layout` never emits hand faces.
  const handIds = new Set(state.objects.filter((object) => object.zone === ZONE.Hand).map((object) => object.id));
  for (const [id, flight] of [...flights.entries()]) {
    if (!flight.hold) continue;
    // Unclaimed seed whose card already left hand: the provenance that would have rebound it is
    // gone (snapshot clears provenance; a delta carries only its own). No later sync can release
    // it and a settled hold gets no more clock ticks — it would stay painted for the rest of the
    // game. Cut to the authoritative face instead.
    if (!authorityOwnsFlightDestination(fold, { ...flight, id })) {
      if (handIds.has(flight.fromCardId ?? id)) continue;
      traceFlightSync({
        op: "synced-drop",
        zone: flight.kind,
        id,
        hold: true,
        phase: flight.phase,
        remainingPx: Math.hypot(flight.targetX - flight.x, flight.targetY - flight.y),
        note: "stale-seed",
      });
      flights.delete(id);
      handHidden.delete(flight.fromCardId ?? id);
      continue;
    }
    if (flight.kind === "stack") {
      const aim = stackFlightAimForSource(model, state.stack, id);
      if (poseAtTarget(flight, aim) || poseNearHandoff(flight, aim)) {
        flights.delete(id);
        if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
        continue;
      }
      flights.set(
        id,
        retargetFlight(
          flight,
          { x: aim.x, y: aim.y, scale: aim.scale },
          {
            retainHold: true,
            zone: "stack",
            note: "post-hold-refresh",
          },
        ),
      );
      continue;
    }
    if (flight.kind !== "battlefield") continue;
    const card = cardsById.get(id);
    if (card == null) continue;
    const target = cardTarget(model.camera, card);
    const aim = { x: target.x, y: target.y, scale: 1 };
    if (poseAtTarget(flight, aim) || poseNearHandoff(flight, aim)) {
      flights.delete(id);
      if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
      continue;
    }
    flights.set(id, retargetFlight(flight, aim, { retainHold: true, zone: "land", note: "post-hold-refresh" }));
  }

  for (const id of new Set([...fold.provenance.resolvedFromStack, ...fold.provenance.leftStackToPile])) {
    if (battlefieldExitIds.has(id)) continue;
    const card = cardsById.get(id);
    if (card == null) continue;

    const fromSpell = fold.provenance.zoneMoves.get(id);
    if (fromSpell != null && !flights.has(id) && flights.has(fromSpell)) {
      flights = rebindFlightId(flights, fromSpell, id);
    }

    const existing = flights.get(id);
    if (existing != null) {
      flights.set(id, retargetFlightToCard({ ...existing, id, kind: "from-stack" }, model, card));
      continue;
    }

    const startAim = stackFlightAim(model, {
      count: Math.max(1, state.stack.length + 1),
      row: state.stack.length,
    });
    const target = cardTarget(model.camera, card);
    flights.set(
      id,
      spawnFlight({
        id,
        print: card.print,
        name: card.name,
        x: startAim.x,
        y: startAim.y,
        scale: startAim.scale,
        targetX: target.x,
        targetY: target.y,
        targetScale: 1,
        kind: "from-stack",
      }),
    );
  }

  for (const [id, from] of fold.provenance.zoneMoves) {
    if (battlefieldExitIds.has(id)) continue;
    if (flights.has(id)) continue;
    const card = cardsById.get(id);
    if (card == null) continue;

    const target = cardTarget(model.camera, card);
    const prior = cardsById.get(from);
    const startAim =
      prior == null
        ? stackFlightAim(model, { count: Math.max(1, state.stack.length + 1), row: state.stack.length })
        : { ...cardTarget(model.camera, prior), scale: 1 };
    flights.set(
      id,
      spawnFlight({
        id,
        print: card.print,
        name: card.name,
        x: startAim.x,
        y: startAim.y,
        scale: startAim.scale,
        targetX: target.x,
        targetY: target.y,
        targetScale: 1,
        kind: "battlefield",
      }),
    );
  }

  const stackSources = new Set(state.stack.map((stackObject) => stackObject.source));
  const pendingResolve = fold.provenance.resolvedFromStack.size > 0 || fold.provenance.leftStackToPile.size > 0;
  for (const [id, flight] of flights) {
    if (flight.kind !== "stack") continue;
    if (flight.hold) continue;
    if (stackSources.has(id)) continue;
    if (pendingResolve) continue;
    flights.delete(id);
    if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
  }

  return {
    ...model,
    exitFx,
    flights,
    handHidden,
    hideCardIds: hiddenCardIds(flights, exitFx),
    lastBattlefieldPoses: currentBattlefieldPoses(model, cards),
    lastProvenanceSeq: fold.seq,
    ownedIds: new Set(flights.keys()),
  };
}

function pointerDownModel(model: BoardModel, fold: GameFoldState, x: number, y: number): BoardModel {
  const state = fold.state;
  if (state == null) return model;

  return {
    ...model,
    cursor: { x, y },
    pointer: pointerDown(cardAt(fold, model, x, y), x, y, stageableSeats(fold)),
  };
}

function pointerMoveModel(model: BoardModel, x: number, y: number): BoardModel {
  const moved = pointerMove(model.pointer, x, y);
  if (moved.pan == null) {
    return { ...model, cursor: { x, y }, pointer: moved.phase };
  }

  return {
    ...model,
    camera: panBy(model.camera, moved.pan.dx, moved.pan.dy),
    cameraUserMoved: true,
    cursor: { x, y },
    pointer: moved.phase,
  };
}

function avatarSeatAt(fold: GameFoldState, model: BoardModel, x: number, y: number): number | null {
  const state = fold.state;
  if (state == null) return null;
  const count = Math.max(1, state.players.length);
  const positions: Record<number, Vec2> = {};
  for (const p of state.players) {
    positions[p.player] = avatarPos(p.player, state.viewer, count);
  }
  return hitAvatar(model.camera, x, y, positions);
}

function stagedLegalObjectIds(staged: StagedAction): Set<number> {
  const out = new Set<number>();
  for (const t of staged.action.targets ?? []) {
    if (t.kind === "object") out.add(t.id);
  }
  return out;
}

function stagedLegalPlayerSeats(staged: StagedAction): Set<number> {
  const out = new Set<number>();
  for (const t of staged.action.targets ?? []) {
    if (t.kind === "player") out.add(t.player);
  }
  return out;
}

function pointerUpModel(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  x: number,
  y: number,
): BoardReturn {
  const idle: BoardModel = { ...model, pointer: { kind: "idle" } };
  const hitCard = cardAt(fold, model, x, y);
  const release = pointerUp(model.pointer, x, y, hitCard);

  if (release.kind === "combat-drop") {
    const from = fold.state?.objects.find((o) => o.id === release.card.id) ?? null;
    const defenderSeat = avatarSeatAt(fold, model, x, y);
    const blockAttackerId = hitCard?.id ?? null;
    return combatDropModel(idle, fold, from, defenderSeat, blockAttackerId);
  }

  if (release.kind === "click") {
    // Alt held: pin inspect for this card (non-pile only).
    if (model.altDown && release.card.pile === 0) {
      const pin = pinFromCard(
        true,
        {
          name: release.card.name,
          faceDown: release.card.faceDown,
          prepared: release.card.prepared,
          id: release.card.id,
          zone: release.card.zone,
          pile: release.card.pile,
          cardId: release.card.cardId,
          print: release.card.print,
        },
        ZONE.Battlefield,
      );
      if (pin != null) {
        const changed = inspectPinChanged(idle.inspectPin, pin);
        const newPin = changed ? pin : idle.inspectPin;
        const cmds: BoardCmd[] =
          changed && pin.cardId ? [FetchInspectCard({ cardId: pin.cardId }) as unknown as BoardCmd] : [];
        return [
          {
            ...idle,
            inspectPin: newPin,
            inspectCard: changed ? undefined : idle.inspectCard,
            inspectFace: changed ? "front" : idle.inspectFace,
          },
          cmds,
        ];
      }
    }
    // Pile card: open the pile overlay.
    if (release.card.pile > 0) {
      return [{ ...idle, pileExpand: { zone: release.card.zone, owner: release.card.owner } }, []];
    }
    if (model.sacrificePick != null && fold.state != null) {
      const costIds = sacrificeCostObjectIds(model.sacrificePick.action.sacrifice_choices, fold.state);
      if (costIds?.has(release.card.id)) {
        const settled = settleSacrificePick(model.sacrificePick, release.card.id);
        return continueAfterCostPick(
          { ...idle, sacrificePick: null },
          fold,
          tableId,
          settled.action,
          settled.card,
          settled.picks,
          settled.dropSeed,
          settled.screenOrigin,
        );
      }
    }
    if (model.staged != null) {
      const legalObjects = stagedLegalObjectIds(model.staged);
      if (legalObjects.has(release.card.id)) {
        return completeStagedTarget(idle, fold, tableId, { kind: "object", id: release.card.id });
      }
      return [idle, []];
    }
    const pc = fold.state?.pending_choice ?? null;
    const damageBlockers = fold.state != null ? pendingDamageAssignBlockers(pc, fold.state) : null;
    if (
      damageBlockers?.has(release.card.id) &&
      (pc?.kind === "assign_combat_damage" || pc?.kind === "divide_counters")
    ) {
      const synced = syncPromptDraft(idle, fold);
      const draft = synced.promptDraft?.kind === "damage" ? synced.promptDraft : null;
      if (draft == null) return [synced, []];
      if (pc.kind === "assign_combat_damage") {
        const source = fold.state?.objects.find((o) => o.id === pc.source);
        const power = source?.power ?? 0;
        const trample = source?.keywords?.includes("trample") ?? false;
        const amounts = clickDamageAssign(draft.amounts, release.card.id, power, trample);
        return [{ ...synced, promptDraft: { kind: "damage", amounts } }, []];
      }
      const amounts = clickDamageAssign(draft.amounts, release.card.id, pc.total, false);
      return [{ ...synced, promptDraft: { kind: "damage", amounts } }, []];
    }
    const divideIndexes = fold.state != null ? pendingDivideSpellObjectIndexes(pc, fold.state) : null;
    if (divideIndexes != null && pc?.kind === "divide_spell_damage") {
      const itemIndex = divideIndexes.get(release.card.id);
      if (itemIndex != null) {
        const synced = syncPromptDraft(idle, fold);
        const draft = synced.promptDraft?.kind === "divide" ? synced.promptDraft : null;
        if (draft == null) return [synced, []];
        const amounts = clickDamageAssign(draft.amounts, itemIndex, pc.total, false);
        return [{ ...synced, promptDraft: { kind: "divide", amounts } }, []];
      }
    }
    const syncedForDig = syncPromptDraft(idle, fold);
    const digHostAim = fold.state != null ? pendingDigCastHostMode(pc, fold.state, syncedForDig.promptDraft) : null;
    if (digHostAim != null && pc != null && digHostAim.objects.has(release.card.id)) {
      if (syncedForDig.promptDraft?.kind !== "card-pick" || syncedForDig.promptDraft.picked.length !== 1) {
        return [syncedForDig, []];
      }
      const draft = { ...syncedForDig.promptDraft, host: release.card.id };
      const answer = buildAnswerFromDraft(pc, draft);
      if (answer == null) return [syncedForDig, []];
      return [
        { ...syncedForDig, promptDraft: null, pendingChoiceKey: null, pileExpand: null },
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
    }
    const pendingAim = fold.state != null ? pendingBoardTargetMode(pc, fold.state) : null;
    if (pendingAim != null && pc != null && pendingAim.objects.has(release.card.id)) {
      if (pendingTargetOneClick(pc)) {
        const answer = answerFromBoardTarget(pc, { kind: "object", id: release.card.id });
        if (answer != null) {
          return [idle, boardIntentSubmit(tableId, choiceIntent(pc, answer))];
        }
      } else {
        return togglePendingObjectAimPick(idle, fold, pc, release.card.id);
      }
    }
    // Combat cancel + permanent select share `resolveClick` so tap-in-place on a staged
    // attacker/blocker un-stages it before the activation radial can open.
    if (fold.state != null) {
      const click = resolveClick(fold.state, fold.state.viewer, release.card, {
        spectating: false,
        staged: null,
        stagedTargets: new Set(),
        attackers: idle.combatAttackers,
        blocks: idle.combatBlocks,
      });
      if (click.kind === "cancel-attacker") {
        return updateBoard(idle, CombatCancelAttacker({ attackerId: click.id }), fold, tableId);
      }
      if (click.kind === "cancel-blocker") {
        return updateBoard(idle, CombatCancelBlocker({ blockerId: click.id }), fold, tableId);
      }
    }
    if (
      !canSelectPermanent(release.card.id, release.card.tapsForMana, fold.state?.actions, {
        summoningSick: release.card.summoningSick,
        hasHaste: release.card.hasHaste,
      })
    ) {
      return [idle, []];
    }
    return [{ ...idle, selectedId: release.card.id, radialPress: { armed: null }, radialHover: null }, []];
  }

  // No card hit — release may still complete a staged player target on an avatar.
  if (model.staged != null) {
    const seat = avatarSeatAt(fold, model, x, y);
    if (seat != null && stagedLegalPlayerSeats(model.staged).has(seat)) {
      return completeStagedTarget(idle, fold, tableId, { kind: "player", player: seat });
    }
  }
  const pc = fold.state?.pending_choice ?? null;
  const pendingAim = fold.state != null ? pendingBoardTargetMode(pc, fold.state) : null;
  if (pendingAim != null && pc != null) {
    const seat = avatarSeatAt(fold, model, x, y);
    if (seat != null && pendingAim.players.has(seat)) {
      if (pendingTargetOneClick(pc)) {
        const answer = answerFromBoardTarget(pc, { kind: "player", player: seat });
        if (answer != null) {
          return [idle, boardIntentSubmit(tableId, choiceIntent(pc, answer))];
        }
      } else {
        return togglePendingPlayerAimPick(idle, fold, seat);
      }
    }
  }
  const playerSeats = fold.state != null ? pendingPlayerAimSeats(pc, fold.state) : null;
  if (playerSeats != null && pc != null) {
    const seat = avatarSeatAt(fold, model, x, y);
    if (seat != null && playerSeats.has(seat)) {
      if (pendingPlayerAimOneClick(pc)) {
        if (pc.kind === "choose_splitting_opponent") {
          const answer = { kind: "target" as const, id: 0, player: seat };
          return [idle, boardIntentSubmit(tableId, choiceIntent(pc, answer))];
        }
        if (pc.kind === "choose_target_players") {
          const answer = { kind: "target_players" as const, players: [seat] };
          return [idle, boardIntentSubmit(tableId, choiceIntent(pc, answer))];
        }
      } else if (pc.kind === "choose_target_players") {
        const synced = syncPromptDraft(idle, fold);
        const players = synced.promptDraft?.kind === "player-pick" ? synced.promptDraft.players : [];
        let next: number[];
        if (players.includes(seat)) {
          next = players.filter((p) => p !== seat);
        } else if (players.length >= pc.max) {
          return [synced, []];
        } else {
          next = [...players, seat];
        }
        return [{ ...synced, promptDraft: { kind: "player-pick", players: next } }, []];
      }
    }
  }
  // Empty board click dismisses the activation radial.
  if (model.selectedId != null) {
    return [{ ...idle, selectedId: null, radialPress: { armed: null }, radialHover: null }, []];
  }
  return [idle, []];
}

function completeStagedTarget(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  target: WireTarget,
): BoardReturn {
  const staged = model.staged;
  if (staged == null) return [model, []];
  const nextModel: BoardModel = { ...model, staged: null };
  if (staged.action.has_x) {
    const xPrompt = ensureXPrompt(fold, staged.action, target, [], staged.picks);
    if (xPrompt != null) return [{ ...nextModel, xPrompt }, []];
  }
  return [nextModel, boardIntentSubmit(tableId, takeAction(fold, staged.action, target, 0, [], staged.picks))];
}

/** Toggle a player seat into/out of the multi-aim card-pick draft (proliferate, CR 701.27). */
function togglePendingPlayerAimPick(model: BoardModel, fold: GameFoldState, seat: number): BoardReturn {
  const synced = syncPromptDraft(model, fold);
  const draft =
    synced.promptDraft?.kind === "card-pick"
      ? synced.promptDraft
      : { kind: "card-pick" as const, picked: [], filter: "" };
  const players = draft.players ?? [];
  const next = players.includes(seat) ? players.filter((p) => p !== seat) : [...players, seat];
  return [{ ...synced, promptDraft: { ...draft, players: next } }, []];
}

/** Toggle an object id into/out of the multi-aim card-pick draft (no submit). */
function togglePendingObjectAimPick(
  model: BoardModel,
  fold: GameFoldState,
  pc: NonNullable<GameFoldState["state"]>["pending_choice"],
  objectId: number,
): BoardReturn {
  if (pc == null) return [model, []];
  const synced = syncPromptDraft(model, fold);
  if (synced.promptDraft?.kind !== "card-pick") {
    return [{ ...synced, promptDraft: { kind: "card-pick", picked: [objectId], filter: "" } }, []];
  }
  const picked = synced.promptDraft.picked;
  const max =
    pc.kind === "choose_target"
      ? pc.max
      : pc.kind === "shuffle_from_graveyard"
        ? pc.max
        : pc.kind === "choose_activation_cost_targets" || pc.kind === "pay_cumulative_upkeep_or_sacrifice"
          ? pc.count
          : (cardPickRequiredCount(pc) ?? undefined);
  let next: number[];
  if (picked.includes(objectId)) {
    next = picked.filter((id) => id !== objectId);
  } else if (max != null && picked.length >= max) {
    return [synced, []];
  } else {
    next = [...picked, objectId];
  }
  return [
    {
      ...synced,
      promptDraft: {
        ...synced.promptDraft,
        picked: next,
      },
    },
    [],
  ];
}

function submitPendingHandPick(
  model: BoardModel,
  fold: GameFoldState,
  _tableId: string | null,
  pc: NonNullable<NonNullable<GameFoldState["state"]>["pending_choice"]>,
  objectId: number,
): BoardReturn {
  const idle = { ...model, handDrag: null, hoverActionId: null };
  return togglePendingObjectAimPick(idle, fold, pc, objectId);
}

/** True when the flight id already names the resting destination object (post-rebind). */
function authorityOwnsFlightDestination(fold: BoardFold | null, flight: CardFlight): boolean {
  const state = fold?.state;
  if (state == null) return false;

  if (flight.kind === "stack") {
    return state.stack.some((entry) => entry.source === flight.id);
  }

  if (flight.kind === "battlefield") {
    return state.objects.some((object) => object.id === flight.id && object.zone === ZONE.Battlefield);
  }

  return false;
}

function applyFlightsSynced(
  model: BoardModel,
  flightsIn: readonly CardFlight[],
  exitFxIn: readonly ExitFx[],
  now: number,
  fold: BoardFold | null,
): BoardModel {
  const flights = new Map<number, CardFlight>();
  const exitFx = new Map<number, ExitFx>(exitFxIn.map((fx) => [fx.id, fx]));
  const handHidden = new Set(model.handHidden);
  const retainedSourceIds = new Set<number>();

  for (const flight of flightsIn) {
    if (flight.fromCardId != null) retainedSourceIds.add(flight.fromCardId);

    // Settled + hold with authority already at the destination: hand off now. Keeping the
    // screen-space flight would hide the resting face and track the camera until the next
    // provenance sync.
    if (flight.phase === "settled" && flight.hold === true && authorityOwnsFlightDestination(fold, flight)) {
      traceFlightSync({
        op: "synced-drop",
        zone: flight.kind,
        id: flight.id,
        hold: true,
        phase: flight.phase,
        remainingPx: Math.hypot(flight.targetX - flight.x, flight.targetY - flight.y),
        note: "authority-owns-destination",
      });
      if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
      continue;
    }

    // Keep held seeds after they park at the aim pose so stack/land sync can rebind them
    // instead of spawning a second flight from the avatar.
    if (flightOwnsId(flight)) {
      traceFlightSync({
        op: "synced-keep",
        zone: flight.kind,
        id: flight.id,
        hold: flight.hold === true,
        phase: flight.phase,
        remainingPx: Math.hypot(flight.targetX - flight.x, flight.targetY - flight.y),
        toTarget: { x: flight.targetX, y: flight.targetY, scale: flight.targetScale },
      });
      flights.set(flight.id, flight);
      if (flight.fromCardId != null) handHidden.add(flight.fromCardId);
      continue;
    }

    traceFlightSync({
      op: "synced-drop",
      zone: flight.kind,
      id: flight.id,
      hold: flight.hold === true,
      phase: flight.phase,
      remainingPx: Math.hypot(flight.targetX - flight.x, flight.targetY - flight.y),
      note: "unowned-settled",
    });
    if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
  }

  for (const previousFlight of model.flights.values()) {
    if (previousFlight.fromCardId == null) continue;
    if (retainedSourceIds.has(previousFlight.fromCardId)) continue;
    handHidden.delete(previousFlight.fromCardId);
  }

  return {
    ...model,
    exitFx,
    flights,
    handHidden,
    hideCardIds: hiddenCardIds(flights, exitFx),
    lastFlightFrame: flights.size === 0 && exitFx.size === 0 ? null : now,
    ownedIds: new Set(flights.keys()),
  };
}

type Vec = { x: number; y: number };
export type OutMessage = Message | GameMessage;
type BoardCmd = FoldkitCommand.Command<OutMessage, never, RpcClient>;
type BoardReturn = readonly [BoardModel, ReadonlyArray<BoardCmd>];

function undecidedMulliganInspectLock(state: VisibleState | null | undefined): boolean {
  if (state == null) return false;
  const chrome = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  });
  return chrome.show && chrome.showControls;
}

function clearInspectState(model: BoardModel): BoardModel {
  if (model.inspectPin == null && model.inspectCard === undefined && !model.altDown) return model;
  return { ...model, altDown: false, inspectPin: null, inspectCard: undefined };
}

/** DOM overlay hover for Alt-inspect — hand preferred over stack (Solid `setAuxHover`). */
export type InspectAuxCard = {
  name: string;
  cardId?: string;
  print?: string;
};

function applyInspectPin(model: BoardModel, pin: InspectPin | null): BoardReturn {
  if (pin == null) return [model, []];
  const changed = inspectPinChanged(model.inspectPin, pin);
  if (!changed) return [model, []];
  const cmds: BoardCmd[] = pin.cardId ? [FetchInspectCard({ cardId: pin.cardId }) as unknown as BoardCmd] : [];
  return [
    {
      ...model,
      inspectPin: pin,
      inspectCard: undefined,
      inspectFace: "front",
    },
    cmds,
  ];
}

function applyLiveInspectPin(model: BoardModel, fold: GameFoldState): BoardReturn {
  if (undecidedMulliganInspectLock(fold.state)) return [clearInspectState(model), []];
  if (!model.altDown) return [model, []];
  return applyInspectPin(model, tryPinInspect(model, fold));
}

/** True when the cursor is still over the hand fan (including raised faces above the bar). */
function cursorInHandInspectBand(model: BoardModel): boolean {
  return model.cursor.y >= model.viewport.height - handMetrics(model.viewport).stickyBand;
}

/**
 * Release the latched hand hover once the pointer is verifiably back over the canvas outside the
 * hand sticky band. Only call this from BoardPointerMove, where the cursor is fresh: over the
 * HTML hand bar no board move fires, so the cursor is stale and the band check cannot be trusted
 * (the leave side of the aux storm keeps the latch unconditionally — see InspectAuxHovered).
 */
function releaseStickyHandInspect(model: BoardModel): BoardModel {
  if (model.handInspectHover == null) return model;
  if (cursorInHandInspectBand(model)) return model;
  return { ...model, handInspectHover: null };
}

/** Pin from hand/stack aux hover, else face-up card under cursor, else life-orb seat. */
function tryPinInspect(model: BoardModel, fold: GameFoldState): InspectPin | null {
  const aux = model.handInspectHover ?? model.stackInspectHover;
  if (aux != null) {
    return {
      name: aux.name,
      prepared: false,
      ...(aux.cardId ? { cardId: aux.cardId } : {}),
      ...(aux.print ? { print: aux.print } : {}),
    };
  }
  const hit = cardAt(fold, model, model.cursor.x, model.cursor.y);
  if (hit != null) {
    return pinFromCard(
      true,
      {
        name: hit.name,
        faceDown: hit.faceDown,
        prepared: hit.prepared,
        id: hit.id,
        zone: hit.zone,
        pile: hit.pile,
        cardId: hit.cardId || undefined,
        print: hit.print || undefined,
      },
      ZONE.Battlefield,
    );
  }
  const seat = avatarSeatAt(fold, model, model.cursor.x, model.cursor.y);
  const player = fold.state?.players.find((p) => p.player === seat) ?? null;
  return pinFromPlayer(true, seat, player);
}

function objectByAction(fold: GameFoldState, action: ActionView): ObjectView | null {
  if (action.object == null) return null;
  return fold.state?.objects.find((o) => o.id === action.object) ?? null;
}

function submitCmd(tableId: string | null, intent: WireIntent): BoardCmd[] {
  if (tableId == null) return [];
  return [SubmitIntent({ tableId, intent }) as unknown as BoardCmd];
}

function boardIntentSubmit(tableId: string | null, intent: WireIntent): BoardCmd[] {
  // SubmitIntent's Command emits game acknowledgements, which the parent app now wraps through
  // GotGameMessage before folding them back into `game.board.reject`.
  return submitCmd(tableId, intent);
}

function boardLogText(fold: GameFoldState): string {
  return fold.log.map((line) => line.text).join("\n");
}

function takeAction(
  fold: GameFoldState,
  action: ActionView,
  target: WireTarget | null,
  x: number,
  modes: WireModeChoice[],
  picks: CostPicks,
): WireIntent {
  return buildTakeActionIntent(fold.state?.viewer ?? 0, action.id, target, x, modes, picks);
}

function ensureXPrompt(
  fold: GameFoldState,
  action: ActionView,
  target: WireTarget | null,
  modes: WireModeChoice[],
  picks: CostPicks,
): XPromptState | null {
  if (!action.has_x) return null;
  const card = objectByAction(fold, action);
  const xCost: WireCost =
    action.x_cost ?? ({ generic: 0, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 1 } as WireCost);
  const minX = action.min_x ?? 0;
  const maxX = action.max_x ?? 0;
  return {
    action,
    target,
    picks,
    modes,
    name: action.kind === "cast_prepared" ? formatMessage(action.label) : (card?.name ?? formatMessage(action.label)),
    minX,
    maxX,
    draftX: clampX(maxX, minX, maxX),
    xCost,
  };
}

/** Provisional battlefield aim: seat lands-row center so land seeds keep moving until authority. */
function provisionalLandAim(
  model: BoardModel,
  fold: BoardFold,
  controller: number,
): { x: number; y: number; scale: number } {
  const playerCount = Math.max(1, fold.state?.players.length ?? 1);
  const viewer = fold.state?.viewer ?? 0;
  const world = landRowCenter(controller, viewer, playerCount);
  const screen = worldToScreen(model.camera, world.x, world.y);
  return { x: screen.x, y: screen.y, scale: 1 };
}

/** Solid `spawnFromHand` / `seedDrop`: hide the bar tile immediately and fly from the drop point. */
function seedDropFromHand(
  model: BoardModel,
  card: ObjectView,
  screenOrigin: Vec,
  kind: "battlefield" | "stack",
  stackCount = 0,
  fold: BoardFold | null = null,
): BoardModel {
  const flights = new Map(model.flights);
  const handHidden = new Set(model.handHidden);
  const aim =
    kind === "stack"
      ? stackFlightAim(model, { count: Math.max(1, stackCount + 1), row: stackCount })
      : fold != null
        ? provisionalLandAim(model, fold, card.controller)
        : { x: screenOrigin.x, y: screenOrigin.y, scale: 1 };

  // Play-mode / cost pipelines may seed before the final runAction cast. Replacing that flight
  // from screenOrigin restarts the glide and reads as a second animation.
  const existing =
    flights.get(card.id) ??
    [...flights.values()].find((flight) => flight.fromCardId === card.id || flight.id === card.id);
  if (existing != null) {
    if (existing.id !== card.id) flights.delete(existing.id);
    const continued = {
      ...existing,
      id: card.id,
      print: card.print ?? existing.print,
      name: card.name,
      targetX: aim.x,
      targetY: aim.y,
      targetScale: aim.scale,
      kind,
      fromCardId: card.id,
      hold: true,
      phase: "flying" as const,
    };
    traceFlightSync({
      op: "seed",
      zone: kind === "stack" ? "stack" : "land",
      id: card.id,
      hold: true,
      phase: existing.phase,
      remainingPx: Math.hypot(aim.x - existing.x, aim.y - existing.y),
      aimDeltaPx: Math.hypot(aim.x - existing.targetX, aim.y - existing.targetY),
      aimDeltaScale: Math.abs(aim.scale - existing.targetScale),
      fromTarget: { x: existing.targetX, y: existing.targetY, scale: existing.targetScale },
      toTarget: { x: aim.x, y: aim.y, scale: aim.scale },
      note: `seedDrop continue kind=${kind}`,
    });
    flights.set(card.id, continued);
    handHidden.add(card.id);
    return {
      ...model,
      flights,
      handHidden,
      hideCardIds: hiddenCardIds(flights, model.exitFx),
      ownedIds: new Set(flights.keys()),
    };
  }

  const startScale = handFlightScale(model.camera.zoom, handMetrics(model.viewport).cardW);
  const seeded = spawnFlight({
    id: card.id,
    print: card.print ?? "",
    name: card.name,
    x: screenOrigin.x,
    y: screenOrigin.y,
    scale: startScale,
    targetX: aim.x,
    targetY: aim.y,
    targetScale: aim.scale,
    kind,
    fromCardId: card.id,
    hold: true,
  });
  traceFlightSync({
    op: "seed",
    zone: kind === "stack" ? "stack" : "land",
    id: card.id,
    hold: true,
    phase: "flying",
    remainingPx: Math.hypot(aim.x - screenOrigin.x, aim.y - screenOrigin.y),
    toTarget: { x: aim.x, y: aim.y, scale: aim.scale },
    note: `seedDrop kind=${kind}`,
  });
  flights.set(card.id, seeded);
  handHidden.add(card.id);
  return {
    ...model,
    flights,
    handHidden,
    hideCardIds: hiddenCardIds(flights, model.exitFx),
    ownedIds: new Set(flights.keys()),
  };
}

/** Solid `clearPlayOrigin`: drop a seeded flight so cancel doesn't race the return animation. */
function clearPlayOrigin(model: BoardModel, cardId: number): BoardModel {
  const flights = new Map(model.flights);
  for (const [id, flight] of model.flights) {
    if (id === cardId || flight.fromCardId === cardId) flights.delete(id);
  }
  const handHidden = new Set(model.handHidden);
  handHidden.delete(cardId);
  return {
    ...model,
    flights,
    handHidden,
    hideCardIds: hiddenCardIds(flights, model.exitFx),
    ownedIds: new Set(flights.keys()),
  };
}

/**
 * Drop optimistic seeds when the intent that would have justified them was rejected. A rejected play
 * never produces provenance and a reject does not advance `fold.seq`, so nothing else re-examines a
 * parked hold — it would stay painted (and its hand tile hidden) for the rest of the game.
 */
export function dropHeldSeeds(model: BoardModel): BoardModel {
  const flights = new Map(model.flights);
  const handHidden = new Set(model.handHidden);
  for (const [id, flight] of model.flights) {
    if (flight.hold !== true) continue;
    flights.delete(id);
    handHidden.delete(flight.fromCardId ?? id);
  }
  if (flights.size === model.flights.size) return model;
  return {
    ...model,
    flights,
    handHidden,
    hideCardIds: hiddenCardIds(flights, model.exitFx),
    ownedIds: new Set(flights.keys()),
  };
}

function runAction(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  action: ActionView,
  card: ObjectView | null,
  picks: CostPicks,
  dropSeed: Vec,
  screenOrigin: Vec,
): BoardReturn {
  const plan = planRunAction(action, card, picks, fold.state);
  if (plan.kind === "noop") return [model, []];
  if (plan.kind === "reject") {
    return [{ ...model, reject: humanReason(plan.reason) }, []];
  }
  if (plan.kind === "stage") {
    // A permanent activating an ability doesn't change zones: only its ability goes on the stack.
    // Seeding a flight here would hide the resting battlefield art, and activated abilities emit no
    // stack-entrance provenance to hand the seed off to, so it would also leak.
    const seeded =
      plan.card.zone === ZONE.Battlefield
        ? model
        : seedDropFromHand(model, plan.card, screenOrigin, "stack", fold.state?.stack.length ?? 0, fold);
    return [
      {
        ...seeded,
        staged: {
          card: plan.card,
          action: plan.action,
          picks: plan.picks,
          preferPick: usedCostPick(plan.picks),
          playOrigin: dropSeed,
          playOriginScreen: screenOrigin,
        },
      },
      [],
    ];
  }
  if (plan.kind === "play-land") {
    const seeded = card != null ? seedDropFromHand(model, card, screenOrigin, "battlefield", 0, fold) : model;
    return [seeded, boardIntentSubmit(tableId, takeAction(fold, action, null, 0, [], plan.picks))];
  }
  if (plan.kind === "cast") {
    const seeded =
      card != null ? seedDropFromHand(model, card, screenOrigin, "stack", fold.state?.stack.length ?? 0, fold) : model;
    const xPrompt = ensureXPrompt(fold, plan.action, null, [], plan.picks);
    if (xPrompt != null) return [{ ...seeded, xPrompt }, []];
    return [seeded, boardIntentSubmit(tableId, takeAction(fold, plan.action, null, 0, [], plan.picks))];
  }
  // take (activate / cycle)
  if (action.has_x) {
    const xPrompt = ensureXPrompt(fold, action, null, [], plan.picks);
    if (xPrompt != null) return [{ ...model, xPrompt }, []];
  }
  return [model, boardIntentSubmit(tableId, takeAction(fold, action, null, 0, [], plan.picks))];
}

function settleGyExilePick(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  pick: CostPickState,
  ids: ReadonlyArray<number>,
): BoardReturn {
  const picks: CostPicks = { ...pick.picks, graveyard_exile: [...ids], gy_exile_settled: true };
  return continueAfterCostPick(
    { ...model, gyExilePick: null, pileExpand: null },
    fold,
    tableId,
    pick.action,
    pick.card,
    picks,
    pick.dropSeed,
    pick.screenOrigin,
  );
}

/** Enter/Space confirm for multi gy-exile when min < max (exact counts auto-settle on last click). */
function tryConfirmGyExile(model: BoardModel, fold: GameFoldState, tableId: string | null): BoardReturn | null {
  const pick = model.gyExilePick;
  if (pick == null) return null;
  const min = pick.action.graveyard_exile_min ?? 0;
  const max = pick.action.graveyard_exile_max ?? 0;
  if (max <= 1 || min >= max) return null;
  const selected = pick.picks.graveyard_exile;
  if (selected.length < min || selected.length > max) return null;
  return settleGyExilePick(model, fold, tableId, pick, selected);
}

function settleDiscardCostPick(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  pick: CostPickState,
): BoardReturn {
  const selected = pick.picks.discard_cost;
  const picks: CostPicks = { ...pick.picks, discard_cost: selected, discard_settled: true };
  return continueAfterCostPick(
    { ...model, discardPick: null },
    fold,
    tableId,
    pick.action,
    pick.card,
    picks,
    pick.dropSeed,
    pick.screenOrigin,
  );
}

/** Enter/Space confirm for local discard cost when exactly one card is selected. */
function tryConfirmDiscardCost(model: BoardModel, fold: GameFoldState, tableId: string | null): BoardReturn | null {
  const pick = model.discardPick;
  if (pick == null) return null;
  const selected = pick.picks.discard_cost;
  if (selected.length !== 1) return [model, []];
  return settleDiscardCostPick(model, fold, tableId, pick);
}

function continueAfterCostPick(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  action: ActionView,
  card: ObjectView | null,
  picks: CostPicks,
  dropSeed: Vec,
  screenOrigin: Vec,
): BoardReturn {
  const plan = planCostPipeline(action, card, picks);
  if (plan.kind === "reject") return [{ ...model, reject: humanReason(plan.reason) }, []];
  if (plan.kind === "sacrifice-pick") {
    return [{ ...model, sacrificePick: { action, card, dropSeed, screenOrigin, picks } }, []];
  }
  if (plan.kind === "discard-pick") {
    return [{ ...model, discardPick: { action, card, dropSeed, screenOrigin, picks } }, []];
  }
  if (plan.kind === "gy-exile-pick") {
    const pile = fold.state != null ? gyExileCostPile(action.graveyard_exile_choices, fold.state) : null;
    return [
      {
        ...model,
        gyExilePick: { action, card, dropSeed, screenOrigin, picks },
        pileExpand: pile ?? model.pileExpand,
      },
      [],
    ];
  }
  if (plan.kind === "modal") {
    return [
      {
        ...model,
        modalCast: {
          action: plan.action,
          modes: plan.modes,
          chosen: null,
          answers: [],
          picks: plan.picks,
          modeDraft: [],
        },
      },
      [],
    ];
  }
  if (plan.kind === "run") {
    return runAction(model, fold, tableId, plan.action, plan.card, plan.picks, dropSeed, screenOrigin);
  }
  return [model, []];
}

export function drainPlayModeIfSingleton(model: BoardModel, fold: GameFoldState, tableId: string | null): BoardReturn {
  const pick = model.playModePick;
  if (pick == null || pick.modes.length !== 1) return [model, []];

  const chosen = pick.modes[0];
  if (chosen == null) return [model, []];

  const [next, commands] = continueAfterCostPick(
    { ...model, playModePick: null, reject: null },
    fold,
    tableId,
    chosen,
    pick.card,
    emptyCostPicks(),
    pick.dropSeed,
    pick.screenOrigin,
  );
  if (next.reject != null) return [clearPlayOrigin(next, pick.card.id), commands];
  return [next, commands];
}

function revealTimer(reveal: FirstPlayerReveal): BoardCmd[] {
  const next = reveal.steps[reveal.index + 1];
  if (next != null) return [RevealStepTimer({ ms: next.delayMs }) as unknown as BoardCmd];
  const hold = reveal.steps.length === 1 ? REVEAL_HOLD_REDUCED_MS : REVEAL_HOLD_MS;
  return [RevealHoldTimer({ ms: hold }) as unknown as BoardCmd];
}

const toCardNameComboboxMessage = (message: Combobox.Message): OutMessage => GotCardNameComboboxMessage({ message });
const toConcedeDialogMessage = (message: Dialog.Message): OutMessage => GotConcedeDialogMessage({ message });
const toResultDialogMessage = (message: Dialog.Message): OutMessage => GotResultDialogMessage({ message });

/** Dismisses the concede confirmation. */
function closeConcedeConfirm(model: BoardModel): BoardReturn {
  const [concedeDialog, commands] = Dialog.close(model.concedeDialog);
  return [{ ...model, concedeDialog }, Command.mapMessages(commands, toConcedeDialogMessage)];
}

/** CR 104 — raise the one-shot result overlay on the fold that ends the game for the viewer. */
export function raiseResultDialog(model: BoardModel, fold: BoardFold): BoardReturn {
  if (model.resultRaised) return [model, []];
  const state = fold.state;
  if (state == null || outcome(state.players, state.viewer).kind === "playing") return [model, []];

  const [resultDialog, commands] = Dialog.open(model.resultDialog);
  return [{ ...model, resultDialog, resultRaised: true }, Command.mapMessages(commands, toResultDialogMessage)];
}

/** CR 103.1 — arm the one-shot starting-player spotlight on the first mulligan fold. */
export function armFirstPlayerReveal(model: BoardModel, fold: BoardFold, tableId: string | null): BoardReturn {
  if (model.firstPlayerReveal != null) return [model, []];
  const state = fold.state;
  if (state == null || !state.mulliganing) return [model, []];
  if (tableId == null || revealSeen(tableId)) return [model, []];

  markRevealSeen(tableId);
  const count = Math.max(1, state.players.length);
  const slot = seatSlot(state.active_player, state.viewer, count);
  const reveal: FirstPlayerReveal = {
    winner: state.active_player,
    steps: spotlightSteps(slot, count, prefersReducedMotion()),
    index: 0,
  };
  return [{ ...model, firstPlayerReveal: reveal }, revealTimer(reveal)];
}

function hideHintOnHandUse(model: BoardModel): BoardModel {
  if (model.hintAutoHidden) return model;
  return { ...model, hintAutoHidden: true };
}

function radialHoverActionId(model: BoardModel, fold: GameFoldState, index: number | null): number | null {
  if (index == null) return null;
  const state = fold.state;
  if (state == null) return null;
  const options = selectedRadialOptions(model, state);
  const opt = options[index];
  if (opt == null) return null;
  if (opt.kind === "tap_for_mana") return null;
  return opt.action.id;
}

function handActivated(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  action: ActionView,
  x: number,
  y: number,
): BoardReturn {
  const state = fold.state;
  const pc = state?.pending_choice ?? null;
  if (state != null && pc != null && action.object != null) {
    const handIds = pendingHandPickIds(pc, state);
    const objectId = action.object;
    if (handIds != null) {
      if (!handIds.has(objectId)) {
        return [{ ...model, handDrag: null, hoverActionId: null }, []];
      }
      return submitPendingHandPick(model, fold, tableId, pc, objectId);
    }
  }
  if (model.discardPick != null) {
    const choices = model.discardPick.action.discard_choices ?? [];
    const objectId = action.object;
    if (objectId == null || !choices.includes(objectId)) {
      return [{ ...model, handDrag: null, hoverActionId: null }, []];
    }
    const current = model.discardPick.picks.discard_cost;
    const next = current.includes(objectId)
      ? current.filter((id) => id !== objectId)
      : current.length >= 1
        ? current
        : [...current, objectId];
    return [
      {
        ...model,
        handDrag: null,
        hoverActionId: null,
        discardPick: {
          ...model.discardPick,
          picks: { ...model.discardPick.picks, discard_cost: next, discard_settled: false },
        },
      },
      [],
    ];
  }
  const bar = handMetrics(model.viewport);
  const threshold = model.viewport.height - bar.barH + bar.playSlack;
  const objectId = action.object;
  const modes =
    action.section === "hand" && objectId != null ? modesForObject(state?.actions ?? [], objectId) : [action];
  const playPlan = planHandPlay(modes, y, threshold);
  if (playPlan.kind === "ignore") return [model, []];
  const withHint = hideHintOnHandUse(model);
  const world = screenToWorld(withHint.camera, x, y);
  const dropSeed: Vec = { x: world.x - CARD_W / 2, y: world.y - CARD_H / 2 };
  const screenOrigin: Vec = { x, y };
  if (playPlan.kind === "choose") {
    const firstMode = playPlan.modes[0];
    if (firstMode == null) return [{ ...withHint, reject: humanReason("UnknownObject") }, []];
    const card = objectByAction(fold, firstMode) ?? objectByAction(fold, action);
    if (card == null) return [{ ...withHint, reject: humanReason("UnknownObject") }, []];
    const seeded = seedDropFromHand(
      clearActionSessionsForPlayMode(withHint),
      card,
      screenOrigin,
      "stack",
      fold.state?.stack.length ?? 0,
    );
    return [
      {
        ...seeded,
        playModePick: { card, modes: playPlan.modes, dropSeed, screenOrigin },
      },
      [],
    ];
  }
  const card = objectByAction(fold, playPlan.action);
  const plan = planHandDrop(playPlan.action, card, y, threshold);
  if (plan.kind === "ignore") return [model, []];
  if (plan.kind === "reject") return [{ ...withHint, reject: humanReason(plan.reason) }, []];
  if (plan.kind === "sacrifice-pick") {
    return [
      {
        ...withHint,
        sacrificePick: { action: plan.action, card: plan.card, dropSeed, screenOrigin, picks: plan.picks },
      },
      [],
    ];
  }
  if (plan.kind === "discard-pick") {
    return [
      { ...withHint, discardPick: { action: plan.action, card: plan.card, dropSeed, screenOrigin, picks: plan.picks } },
      [],
    ];
  }
  if (plan.kind === "gy-exile-pick") {
    const pile = state != null ? gyExileCostPile(plan.action.graveyard_exile_choices, state) : null;
    return [
      {
        ...withHint,
        gyExilePick: { action: plan.action, card: plan.card, dropSeed, screenOrigin, picks: plan.picks },
        pileExpand: pile ?? withHint.pileExpand,
      },
      [],
    ];
  }
  if (plan.kind === "modal") {
    return [
      {
        ...withHint,
        modalCast: {
          action: plan.action,
          modes: plan.modes,
          chosen: null,
          answers: [],
          picks: plan.picks,
          modeDraft: [],
        },
      },
      [],
    ];
  }
  if (plan.kind === "run") {
    return runAction(withHint, fold, tableId, plan.action, plan.card, plan.picks, dropSeed, screenOrigin);
  }
  return [withHint, []];
}

function cancelAll(model: BoardModel): BoardModel {
  // Cancel closes every action session below, so no held seed outlives it: an X prompt / modal /
  // sacrifice / discard / gy-exile seed left behind gets no session, no provenance and no clock
  // tick, and stays painted over a hidden hand tile for the rest of the game. Dropping all held
  // seeds (rather than the ids of the sessions we happen to remember) keeps that true when a new
  // session kind is added. An already-submitted seed re-flies from provenance when its delta lands.
  return {
    ...dropHeldSeeds(model),
    staged: null,
    playModePick: null,
    xPrompt: null,
    modalCast: null,
    sacrificePick: null,
    discardPick: null,
    gyExilePick: null,
    reject: null,
    selectedId: null,
    radialPress: { armed: null },
    radialHover: null,
    pileExpand: null,
    stackExpand: false,
    pendingChoiceKey: null,
    promptDraft: null,
    promptSubmitInFlight: false,
    promptSubmitSeq: null,
    cardNameSuggestions: null,
    cardNameCombobox: Combobox.init({ id: CARD_NAME_COMBOBOX_ID }),
    promptOptionFilter: "",
    orderPickPos: null,
    handDrag: null,
    hoverActionId: null,
  };
}

function clearActionSessionsForPlayMode(model: BoardModel): BoardModel {
  const ids = [
    model.staged?.card.id,
    model.xPrompt?.action.object,
    model.modalCast?.action.object,
    model.sacrificePick?.card?.id,
    model.discardPick?.card?.id,
    model.gyExilePick?.card?.id,
  ];
  let cleared = model;
  for (const id of ids) {
    if (id != null) cleared = clearPlayOrigin(cleared, id);
  }
  return {
    ...cleared,
    staged: null,
    xPrompt: null,
    modalCast: null,
    sacrificePick: null,
    discardPick: null,
    gyExilePick: null,
    pileExpand: null,
  };
}

function clearRadial(model: BoardModel): BoardModel {
  return { ...model, selectedId: null, radialPress: { armed: null }, radialHover: null };
}

function commitRadialIndex(model: BoardModel, fold: GameFoldState, tableId: string | null, index: number): BoardReturn {
  const id = model.selectedId;
  if (id == null || fold.state == null) return [clearRadial(model), []];
  const options = selectedRadialOptions(model, fold.state);
  const opt = options[index];
  if (opt == null) return [clearRadial(model), []];
  if (opt.disabled) return [model, []];
  const cleared = clearRadial(model);
  if (opt.kind === "tap_for_mana") {
    return [cleared, boardIntentSubmit(tableId, { kind: "tap_for_mana", player: fold.state.viewer, object: id })];
  }
  // On a cluster the selection is the face, but the offered action may belong to another copy.
  const card = fold.state.objects.find((o) => o.id === (opt.action.object ?? id)) ?? null;
  return continueAfterCostPick(
    cleared,
    fold,
    tableId,
    opt.action,
    card,
    emptyCostPicks(),
    { x: 0, y: 0 },
    { x: model.viewport.width / 2, y: model.viewport.height / 2 },
  );
}

function primaryFor(
  fold: GameFoldState,
  model: BoardModel,
): {
  kind: "pass" | "confirm-attackers" | "confirm-blockers";
  label: string;
} {
  const state = fold.state;
  if (state == null) return { kind: "pass", label: "Next" };
  const attackers = stagedAttackersForDisplay(
    model.combatAttackers,
    state.actions?.find((a) => a.kind === "declare_attackers")?.required_attacks ?? [],
    model.attackersConfirmed || state.combat.attackers_declared,
  );
  return primaryActionFor({
    step: state.step,
    activePlayer: state.active_player,
    me: state.viewer,
    actions: state.actions,
    attackers,
    blocks: model.combatBlocks,
    attackersConfirmed: model.attackersConfirmed,
    blockersConfirmed: model.blockersConfirmed,
    attackersDeclared: state.combat.attackers_declared,
    blockersDeclared: state.combat.blockers_declared.includes(state.viewer),
  });
}

/** Submit a ready multi-aim or on-board damage-assign draft; null when nothing to submit. */
function trySubmitReadyPendingDraft(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
): BoardReturn | null {
  const state = fold.state;
  if (state == null) return null;
  if (model.promptSubmitInFlight) return null;
  const synced = syncPromptDraft(model, fold);
  const pc = state.pending_choice;
  if (
    pc != null &&
    pendingBoardTargetMode(pc, state) != null &&
    !pendingTargetOneClick(pc) &&
    synced.promptDraft?.kind === "card-pick" &&
    cardPickReady(pc, synced.promptDraft.picked)
  ) {
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [withPromptSubmitInFlight(synced, fold.seq), boardIntentSubmit(tableId, choiceIntent(pc, answer))];
    }
  }
  if (
    (pc?.kind === "assign_combat_damage" || pc?.kind === "divide_counters") &&
    pendingDamageAssignBlockers(pc, state) != null &&
    synced.promptDraft != null &&
    damageAssignReady(pc, synced.promptDraft, state)
  ) {
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [withPromptSubmitInFlight(synced, fold.seq), boardIntentSubmit(tableId, choiceIntent(pc, answer))];
    }
  }
  if (
    pc?.kind === "divide_spell_damage" &&
    pendingDivideSpellObjectIndexes(pc, state) != null &&
    synced.promptDraft?.kind === "divide"
  ) {
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [withPromptSubmitInFlight(synced, fold.seq), boardIntentSubmit(tableId, choiceIntent(pc, answer))];
    }
  }
  if (
    pc?.kind === "choose_target_players" &&
    pendingPlayerAimSeats(pc, state) != null &&
    !pendingPlayerAimOneClick(pc) &&
    synced.promptDraft?.kind === "player-pick"
  ) {
    const count = synced.promptDraft.players.length;
    if (count >= pc.min && count <= pc.max) {
      const answer = buildAnswerFromDraft(pc, synced.promptDraft);
      if (answer != null) {
        return [withPromptSubmitInFlight(synced, fold.seq), boardIntentSubmit(tableId, choiceIntent(pc, answer))];
      }
    }
  }
  if (
    pc != null &&
    synced.promptDraft != null &&
    (pc.kind === "order_triggers" ||
      pc.kind === "scry" ||
      pc.kind === "surveil" ||
      pc.kind === "reorder_top" ||
      pc.kind === "select_from_top" ||
      pc.kind === "distribute_top" ||
      pc.kind === "partition_revealed")
  ) {
    if (synced.promptDraft.kind === "card-pick" && !cardPickReady(pc, synced.promptDraft.picked)) {
      return null;
    }
    if (
      synced.promptDraft.kind === "partition" &&
      (pc.kind === "partition_revealed" || pc.kind === "distribute_top") &&
      !partitionReady(pc, synced.promptDraft)
    ) {
      return null;
    }
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [withPromptSubmitInFlight(synced, fold.seq), boardIntentSubmit(tableId, choiceIntent(pc, answer))];
    }
  }
  if (
    pc != null &&
    pendingHandPickIds(pc, state) != null &&
    !pendingHandPickOneClick(pc) &&
    synced.promptDraft?.kind === "card-pick" &&
    cardPickReady(pc, synced.promptDraft.picked)
  ) {
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [
        withPromptSubmitInFlight(synced, fold.seq, { pileExpand: null }),
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
    }
  }
  if (
    pc != null &&
    pendingGraveyardPickIds(pc, state) != null &&
    !pendingGraveyardPickOneClick(pc) &&
    synced.promptDraft?.kind === "card-pick" &&
    cardPickReady(pc, synced.promptDraft.picked)
  ) {
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [
        withPromptSubmitInFlight(synced, fold.seq, { pileExpand: null }),
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
    }
  }
  if (
    pc != null &&
    pendingExilePickIds(pc, state) != null &&
    !pendingExilePickOneClick(pc) &&
    synced.promptDraft?.kind === "card-pick" &&
    cardPickReady(pc, synced.promptDraft.picked)
  ) {
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [
        withPromptSubmitInFlight(synced, fold.seq, { pileExpand: null }),
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
    }
  }
  return null;
}

function primaryClickModel(model: BoardModel, fold: GameFoldState, tableId: string | null): BoardReturn {
  const state = fold.state;
  if (state == null) return [model, []];
  const action = primaryFor(fold, model);
  const me = state.viewer;
  if (action.kind === "confirm-attackers") {
    // Submit the same merged list the button label uses (goad required_attacks), not bare
    // local staging — otherwise Attack (1) races an empty declare and latches confirmed.
    const attackers = stagedAttackersForDisplay(
      model.combatAttackers,
      state.actions?.find((a) => a.kind === "declare_attackers")?.required_attacks ?? [],
      model.attackersConfirmed || state.combat.attackers_declared,
    );
    const intent: WireIntent = { kind: "declare_attackers", player: me, attackers };
    return [{ ...model, combatAttackers: [], attackersConfirmed: true }, boardIntentSubmit(tableId, intent)];
  }
  if (action.kind === "confirm-blockers") {
    const intent: WireIntent = { kind: "declare_blockers", player: me, blocks: model.combatBlocks };
    return [{ ...model, combatBlocks: [], blockersConfirmed: true }, boardIntentSubmit(tableId, intent)];
  }
  return [model, boardIntentSubmit(tableId, { kind: "pass_priority", player: me })];
}

function combatDropModel(
  model: BoardModel,
  fold: GameFoldState,
  from: ObjectView | null,
  defenderSeat: number | null,
  blockAttackerId: number | null,
): BoardReturn {
  const state = fold.state;
  if (state == null || from == null) return [model, []];
  const mode = combatMode(state.actions, false, {
    attackersDeclared: model.attackersConfirmed || state.combat.attackers_declared,
    blockersDeclared: model.blockersConfirmed || state.combat.blockers_declared.includes(state.viewer),
  });
  const seats = declaresFor(state.actions, mode);
  const dropOn = blockAttackerId != null ? (state.objects.find((o) => o.id === blockAttackerId) ?? null) : null;
  // ObjectView.kind is a WireKind object; RenderCard.kind (what attackablePlaneswalker reads) is the
  // bare tag string — normalize so the planeswalker check is live, not a runtime type mismatch.
  const dropTarget = dropOn != null ? { ...dropOn, kind: dropOn.kind.kind } : null;
  const cardShape = {
    id: from.id,
    tapped: from.tapped,
    summoningSick: from.summoning_sick,
    hasHaste: from.has_haste,
  };
  // Opponents of the seat being declared for, not of the viewer — a moved declaration attacks on
  // someone else's behalf, and you may not send their creatures at their own planeswalker.
  const opponents = state.players.map((p) => p.player).filter((p) => !seats.includes(p));
  const result = handleCombatDrop(
    mode,
    model.combatAttackers,
    model.combatBlocks,
    cardShape as unknown as Parameters<typeof handleCombatDrop>[3],
    defenderSeat,
    dropTarget as unknown as Parameters<typeof handleCombatDrop>[5],
    state.combat.attackers,
    seats,
    opponents,
  );
  if (result.kind === "attackers") return [{ ...model, combatAttackers: result.value }, []];
  if (result.kind === "blockers") return [{ ...model, combatBlocks: result.value }, []];
  return [model, []];
}

/** Clear combat staging on real step transitions (not same-step SSE churn). */
export function syncCombatStaging(model: BoardModel, fold: Pick<GameFoldState, "state">): BoardModel {
  const step = fold.state?.step ?? null;
  if (step == null) return model;
  if (!combatStagingClearsOnStepChange(model.priorStep ?? undefined, step)) {
    if (model.priorStep === step) return model;
    return { ...model, priorStep: step };
  }
  return {
    ...model,
    combatAttackers: [],
    combatBlocks: [],
    attackersConfirmed: false,
    blockersConfirmed: false,
    priorStep: step,
  };
}

export function updateBoard(
  model: BoardModel,
  message: Message,
  fold: GameFoldState,
  tableId: string | null,
): BoardReturn {
  switch (message._tag) {
    case "ArtLoaded":
      return [model, []];
    case "BoardCameraZoomed":
      if (!Number.isFinite(message.factor) || message.factor <= 0) return [model, []];
      {
        const camera = zoomAt(model.camera, message.x, message.y, message.factor);
        return [
          {
            ...model,
            flights: remapFlightsForZoom(model.flights, model.camera.zoom, camera.zoom),
            camera,
            cameraUserMoved: true,
          },
          [],
        ];
      }
    case "BoardViewportResized": {
      if (!(message.width > 0) || !(message.height > 0)) return [model, []];
      const viewport = { width: message.width, height: message.height };
      const dpr = message.dpr > 0 ? Math.min(message.dpr, 3) : model.dpr;
      if (model.cameraUserMoved || model.cameraFitPlayers == null) return [{ ...model, viewport, dpr }, []];

      const fitted = fitCamera(
        { x: viewport.width, y: viewport.height },
        model.cameraFitPlayers,
        handMetrics(viewport).barH,
      );
      return [
        {
          ...model,
          viewport,
          dpr,
          flights: remapFlightsForZoom(model.flights, model.camera.zoom, fitted.zoom),
          camera: fitted,
        },
        [],
      ];
    }
    case "BoardPointerDown":
      return [pointerDownModel(model, fold, message.x, message.y), []];
    case "BoardPointerMove": {
      const moved = releaseStickyHandInspect(pointerMoveModel(model, message.x, message.y));
      return applyLiveInspectPin(moved, fold);
    }
    case "BoardPointerUp":
      return pointerUpModel(model, fold, tableId, message.x, message.y);
    case "FlightsSynced":
      return [applyFlightsSynced(model, message.flights, message.exitFx, message.now, fold), []];
    case "HandActionActivated": {
      const x = message.x ?? model.viewport.width / 2;
      const y = message.y ?? model.viewport.height / 2;
      return handActivated(
        { ...model, reject: null, handDrag: null, hoverActionId: null },
        fold,
        tableId,
        message.action,
        x,
        y,
      );
    }
    case "HandDragStarted":
      return [
        {
          ...hideHintOnHandUse(model),
          handDrag: {
            action: message.action,
            name: message.name,
            print: message.print,
            manaCost: message.manaCost,
            kind: message.kind,
            zone: message.zone,
            x: message.x,
            y: message.y,
          },
          hoverActionId: message.action.id,
          cursor: { x: message.x, y: message.y },
        },
        [],
      ];
    case "HandDragMoved":
      if (model.handDrag == null) return [model, []];
      return [
        {
          ...model,
          handDrag: { ...model.handDrag, x: message.x, y: message.y },
          cursor: { x: message.x, y: message.y },
        },
        [],
      ];
    case "HandDragEnded": {
      const drag = model.handDrag;
      if (drag == null) return [model, []];
      return handActivated(
        { ...model, handDrag: null, hoverActionId: null },
        fold,
        tableId,
        drag.action,
        message.x,
        message.y,
      );
    }
    case "HandActionHovered":
      return [{ ...model, hoverActionId: message.actionId }, []];
    case "PrimaryClicked":
      if (fold.state?.mulliganing) return [model, []];
      return primaryClickModel(model, fold, tableId);
    case "PassClicked": {
      if (fold.state == null) return [model, []];
      return [model, boardIntentSubmit(tableId, { kind: "pass_priority", player: fold.state.viewer })];
    }
    case "KeepHandClicked": {
      if (fold.state == null) return [model, []];
      return [model, boardIntentSubmit(tableId, { kind: "keep_hand", player: fold.state.viewer })];
    }
    case "MulliganClicked": {
      if (fold.state == null) return [model, []];
      if (!(fold.state.players.find((p) => p.player === fold.state?.viewer)?.can_mulligan ?? false)) {
        return [model, []];
      }
      return [model, boardIntentSubmit(tableId, { kind: "mulligan", player: fold.state.viewer })];
    }
    case "StackYieldArmed": {
      if (tableId == null) return [model, []];
      return [model, [SetYield({ tableId, enabled: true }) as unknown as BoardCmd]];
    }
    case "TurnYieldToggled": {
      if (tableId == null) return [model, []];
      return [model, [SetTurnYield({ tableId, enabled: message.enabled }) as unknown as BoardCmd]];
    }
    case "CancelActionClicked":
      return [cancelAll(model), []];
    case "PlayModeChosen": {
      const pick = model.playModePick;
      if (pick == null) return [model, []];
      const chosen = pick.modes.find((mode) => mode.id === message.actionId);
      if (chosen == null) return [{ ...clearPlayOrigin(model, pick.card.id), playModePick: null }, []];
      const [next, commands] = continueAfterCostPick(
        { ...model, playModePick: null, reject: null },
        fold,
        tableId,
        chosen,
        pick.card,
        emptyCostPicks(),
        pick.dropSeed,
        pick.screenOrigin,
      );
      if (next.reject != null) return [clearPlayOrigin(next, pick.card.id), commands];
      return [next, commands];
    }
    case "CommanderCastClicked": {
      const action = findCastActionForObject(fold.state?.actions, message.objectId);
      if (action == null) {
        return [{ ...model, reject: humanReason("NotCastable") }, []];
      }
      const card = fold.state?.objects.find((o) => o.id === message.objectId) ?? null;
      return runAction(
        { ...model, reject: null },
        fold,
        tableId,
        action,
        card,
        emptyCostPicks(),
        { x: 0, y: 0 },
        { x: model.viewport.width / 2, y: model.viewport.height / 2 },
      );
    }
    case "TargetChosen": {
      if (model.staged != null) {
        return completeStagedTarget(model, fold, tableId, message.target);
      }
      const pc = fold.state?.pending_choice ?? null;
      const state = fold.state;
      if (pc == null || state == null) return [model, []];
      const synced = syncPromptDraft(model, fold);
      const digHostAim = pendingDigCastHostMode(pc, state, synced.promptDraft);
      if (digHostAim != null) {
        if (message.target.kind !== "object" || !digHostAim.objects.has(message.target.id)) {
          return [synced, []];
        }
        if (synced.promptDraft?.kind !== "card-pick" || synced.promptDraft.picked.length !== 1) {
          return [synced, []];
        }
        const draft = { ...synced.promptDraft, host: message.target.id };
        const answer = buildAnswerFromDraft(pc, draft);
        if (answer == null) return [synced, []];
        return [
          { ...synced, promptDraft: null, pendingChoiceKey: null, pileExpand: null },
          boardIntentSubmit(tableId, choiceIntent(pc, answer)),
        ];
      }
      const pendingAim = pendingBoardTargetMode(pc, state);
      if (pendingAim == null) return [model, []];
      if (message.target.kind === "object" && !pendingAim.objects.has(message.target.id)) {
        return [model, []];
      }
      if (message.target.kind === "player" && !pendingAim.players.has(message.target.player)) {
        return [model, []];
      }
      if (!pendingTargetOneClick(pc)) {
        if (message.target.kind !== "object") return [model, []];
        return togglePendingObjectAimPick(model, fold, pc, message.target.id);
      }
      const answer = answerFromBoardTarget(pc, message.target);
      if (answer == null) return [model, []];
      return [model, boardIntentSubmit(tableId, choiceIntent(pc, answer))];
    }
    case "ModalModesChosen": {
      if (model.modalCast == null) return [model, []];
      const chosen = [...message.chosen];
      const step = advance(model.modalCast.modes, chosen, []);
      if (step.kind === "submit") {
        return [
          { ...model, modalCast: null },
          boardIntentSubmit(
            tableId,
            takeAction(fold, model.modalCast.action, null, 0, step.modes, model.modalCast.picks),
          ),
        ];
      }
      return [{ ...model, modalCast: { ...model.modalCast, chosen, answers: [] } }, []];
    }
    case "ModalTargetChosen": {
      const mc = model.modalCast;
      if (mc?.chosen == null) return [model, []];
      const step = advance(mc.modes, mc.chosen, mc.answers);
      if (step.kind !== "ask") return [model, []];
      const answers = [...mc.answers, { index: step.index, target: message.target }];
      const next = advance(mc.modes, mc.chosen, answers);
      if (next.kind === "submit") {
        return [
          { ...model, modalCast: null },
          boardIntentSubmit(tableId, takeAction(fold, mc.action, null, 0, next.modes, mc.picks)),
        ];
      }
      return [{ ...model, modalCast: { ...mc, answers } }, []];
    }
    case "XDraftSet": {
      if (model.xPrompt == null) return [model, []];
      const { minX, maxX } = model.xPrompt;
      return [
        {
          ...model,
          xPrompt: { ...model.xPrompt, draftX: clampX(message.x, minX, maxX) },
        },
        [],
      ];
    }
    case "XSubmitted": {
      if (model.xPrompt == null) return [model, []];
      const { action, target, picks, modes, minX, maxX } = model.xPrompt;
      const x = clampX(message.x, minX, maxX);
      return [
        { ...model, xPrompt: null },
        boardIntentSubmit(tableId, takeAction(fold, action, target, x, modes, picks)),
      ];
    }
    case "SacrificeChosen": {
      const pick = model.sacrificePick;
      if (pick == null) return [model, []];
      const settled = settleSacrificePick(pick, message.objectId);
      return continueAfterCostPick(
        { ...model, sacrificePick: null },
        fold,
        tableId,
        settled.action,
        settled.card,
        settled.picks,
        settled.dropSeed,
        settled.screenOrigin,
      );
    }
    case "DiscardChosen": {
      const pick = model.discardPick;
      if (pick != null) {
        const objectId = message.ids[0];
        if (objectId == null) return [model, []];
        const choices = pick.action.discard_choices ?? [];
        if (!choices.includes(objectId)) return [model, []];
        const state = fold.state;
        const handIds =
          state != null
            ? new Set(state.objects.filter((o) => o.zone === ZONE.Hand && o.owner === state.viewer).map((o) => o.id))
            : new Set<number>();
        const onHand = choices.length > 0 && choices.every((id) => handIds.has(id));
        if (!onHand) {
          const picks: CostPicks = { ...pick.picks, discard_cost: [objectId], discard_settled: true };
          return continueAfterCostPick(
            { ...model, discardPick: null },
            fold,
            tableId,
            pick.action,
            pick.card,
            picks,
            pick.dropSeed,
            pick.screenOrigin,
          );
        }
        const current = pick.picks.discard_cost;
        const next = current.includes(objectId)
          ? current.filter((id) => id !== objectId)
          : current.length >= 1
            ? current
            : [...current, objectId];
        return [
          {
            ...model,
            discardPick: {
              ...pick,
              picks: { ...pick.picks, discard_cost: next, discard_settled: false },
            },
          },
          [],
        ];
      }
      const state = fold.state;
      const pc = state?.pending_choice ?? null;
      const objectId = message.ids[0];
      if (state == null || pc == null || objectId == null) return [model, []];
      const handIds = pendingHandPickIds(pc, state);
      if (handIds == null || !handIds.has(objectId)) return [model, []];
      return submitPendingHandPick(model, fold, tableId, pc, objectId);
    }
    case "GyExileChosen": {
      const pick = model.gyExilePick;
      if (pick == null) return [model, []];
      const choices = pick.action.graveyard_exile_choices ?? [];
      const choiceSet = new Set(choices);
      const min = pick.action.graveyard_exile_min ?? 0;
      const max = pick.action.graveyard_exile_max ?? 0;
      const objectId = message.ids[0];
      if (objectId == null || !choiceSet.has(objectId)) return [model, []];
      if (max <= 1) {
        return settleGyExilePick(model, fold, tableId, pick, [objectId]);
      }
      const current = pick.picks.graveyard_exile;
      let next: number[];
      if (current.includes(objectId)) {
        next = current.filter((id) => id !== objectId);
      } else if (current.length >= max) {
        return [model, []];
      } else {
        next = [...current, objectId];
      }
      if (next.length === max && max === min && max > 0) {
        return settleGyExilePick(model, fold, tableId, pick, next);
      }
      return [
        {
          ...model,
          gyExilePick: { ...pick, picks: { ...pick.picks, graveyard_exile: next } },
        },
        [],
      ];
    }
    case "GyExileConfirmed": {
      const pick = model.gyExilePick;
      if (pick == null) return [model, []];
      const min = pick.action.graveyard_exile_min ?? 0;
      const max = pick.action.graveyard_exile_max ?? 0;
      const selected = pick.picks.graveyard_exile;
      if (selected.length < min || selected.length > max) return [model, []];
      return settleGyExilePick(model, fold, tableId, pick, selected);
    }
    case "DiscardCostConfirmed": {
      const pick = model.discardPick;
      if (pick == null) return [model, []];
      if (pick.picks.discard_cost.length !== 1) return [model, []];
      return settleDiscardCostPick(model, fold, tableId, pick);
    }
    case "CombatAttackerDropped": {
      const from = fold.state?.objects.find((o) => o.id === message.attackerId) ?? null;
      return combatDropModel(model, fold, from, message.defenderSeat, null);
    }
    case "CombatBlockerDropped": {
      const from = fold.state?.objects.find((o) => o.id === message.blockerId) ?? null;
      return combatDropModel(model, fold, from, null, message.attackerId);
    }
    case "CombatCancelAttacker": {
      const required = new Set(
        (fold.state?.actions?.find((a) => a.kind === "declare_attackers")?.required_attacks ?? []).map(
          (r) => r.attacker,
        ),
      );
      if (required.has(message.attackerId)) return [model, []];
      return [
        { ...model, combatAttackers: model.combatAttackers.filter((a) => a.attacker !== message.attackerId) },
        [],
      ];
    }
    case "CombatCancelBlocker":
      return [{ ...model, combatBlocks: model.combatBlocks.filter((b) => b.blocker !== message.blockerId) }, []];
    case "PromptCardToggled": {
      const synced = syncPromptDraft(model, fold);
      const pc = fold.state?.pending_choice;
      if (pc == null || synced.promptDraft == null || synced.promptSubmitInFlight) return [synced, []];

      if (synced.promptDraft.kind === "card-pick") {
        const required = cardPickRequiredCount(pc);
        const pickOne = required === 1;
        const max = pc.kind === "select_from_top" ? pc.up_to : (required ?? undefined);
        const picked = synced.promptDraft.picked;
        let next: number[];
        if (picked.includes(message.id)) {
          next = picked.filter((id) => id !== message.id);
        } else if (pickOne) {
          next = [message.id];
        } else if (max != null && picked.length >= max) {
          return [synced, []];
        } else {
          next = [...picked, message.id];
        }
        return [
          {
            ...synced,
            promptDraft: {
              ...synced.promptDraft,
              picked: next,
            },
          },
          [],
        ];
      }

      if (synced.promptDraft.kind === "player-pick") {
        if (pc.kind !== "choose_target_players" && pc.kind !== "choose_splitting_opponent") return [synced, []];
        const max = pc.kind === "choose_target_players" ? pc.max : 1;
        const players = synced.promptDraft.players;
        let next: number[];
        if (players.includes(message.id)) {
          next = players.filter((player) => player !== message.id);
        } else if (max === 1) {
          next = [message.id];
        } else if (players.length >= max) {
          return [synced, []];
        } else {
          next = [...players, message.id];
        }
        return [{ ...synced, promptDraft: { kind: "player-pick", players: next } }, []];
      }

      if (synced.promptDraft.kind === "partition" && pc.kind === "partition_revealed") {
        const pileA = synced.promptDraft.buckets.pile_a ?? [];
        const nextPileA = pileA.includes(message.id) ? pileA.filter((id) => id !== message.id) : [...pileA, message.id];
        return [{ ...synced, promptDraft: { kind: "partition", buckets: { pile_a: nextPileA } } }, []];
      }

      if (
        synced.promptDraft.kind === "partition" &&
        (pc.kind === "scry" || pc.kind === "surveil" || pc.kind === "reorder_top")
      ) {
        const top = synced.promptDraft.buckets.top ?? [];
        const bottom = synced.promptDraft.buckets.bottom ?? [];
        if (top.includes(message.id)) {
          return [
            {
              ...synced,
              promptDraft: {
                kind: "partition",
                buckets: {
                  top: top.filter((id) => id !== message.id),
                  bottom: [...bottom, message.id],
                },
              },
            },
            [],
          ];
        }
        if (bottom.includes(message.id)) {
          return [
            {
              ...synced,
              promptDraft: {
                kind: "partition",
                buckets: {
                  bottom: bottom.filter((id) => id !== message.id),
                  top: [...top, message.id],
                },
              },
            },
            [],
          ];
        }
        return [synced, []];
      }

      return [synced, []];
    }
    case "PromptOrderMoved": {
      const synced = syncPromptDraft(model, fold);
      if (synced.promptDraft?.kind !== "order") return [synced, []];
      const target = message.pos + message.delta;
      if (target < 0 || target >= synced.promptDraft.order.length) return [synced, []];
      const order = [...synced.promptDraft.order];
      [order[message.pos], order[target]] = [order[target], order[message.pos]];
      return [{ ...synced, promptDraft: { kind: "order", order }, orderPickPos: null }, []];
    }
    case "PromptOrderRowClicked": {
      const synced = syncPromptDraft(model, fold);
      if (synced.promptDraft?.kind !== "order") return [synced, []];
      const from = synced.orderPickPos;
      if (from == null) {
        return [{ ...synced, orderPickPos: message.pos }, []];
      }
      if (from === message.pos) {
        return [{ ...synced, orderPickPos: null }, []];
      }
      const order = [...synced.promptDraft.order];
      if (from < 0 || from >= order.length || message.pos < 0 || message.pos >= order.length) {
        return [{ ...synced, orderPickPos: null }, []];
      }
      const [item] = order.splice(from, 1);
      if (item === undefined) return [{ ...synced, orderPickPos: null }, []];
      order.splice(message.pos, 0, item);
      return [{ ...synced, promptDraft: { kind: "order", order }, orderPickPos: null }, []];
    }
    case "PromptOrderDragEnded":
      return [{ ...model, orderPickPos: null }, []];
    case "PromptDamageSet": {
      const synced = syncPromptDraft(model, fold);
      if (synced.promptDraft == null) return [synced, []];
      const pc = fold.state?.pending_choice;
      let amount = Math.max(0, Number.parseInt(String(message.amount), 10) || 0);
      if (pc?.kind === "assign_combat_damage") {
        const power = fold.state?.objects.find((o) => o.id === pc.source)?.power ?? amount;
        amount = clampX(amount, 0, power);
      } else if (pc?.kind === "divide_spell_damage" || pc?.kind === "divide_counters") {
        amount = clampX(amount, 0, pc.total);
      }
      if (synced.promptDraft.kind === "divide") {
        return [
          {
            ...synced,
            promptDraft: {
              kind: "divide",
              amounts: { ...synced.promptDraft.amounts, [message.id]: amount },
            },
          },
          [],
        ];
      }
      if (synced.promptDraft.kind !== "damage") return [synced, []];
      return [
        {
          ...synced,
          promptDraft: {
            kind: "damage",
            amounts: { ...synced.promptDraft.amounts, [message.id]: amount },
          },
        },
        [],
      ];
    }
    case "PromptStringSet": {
      const synced = syncPromptDraft(model, fold);
      if (synced.promptDraft?.kind !== "string") return [synced, []];
      const next = { ...synced, promptDraft: { kind: "string" as const, value: message.value } };
      const pc = fold.state?.pending_choice;
      if (pc?.kind !== "choose_card_name") {
        return [{ ...next, cardNameSuggestions: null }, []];
      }
      const q = message.value.trim();
      if (q.length < 2) {
        return [{ ...next, cardNameSuggestions: null }, []];
      }
      return [next, [SearchCardNames({ query: q }) as unknown as BoardCmd]];
    }
    // Open/close, arrow keys, active descendant, and blur are the Combobox's. The board only has
    // to keep the string draft — what the answer is built from — level with the input.
    case "GotCardNameComboboxMessage": {
      const [cardNameCombobox, commands, outMessage] = CardNameCombobox.update(model.cardNameCombobox, message.message);
      const lifted = Command.mapMessages(commands, toCardNameComboboxMessage);
      // A picked suggestion re-runs the catalog search for a name that is already exact; the
      // popup is closed by then, so the refreshed list is never seen.
      const typed = message.message._tag === "UpdatedInputValue" ? message.message.value : null;
      const picked = Option.isSome(outMessage) && outMessage.value._tag === "Selected" ? outMessage.value.value : null;
      const value = picked ?? typed;
      if (value == null) return [{ ...model, cardNameCombobox }, lifted];
      // Draft first, then seat the input: `PromptStringSet` resyncs the draft, and a resync onto a
      // different prompt resets the combobox — which would drop the keystroke that got us here.
      const [drafted, draftCommands] = updateBoard(model, PromptStringSet({ value }), fold, tableId);
      return [{ ...drafted, cardNameCombobox }, [...lifted, ...draftCommands]];
    }
    case "CardNameSuggestionsFetched": {
      const draft = model.promptDraft;
      if (draft?.kind !== "string") return [model, []];
      if (draft.value.trim() !== message.query.trim()) return [model, []];
      return [{ ...model, cardNameSuggestions: { query: message.query, names: message.names } }, []];
    }
    case "PromptCardFilterSet": {
      const synced = syncPromptDraft(model, fold);
      if (synced.promptDraft?.kind !== "card-pick") return [synced, []];
      return [
        {
          ...synced,
          promptDraft: { kind: "card-pick", picked: synced.promptDraft.picked, filter: message.query },
        },
        [],
      ];
    }
    case "PromptOptionFilterSet": {
      const synced = syncPromptDraft(model, fold);
      return [{ ...synced, promptOptionFilter: message.query }, []];
    }
    case "PromptNumberSet": {
      const synced = syncPromptDraft(model, fold);
      const pc = fold.state?.pending_choice;
      if (synced.promptDraft?.kind !== "number" || pc == null || !("max" in pc)) return [synced, []];
      const count = clampX(message.count, 0, pc.max);
      return [{ ...synced, promptDraft: { kind: "number", count } }, []];
    }
    case "PromptModeChoiceToggled": {
      const synced = syncPromptDraft(model, fold);
      const pc = fold.state?.pending_choice;
      if (pc?.kind !== "choose_trigger_modes" || synced.promptDraft?.kind !== "modes") return [synced, []];
      const choice: WireModeChoice =
        message.target == null ? { index: message.index } : { index: message.index, target: message.target };
      let modes = [...synced.promptDraft.modes];
      if (modes.some((existing) => samePromptModeChoice(existing, choice))) {
        modes = modes.filter((existing) => !samePromptModeChoice(existing, choice));
      } else if (modes.length >= pc.choose) {
        return [synced, []];
      } else {
        modes = [...modes, choice];
      }
      return [{ ...synced, promptDraft: { kind: "modes", modes } }, []];
    }
    case "PromptPartitionSet": {
      const synced = syncPromptDraft(model, fold);
      if (synced.promptDraft?.kind !== "partition" || synced.promptSubmitInFlight) return [synced, []];
      const buckets: Record<string, number[]> = {};
      let currentBucket: string | null = null;
      for (const [bucket, ids] of Object.entries(synced.promptDraft.buckets)) {
        if (ids.includes(message.id)) currentBucket = bucket;
        buckets[bucket] = ids.filter((id) => id !== message.id);
      }
      const nextBucket = currentBucket === message.bucket ? null : message.bucket;
      if (nextBucket != null) {
        const ids = buckets[nextBucket] ?? [];
        buckets[nextBucket] = [...ids, message.id];
      }
      return [{ ...synced, promptDraft: { kind: "partition", buckets } }, []];
    }
    case "PromptSubmitted": {
      const synced = syncPromptDraft(model, fold);
      const pc = fold.state?.pending_choice;
      const gameState = fold.state;
      if (pc == null || gameState == null || synced.promptDraft == null || synced.promptSubmitInFlight) {
        return [synced, []];
      }
      if (synced.promptDraft.kind === "card-pick" && !cardPickReady(pc, synced.promptDraft.picked)) {
        return [synced, []];
      }
      if (synced.promptDraft.kind === "damage" && !damageAssignReady(pc, synced.promptDraft, gameState)) {
        return [synced, []];
      }
      if (synced.promptDraft.kind === "divide" && buildAnswerFromDraft(pc, synced.promptDraft) == null) {
        return [synced, []];
      }
      if (synced.promptDraft.kind === "player-pick") {
        const count = synced.promptDraft.players.length;
        if (pc.kind === "choose_target_players" && (count < pc.min || count > pc.max)) {
          return [synced, []];
        }
        if (pc.kind === "choose_splitting_opponent" && count !== 1) {
          return [synced, []];
        }
      }
      if (
        synced.promptDraft.kind === "modes" &&
        pc.kind === "choose_trigger_modes" &&
        synced.promptDraft.modes.length !== pc.choose &&
        !(pc.optional && synced.promptDraft.modes.length === 0)
      ) {
        return [synced, []];
      }
      if (
        synced.promptDraft.kind === "partition" &&
        (pc.kind === "partition_revealed" || pc.kind === "distribute_top") &&
        !partitionReady(pc, synced.promptDraft)
      ) {
        return [synced, []];
      }
      const answer = buildAnswerFromDraft(pc, synced.promptDraft);
      if (answer == null) return [synced, []];
      return [withPromptSubmitInFlight(synced, fold.seq), boardIntentSubmit(tableId, choiceIntent(pc, answer))];
    }
    case "PromptDeclined": {
      const synced = syncPromptDraft(model, fold);
      const pc = fold.state?.pending_choice;
      if (pc == null || synced.promptSubmitInFlight) return [synced, []];
      const answer = declineAnswer(pc);
      if (answer == null) return [synced, []];
      return [withPromptSubmitInFlight(synced, fold.seq), boardIntentSubmit(tableId, choiceIntent(pc, answer))];
    }
    case "ModalModeToggled": {
      const mc = model.modalCast;
      if (mc == null || mc.chosen != null) return [model, []];
      const chooseMax = mc.action.modal?.choose_max ?? 1;
      let draft = [...mc.modeDraft];
      if (draft.includes(message.index)) {
        draft = draft.filter((i) => i !== message.index);
      } else if (draft.length >= chooseMax) {
        return [model, []];
      } else {
        draft = [...draft, message.index];
      }
      return [{ ...model, modalCast: { ...mc, modeDraft: draft } }, []];
    }
    case "PendingChoiceAnswered":
      return [model, boardIntentSubmit(tableId, message.intent)];
    case "StackDwellChanged": {
      if (tableId == null) return [model, []];
      return [model, [SetStackDwell({ tableId, dwelling: message.dwelling }) as unknown as BoardCmd]];
    }
    case "StackExpandClicked":
      return [{ ...model, stackExpand: true }, []];
    case "StackCollapseClicked":
      return [{ ...model, stackExpand: false }, []];
    case "LogExpandToggled":
      return [{ ...model, logExpanded: !model.logExpanded, logCopied: false, logCopyFailed: false }, []];
    case "LogCopyRequested": {
      const text = boardLogText(fold);
      if (text === "") return [{ ...model, logCopied: false, logCopyFailed: false }, []];
      return [{ ...model, logCopied: false, logCopyFailed: false }, [CopyBoardLog({ text }) as unknown as BoardCmd]];
    }
    case "LogCopyCompleted":
      return [{ ...model, logCopied: message.ok, logCopyFailed: !message.ok }, []];
    case "RadialWedgeArmed":
      return [{ ...model, radialPress: radialPressDown(model.radialPress, message.index) }, []];
    case "RadialWedgeHovered":
      return [
        {
          ...model,
          radialHover: message.index,
          hoverActionId: radialHoverActionId(model, fold, message.index),
        },
        [],
      ];
    case "RadialDismissed":
      return [clearRadial({ ...model, hoverActionId: null }), []];
    case "RadialOptionPicked":
      return commitRadialIndex(model, fold, tableId, message.index);
    case "RadialWedgeReleased": {
      const result = radialPressUp(model.radialPress, message.index);
      const next: BoardModel = { ...model, radialPress: result.state };
      if (result.dismiss) return [clearRadial(next), []];
      if (result.commit != null) return commitRadialIndex(next, fold, tableId, result.commit);
      return [next, []];
    }
    // ── Alt-pin inspect ─────────────────────────────────────────────────────
    case "AltDown": {
      if (undecidedMulliganInspectLock(fold.state)) return [clearInspectState(model), []];
      const withAlt = { ...model, altDown: true };
      return applyInspectPin(withAlt, tryPinInspect(withAlt, fold));
    }
    case "AltUp":
      return [{ ...model, altDown: false, inspectPin: null, inspectCard: undefined }, []];
    case "InspectAuxHovered": {
      if (message.source === "hand") {
        // A hand aux enter/leave is itself proof the pointer is over the hand bar: the bar is an
        // HTML overlay above the canvas, so BoardPointerMove never fires there and model.cursor
        // is stale — a band check against it would drop the latch mid enter/leave storm (raised
        // face art is pointer-events-none) and hover-then-Alt could never pin. Keep the latch on
        // leave; it releases on a canvas BoardPointerMove outside the band (fresh cursor, see
        // releaseStickyHandInspect) or when a new hand/stack aux enter replaces it.
        if (message.card == null && model.handInspectHover != null) {
          return [model, []];
        }
        return applyLiveInspectPin({ ...model, handInspectHover: message.card }, fold);
      }
      // A stack aux enter supersedes a latched hand hover — the cursor moved to the stack.
      if (message.card != null) {
        return applyLiveInspectPin({ ...model, handInspectHover: null, stackInspectHover: message.card }, fold);
      }
      return applyLiveInspectPin({ ...model, stackInspectHover: null }, fold);
    }
    case "InspectCardFetched":
      return [{ ...model, inspectCard: message.card }, []];
    case "InspectFlipFace":
      return [{ ...model, inspectFace: model.inspectFace === "front" ? "back" : "front" }, []];
    case "InspectDismissed":
      return [{ ...model, inspectPin: null, inspectCard: undefined, altDown: false }, []];
    // ── Pile overlay ─────────────────────────────────────────────────────────
    case "PileExpanded":
      return [{ ...model, pileExpand: { zone: message.zone, owner: message.owner } }, []];
    case "PileCardClicked": {
      const pick = model.gyExilePick;
      if (pick != null) {
        return updateBoard(model, GyExileChosen({ ids: [message.id] }), fold, tableId);
      }
      const state = fold.state;
      const pc = state?.pending_choice ?? null;
      if (state == null || pc == null) return [model, []];
      const pileIds = pendingGraveyardPickIds(pc, state) ?? pendingExilePickIds(pc, state);
      if (pileIds == null || !pileIds.has(message.id)) return [model, []];
      const oneClick = pendingGraveyardPickOneClick(pc) || pendingExilePickOneClick(pc);
      if (oneClick) {
        if (digCastNeedsHost(pc)) {
          return [
            {
              ...model,
              promptDraft: { kind: "card-pick", picked: [message.id], filter: "" },
              pileExpand: null,
            },
            [],
          ];
        }
        const answer = buildAnswerFromDraft(pc, { kind: "card-pick", picked: [message.id], filter: "" });
        if (answer == null) return [model, []];
        return [
          { ...model, promptDraft: null, pendingChoiceKey: null, pileExpand: null },
          boardIntentSubmit(tableId, choiceIntent(pc, answer)),
        ];
      }
      return togglePendingObjectAimPick(model, fold, pc, message.id);
    }
    case "PileOverlayClosed": {
      // Keep the pile open while pile-aim cost / pending GY or exile pick is live.
      if (model.gyExilePick != null && fold.state != null) {
        const pile = gyExileCostPile(model.gyExilePick.action.graveyard_exile_choices, fold.state);
        if (pile != null) return [{ ...model, pileExpand: pile }, []];
      }
      if (fold.state != null) {
        const pile = pendingPilePickPile(fold.state.pending_choice, fold.state);
        if (pile != null) return [{ ...model, pileExpand: pile }, []];
      }
      return [{ ...model, pileExpand: null }, []];
    }
    // ── Concede ───────────────────────────────────────────────────────────────
    case "ConcedeClicked": {
      const [concedeDialog, commands] = Dialog.open(model.concedeDialog);
      return [{ ...model, concedeDialog }, Command.mapMessages(commands, toConcedeDialogMessage)];
    }
    case "ConcedeConfirmed": {
      const [closed, closeCommands] = closeConcedeConfirm(model);
      if (fold.state == null) return [closed, closeCommands];
      return [
        closed,
        [...closeCommands, ...boardIntentSubmit(tableId, { kind: "concede", player: fold.state.viewer })],
      ];
    }
    // Escape, a backdrop click, and Cancel all close the dialog inside Dialog.update — cancelling a
    // concede leaves nothing else to undo, so its Closed out-message needs no handling here.
    case "GotConcedeDialogMessage": {
      const [concedeDialog, commands] = Dialog.update(model.concedeDialog, message.message);
      return [{ ...model, concedeDialog }, Command.mapMessages(commands, toConcedeDialogMessage)];
    }
    // ── Game result ───────────────────────────────────────────────────────────
    // "Stay on the board", Escape, and the backdrop all dismiss it the same way. `resultRaised`
    // already latched when it was raised, so a dismissed result stays dismissed.
    case "GotResultDialogMessage": {
      const [resultDialog, commands] = Dialog.update(model.resultDialog, message.message);
      return [{ ...model, resultDialog }, Command.mapMessages(commands, toResultDialogMessage)];
    }
    case "HintDismissed":
      persistHintDismissed();
      return [{ ...model, hintDismissed: true }, []];
    case "HintAutoHidden":
      return [{ ...model, hintAutoHidden: true }, []];
    case "SoundToggled": {
      const next = !model.soundOn;
      setSoundEnabled(next);
      if (next) {
        unlockTableAudio();
        playUnmuteTick();
      }
      return [{ ...model, soundOn: next }, []];
    }
    case "PriorityElapsed":
      return [{ ...model, priorityElapsed: message.seconds }, []];
    case "LegendToggled":
      return [{ ...model, legendOpen: !model.legendOpen }, []];
    case "LeaveGame":
      // Handled at app level (update.ts) — board model unchanged.
      return [model, []];
    // ── Global keyboard shortcuts ─────────────────────────────────────────────
    case "KeyboardSpacePressed": {
      // Opening mulligans own the chrome — don't pass/confirm via Space.
      if (fold.state?.mulliganing) return [model, []];
      const gyExile = tryConfirmGyExile(model, fold, tableId);
      if (gyExile != null) return gyExile;
      const discardCost = tryConfirmDiscardCost(model, fold, tableId);
      if (discardCost != null) return discardCost;
      const submitted = trySubmitReadyPendingDraft(model, fold, tableId);
      if (submitted != null) return submitted;
      return primaryClickModel(model, fold, tableId);
    }
    case "KeyboardEnterPressed": {
      const state = fold.state;
      if (state == null) return [model, []];
      if (state.mulliganing) return [model, []];
      const gyExile = tryConfirmGyExile(model, fold, tableId);
      if (gyExile != null) return gyExile;
      const discardCost = tryConfirmDiscardCost(model, fold, tableId);
      if (discardCost != null) return discardCost;
      const submitted = trySubmitReadyPendingDraft(model, fold, tableId);
      if (submitted != null) return submitted;
      const me = state.viewer;
      const active = state.active_player;
      // Enter toggles End Turn when it's the viewer's turn (and stack is empty), or
      // toggles Turn Yield when it's another player's turn.
      if (tableId == null) return [model, []];
      const enabled = !(state.turn_yielded ?? false);
      if (me === active && state.stack.length === 0) {
        // Arming End Turn only — cancelling "Ending turn…" stays available. Match the
        // priority-bar gate so Enter cannot arm through a forced goad declaration.
        const pendingAttackers = model.combatAttackers.length > 0 && !model.attackersConfirmed;
        if (enabled && !canArmEndTurn(state, pendingAttackers)) {
          return [model, []];
        }
        return [model, [SetTurnYield({ tableId, enabled }) as unknown as BoardCmd]];
      }
      if (me !== active) {
        return [model, [SetTurnYield({ tableId, enabled }) as unknown as BoardCmd]];
      }
      return [model, []];
    }
    case "KeyboardEscape": {
      // Dismiss inspect first, then radial, then stack expand, then cancel everything + close pile.
      if (model.inspectPin != null) {
        return [{ ...model, inspectPin: null, inspectCard: undefined, altDown: false }, []];
      }
      if (model.selectedId != null) {
        return [clearRadial(model), []];
      }
      if (model.stackExpand) {
        return [{ ...model, stackExpand: false }, []];
      }
      return [cancelAll(model), []];
    }
    case "FirstPlayerRevealStepped": {
      const reveal = model.firstPlayerReveal;
      if (reveal == null) return [model, []];
      const next = { ...reveal, index: Math.min(reveal.index + 1, reveal.steps.length - 1) };
      return [{ ...model, firstPlayerReveal: next }, revealTimer(next)];
    }
    case "FirstPlayerRevealFinished":
      return [{ ...model, firstPlayerReveal: null }, []];
    default: {
      const _exhaustive: never = message;
      return [model, []];
    }
  }
}
