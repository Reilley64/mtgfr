// PendingChoice answer packing + prompt draft helpers.
// Forms collect AnswerInput; choiceIntent maps to WireIntent (wire-protocol-and-visibility spec).

import * as Match from "effect/Match";
import type {
  PendingChoiceView,
  VisibleState,
  WireDamage,
  WireIntent,
  WireModeChoice,
  WireSpellDamage,
} from "~/wire/types";

type ChoiceItemLike = { id: number; label?: string; player?: number | null };

export type FormulatorId =
  | "cardPick"
  | "orderTriggers"
  | "damageAssign"
  | "yesNo"
  | "payCost"
  | "modeList"
  | "playerPick"
  | "divideTotal"
  | "pilePick"
  | "partition"
  | "colorPick"
  | "stringPick"
  | "numberPick"
  | "destinationPick";

export const FORMULATOR_FOR_KIND: { [K in PendingChoiceView["kind"]]: FormulatorId } = {
  order_triggers: "orderTriggers",
  choose_target: "cardPick",
  choose_spell_targets: "cardPick",
  choose_target_players: "playerPick",
  may_yes_no: "yesNo",
  decline_untap: "cardPick",
  pay_cost: "payCost",
  pay_or_counter: "payCost",
  pay_or_controller_draws: "payCost",
  choose_countered_spell_destination: "destinationPick",
  pay_echo_or_sacrifice: "payCost",
  pay_recover_or_exile: "payCost",
  sacrifice_unless_pay: "payCost",
  sacrifice_unless_return_land: "cardPick",
  assign_combat_damage: "damageAssign",
  divide_spell_damage: "divideTotal",
  divide_counters: "divideTotal",
  scry: "cardPick",
  surveil: "cardPick",
  search_library: "cardPick",
  select_from_top: "cardPick",
  distribute_top: "partition",
  shuffle_from_graveyard: "cardPick",
  sacrifice_edict: "cardPick",
  proliferate: "cardPick",
  phase_out: "cardPick",
  choose_ability_targets: "cardPick",
  choose_activation_cost_targets: "cardPick",
  may_sacrifice: "cardPick",
  choose_own_sacrifices: "cardPick",
  devour: "cardPick",
  exile_from_graveyard: "cardPick",
  caster_keep_permanents: "cardPick",
  choose_counter_target_for_player: "cardPick",
  may_return_from_graveyard: "cardPick",
  may_discard: "cardPick",
  discard: "cardPick",
  put_land_from_hand: "cardPick",
  put_creature_from_hand: "cardPick",
  choose_dredge: "cardPick",
  cast_creature_face_down: "cardPick",
  choose_exiled_with_card: "cardPick",
  choose_exiled_with_card_to_cast: "cardPick",
  choose_exiled_dig_to_cast_free: "cardPick",
  dance_exile_more: "yesNo",
  opponent_chooses_pile: "pilePick",
  opponent_chooses_exiled_nonland: "cardPick",
  choose_splitting_opponent: "playerPick",
  partition_revealed: "partition",
  choose_pile_for_hand: "pilePick",
  choose_exiled_to_cast_free: "cardPick",
  revealed_card_to_battlefield_or_hand: "destinationPick",
  choose_mode: "modeList",
  choose_trigger_modes: "modeList",
  choose_mana_color: "colorPick",
  choose_creature_type: "stringPick",
  choose_color: "colorPick",
  choose_copy_target: "cardPick",
  choose_attach_host: "cardPick",
  put_from_hand_on_top: "cardPick",
  opponent_chooses_revealed_to_graveyard: "cardPick",
  pay_cumulative_upkeep_or_sacrifice: "cardPick",
  may_draw_up_to: "numberPick",
  trade_secrets_caster_draw: "numberPick",
  pay_any_amount_of_mana: "numberPick",
  choose_card_name: "stringPick",
  trade_secrets_repeat: "yesNo",
};

