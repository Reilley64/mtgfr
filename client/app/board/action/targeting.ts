// How the board asks for a staged action's target.
//
// The engine already enumerates what's legal (`Game::legal_targets`, on the wire as
// `ActionView.targets`), so nothing here re-derives `TargetSpec` — this module only decides *how
// to ask*, which depends on where the legal targets live:
//
//   - battlefield permanents and players are on the canvas → point at them (arrow)
//   - graveyard/exile cards are collapsed into a single pile card, and stack objects live in the
//     DOM overlay, so neither can be clicked → offer them as a picker instead

import { colors } from "~/design-tokens.generated";
import type { ActionView, PendingChoiceView, VisibleState, WireTarget } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import type { StagedAction } from "./execution";

/** Shared target-arrow / staged-preview accent (canvas stroke + DOM ring). */
export const TARGET_COLOR = colors.islandBlue;

export type Vec = { x: number; y: number };

export type StagingOverlay = {
  aiming: boolean;
  targetObjects: ReadonlySet<number>;
  targetPlayers: ReadonlySet<number>;
  aimFrom: Vec | null;
};

// Stack overlay geometry — one source for the DOM overlay and the canvas aim origin.
const STACK_CARD_W = 180;
const STACK_OVERLAY_RIGHT = 16;
const STACK_PEEK = 34;
const STACK_ANCHOR_FROM_RIGHT = STACK_OVERLAY_RIGHT + STACK_CARD_W / 2;

function stackCardH(cardW = STACK_CARD_W): number {
  return cardW / 0.716;
}

/** Screen-space center of the top card in a right-edge pile of `count` cards. */
export function stackAimOrigin(viewportW: number, viewportH: number, count: number, peek = STACK_PEEK): Vec {
  const n = Math.max(1, count);
  const cardH = stackCardH();
  const pileH = cardH + (n - 1) * peek;
  return {
    x: viewportW - STACK_ANCHOR_FROM_RIGHT,
    y: viewportH / 2 + pileH / 2 - (n - 1) * peek - cardH / 2,
  };
}

export function stagingOverlay(
  staged: StagedAction | null,
  state: VisibleState,
  viewport: { width: number; height: number },
  stackLen: number,
): StagingOverlay {
  const idle: StagingOverlay = {
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    aimFrom: null,
  };
  if (staged == null) return idle;

  const mode = targetMode(staged.action, state);
  if (mode.kind !== "arrow" || staged.preferPick) return idle;

  return {
    aiming: true,
    targetObjects: mode.objects,
    targetPlayers: mode.players,
    aimFrom: stackAimOrigin(viewport.width, viewport.height, stackLen + 1),
  };
}

export type TargetMode =
  | { kind: "none" }
  | { kind: "impossible" }
  | { kind: "arrow"; objects: ReadonlySet<number>; players: ReadonlySet<number> }
  | { kind: "pick"; targets: WireTarget[] };

export function onBoard(target: WireTarget, state: VisibleState): boolean {
  if (target.kind === "player") return true;
  const obj = state.objects.find((o) => o.id === target.id);
  if (obj == null) return false;
  if (obj.zone === ZONE.Battlefield || obj.zone === ZONE.Stack) return true;
  return state.stack.some((entry) => entry.source === target.id);
}

export function askFor(targets: WireTarget[], state: VisibleState): TargetMode {
  if (targets.length === 0) return { kind: "impossible" };
  if (targets.every((t) => onBoard(t, state))) {
    return {
      kind: "arrow",
      objects: new Set(targets.filter((t) => t.kind === "object").map((t) => t.id)),
      players: new Set(targets.filter((t) => t.kind === "player").map((t) => t.player)),
    };
  }
  return { kind: "pick", targets };
}

export function targetMode(action: ActionView, state: VisibleState): TargetMode {
  if (!action.needs_target) return { kind: "none" };
  return askFor(action.targets ?? [], state);
}

/** Legal targets for the staged-action picker, or null when the arrow should ask instead. */
export function stagedPickTargets(staged: StagedAction, state: VisibleState): WireTarget[] | null {
  const mode = targetMode(staged.action, state);
  if (mode.kind === "none" || mode.kind === "impossible") return null;
  if (mode.kind === "pick") return mode.targets;
  if (staged.preferPick && mode.kind === "arrow") {
    return [
      ...[...mode.objects].map((id) => ({ kind: "object" as const, id })),
      ...[...mode.players].map((player) => ({ kind: "player" as const, player })),
    ];
  }
  return null;
}

