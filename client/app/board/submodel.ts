import type { Command as FoldkitCommand } from "foldkit";
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
import type {
  ActionView,
  CatalogCard,
  ObjectView,
  WireAttack,
  WireBlock,
  WireCost,
  WireIntent,
  WireModeChoice,
  WireTarget,
} from "~/wire/types";
import { clampX } from "~/xCost";
import { type InspectPin, inspectPinChanged, pinFromCard, pinFromPlayer } from "../../lib/inspect";
import { humanReason } from "../../lib/reject";
import { isSoundEnabled, playUnmuteTick, setSoundEnabled, unlockTableAudio } from "../../lib/tableAudio";
import type { GameFoldState } from "../game/fold";
import {
  FetchInspectCard,
  SearchCardNames,
  SetStackDwell,
  SetTurnYield,
  SetYield,
  SubmitIntent,
} from "../game/intents";
import type { RpcClient } from "../resources";
import {
  buildTakeActionIntent,
  type CostPickState,
  type CostPicks,
  emptyCostPicks,
  findCastActionForObject,
  type ModalCast,
  planCostPipeline,
  planHandDrop,
  planRunAction,
  type StagedAction,
  settleSacrificePick,
  usedCostPick,
  type XPromptState,
} from "./action/execution";
import { advance } from "./action/modal";
import {
  pendingBoardTargetMode,
  pendingDamageAssignBlockers,
  pendingDivideSpellObjectIndexes,
  pendingHandPickIds,
  pendingHandPickOneClick,
  pendingPlayerAimOneClick,
  pendingPlayerAimSeats,
  pendingTargetOneClick,
  sacrificeCostObjectIds,
  stagedPickTargets,
} from "./action/targeting";
import type { Camera, Vec2 } from "./geometry/camera";
import { panBy, screenToWorld, worldToScreen } from "./geometry/camera";
import {
  combatStagingClearsOnStepChange,
  handleCombatDrop,
  stagedAttackersForDisplay,
} from "./geometry/combat-staging";
import { hitAvatar, hitTest } from "./geometry/hit-test";
import {
  canSelectPermanent,
  combatMode,
  fitCamera,
  type PointerPhase,
  pointerDown,
  pointerMove,
  pointerUp,
} from "./geometry/interaction";
import { avatarPos, CARD_H, CARD_W, layout, type RenderCard, STEP, ZONE } from "./geometry/layout";
import { type RadialPress, radialPressDown, radialPressUp } from "./geometry/radial";
import {
  STACK_HOLD_MAX_MS,
  STACK_VERTICAL_RESERVED,
  shouldAutoCollapseStackExpand,
  stackPeekFor,
} from "./geometry/stackLayout";
import { selectedRadialOptions } from "./html/activation-radial";
import { persistHintDismissed, readHintDismissed } from "./html/discoverability";
import { HAND_BAR_H, HAND_INSPECT_STICKY_BAND, HAND_PLAY_SLACK_PX } from "./html/hand";
import type { Message } from "./messages";
import {
  type CardFlight,
  flyingCardIds,
  handFlightScale,
  rebindFlightId,
  retargetFlight,
  spawnFlight,
  stackFlightScale,
} from "./motion/flights";

export const BOARD_VIEWPORT = { width: 1440, height: 900 } as const;
/** Bottom bar height — Arena-scale tuck + pip row (re-exported from html/hand.ts). */
export { HAND_BAR_H, HAND_INSPECT_STICKY_BAND };

export type HandDragState = {
  action: ActionView;
  name: string;
  print: string;
  manaCost: WireCost;
  kind?: string;
  x: number;
  y: number;
};

