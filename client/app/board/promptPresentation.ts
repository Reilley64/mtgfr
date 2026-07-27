import type { VisibleState } from "~/wire/types";
import {
  gyExileCostObjectIds,
  pendingBoardTargetMode,
  pendingDamageAssignBlockers,
  pendingDigCastHostMode,
  pendingDivideSpellObjectIndexes,
  pendingExilePickIds,
  pendingGraveyardPickIds,
  pendingHandPickIds,
  pendingPlayerAimSeats,
  sacrificeCostObjectIds,
  targetMode,
} from "./action/targeting";
import { ZONE } from "./geometry/layout";
import type { BoardModel } from "./submodel";

export type PromptPresentation =
  | { mode: "none" }
  | {
      mode: "simple";
      source: "pending" | "local";
      /** true → coach stays bottom-docked board-aim chrome */
      boardAim: boolean;
    }
  | { mode: "modal"; source: "pending" | "local" };

function simple(source: "pending" | "local", boardAim: boolean): PromptPresentation {
  return { mode: "simple", source, boardAim };
}

function modal(source: "pending" | "local"): PromptPresentation {
  return { mode: "modal", source };
}

function none(): PromptPresentation {
  return { mode: "none" };
}

function viewerIsSeated(state: VisibleState): boolean {
  return state.players.some((player) => player.player === state.viewer);
}

function localDiscardPickIsHandAim(board: NonNullable<BoardModel["discardPick"]>, state: VisibleState): boolean {
  const choices = board.action.discard_choices ?? [];
  if (choices.length === 0) return false;

  const handIds = new Set(
    state.objects
      .filter((object) => object.zone === ZONE.Hand && object.owner === state.viewer)
      .map((object) => object.id),
  );

  return choices.every((id) => handIds.has(id));
}

function localPresentation(board: BoardModel, state: VisibleState): PromptPresentation | null {
  if (board.playModePick != null) return simple("local", false);
  if (board.xPrompt != null) return modal("local");
  if (board.modalCast != null) return simple("local", board.modalCast.chosen != null);

  if (board.sacrificePick != null) {
    const choices = board.sacrificePick.action.sacrifice_choices ?? [];
    return sacrificeCostObjectIds(choices, state) != null ? simple("local", true) : modal("local");
  }

  if (board.discardPick != null) {
    return localDiscardPickIsHandAim(board.discardPick, state) ? simple("local", true) : modal("local");
  }

  if (board.gyExilePick != null) {
    const choices = board.gyExilePick.action.graveyard_exile_choices ?? [];
    return gyExileCostObjectIds(choices, state) != null ? simple("local", true) : modal("local");
  }

  if (board.staged != null) {
    return targetMode(board.staged.action, state).kind === "pick" ? modal("local") : none();
  }

  return null;
}

function pendingBoardAimPresentation(board: BoardModel, state: VisibleState): PromptPresentation | null {
  const pending = state.pending_choice;
  if (pending == null) return null;

  if (pendingGraveyardPickIds(pending, state) != null) return simple("pending", true);
  if (pendingExilePickIds(pending, state) != null) return simple("pending", true);
  if (pendingHandPickIds(pending, state) != null) return simple("pending", true);
  if (pendingBoardTargetMode(pending, state) != null) return simple("pending", true);
  if (pendingDigCastHostMode(pending, state, board.promptDraft) != null) return simple("pending", true);
  if (pendingPlayerAimSeats(pending, state) != null) return simple("pending", true);
  if (pendingDivideSpellObjectIndexes(pending, state) != null) return simple("pending", true);
  if (pendingDamageAssignBlockers(pending, state) != null) return simple("pending", true);

  if (pending.kind === "opponent_chooses_revealed_to_graveyard") {
    return simple("pending", true);
  }

  return null;
}

export function promptPresentation(board: BoardModel, state: VisibleState): PromptPresentation {
  const local = localPresentation(board, state);
  if (local != null) return local;

  const pending = state.pending_choice;
  if (pending == null) return none();
  if (!viewerIsSeated(state)) return none();
  if (pending.player !== state.viewer) return none();

  const boardAim = pendingBoardAimPresentation(board, state);
  if (boardAim != null) return boardAim;

  switch (pending.kind) {
    case "may_yes_no":
    case "dance_exile_more":
    case "choose_mode":
    case "choose_trigger_modes":
    case "choose_pile_for_hand":
    case "opponent_chooses_pile":
    case "choose_countered_spell_destination":
    case "revealed_card_to_battlefield_or_hand":
    case "pay_cost":
    case "pay_or_counter":
    case "pay_or_controller_draws":
    case "pay_echo_or_sacrifice":
    case "pay_recover_or_exile":
    case "sacrifice_unless_pay":
    case "pay_life_or_enters_tapped":
      return simple("pending", false);
    case "search_library":
    case "choose_color":
    case "choose_mana_color":
    case "choose_creature_type":
    case "choose_card_name":
    case "order_triggers":
    case "scry":
    case "surveil":
    case "select_from_top":
    case "distribute_top":
    case "partition_revealed":
    case "may_draw_up_to":
    case "pay_any_amount_of_mana":
    case "choose_target":
    case "sacrifice_edict":
    case "choose_own_sacrifices":
    case "may_sacrifice":
    case "devour":
    case "proliferate":
    case "phase_out":
    case "decline_untap":
    case "choose_attach_host":
    case "choose_legendary_keep":
    case "sacrifice_unless_return_land":
    case "choose_copy_target":
    case "choose_counter_target_for_player":
    case "caster_keep_permanents":
    case "assign_combat_damage":
    case "divide_counters":
    case "divide_spell_damage":
    case "choose_target_players":
    case "choose_splitting_opponent":
    case "exile_from_graveyard":
    case "may_return_from_graveyard":
    case "may_exile_discarded_to_play":
    case "shuffle_from_graveyard":
    case "choose_dredge":
    case "choose_activation_cost_targets":
    case "choose_exiled_with_card":
    case "choose_exiled_with_card_to_cast":
    case "choose_exiled_dig_to_cast_free":
    case "opponent_chooses_exiled_nonland":
    case "choose_exiled_to_cast_free":
    case "discard":
    case "may_discard":
    case "put_land_from_hand":
    case "put_creature_from_hand":
    case "put_from_hand_on_top":
    case "cast_creature_face_down":
    case "opponent_chooses_revealed_to_graveyard":
      return modal("pending");
    default:
      return modal("pending");
  }
}