export function objectName(state: VisibleState, id: number): string {
  return state.objects.find((o) => o.id === id)?.name ?? `#${id}`;
}

export function playerSeatLabel(state: VisibleState, seat: number): string {
  const name = state.players.find((p) => p.player === seat)?.username?.trim();
  return name || `P${seat}`;
}

export function choiceItemsAsWireTargets(items: ReadonlyArray<{ id: number; player?: number | null }>): WireTarget[] {
  return items.map((item) =>
    item.player != null ? { kind: "player" as const, player: item.player } : { kind: "object" as const, id: item.id },
  );
}

/** Pending kinds that aim at on-board permanents when every legal item is clickable on the canvas. */
const ONBOARD_CARD_PICK_KINDS = new Set<PendingChoiceView["kind"]>([
  "sacrifice_edict",
  "choose_own_sacrifices",
  "may_sacrifice",
  "devour",
  "proliferate",
  "phase_out",
  "decline_untap",
  "choose_attach_host",
  "sacrifice_unless_return_land",
  "choose_copy_target",
  "choose_counter_target_for_player",
  "caster_keep_permanents",
  "choose_activation_cost_targets",
]);

/** True when this pending choice can be answered by aiming on the board (Arena aim). */
export function pendingBoardTargetMode(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): Extract<TargetMode, { kind: "arrow" }> | null {
  if (pc == null) return null;
  if (pc.player !== state.viewer) return null;
  if (!("items" in pc) || !Array.isArray(pc.items)) return null;

  if (pc.kind === "choose_target") {
    if (pc.max < 1) return null;
  } else if (pc.kind === "choose_spell_targets" || pc.kind === "choose_ability_targets") {
    if (pc.max < 1) return null;
  } else if (!ONBOARD_CARD_PICK_KINDS.has(pc.kind)) {
    return null;
  }

  const mode = askFor(choiceItemsAsWireTargets(pc.items), state);
  if (mode.kind !== "arrow") return null;
  return mode;
}

/** One legal click completes the answer; otherwise clicks accumulate until Confirm. */
export function pendingTargetOneClick(pc: PendingChoiceView): boolean {
  if (pc.kind === "choose_target") return pc.max === 1;
  if (pc.kind === "choose_spell_targets" || pc.kind === "choose_ability_targets") {
    return pc.min === 1 && pc.max === 1;
  }
  if (
    pc.kind === "choose_attach_host" ||
    pc.kind === "sacrifice_unless_return_land" ||
    pc.kind === "choose_copy_target"
  ) {
    return true;
  }
  if (pc.kind === "sacrifice_edict") return !pc.keep_one;
  if (pc.kind === "choose_own_sacrifices" || pc.kind === "choose_activation_cost_targets") {
    return pc.count === 1;
  }
  return false;
}

/** Aim overlay for on-board pending targets; idle when the modal picker should ask. */
export function pendingTargetingOverlay(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
  viewport: { width: number; height: number },
  stackLen: number,
): StagingOverlay {
  const idle: StagingOverlay = {
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    aimFrom: null,
  };
  const mode = pendingBoardTargetMode(pc, state);
  if (mode == null) return idle;
  return {
    aiming: true,
    targetObjects: mode.objects,
    targetPlayers: mode.players,
    aimFrom: stackAimOrigin(viewport.width, viewport.height, stackLen + 1),
  };
}

/** Object ids when combat damage or counter division can be assigned by clicking battlefield permanents. */
export function pendingDamageAssignBlockers(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (pc == null) return null;
  if (pc.kind !== "assign_combat_damage" && pc.kind !== "divide_counters") return null;
  if (pc.player !== state.viewer) return null;
  if (pc.items.length === 0) return null;
  const ids = new Set<number>();
  for (const item of pc.items) {
    const obj = state.objects.find((o) => o.id === item.id);
    if (obj == null || obj.zone !== ZONE.Battlefield) return null;
    ids.add(item.id);
  }
  return ids;
}

/** Highlight permanents during on-board combat damage / counter assign (no aim arrow). */
export function pendingDamageAssignOverlay(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): StagingOverlay {
  const idle: StagingOverlay = {
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    aimFrom: null,
  };
  const blockers = pendingDamageAssignBlockers(pc, state);
  if (blockers == null) return idle;
  return {
    aiming: true,
    targetObjects: blockers,
    targetPlayers: new Set(),
    aimFrom: null,
  };
}

