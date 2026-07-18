// PendingChoice client module: answer packing + prompt chrome helpers.
// Forms stay dumb — they collect AnswerInput; choiceIntent maps to WireIntent (ADR 0006).
// Schema counterpart: `crates/schema/src/answer_protocol.rs` (`encode_answer`) — keep in sync.

import * as Match from "effect/Match";
import type { PendingChoiceView, VisibleState, WireDamage, WireIntent, WireModeChoice } from "~/wire/types";

/** The viewer's pending choice, if any. */
export function myChoice(state: VisibleState, me: number): PendingChoiceView | null {
  const pc = state.pending_choice;
  return pc && pc.player === me ? pc : null;
}

/**
 * Identity for a pending choice surface: kind + answering seat. Useful when remounting a form
 * only when the choice *type* changes (not on every same-kind delta).
 */
export function choiceShowKey(state: VisibleState, me: number): string | false {
  const c = myChoice(state, me);
  return c ? `${c.kind}:${c.player}` : false;
}

/**
 * Whether a choose_target prompt can use the card-image picker. Player seats (Bojuka Bog's
 * "exile target player's graveyard") have no art — those need the life-orb PickDialog instead.
 */
export function chooseTargetIsCardPick(
  items: ReadonlyArray<{ id?: number; label?: string; player?: number | null }>,
): boolean {
  return items.length > 0 && items.every((it) => it.player == null);
}

// What a prompt form produces — the raw answer, tagged so the mapping (and TypeScript) can tell the
// choices apart. One per PendingChoice kind that needs an answer.
export type AnswerInput =
  | { kind: "order"; order: number[] } // order_triggers
  | { kind: "target"; id: number; player?: number } // choose_target (object or player seat)
  | { kind: "targets"; ids: number[] } // choose_spell_targets (object ids)
  | { kind: "may"; yes: boolean } // may_yes_no
  | { kind: "pay"; pay: boolean } // pay_cost
  | { kind: "assign"; assignment: WireDamage[] } // assign_combat_damage
  | { kind: "arrange"; top: number[]; bottom: number[] } // scry / surveil
  | { kind: "search"; choice: number | null } // search_library (null = fail to find)
  | { kind: "sacrifice"; ids: number[] } // sacrifice_edict
  | { kind: "discard"; cards: number[] } // discard
  | { kind: "put_land"; choice: number | null } // put_land_from_hand (null = decline)
  | { kind: "choose_exiled"; choice: number | null } // choose_exiled_with_card (null = decline)
  | { kind: "select_top"; cards: number[] } // select_from_top
  | { kind: "mode"; mode: number } // choose_mode
  | { kind: "target_players"; players: number[] } // choose_target_players (chosen seats)
  | { kind: "distribute"; to_hand: number[]; to_bottom: number[]; to_exile_may_play: number[] } // distribute_top
  | { kind: "shuffle_gy"; cards: number[] } // shuffle_from_graveyard (subset)
  | { kind: "choose_exiled_cast"; choice: number | null } // choose_exiled_with_card_to_cast (null = decline)
  | { kind: "choose_exiled_dig"; choice: number | null } // choose_exiled_dig_to_cast_free (null = decline)
  | { kind: "trigger_modes"; modes: WireModeChoice[] } // choose_trigger_modes (empty = decline)
  | { kind: "mana_color"; color: number } // choose_mana_color (WUBRG index)
  | { kind: "creature_type"; subtype: string } // choose_creature_type
  | { kind: "color"; color: number } // choose_color (WUBRG index)
  | { kind: "opponent_pile"; pile: number } // opponent_chooses_pile (0 or 1)
  | { kind: "revealed"; choice: number | null } // revealed_card_to_battlefield_or_hand
  | { kind: "copy_target"; copy: number | null } // choose_copy_target (null = decline the "you may")
  | { kind: "attach_host"; host: number | null } // choose_attach_host (null = decline, Equipment's optional host)
  | { kind: "keep_tapped"; ids: number[] } // decline_untap (empty = untap everything)
  | { kind: "top_or_bottom"; top: boolean } // choose_countered_spell_destination
  | { kind: "return_land"; land: number | null } // sacrifice_unless_return_land (null = sacrifice)
  | { kind: "cast_face_down_choice"; choice: number | null }; // cast_creature_face_down (null = decline)

/** Map a pending choice and the player's answer to the wire intent that answers it. `pc` supplies
 * the answering `player`; the intent shape follows from the answer's tag. `discriminatorsExhaustive`
 * is the compile-time gate — a new `AnswerInput` kind without an arm here is a build failure. */
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
      choose_exiled: (a) => ({
        kind: "choose_exiled_with_card",
        player,
        choice: a.choice,
      }),
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
    }),
  );
}