export function assertAllKindsRegistered(kinds: readonly PendingChoiceView["kind"][]): void {
  const actual = Object.keys(FORMULATOR_FOR_KIND).sort();
  const expected = [...kinds].sort();
  if (actual.length !== expected.length) {
    throw new Error(`Expected ${expected.length} registered kinds, got ${actual.length}`);
  }
  for (let i = 0; i < expected.length; i++) {
    if (actual[i] === expected[i]) continue;
    throw new Error(`Kind registry mismatch at ${i}: expected ${expected[i]}, got ${actual[i] ?? "<missing>"}`);
  }
}

export type PromptDraft =
  | { kind: "card-pick"; picked: number[] }
  | { kind: "order"; order: number[] }
  | { kind: "damage"; amounts: Record<number, number> }
  | { kind: "divide"; amounts: Record<number, number> }
  | { kind: "target"; id: number; player?: number }
  | { kind: "targets"; ids: number[] }
  | { kind: "player-pick"; players: number[] }
  | { kind: "pile"; pile: 0 | 1 }
  | { kind: "partition"; buckets: Record<string, number[]> }
  | { kind: "number"; count: number }
  | { kind: "string"; value: string }
  | { kind: "color"; color: number }
  | { kind: "mode"; mode: number }
  | { kind: "modes"; modes: WireModeChoice[] }
  | { kind: "pay"; pay: boolean }
  | { kind: "may"; yes: boolean }
  | { kind: "destination"; choice: number | null | boolean };

export type AnswerInput =
  | { kind: "order"; order: number[] }
  | { kind: "target"; id: number; player?: number }
  | { kind: "targets"; ids: number[] }
  | { kind: "may"; yes: boolean }
  | { kind: "pay"; pay: boolean }
  | { kind: "assign"; assignment: WireDamage[] }
  | { kind: "divide_spell"; assignment: WireSpellDamage[] }
  | { kind: "arrange"; top: number[]; bottom: number[] }
  | { kind: "search"; choice: number | null }
  | { kind: "sacrifice"; ids: number[] }
  | { kind: "discard"; cards: number[] }
  | { kind: "put_land"; choice: number | null }
  | { kind: "put_creature"; choice: number | null }
  | { kind: "choose_exiled"; choice: number | null }
  | { kind: "select_top"; cards: number[] }
  | { kind: "mode"; mode: number }
  | { kind: "target_players"; players: number[] }
  | { kind: "distribute"; to_hand: number[]; to_bottom: number[]; to_exile_may_play: number[] }
  | { kind: "shuffle_gy"; cards: number[] }
  | { kind: "choose_exiled_cast"; choice: number | null }
  | { kind: "choose_exiled_dig"; choice: number | null }
  | { kind: "trigger_modes"; modes: WireModeChoice[] }
  | { kind: "mana_color"; color: number }
  | { kind: "creature_type"; subtype: string }
  | { kind: "color"; color: number }
  | { kind: "opponent_pile"; pile: number }
  | { kind: "revealed"; choice: number | null }
  | { kind: "copy_target"; copy: number | null }
  | { kind: "attach_host"; host: number | null }
  | { kind: "keep_tapped"; ids: number[] }
  | { kind: "top_or_bottom"; top: boolean }
  | { kind: "return_land"; land: number | null }
  | { kind: "cast_face_down_choice"; choice: number | null }
  | { kind: "dredge"; dredger: number | null }
  | { kind: "hand_on_top"; cards: number[] }
  | { kind: "draw_count"; count: number }
  | { kind: "pay_amount"; amount: number }
  | { kind: "name"; name: string };

export function chooseTargetIsCardPick(
  items: ReadonlyArray<{ id?: number; label?: string; player?: number | null }>,
): boolean {
  return items.length > 0 && items.every((it) => it.player == null);
}

function pickSingleCard(picked: number[]): number | null {
  if (picked.length === 0) return null;
  if (picked.length === 1) return picked[0] ?? null;
  return null;
}