type PendingHandPickChoice = Extract<
  PendingChoiceView,
  {
    kind:
      | "discard"
      | "may_discard"
      | "put_land_from_hand"
      | "put_creature_from_hand"
      | "put_from_hand_on_top"
      | "cast_creature_face_down";
  }
>;

function isPendingHandPick(pc: PendingChoiceView): pc is PendingHandPickChoice {
  return (
    pc.kind === "discard" ||
    pc.kind === "may_discard" ||
    pc.kind === "put_land_from_hand" ||
    pc.kind === "put_creature_from_hand" ||
    pc.kind === "put_from_hand_on_top" ||
    pc.kind === "cast_creature_face_down"
  );
}

/** True when a hand-bar click should auto-submit (no accumulate chrome). */
export function pendingHandPickOneClick(pc: PendingChoiceView | null | undefined): boolean {
  if (pc == null || !isPendingHandPick(pc)) return false;
  if (pc.kind === "discard" || pc.kind === "may_discard") return false;
  if (
    pc.kind === "put_land_from_hand" ||
    pc.kind === "put_creature_from_hand" ||
    pc.kind === "cast_creature_face_down"
  ) {
    return true;
  }
  if (pc.kind === "put_from_hand_on_top") return pc.count === 1;
  return false;
}

/**
 * Legal hand object ids for pending discard / put-from-hand choices when every item is in the
 * viewer's hand. Off-hand items keep the modal card picker.
 */
export function pendingHandPickIds(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (pc == null || !isPendingHandPick(pc)) return null;
  if (pc.player !== state.viewer) return null;
  if (pc.items.length === 0) return null;
  const handIds = new Set(
    state.objects.filter((o) => o.zone === ZONE.Hand && o.owner === state.viewer).map((o) => o.id),
  );
  const ids = new Set<number>();
  for (const item of pc.items) {
    if (!handIds.has(item.id)) return null;
    ids.add(item.id);
  }
  return ids;
}

/** Legal hand object ids for pending discard / may_discard when every item is in the viewer's hand. */
export function pendingDiscardHandIds(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (pc == null) return null;
  if (pc.kind !== "discard" && pc.kind !== "may_discard") return null;
  return pendingHandPickIds(pc, state);
}

/** Shared zone pile when every id is in the same owner's zone; otherwise null. */
export function sharedZonePile(
  zone: number,
  ids: ReadonlyArray<number> | null | undefined,
  state: VisibleState,
): { zone: number; owner: number } | null {
  if (ids == null || ids.length === 0) return null;
  let owner: number | null = null;
  for (const id of ids) {
    const obj = state.objects.find((o) => o.id === id);
    if (obj == null || obj.zone !== zone) return null;
    if (owner == null) owner = obj.owner;
    else if (obj.owner !== owner) return null;
  }
  if (owner == null) return null;
  return { zone, owner };
}

/** Shared graveyard pile when every id is in the same owner's GY; otherwise null. */
export function sharedGraveyardPile(
  ids: ReadonlyArray<number> | null | undefined,
  state: VisibleState,
): { zone: number; owner: number } | null {
  return sharedZonePile(ZONE.Graveyard, ids, state);
}

/** Shared exile pile when every id is in the same owner's exile; otherwise null. */
export function sharedExilePile(
  ids: ReadonlyArray<number> | null | undefined,
  state: VisibleState,
): { zone: number; owner: number } | null {
  return sharedZonePile(ZONE.Exile, ids, state);
}

/** Shared graveyard pile for on-pile gy-exile aim, or null when modal fallback is required. */
export function gyExileCostPile(
  choices: ReadonlyArray<number> | null | undefined,
  state: VisibleState,
): { zone: number; owner: number } | null {
  return sharedGraveyardPile(choices, state);
}

/**
 * Legal graveyard object ids for a local pre-submit gy-exile cost when every choice shares one
 * graveyard pile. Mixed zones/owners keep the modal cost grid.
 */
export function gyExileCostObjectIds(
  choices: ReadonlyArray<number> | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (gyExileCostPile(choices, state) == null || choices == null) return null;
  return new Set(choices);
}

type PendingGraveyardPickChoice = Extract<
  PendingChoiceView,
  {
    kind:
      | "exile_from_graveyard"
      | "may_return_from_graveyard"
      | "shuffle_from_graveyard"
      | "choose_dredge"
      | "pay_cumulative_upkeep_or_sacrifice"
      | "choose_activation_cost_targets"
      | "choose_target"
      | "choose_spell_targets"
      | "choose_ability_targets";
  }