export type BoardModel = {
  camera: Camera;
  cameraFitPlayers: number | null;
  flights: Map<number, CardFlight>;
  handHidden: Set<number>;
  hideCardIds: Set<number>;
  lastFlightFrame: number | null;
  lastProvenanceSeq: number | null;
  ownedIds: Set<number>;
  pointer: PointerPhase;
  selectedId: number | null;
  /** Activation radial pointer arm (down on a wedge). */
  radialPress: RadialPress;
  /** Activation radial hover highlight index. */
  radialHover: number | null;
  viewport: { width: number; height: number };
  cursor: Vec2;
  // Action session state (pre-submit chrome, cost pipeline, staging).
  staged: StagedAction | null;
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
  // Concede.
  confirmConcede: boolean;
  // Game result.
  resultSeen: boolean;
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
  /** Catalog name suggestions for `choose_card_name` (query must match current draft). */
  cardNameSuggestions: { query: string; names: ReadonlyArray<string> } | null;
  /** Filter query for closed option prompts (creature types). */
  promptOptionFilter: string;
  /** Selected row while click-to-place reordering `order_triggers` (null when idle). */
  orderPickPos: number | null;
  /** Window-captured hand-bar drag ghost (null when idle). */
  handDrag: HandDragState | null;
  /** Hovered hand/radial action id — resolves `auto_tap` from the live action list. */
  hoverActionId: number | null;
};

export function initialBoardModel(): BoardModel {
  return {
    camera: { panX: 0, panY: 0, zoom: 1 },
    cameraFitPlayers: null,
    flights: new Map(),
    handHidden: new Set(),
    hideCardIds: new Set(),
    lastFlightFrame: null,
    lastProvenanceSeq: null,
    ownedIds: new Set(),
    pointer: { kind: "idle" },
    selectedId: null,
    radialPress: { armed: null },
    radialHover: null,
    viewport: { ...BOARD_VIEWPORT },
    cursor: { x: 0, y: 0 },
    staged: null,
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
    confirmConcede: false,
    resultSeen: false,
    hintDismissed: readHintDismissed(),
    hintAutoHidden: false,
    legendOpen: false,
    soundOn: isSoundEnabled(),
    priorityElapsed: 0,
    lastPriorityHolder: null,
    pendingChoiceKey: null,
    promptDraft: null,
    cardNameSuggestions: null,
    promptOptionFilter: "",
    orderPickPos: null,
    handDrag: null,
    hoverActionId: null,
  };
}

type BoardFold = Pick<GameFoldState, "provenance" | "seq" | "state">;