function choiceItemTarget(item: ChoiceItemLike): { kind: "object"; id: number } | { kind: "player"; player: number } {
  if (item.player != null) {
    return { kind: "player", player: item.player };
  }
  return { kind: "object", id: item.id };
}

export function choiceDraftKey(pc: PendingChoiceView): string {
  const base = `${pc.kind}:${pc.player}`;
  if ("items" in pc && Array.isArray(pc.items)) {
    return `${base}:${pc.items.map((it) => it.id).join(",")}`;
  }
  if ("labels" in pc && Array.isArray(pc.labels)) {
    return `${base}:${pc.labels.join("|")}`;
  }
  return base;
}

export function initPromptDraft(pc: PendingChoiceView, state: VisibleState): PromptDraft {
  switch (pc.kind) {
    case "order_triggers":
      return { kind: "order", order: pc.labels.map((_, i) => i) };
    case "assign_combat_damage": {
      const power = state.objects.find((o) => o.id === pc.source)?.power ?? 0;
      const first = pc.items[0]?.id;
      return { kind: "damage", amounts: first != null ? { [first]: power } : {} };
    }
    case "divide_spell_damage":
      return { kind: "divide", amounts: pc.items.length > 0 ? { 0: pc.total } : {} };
    case "divide_counters": {
      const first = pc.items[0]?.id;
      return { kind: "damage", amounts: first != null ? { [first]: pc.total } : {} };
    }
    case "may_yes_no":
    case "dance_exile_more":
    case "trade_secrets_repeat":
      return { kind: "may", yes: false };
    case "pay_cost":
    case "pay_or_counter":
    case "pay_or_controller_draws":
    case "pay_echo_or_sacrifice":
    case "pay_recover_or_exile":
    case "sacrifice_unless_pay":
      return { kind: "pay", pay: false };
    case "choose_mode":
      return { kind: "mode", mode: 0 };
    case "choose_trigger_modes":
      return { kind: "modes", modes: [] };
    case "choose_target_players":
    case "choose_splitting_opponent":
      return { kind: "player-pick", players: [] };
    case "opponent_chooses_pile":
    case "choose_pile_for_hand":
      return { kind: "pile", pile: 0 };
    case "distribute_top":
      return {
        kind: "partition",
        buckets: { to_hand: [], to_bottom: [], to_exile_may_play: [] },
      };
    case "partition_revealed":
      return { kind: "partition", buckets: { pile_a: [] } };
    case "choose_color":
    case "choose_mana_color":
      return { kind: "color", color: 0 };
    case "choose_creature_type":
      return { kind: "string", value: pc.options[0] ?? "" };
    case "may_draw_up_to":
      return { kind: "number", count: 0 };
    case "trade_secrets_caster_draw":
      return { kind: "number", count: pc.max };
    case "pay_any_amount_of_mana":
      return { kind: "number", count: 0 };
    case "choose_card_name":
      return { kind: "string", value: "" };
    case "choose_countered_spell_destination":
      return { kind: "destination", choice: false };
    case "revealed_card_to_battlefield_or_hand":
      return { kind: "destination", choice: null };
    default:
      return { kind: "card-pick", picked: [] };
  }
}