>;

function isPendingGraveyardPick(pc: PendingChoiceView): pc is PendingGraveyardPickChoice {
  return (
    pc.kind === "exile_from_graveyard" ||
    pc.kind === "may_return_from_graveyard" ||
    pc.kind === "shuffle_from_graveyard" ||
    pc.kind === "choose_dredge" ||
    pc.kind === "pay_cumulative_upkeep_or_sacrifice" ||
    pc.kind === "choose_activation_cost_targets" ||
    pc.kind === "choose_target" ||
    pc.kind === "choose_spell_targets" ||
    pc.kind === "choose_ability_targets"
  );
}

/** True when a pile click should auto-submit (no accumulate chrome). */
export function pendingGraveyardPickOneClick(pc: PendingChoiceView | null | undefined): boolean {
  if (pc == null || !isPendingGraveyardPick(pc)) return false;
  if (pc.kind === "choose_dredge") return true;
  if (pc.kind === "choose_target") return pc.max === 1;
  if (pc.kind === "choose_spell_targets" || pc.kind === "choose_ability_targets") {
    return pc.min === 1 && pc.max === 1;
  }
  if (pc.kind === "shuffle_from_graveyard") return pc.max === 1;
  if (pc.kind === "pay_cumulative_upkeep_or_sacrifice" || pc.kind === "choose_activation_cost_targets") {
    return pc.count === 1;
  }
  return false;
}

/**
 * Legal graveyard ids for engine GY card-picks when every item shares one graveyard pile.
 * Mixed owners / off-GY items keep the modal card picker.
 */
export function pendingGraveyardPickIds(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (pc == null || !isPendingGraveyardPick(pc)) return null;
  if (pc.player !== state.viewer) return null;
  if (pc.items.length === 0) return null;
  const ids = pc.items.map((item) => item.id);
  if (sharedGraveyardPile(ids, state) == null) return null;
  return new Set(ids);
}

/** Shared GY pile for pending graveyard card-picks, or null when modal fallback is required. */
export function pendingGraveyardPickPile(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): { zone: number; owner: number } | null {
  const ids = pendingGraveyardPickIds(pc, state);
  if (ids == null) return null;
  return sharedGraveyardPile([...ids], state);
}

type PendingExilePickChoice = Extract<
  PendingChoiceView,
  {
    kind:
      | "choose_exiled_with_card"
      | "choose_exiled_with_card_to_cast"
      | "choose_exiled_dig_to_cast_free"
      | "opponent_chooses_exiled_nonland"
      | "choose_exiled_to_cast_free";
  }
>;

function isPendingExilePick(pc: PendingChoiceView): pc is PendingExilePickChoice {
  return (
    pc.kind === "choose_exiled_with_card" ||
    pc.kind === "choose_exiled_with_card_to_cast" ||
    pc.kind === "choose_exiled_dig_to_cast_free" ||
    pc.kind === "opponent_chooses_exiled_nonland" ||
    pc.kind === "choose_exiled_to_cast_free"
  );
}

/** True when an exile-pile click should auto-submit (no accumulate chrome). */
export function pendingExilePickOneClick(pc: PendingChoiceView | null | undefined): boolean {
  if (pc == null || !isPendingExilePick(pc)) return false;
  if (pc.kind === "choose_exiled_to_cast_free") return pc.count === 1;
  return true;
}

/**
 * Legal exile ids for engine exile card-picks when every item shares one exile pile.
 * Mixed owners / off-exile items keep the modal card picker.
 */
export function pendingExilePickIds(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (pc == null || !isPendingExilePick(pc)) return null;
  if (pc.player !== state.viewer) return null;
  if (pc.items.length === 0) return null;
  const ids = pc.items.map((item) => item.id);
  if (sharedExilePile(ids, state) == null) return null;
  return new Set(ids);
}

/** Shared exile pile for pending exile card-picks, or null when modal fallback is required. */
export function pendingExilePickPile(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): { zone: number; owner: number } | null {
  const ids = pendingExilePickIds(pc, state);
  if (ids == null) return null;
  return sharedExilePile([...ids], state);
}

/** Any pending pile aim (GY or exile) that should keep/auto-open the pile overlay. */
export function pendingPilePickPile(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): { zone: number; owner: number } | null {
  return pendingGraveyardPickPile(pc, state) ?? pendingExilePickPile(pc, state);
}