export function syncBoardWithGame(model: BoardModel, fold: BoardFold): BoardModel {
  if (fold.state == null) return model;

  let next = syncCombatStaging(model, fold);
  next = syncPromptDraft(next, fold);
  if (next.lastPriorityHolder !== fold.state.priority) {
    next = { ...next, priorityElapsed: 0, lastPriorityHolder: fold.state.priority };
  }
  const playerCount = Math.max(1, fold.state.players.length);
  if (next.cameraFitPlayers !== playerCount) {
    next = {
      ...next,
      camera: fitCamera({ x: next.viewport.width, y: next.viewport.height }, playerCount, 0),
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
  return syncStackChrome(next, fold);
}

function syncStackChrome(model: BoardModel, fold: BoardFold): BoardModel {
  const state = fold.state;
  if (state == null) return model;

  const holdMs = state.stack_hold_remaining_ms ?? 0;
  const stackHoldPeak = holdMs > 0 ? Math.min(STACK_HOLD_MAX_MS, Math.max(model.stackHoldPeak, holdMs)) : 0;

  const showStaged = model.staged != null && stagedPickTargets(model.staged, state) === null;
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
  if (key === model.pendingChoiceKey) return model;
  return {
    ...model,
    pendingChoiceKey: key,
    promptDraft: pc != null && gameState != null ? initPromptDraft(pc, gameState) : null,
    cardNameSuggestions: null,
    promptOptionFilter: "",
    orderPickPos: null,
  };
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

function cardsFor(fold: GameFoldState): RenderCard[] {
  if (fold.state == null) return [];
  return layout(fold.state, fold.state.viewer);
}

function cardAt(fold: GameFoldState, model: BoardModel, x: number, y: number): RenderCard | null {
  const cards = cardsFor(fold);
  const hitId = hitTest(model.camera, x, y, cards);
  if (hitId == null) return null;
  return cards.find((card) => card.id === hitId) ?? null;
}

function combatStepFor(fold: GameFoldState): boolean {
  const state = fold.state;
  if (state == null) return false;

  const mode = combatMode(
    state.step,
    state.active_player === state.viewer,
    false,
    state.combat.attackers,
    state.viewer,
    {
      attackersDeclared: state.combat.attackers_declared,
      blockersDeclared: state.combat.blockers_declared.includes(state.viewer),
    },
  );
  return mode != null;
}

function stackTarget(model: BoardModel): Vec2 {
  return { x: model.viewport.width - 160, y: model.viewport.height / 2 };
}

function cardTarget(camera: Camera, card: RenderCard): Vec2 {
  return worldToScreen(camera, card.x + card.w / 2, card.y + card.h / 2);
}

function playerOrigin(model: BoardModel, fold: BoardFold, seat: number): Vec2 {
  if (fold.state == null) return stackTarget(model);
  const count = Math.max(1, fold.state.players.length);
  const pos = avatarPos(seat, fold.state.viewer, count);
  return worldToScreen(model.camera, pos.x, pos.y);
}

function retargetFlightToCard(flight: CardFlight, model: BoardModel, card: RenderCard): CardFlight {
  const target = cardTarget(model.camera, card);
  return retargetFlight(flight, { x: target.x, y: target.y, scale: 1 });
}

function syncFlightsWithGame(model: BoardModel, fold: BoardFold): BoardModel {
  const state = fold.state;
  if (state == null) return model;

  const cards = layout(state, state.viewer);
  const cardsById = new Map(cards.map((card) => [card.id, card]));
  const handHidden = new Set(model.handHidden);
  let flights = new Map(model.flights);

  for (const [id, flight] of flights) {
    const card = cardsById.get(id);
    if (card != null) {
      flights.set(id, retargetFlightToCard(flight, model, card));
      continue;
    }
    if (flight.kind !== "stack") continue;
    const target = stackTarget(model);
    flights.set(id, retargetFlight(flight, { x: target.x, y: target.y, scale: stackFlightScale(model.camera.zoom) }));
  }

  for (const [permanent, from] of fold.provenance.landPlayFrom) {
    const card = cardsById.get(permanent);
    if (card == null) continue;

    if (!flights.has(permanent) && flights.has(from)) {
      flights = rebindFlightId(flights, from, permanent);
    }

    const existing = flights.get(permanent);
    if (existing != null) {
      flights.set(permanent, retargetFlightToCard({ ...existing, kind: "battlefield", fromCardId: from }, model, card));
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
        scale: handFlightScale(model.camera.zoom),
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
    const target = stackTarget(model);
    if (!flights.has(spell) && flights.has(meta.from)) {
      flights = rebindFlightId(flights, meta.from, spell);
    }

    const existing = flights.get(spell);
    if (existing != null) {
      flights.set(
        spell,
        retargetFlight(
          { ...existing, kind: "stack", fromCardId: meta.from },
          {
            x: target.x,
            y: target.y,
            scale: stackFlightScale(model.camera.zoom),
          },
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
        scale: handFlightScale(model.camera.zoom),
        targetX: target.x,
        targetY: target.y,
        targetScale: stackFlightScale(model.camera.zoom),
        kind: "stack",
        fromCardId: meta.from,
      }),
    );
    handHidden.add(meta.from);
  }

  for (const id of new Set([...fold.provenance.resolvedFromStack, ...fold.provenance.leftStackToPile])) {
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

    const start = stackTarget(model);
    const target = cardTarget(model.camera, card);
    flights.set(
      id,
      spawnFlight({
        id,
        print: card.print,
        name: card.name,
        x: start.x,
        y: start.y,
        scale: stackFlightScale(model.camera.zoom),
        targetX: target.x,
        targetY: target.y,
        targetScale: 1,
        kind: "from-stack",
      }),
    );
  }

  for (const [id, from] of fold.provenance.zoneMoves) {
    if (flights.has(id)) continue;
    const card = cardsById.get(id);
    if (card == null) continue;

    const target = cardTarget(model.camera, card);
    const prior = cardsById.get(from);
    const start = prior == null ? stackTarget(model) : cardTarget(model.camera, prior);
    flights.set(
      id,
      spawnFlight({
        id,
        print: card.print,
        name: card.name,
        x: start.x,
        y: start.y,
        scale: prior == null ? stackFlightScale(model.camera.zoom) : 1,
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
    if (stackSources.has(id)) continue;
    if (pendingResolve) continue;
    flights.delete(id);
    if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
  }

  return {
    ...model,
    flights,
    handHidden,
    hideCardIds: flyingCardIds(flights),
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
    pointer: pointerDown(cardAt(fold, model, x, y), x, y, combatStepFor(fold), state.viewer),
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
    cameraFitPlayers: null,
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
    if (seat != null && pendingAim.players.has(seat) && pendingTargetOneClick(pc)) {
      const answer = answerFromBoardTarget(pc, { kind: "player", player: seat });
      if (answer != null) {
        return [idle, boardIntentSubmit(tableId, choiceIntent(pc, answer))];
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
      : pc.kind === "choose_spell_targets" || pc.kind === "choose_ability_targets"
        ? pc.max
        : (cardPickRequiredCount(pc) ?? undefined);
  let next: number[];
  if (picked.includes(objectId)) {
    next = picked.filter((id) => id !== objectId);
  } else if (max != null && picked.length >= max) {
    return [synced, []];
  } else {
    next = [...picked, objectId];
  }
  return [{ ...synced, promptDraft: { kind: "card-pick", picked: next, filter: synced.promptDraft.filter } }, []];
}

function submitPendingHandPick(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
  pc: NonNullable<NonNullable<GameFoldState["state"]>["pending_choice"]>,
  objectId: number,
): BoardReturn {
  const idle = { ...model, handDrag: null, hoverActionId: null };
  if (pc.kind === "discard" && pc.count === 1) {
    return [
      { ...idle, promptDraft: null, pendingChoiceKey: null },
      boardIntentSubmit(tableId, choiceIntent(pc, { kind: "discard", cards: [objectId] })),
    ];
  }
  if (pc.kind === "put_land_from_hand") {
    return [
      { ...idle, promptDraft: null, pendingChoiceKey: null },
      boardIntentSubmit(tableId, choiceIntent(pc, { kind: "put_land", choice: objectId })),
    ];
  }
  if (pc.kind === "put_creature_from_hand") {
    return [
      { ...idle, promptDraft: null, pendingChoiceKey: null },
      boardIntentSubmit(tableId, choiceIntent(pc, { kind: "put_creature", choice: objectId })),
    ];
  }
  if (pc.kind === "put_from_hand_on_top" && pc.count === 1) {
    return [
      { ...idle, promptDraft: null, pendingChoiceKey: null },
      boardIntentSubmit(tableId, choiceIntent(pc, { kind: "hand_on_top", cards: [objectId] })),
    ];
  }
  return togglePendingObjectAimPick(idle, fold, pc, objectId);
}

function applyFlightsSynced(model: BoardModel, flightsIn: readonly CardFlight[], now: number): BoardModel {
  const flights = new Map<number, CardFlight>();
  const handHidden = new Set(model.handHidden);
  const retainedSourceIds = new Set<number>();

  for (const flight of flightsIn) {
    if (flight.fromCardId != null) retainedSourceIds.add(flight.fromCardId);

    if (flight.phase === "flying") {
      flights.set(flight.id, flight);
      if (flight.fromCardId != null) handHidden.add(flight.fromCardId);
      continue;
    }

    if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
  }

  for (const previousFlight of model.flights.values()) {
    if (previousFlight.fromCardId == null) continue;
    if (retainedSourceIds.has(previousFlight.fromCardId)) continue;
    handHidden.delete(previousFlight.fromCardId);
  }

  return {
    ...model,
    flights,
    handHidden,
    hideCardIds: flyingCardIds(flights),
    lastFlightFrame: flights.size === 0 ? null : now,
    ownedIds: new Set(flights.keys()),
  };
}

type Vec = { x: number; y: number };
type BoardCmd = FoldkitCommand.Command<Message, never, RpcClient>;
type BoardReturn = readonly [BoardModel, ReadonlyArray<BoardCmd>];

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
  if (!model.altDown) return [model, []];
  return applyInspectPin(model, tryPinInspect(model, fold));
}

/** True when the cursor is still over the hand fan (including raised faces above the bar). */
function cursorInHandInspectBand(model: BoardModel): boolean {
  return model.cursor.y >= model.viewport.height - HAND_INSPECT_STICKY_BAND;
}

/**
 * Hand peek leave clears aux while the cursor is still over pointer-events-none face art; with Alt
 * live re-pin that would steal to the battlefield under the hand. Keep the last hand hover latched
 * until the pointer leaves the hand sticky band (or Alt releases / a new hand card enters).
 */
function releaseStickyHandInspect(model: BoardModel): BoardModel {
  if (model.handInspectHover == null) return model;
  if (!model.altDown) return model;
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
  // SubmitIntent's Command emits app-level IntentAcked/IntentRejected. The top-level `update`
  // folds those results into `game.board.reject`, so the board's own case handlers don't need to
  // observe them directly.
  return submitCmd(tableId, intent);
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
    name: action.kind === "cast_prepared" ? action.label : (card?.name ?? action.label),
    minX,
    maxX,
    draftX: clampX(maxX, minX, maxX),
    xCost,
  };
}

/** Solid `spawnFromHand` / `seedDrop`: hide the bar tile immediately and fly from the drop point. */
function seedDropFromHand(
  model: BoardModel,
  card: ObjectView,
  screenOrigin: Vec,
  kind: "battlefield" | "stack",
): BoardModel {
  const flights = new Map(model.flights);
  const handHidden = new Set(model.handHidden);
  const startScale = handFlightScale(model.camera.zoom);
  const stackAim = stackTarget(model);
  const targetX = kind === "stack" ? stackAim.x : screenOrigin.x;
  const targetY = kind === "stack" ? stackAim.y : screenOrigin.y;
  const targetScale = kind === "stack" ? stackFlightScale(model.camera.zoom) : 1;
  flights.set(
    card.id,
    spawnFlight({
      id: card.id,
      print: card.print ?? "",
      name: card.name,
      x: screenOrigin.x,
      y: screenOrigin.y,
      scale: startScale,
      targetX,
      targetY,
      targetScale,
      kind,
      fromCardId: card.id,
    }),
  );
  handHidden.add(card.id);
  return {
    ...model,
    flights,
    handHidden,
    hideCardIds: flyingCardIds(flights),
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
    hideCardIds: flyingCardIds(flights),
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
    const seeded = seedDropFromHand(model, plan.card, screenOrigin, "stack");
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
    const seeded = card != null ? seedDropFromHand(model, card, screenOrigin, "battlefield") : model;
    return [seeded, boardIntentSubmit(tableId, takeAction(fold, action, null, 0, [], plan.picks))];
  }
  if (plan.kind === "cast") {
    const seeded = card != null ? seedDropFromHand(model, card, screenOrigin, "stack") : model;
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
    return [{ ...model, gyExilePick: { action, card, dropSeed, screenOrigin, picks } }, []];
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
    const picks: CostPicks = {
      ...model.discardPick.picks,
      discard_cost: [objectId],
      discard_settled: true,
    };
    return continueAfterCostPick(
      { ...model, discardPick: null, handDrag: null, hoverActionId: null },
      fold,
      tableId,
      model.discardPick.action,
      model.discardPick.card,
      picks,
      model.discardPick.dropSeed,
      model.discardPick.screenOrigin,
    );
  }
  const threshold = model.viewport.height - HAND_BAR_H + HAND_PLAY_SLACK_PX;
  const card = objectByAction(fold, action);
  const plan = planHandDrop(action, card, y, threshold);
  if (plan.kind === "ignore") return [model, []];
  const withHint = hideHintOnHandUse(model);
  const world = screenToWorld(withHint.camera, x, y);
  const dropSeed: Vec = { x: world.x - CARD_W / 2, y: world.y - CARD_H / 2 };
  const screenOrigin: Vec = { x, y };
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
    return [
      { ...withHint, gyExilePick: { action: plan.action, card: plan.card, dropSeed, screenOrigin, picks: plan.picks } },
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
  const clearedOrigin = model.staged != null ? clearPlayOrigin(model, model.staged.card.id) : model;
  return {
    ...clearedOrigin,
    staged: null,
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
    cardNameSuggestions: null,
    promptOptionFilter: "",
    orderPickPos: null,
    handDrag: null,
    hoverActionId: null,
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
  const card = fold.state.objects.find((o) => o.id === id) ?? null;
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
  // Same-signature as primaryActionFor but done inline to avoid crate churn.
  const step = state.step;
  const me = state.viewer;
  const active = state.active_player;
  const declaredAttackers = state.combat.attackers;
  const attackDone = model.attackersConfirmed || state.combat.attackers_declared || declaredAttackers.length > 0;
  const blockDone = model.blockersConfirmed || state.combat.blockers_declared.includes(me);
  const attackingMe = declaredAttackers.some((a) => a.defender === me);
  if (step === STEP.DeclareAttackers && active === me && !attackDone) {
    return attackers.length
      ? { kind: "confirm-attackers", label: `Attack (${attackers.length})` }
      : { kind: "confirm-attackers", label: "No attackers" };
  }
  if (step === STEP.DeclareBlockers && attackingMe && !blockDone) {
    return model.combatBlocks.length
      ? { kind: "confirm-blockers", label: `Block (${model.combatBlocks.length})` }
      : { kind: "confirm-blockers", label: "No blockers" };
  }
  if (step === STEP.Draw && active === me) return { kind: "pass", label: "Draw" };
  return { kind: "pass", label: "Next" };
}

/** Submit a ready multi-aim or on-board damage-assign draft; null when nothing to submit. */
function trySubmitReadyPendingDraft(
  model: BoardModel,
  fold: GameFoldState,
  tableId: string | null,
): BoardReturn | null {
  const state = fold.state;
  if (state == null) return null;
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
      return [
        { ...synced, promptDraft: null, pendingChoiceKey: null },
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
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
      return [
        { ...synced, promptDraft: null, pendingChoiceKey: null },
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
    }
  }
  if (
    pc?.kind === "divide_spell_damage" &&
    pendingDivideSpellObjectIndexes(pc, state) != null &&
    synced.promptDraft?.kind === "divide"
  ) {
    const answer = buildAnswerFromDraft(pc, synced.promptDraft);
    if (answer != null) {
      return [
        { ...synced, promptDraft: null, pendingChoiceKey: null },
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
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
        return [
          { ...synced, promptDraft: null, pendingChoiceKey: null },
          boardIntentSubmit(tableId, choiceIntent(pc, answer)),
        ];
      }
    }
  }
  if (
    pc != null &&
    synced.promptDraft != null &&
    (pc.kind === "order_triggers" ||
      pc.kind === "scry" ||
      pc.kind === "surveil" ||
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
      return [
        { ...synced, promptDraft: null, pendingChoiceKey: null },
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
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
        { ...synced, promptDraft: null, pendingChoiceKey: null },
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
    const intent: WireIntent = { kind: "declare_attackers", player: me, attackers: model.combatAttackers };
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
  const mode = combatMode(
    state.step,
    state.active_player === state.viewer,
    false,
    state.combat.attackers,
    state.viewer,
    {
      attackersDeclared: model.attackersConfirmed || state.combat.attackers_declared,
      blockersDeclared: model.blockersConfirmed || state.combat.blockers_declared.includes(state.viewer),
    },
  );
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
  const opponents = state.players.map((p) => p.player).filter((p) => p !== state.viewer);
  const result = handleCombatDrop(
    mode,
    model.combatAttackers,
    model.combatBlocks,
    cardShape as unknown as Parameters<typeof handleCombatDrop>[3],
    defenderSeat,
    dropTarget as unknown as Parameters<typeof handleCombatDrop>[5],
    state.combat.attackers,
    state.viewer,
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
    case "BoardPointerDown":
      return [pointerDownModel(model, fold, message.x, message.y), []];
    case "BoardPointerMove": {
      const moved = releaseStickyHandInspect(pointerMoveModel(model, message.x, message.y));
      return applyLiveInspectPin(moved, fold);
    }
    case "BoardPointerUp":
      return pointerUpModel(model, fold, tableId, message.x, message.y);
    case "FlightsSynced":
      return [applyFlightsSynced(model, message.flights, message.now), []];
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
      const pendingAim = fold.state != null ? pendingBoardTargetMode(pc, fold.state) : null;
      if (pendingAim == null || pc == null) return [model, []];
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
        const picks: CostPicks = { ...pick.picks, discard_cost: [...message.ids], discard_settled: true };
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
      const picks: CostPicks = { ...pick.picks, graveyard_exile: [...message.ids], gy_exile_settled: true };
      return continueAfterCostPick(
        { ...model, gyExilePick: null },
        fold,
        tableId,
        pick.action,
        pick.card,
        picks,
        pick.dropSeed,
        pick.screenOrigin,
      );
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
      if (pc == null || synced.promptDraft == null) return [synced, []];

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
        return [{ ...synced, promptDraft: { kind: "card-pick", picked: next, filter: synced.promptDraft.filter } }, []];
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

      if (synced.promptDraft.kind === "partition" && (pc.kind === "scry" || pc.kind === "surveil")) {
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
      if (synced.promptDraft?.kind !== "partition") return [synced, []];
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
      if (pc == null || gameState == null || synced.promptDraft == null) return [synced, []];
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
      return [
        { ...synced, promptDraft: null, pendingChoiceKey: null },
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
    }
    case "PromptDeclined": {
      const synced = syncPromptDraft(model, fold);
      const pc = fold.state?.pending_choice;
      if (pc == null) return [synced, []];
      const answer = declineAnswer(pc);
      if (answer == null) return [synced, []];
      return [
        { ...synced, promptDraft: null, pendingChoiceKey: null },
        boardIntentSubmit(tableId, choiceIntent(pc, answer)),
      ];
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
      const withAlt = { ...model, altDown: true };
      return applyInspectPin(withAlt, tryPinInspect(withAlt, fold));
    }
    case "AltUp":
      return [{ ...model, altDown: false, inspectPin: null, inspectCard: undefined }, []];
    case "InspectAuxHovered": {
      if (message.source === "hand") {
        // Peek-strip leave while the cursor is still in the hand fan (face art is
        // pointer-events-none) must not drop aux — Alt live re-pin would steal to BF underneath.
        if (message.card == null && model.altDown && model.handInspectHover != null && cursorInHandInspectBand(model)) {
          return [model, []];
        }
        return applyLiveInspectPin({ ...model, handInspectHover: message.card }, fold);
      }
      return applyLiveInspectPin({ ...model, stackInspectHover: message.card }, fold);
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
    case "PileOverlayClosed":
      return [{ ...model, pileExpand: null }, []];
    // ── Concede ───────────────────────────────────────────────────────────────
    case "ConcedeClicked":
      return [{ ...model, confirmConcede: true }, []];
    case "ConcedeCancelled":
      return [{ ...model, confirmConcede: false }, []];
    case "ConcedeConfirmed": {
      if (fold.state == null) return [{ ...model, confirmConcede: false }, []];
      return [
        { ...model, confirmConcede: false },
        boardIntentSubmit(tableId, { kind: "concede", player: fold.state.viewer }),
      ];
    }
    // ── Game result ───────────────────────────────────────────────────────────
    case "ResultSeen":
      return [{ ...model, resultSeen: true }, []];
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
      const submitted = trySubmitReadyPendingDraft(model, fold, tableId);
      if (submitted != null) return submitted;
      return primaryClickModel(model, fold, tableId);
    }
    case "KeyboardEnterPressed": {
      const state = fold.state;
      if (state == null) return [model, []];
      if (state.mulliganing) return [model, []];
      const submitted = trySubmitReadyPendingDraft(model, fold, tableId);
      if (submitted != null) return submitted;
      const me = state.viewer;
      const active = state.active_player;
      // Enter toggles End Turn when it's the viewer's turn (and stack is empty), or
      // toggles Turn Yield when it's another player's turn.
      if (tableId == null) return [model, []];
      const enabled = !(state.turn_yielded ?? false);
      if (me === active && state.stack.length === 0) {
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
    default: {
      const _exhaustive: never = message;
      return [model, []];
    }
  }
}