export function answerFromDraft(pc: PendingChoiceView, draft: PromptDraft): AnswerInput | null {
  switch (pc.kind) {
    case "order_triggers":
      if (draft.kind !== "order") return null;
      return { kind: "order", order: draft.order };
    case "assign_combat_damage":
      if (draft.kind !== "damage") return null;
      return {
        kind: "assign",
        assignment: pc.items.map((it) => ({ blocker: it.id, amount: draft.amounts[it.id] ?? 0 })),
      };
    case "divide_counters":
      if (draft.kind !== "damage") return null;
      return {
        kind: "assign",
        assignment: pc.items.map((it) => ({ blocker: it.id, amount: draft.amounts[it.id] ?? 0 })),
      };
    case "divide_spell_damage": {
      if (draft.kind !== "divide") return null;
      const assignment = pc.items.map((item, index) => ({
        amount: draft.amounts[index] ?? 0,
        target: choiceItemTarget(item),
      }));
      if (assignment.some((entry) => entry.amount < 0)) return null;
      const total = assignment.reduce((sum, entry) => sum + entry.amount, 0);
      if (total !== pc.total) return null;
      return { kind: "divide_spell", assignment };
    }
    case "may_yes_no":
    case "dance_exile_more":
    case "trade_secrets_repeat":
      if (draft.kind !== "may") return null;
      return { kind: "may", yes: draft.yes };
    case "pay_cost":
    case "pay_or_counter":
    case "pay_or_controller_draws":
    case "pay_echo_or_sacrifice":
    case "pay_recover_or_exile":
    case "sacrifice_unless_pay":
      if (draft.kind !== "pay") return null;
      return { kind: "pay", pay: draft.pay };
    case "choose_mode":
      if (draft.kind !== "mode") return null;
      return { kind: "mode", mode: draft.mode };
    case "choose_trigger_modes":
      if (draft.kind !== "modes") return null;
      return { kind: "trigger_modes", modes: draft.modes };
    case "choose_target_players":
      if (draft.kind !== "player-pick") return null;
      return { kind: "target_players", players: draft.players };
    case "choose_splitting_opponent":
      if (draft.kind === "player-pick") {
        if (draft.players.length !== 1) return null;
        return { kind: "target", id: 0, player: draft.players[0] };
      }
      if (draft.kind !== "target" || draft.player == null) return null;
      return { kind: "target", id: draft.id, player: draft.player };
    case "opponent_chooses_pile":
    case "choose_pile_for_hand":
      if (draft.kind !== "pile") return null;
      return { kind: "opponent_pile", pile: draft.pile };
    case "distribute_top":
      if (draft.kind !== "partition") return null;
      return {
        kind: "distribute",
        to_hand: draft.buckets.to_hand ?? [],
        to_bottom: draft.buckets.to_bottom ?? [],
        to_exile_may_play: draft.buckets.to_exile_may_play ?? [],
      };
    case "partition_revealed":
      if (draft.kind === "partition") {
        return { kind: "sacrifice", ids: draft.buckets.pile_a ?? [] };
      }
      if (draft.kind !== "card-pick") return null;
      return { kind: "sacrifice", ids: draft.picked };
    case "choose_color":
      if (draft.kind !== "color") return null;
      return { kind: "color", color: draft.color };
    case "choose_mana_color":
      if (draft.kind !== "color") return null;
      return { kind: "mana_color", color: draft.color };
    case "choose_creature_type":
      if (draft.kind !== "string") return null;
      return { kind: "creature_type", subtype: draft.value };
    case "may_draw_up_to":
    case "trade_secrets_caster_draw":
      if (draft.kind !== "number") return null;
      return { kind: "draw_count", count: draft.count };
    case "pay_any_amount_of_mana":
      if (draft.kind !== "number") return null;
      return { kind: "pay_amount", amount: draft.count };
    case "choose_card_name": {
      if (draft.kind !== "string") return null;
      const name = draft.value.trim();
      if (name === "") return null;
      return { kind: "name", name };
    }
    case "choose_countered_spell_destination":
      if (draft.kind !== "destination" || typeof draft.choice !== "boolean") return null;
      return { kind: "top_or_bottom", top: draft.choice };
    case "revealed_card_to_battlefield_or_hand":
      if (draft.kind !== "destination") return null;
      if (typeof draft.choice === "boolean") {
        return { kind: "revealed", choice: draft.choice ? pc.item.id : null };
      }
      if (draft.choice == null || typeof draft.choice === "number") {
        return { kind: "revealed", choice: draft.choice };
      }
      return null;
    case "scry":
    case "surveil": {
      if (draft.kind !== "card-pick") return null;
      const all = pc.items.map((it) => it.id);
      const top = draft.picked.filter((id) => all.includes(id));
      const bottom = all.filter((id) => !top.includes(id));
      return { kind: "arrange", top, bottom };
    }
    case "search_library":
      if (draft.kind !== "card-pick") return null;
      return { kind: "search", choice: pickSingleCard(draft.picked) };
    case "select_from_top":
      if (draft.kind !== "card-pick") return null;
      return { kind: "select_top", cards: draft.picked };
    case "discard":
      if (draft.kind !== "card-pick") return null;
      return { kind: "discard", cards: draft.picked };
    case "sacrifice_edict":
    case "proliferate":
    case "choose_own_sacrifices":
    case "phase_out":
    case "may_sacrifice":
    case "devour":
    case "exile_from_graveyard":
    case "caster_keep_permanents":
    case "choose_counter_target_for_player":
    case "may_return_from_graveyard":
    case "may_discard":
    case "choose_exiled_to_cast_free":
    case "pay_cumulative_upkeep_or_sacrifice":
      if (draft.kind !== "card-pick") return null;
      return { kind: "sacrifice", ids: draft.picked };
    case "choose_target":
      if (draft.kind === "target") return { kind: "target", id: draft.id, player: draft.player };
      if (draft.kind !== "card-pick") return null;
      if (draft.picked.length === 0) return pc.optional ? { kind: "targets", ids: [] } : null;
      if (draft.picked.length > pc.max) return null;
      return { kind: "targets", ids: draft.picked };
    case "choose_spell_targets":
    case "choose_ability_targets":
    case "choose_activation_cost_targets":
      if (draft.kind === "targets") return { kind: "targets", ids: draft.ids };
      if (draft.kind !== "card-pick") return null;
      return { kind: "targets", ids: draft.picked };
    case "decline_untap":
      if (draft.kind !== "card-pick") return null;
      return { kind: "keep_tapped", ids: draft.picked };
    case "sacrifice_unless_return_land":
      if (draft.kind !== "card-pick") return null;
      return { kind: "return_land", land: pickSingleCard(draft.picked) };
    case "put_land_from_hand":
      if (draft.kind !== "card-pick") return null;
      return { kind: "put_land", choice: pickSingleCard(draft.picked) };
    case "put_creature_from_hand":
      if (draft.kind !== "card-pick") return null;
      return { kind: "put_creature", choice: pickSingleCard(draft.picked) };
    case "choose_exiled_with_card":
    case "opponent_chooses_exiled_nonland":
    case "opponent_chooses_revealed_to_graveyard":
      if (draft.kind !== "card-pick") return null;
      return { kind: "choose_exiled", choice: pickSingleCard(draft.picked) };
    case "choose_exiled_with_card_to_cast":
      if (draft.kind !== "card-pick") return null;
      return { kind: "choose_exiled_cast", choice: pickSingleCard(draft.picked) };
    case "choose_exiled_dig_to_cast_free":
      if (draft.kind !== "card-pick") return null;
      return { kind: "choose_exiled_dig", choice: pickSingleCard(draft.picked) };
    case "choose_copy_target":
      if (draft.kind !== "card-pick") return null;
      return { kind: "copy_target", copy: pickSingleCard(draft.picked) };
    case "choose_attach_host":
      if (draft.kind !== "card-pick") return null;
      return { kind: "attach_host", host: pickSingleCard(draft.picked) };
    case "put_from_hand_on_top":
      if (draft.kind !== "card-pick") return null;
      return { kind: "hand_on_top", cards: draft.picked };
    case "choose_dredge":
      if (draft.kind !== "card-pick") return null;
      return { kind: "dredge", dredger: pickSingleCard(draft.picked) };
    case "cast_creature_face_down":
      if (draft.kind !== "card-pick") return null;
      return { kind: "cast_face_down_choice", choice: pickSingleCard(draft.picked) };
    case "shuffle_from_graveyard":
      if (draft.kind !== "card-pick") return null;
      return { kind: "shuffle_gy", cards: draft.picked };
    default:
      return null;
  }
}