/**
 * Legal permanent ids for a local pre-submit sacrifice cost when every choice is on the battlefield.
 * Off-board choices keep the modal cost grid.
 */
export function sacrificeCostObjectIds(
  choices: ReadonlyArray<number> | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (choices == null || choices.length === 0) return null;
  const ids = new Set<number>();
  for (const id of choices) {
    const obj = state.objects.find((o) => o.id === id);
    if (obj == null || obj.zone !== ZONE.Battlefield) return null;
    ids.add(id);
  }
  return ids;
}

/** Highlight sacrifice-cost permanents while `sacrificePick` is live (no aim arrow). */
export function sacrificeCostOverlay(
  choices: ReadonlyArray<number> | null | undefined,
  state: VisibleState,
): StagingOverlay {
  const idle: StagingOverlay = {
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    aimFrom: null,
  };
  const ids = sacrificeCostObjectIds(choices, state);
  if (ids == null) return idle;
  return {
    aiming: true,
    targetObjects: ids,
    targetPlayers: new Set(),
    aimFrom: null,
  };
}

/**
 * Object id → divide draft index when every `divide_spell_damage` target is a battlefield permanent.
 * Player targets or off-board items keep the modal steppers only.
 */
export function pendingDivideSpellObjectIndexes(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlyMap<number, number> | null {
  if (pc == null || pc.kind !== "divide_spell_damage") return null;
  if (pc.player !== state.viewer) return null;
  if (pc.items.length === 0) return null;
  const indexes = new Map<number, number>();
  for (let i = 0; i < pc.items.length; i++) {
    const item = pc.items[i];
    if (item == null || item.player != null) return null;
    const obj = state.objects.find((o) => o.id === item.id);
    if (obj == null || obj.zone !== ZONE.Battlefield) return null;
    indexes.set(item.id, i);
  }
  return indexes;
}

/** Highlight battlefield spell-damage targets during on-board divide (no aim arrow). */
export function pendingDivideSpellOverlay(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): StagingOverlay {
  const idle: StagingOverlay = {
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    aimFrom: null,
  };
  const indexes = pendingDivideSpellObjectIndexes(pc, state);
  if (indexes == null) return idle;
  return {
    aiming: true,
    targetObjects: new Set(indexes.keys()),
    targetPlayers: new Set(),
    aimFrom: null,
  };
}

/** Legal player seats for on-board choose_target_players / choose_splitting_opponent aim. */
export function pendingPlayerAimSeats(
  pc: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlySet<number> | null {
  if (pc == null) return null;
  if (pc.player !== state.viewer) return null;
  if (pc.kind !== "choose_target_players" && pc.kind !== "choose_splitting_opponent") return null;
  if (!("items" in pc) || pc.items.length === 0) return null;
  const seats = new Set<number>();
  for (const item of pc.items) {
    if (item.player == null) return null;
    seats.add(item.player);
  }
  return seats;
}

export function pendingPlayerAimOneClick(pc: PendingChoiceView): boolean {
  if (pc.kind === "choose_splitting_opponent") return true;
  if (pc.kind === "choose_target_players") return pc.max === 1;
  return false;
}

/** Highlight life-orb avatars for on-board player-target pending choices. */
export function pendingPlayerAimOverlay(pc: PendingChoiceView | null | undefined, state: VisibleState): StagingOverlay {
  const idle: StagingOverlay = {
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    aimFrom: null,
  };
  const seats = pendingPlayerAimSeats(pc, state);
  if (seats == null) return idle;
  return {
    aiming: true,
    targetObjects: new Set(),
    targetPlayers: seats,
    aimFrom: null,
  };
}

/** Object ids that are legal arrow targets while staged or pending aim is live. */
export function aimingObjectIds(
  staged: StagedAction | null,
  pending: PendingChoiceView | null | undefined,
  state: VisibleState,
): ReadonlySet<number> {
  if (staged != null && !staged.preferPick) {
    const mode = targetMode(staged.action, state);
    if (mode.kind === "arrow") return mode.objects;
  }
  const pendingMode = pendingBoardTargetMode(pending, state);
  if (pendingMode != null) return pendingMode.objects;
  return new Set();
}

/** Title while the player is aiming a staged cast or activation before submitting. */
export function stagedTargetTitle(staged: StagedAction): string {
  const { card, action } = staged;
  if (action.kind === "activate" && action.label !== card.name) {
    return `${action.label} — ${card.name}`;
  }
  return action.label;
}
