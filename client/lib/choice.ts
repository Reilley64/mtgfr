// PendingChoice answer packing + prompt draft helpers.
// Forms collect AnswerInput; choiceIntent maps to WireIntent (wire-protocol-and-visibility spec).

import * as Match from "effect/Match";
import type { PendingChoiceView, VisibleState, WireDamage, WireIntent, WireModeChoice } from "~/wire/types";

export type PromptDraft =
  | { kind: "card-pick"; picked: number[] }
  | { kind: "order"; order: number[] }
  | { kind: "damage"; amounts: Record<number, number> };

export type AnswerInput =
  | { kind: "order"; order: number[] }
  | { kind: "target"; id: number; player?: number }
  | { kind: "targets"; ids: number[] }
  | { kind: "may"; yes: boolean }
  | { kind: "pay"; pay: boolean }
  | { kind: "assign"; assignment: WireDamage[] }
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
  | { kind: "draw_count"; count: number };

export function chooseTargetIsCardPick(
  items: ReadonlyArray<{ id?: number; label?: string; player?: number | null }>,
): boolean {
  return items.length > 0 && items.every((it) => it.player == null);
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
    default:
      return { kind: "card-pick", picked: [] };
  }
}

export function buildAnswerFromDraft(pc: PendingChoiceView, draft: PromptDraft): AnswerInput | null {
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
      return { kind: "search", choice: draft.picked[0] ?? null };
    case "select_from_top":
      if (draft.kind !== "card-pick") return null;
      return { kind: "select_top", cards: draft.picked };
    case "discard":
      if (draft.kind !== "card-pick") return null;
      return { kind: "discard", cards: draft.picked };
    case "choose_target":
      if (draft.kind !== "card-pick" || draft.picked.length !== 1) return null;
      return { kind: "target", id: draft.picked[0] };
    case "sacrifice_edict":
    case "proliferate":
    case "choose_own_sacrifices":
    case "phase_out":
      if (draft.kind !== "card-pick") return null;
      return { kind: "sacrifice", ids: draft.picked };
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
    case "choose_target":
      return 1;
    case "discard":
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
      return null;
    default:
      return null;
  }
}

export function cardPickReady(pc: PendingChoiceView, picked: number[]): boolean {
  const required = cardPickRequiredCount(pc);
  if (required != null) return picked.length === required;
  if (pc.kind === "select_from_top") return picked.length <= pc.up_to;
  return true;
}

export function damageAssignReady(pc: PendingChoiceView, draft: PromptDraft, state: VisibleState): boolean {
  if (draft.kind !== "damage" || pc.kind !== "assign_combat_damage") return false;
  const power = state.objects.find((o) => o.id === pc.source)?.power ?? 0;
  const assigned = Object.values(draft.amounts).reduce((s, n) => s + n, 0);
  return assigned === power;
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
    }),
  );
}

export const FAITHFUL_PROMPT_KINDS = new Set<PendingChoiceView["kind"]>([
  "may_yes_no",
  "order_triggers",
  "search_library",
  "scry",
  "surveil",
  "select_from_top",
  "discard",
  "sacrifice_edict",
  "choose_own_sacrifices",
  "assign_combat_damage",
  "proliferate",
  "choose_mode",
  "choose_target",
]);

export function isFaithfulPromptKind(kind: PendingChoiceView["kind"]): boolean {
  return FAITHFUL_PROMPT_KINDS.has(kind);
}