export const buildAnswerFromDraft = answerFromDraft;

export function declineAnswer(pc: PendingChoiceView): AnswerInput | null {
  switch (pc.kind) {
    case "search_library":
      return { kind: "search", choice: null };
    case "put_land_from_hand":
      return { kind: "put_land", choice: null };
    case "put_creature_from_hand":
      return { kind: "put_creature", choice: null };
    case "choose_exiled_with_card":
    case "opponent_chooses_exiled_nonland":
    case "opponent_chooses_revealed_to_graveyard":
      return { kind: "choose_exiled", choice: null };
    case "choose_exiled_with_card_to_cast":
      return { kind: "choose_exiled_cast", choice: null };
    case "choose_exiled_dig_to_cast_free":
      return { kind: "choose_exiled_dig", choice: null };
    case "choose_attach_host":
      return pc.optional ? { kind: "attach_host", host: null } : null;
    case "choose_target":
      return pc.optional ? { kind: "targets", ids: [] } : null;
    case "pay_cumulative_upkeep_or_sacrifice":
      // Empty sacrifices = decline payment (engine sacrifices the permanent).
      return { kind: "sacrifice", ids: [] };
    default:
      return null;
  }
}

export function cardPickRequiredCount(pc: PendingChoiceView): number | null {
  switch (pc.kind) {
    case "search_library":
    case "put_land_from_hand":
    case "put_creature_from_hand":
    case "choose_exiled_with_card":
    case "choose_exiled_with_card_to_cast":
    case "choose_exiled_dig_to_cast_free":
    case "opponent_chooses_exiled_nonland":
    case "opponent_chooses_revealed_to_graveyard":
    case "sacrifice_unless_return_land":
    case "choose_copy_target":
    case "choose_attach_host":
    case "cast_creature_face_down":
      return 1;
    case "choose_target":
      // Up to `max` targets (CR 601.2c — "up to two target lands"); `1` for the common case.
      return pc.max;
    case "discard":
    case "put_from_hand_on_top":
    case "pay_cumulative_upkeep_or_sacrifice":
    case "choose_exiled_to_cast_free":
      return pc.count;
    case "choose_own_sacrifices":
      return pc.count;
    case "sacrifice_edict":
      return pc.keep_one ? Math.max(0, pc.items.length - 1) : 1;
    case "scry":
    case "surveil":
    case "proliferate":
    case "phase_out":
    case "select_from_top":
    case "shuffle_from_graveyard":
    case "may_sacrifice":
    case "devour":
    case "exile_from_graveyard":
    case "caster_keep_permanents":
    case "choose_counter_target_for_player":
    case "may_return_from_graveyard":
    case "may_discard":
    case "choose_dredge":
      return null;
    default:
      return null;
  }
}

export function cardPickReady(pc: PendingChoiceView, picked: number[]): boolean {
  // Up-to-max targeting: any 1..=max (or 0 when optional) is a submittable answer.
  if (pc.kind === "choose_target") {
    if (picked.length > pc.max) return false;
    return pc.optional || picked.length >= 1;
  }
  const required = cardPickRequiredCount(pc);
  if (required != null) return picked.length === required;
  if (pc.kind === "select_from_top") return picked.length <= pc.up_to;
  return true;
}

export function damageAssignReady(pc: PendingChoiceView, draft: PromptDraft, state: VisibleState): boolean {
  if (draft.kind !== "damage") return false;
  const assigned = Object.values(draft.amounts).reduce((sum, amount) => sum + amount, 0);
  if (pc.kind === "assign_combat_damage") {
    const power = state.objects.find((o) => o.id === pc.source)?.power ?? 0;
    return assigned === power;
  }
  if (pc.kind === "divide_counters") {
    return assigned === pc.total;
  }
  return false;
}

export function choiceIntent(pc: PendingChoiceView, answer: AnswerInput): WireIntent {
  const player = pc.player;
  return Match.value(answer).pipe(
    Match.withReturnType<WireIntent>(),
    Match.discriminatorsExhaustive("kind")({
      order: (a) => ({ kind: "choose_order", player, order: a.order }),
      target: (a) => ({
        kind: "choose_targets",
        player,
        targets: [a.player != null ? { kind: "player", player: a.player } : { kind: "object", id: a.id }],
      }),
      targets: (a) => ({
        kind: "choose_targets",
        player,
        targets: a.ids.map((id) => ({ kind: "object", id })),
      }),
      may: (a) => ({ kind: "answer_may", player, yes: a.yes }),
      pay: (a) => ({ kind: "pay_optional_cost", player, pay: a.pay }),
      assign: (a) => ({ kind: "assign_damage", player, assignment: a.assignment }),
      divide_spell: (a) => ({ kind: "divide_spell_damage", player, assignment: a.assignment }),
      arrange: (a) => ({ kind: "arrange_top", player, top: a.top, bottom: a.bottom }),
      search: (a) => ({ kind: "search_library", player, choice: a.choice }),
      sacrifice: (a) => ({ kind: "choose_sacrifices", player, sacrifices: a.ids }),
      discard: (a) => ({ kind: "discard", player, cards: a.cards }),
      put_land: (a) => ({ kind: "put_land_from_hand", player, choice: a.choice }),
      put_creature: (a) => ({ kind: "put_creature_from_hand", player, choice: a.choice }),
      choose_exiled: (a) => ({ kind: "choose_exiled_with_card", player, choice: a.choice }),
      select_top: (a) => ({ kind: "select_from_top", player, cards: a.cards }),
      mode: (a) => ({ kind: "choose_mode", player, mode: a.mode }),
      target_players: (a) => ({ kind: "choose_target_players", player, players: a.players }),
      distribute: (a) => ({
        kind: "distribute_top",
        player,
        to_hand: a.to_hand,
        to_bottom: a.to_bottom,
        to_exile_may_play: a.to_exile_may_play,
      }),
      shuffle_gy: (a) => ({ kind: "shuffle_from_graveyard", player, cards: a.cards }),
      choose_exiled_cast: (a) => ({ kind: "choose_exiled_with_card_to_cast", player, choice: a.choice }),
      choose_exiled_dig: (a) => ({ kind: "choose_exiled_dig_to_cast_free", player, choice: a.choice }),
      trigger_modes: (a) => ({ kind: "choose_trigger_modes", player, modes: a.modes }),
      mana_color: (a) => ({ kind: "choose_mana_color", player, color: a.color }),
      creature_type: (a) => ({ kind: "choose_creature_type", player, subtype: a.subtype }),
      color: (a) => ({ kind: "choose_color", player, color: a.color }),
      opponent_pile: (a) => ({ kind: "choose_opponent_pile", player, pile: a.pile }),
      revealed: (a) => ({ kind: "revealed_card_to_battlefield_or_hand", player, choice: a.choice }),
      copy_target: (a) => ({ kind: "choose_copy_target", player, copy: a.copy }),
      attach_host: (a) => ({ kind: "choose_attach_host", player, host: a.host }),
      keep_tapped: (a) => ({ kind: "decline_untap", player, keep_tapped: a.ids }),
      top_or_bottom: (a) => ({ kind: "choose_top_or_bottom", player, top: a.top }),
      return_land: (a) => ({ kind: "return_land_or_sacrifice", player, land: a.land }),
      cast_face_down_choice: (a) => ({ kind: "cast_creature_face_down", player, choice: a.choice }),
      dredge: (a) => ({ kind: "choose_dredge", player, dredger: a.dredger }),
      hand_on_top: (a) => ({ kind: "put_from_hand_on_top", player, cards: a.cards }),
      draw_count: (a) => ({ kind: "choose_draw_count", player, count: a.count }),
      pay_amount: (a) => ({ kind: "pay_optional_cost", player, pay: a.amount > 0, x: a.amount }),
      name: (a) => ({ kind: "choose_card_name", player, name: a.name }),
    }),
  );
}
