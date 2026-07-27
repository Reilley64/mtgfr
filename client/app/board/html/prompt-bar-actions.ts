import { type Html, html } from "foldkit/html";
import {
  type AnswerInput,
  buildAnswerFromDraft,
  cardPickReady,
  choiceIntent,
  damageAssignReady,
  declineAnswer,
  initPromptDraft,
} from "~/choice";
import { priorityPrimaryClass } from "~/priorityContextChrome";
import { gameButtonClass } from "~/ui/buttonClass";
import type { MessageRef, PendingChoiceView, VisibleState } from "~/wire/types";
import { costText } from "~/xCost";
import { formatMessage } from "../../domain/i18n/message";
import { modeAvailable } from "../action/modal";
import {
  gyExileCostObjectIds,
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
  pendingPlayerAimOneClick,
  pendingPlayerAimSeats,
  pendingTargetOneClick,
  sacrificeCostObjectIds,
} from "../action/targeting";
import { ZONE } from "../geometry/layout";
import {
  CancelActionClicked,
  DiscardCostConfirmed,
  GyExileConfirmed,
  type Message,
  ModalModesChosen,
  ModalModeToggled,
  PendingChoiceAnswered,
  PlayModeChosen,
  PromptSubmitted,
} from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();
type SimplePayPending = Extract<
  PendingChoiceView,
  {
    kind:
      | "pay_cost"
      | "pay_or_counter"
      | "pay_or_controller_draws"
      | "pay_echo_or_sacrifice"
      | "pay_recover_or_exile"
      | "sacrifice_unless_pay"
      | "pay_life_or_enters_tapped";
  }
>;
type SimpleModePending = Extract<PendingChoiceView, { kind: "choose_mode" | "choose_trigger_modes" }>;
type SimplePilePending = Extract<PendingChoiceView, { kind: "opponent_chooses_pile" | "choose_pile_for_hand" }>;
type SimpleDestinationPending = Extract<
  PendingChoiceView,
  { kind: "choose_countered_spell_destination" | "revealed_card_to_battlefield_or_hand" }
>;

function messageText(message: MessageRef | null | undefined): string {
  return formatMessage(message);
}

function barRow(actions: ReadonlyArray<Html>): Html {
  return h.div([h.Class("flex flex-row-reverse flex-wrap items-center justify-end gap-sm")], actions);
}

function barStack(actions: ReadonlyArray<Html>): Html {
  return h.div([h.Class("flex max-w-[min(100vw-2rem,24rem)] flex-col items-end gap-sm")], actions);
}

function barButton(
  testId: string,
  label: string,
  onClick: Message,
  primary: boolean,
  disabled = false,
  pressed: boolean | null = null,
): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.Disabled(disabled),
      ...(pressed == null ? [] : [h.AriaPressed(pressed ? "true" : "false")]),
      h.OnClick(onClick),
      h.Class(gameButtonClass(primary ? "game" : "game-quiet", primary ? priorityPrimaryClass(true) : null)),
    ],
    [label],
  );
}

function pendingBarButton(
  pending: PendingChoiceView,
  testId: string,
  label: string,
  answer: AnswerInput,
  primary: boolean,
  disabled: boolean,
): Html {
  return barButton(testId, label, PendingChoiceAnswered({ intent: choiceIntent(pending, answer) }), primary, disabled);
}

function submitBarButton(label: string, onClick: Message, disabled: boolean): Html {
  return barButton("prompt-submit", label, onClick, true, disabled);
}

function cancelBarButton(): Html {
  return barButton("prompt-cancel", "Cancel", CancelActionClicked(), false);
}

function payCostDeclineLabel(kind: SimplePayPending["kind"]): string {
  switch (kind) {
    case "pay_or_counter":
      return "Let it be countered";
    case "pay_life_or_enters_tapped":
      return "Enters tapped";
    case "pay_or_controller_draws":
      return "Let them draw";
    case "pay_echo_or_sacrifice":
    case "sacrifice_unless_pay":
      return "Sacrifice";
    case "pay_recover_or_exile":
      return "Exile";
    case "pay_cost":
      return "Don't pay";
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
    }
  }
}

function payCostBarActions(board: BoardModel, pending: SimplePayPending, tableId: string | null): Html {
  const shockland = pending.kind === "pay_life_or_enters_tapped";
  const payLabel = shockland ? `Pay ${pending.life} life` : `Pay ${costText(pending.cost)}`;
  const canPay = !("can_pay" in pending) || pending.can_pay;
  const discardNeed = pending.kind === "pay_cost" ? (pending.discard_count ?? 0) : 0;
  const draft = board.promptDraft;
  const picked = discardNeed > 0 && draft?.kind === "card-pick" ? draft.picked : [];
  const discardReady = discardNeed === 0 || picked.length === discardNeed;
  const payAnswer: AnswerInput =
    discardNeed > 0 ? { kind: "pay", pay: true, discard: picked } : { kind: "pay", pay: true };

  return barRow([
    pendingBarButton(pending, "prompt-pay", payLabel, payAnswer, true, tableId == null || !canPay || !discardReady),
    pendingBarButton(
      pending,
      "prompt-decline",
      payCostDeclineLabel(pending.kind),
      { kind: "pay", pay: false },
      false,
      tableId == null,
    ),
  ]);
}

function chooseModeBarActions(
  pending: Extract<SimpleModePending, { kind: "choose_mode" }>,
  tableId: string | null,
): Html {
  return barStack(
    pending.labels.map((label, index) =>
      pendingBarButton(
        pending,
        `prompt-mode-${index}`,
        messageText(label),
        { kind: "mode", mode: index },
        index === 0,
        tableId == null,
      ),
    ),
  );
}

function chooseTriggerModesBarActions(
  pending: Extract<SimpleModePending, { kind: "choose_trigger_modes" }>,
  board: BoardModel,
  state: VisibleState,
): Html {
  const draft = board.promptDraft ?? initPromptDraft(pending, state);
  const picked = draft.kind === "modes" ? draft.modes : [];
  const ready = picked.length === pending.choose || (pending.optional && picked.length === 0);

  return barRow([
    barButton("prompt-submit", "Choose", PromptSubmitted(), true, !ready),
    barButton("prompt-cancel", "Cancel", CancelActionClicked(), false),
  ]);
}

function pilePickBarActions(pending: SimplePilePending, tableId: string | null): Html {
  return barRow([
    pendingBarButton(pending, "prompt-pile-0", "Pile A", { kind: "opponent_pile", pile: 0 }, true, tableId == null),
    pendingBarButton(pending, "prompt-pile-1", "Pile B", { kind: "opponent_pile", pile: 1 }, false, tableId == null),
  ]);
}

function destinationBarActions(pending: SimpleDestinationPending, tableId: string | null): Html {
  if (pending.kind === "choose_countered_spell_destination") {
    return barRow([
      pendingBarButton(
        pending,
        "prompt-destination-top",
        "Top",
        { kind: "top_or_bottom", top: true },
        true,
        tableId == null,
      ),
      pendingBarButton(
        pending,
        "prompt-destination-bottom",
        "Bottom",
        { kind: "top_or_bottom", top: false },
        false,
        tableId == null,
      ),
    ]);
  }

  return barRow([
    pendingBarButton(
      pending,
      "prompt-destination-battlefield",
      "Battlefield",
      { kind: "revealed", choice: pending.item.id },
      true,
      tableId == null,
    ),
    pendingBarButton(
      pending,
      "prompt-destination-hand",
      "Hand",
      { kind: "revealed", choice: null },
      false,
      tableId == null,
    ),
  ]);
}

function boardAimDeclineLabel(pending: PendingChoiceView): string | null {
  switch (pending.kind) {
    case "put_land_from_hand":
      return "Don't put a land";
    case "put_creature_from_hand":
      return "Don't put a creature";
    case "choose_exiled_with_card":
    case "opponent_chooses_exiled_nonland":
    case "opponent_chooses_revealed_to_graveyard":
      return "Choose none";
    case "choose_exiled_with_card_to_cast":
    case "choose_exiled_dig_to_cast_free":
      return "Don't cast";
    case "may_return_from_graveyard":
      return pending.mandatory ? null : "Don't return";
    case "may_exile_discarded_to_play":
      return "Don't exile";
    case "choose_attach_host":
      return pending.optional ? "Don't attach" : null;
    case "choose_target":
      return pending.min === 0 ? "No target" : null;
    case "pay_cumulative_upkeep_or_sacrifice":
      return "Don't pay";
    case "choose_dredge":
      return "Draw normally";
    default:
      return null;
  }
}

function graveyardSubmitLabel(kind: PendingChoiceView["kind"]): string {
  switch (kind) {
    case "exile_from_graveyard":
      return "Exile";
    case "may_return_from_graveyard":
      return "Return";
    case "may_exile_discarded_to_play":
      return "Exile";
    case "shuffle_from_graveyard":
      return "Shuffle";
    case "pay_cumulative_upkeep_or_sacrifice":
      return "Pay";
    default:
      return "Confirm";
  }
}

function handSubmitLabel(kind: PendingChoiceView["kind"]): string {
  switch (kind) {
    case "may_discard":
      return "Continue";
    case "put_from_hand_on_top":
      return "Put on top";
    default:
      return "Discard";
  }
}

function onHandDiscardCost(board: BoardModel, state: VisibleState): boolean {
  const choices = board.discardPick?.action.discard_choices ?? [];
  if (choices.length === 0) return false;
  const handIds = new Set(
    state.objects
      .filter((object) => object.zone === ZONE.Hand && object.owner === state.viewer)
      .map((object) => object.id),
  );
  return choices.every((id) => handIds.has(id));
}

function localBoardAimBarActions(board: BoardModel, state: VisibleState): Html | null {
  if (board.modalCast?.chosen != null) {
    return barRow([cancelBarButton()]);
  }

  if (board.sacrificePick != null) {
    const choices = board.sacrificePick.action.sacrifice_choices ?? [];
    if (sacrificeCostObjectIds(choices, state) != null) {
      return barRow([cancelBarButton()]);
    }
  }

  if (board.discardPick != null && onHandDiscardCost(board, state)) {
    const ready = board.discardPick.picks.discard_cost.length === 1;
    return barRow([submitBarButton("Confirm", DiscardCostConfirmed(), !ready), cancelBarButton()]);
  }

  if (board.gyExilePick != null) {
    const choices = board.gyExilePick.action.graveyard_exile_choices ?? [];
    if (gyExileCostObjectIds(choices, state) != null) {
      const min = board.gyExilePick.action.graveyard_exile_min ?? 0;
      const max = board.gyExilePick.action.graveyard_exile_max ?? 0;
      const selected = board.gyExilePick.picks.graveyard_exile;
      const oneClick = max <= 1;
      const ready = !oneClick && selected.length >= min && selected.length <= max;
      const actions: Html[] = [cancelBarButton()];
      if (!oneClick && min < max) {
        actions.unshift(submitBarButton("Exile", GyExileConfirmed(), !ready));
      }
      return barRow(actions);
    }
  }

  return null;
}

function pendingBoardAimBarActions(board: BoardModel, state: VisibleState, tableId: string | null): Html | null {
  const pending = state.pending_choice;
  if (pending == null) return null;
  if (pending.kind === "pay_cost") return null;

  if (pending.kind === "opponent_chooses_revealed_to_graveyard") {
    const decline = declineAnswer(pending);
    if (decline == null) return null;
    return barRow([
      pendingBarButton(
        pending,
        "prompt-decline",
        boardAimDeclineLabel(pending) ?? "Decline",
        decline,
        false,
        tableId == null,
      ),
    ]);
  }

  if (pendingGraveyardPickIds(pending, state) != null) {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const oneClick = pendingGraveyardPickOneClick(pending);
    const ready = !oneClick && cardPickReady(pending, picked);
    const actions: Html[] = [];
    if (!oneClick) {
      actions.push(submitBarButton(graveyardSubmitLabel(pending.kind), PromptSubmitted(), tableId == null || !ready));
    }
    const decline = declineAnswer(pending);
    if (decline != null) {
      actions.push(
        pendingBarButton(
          pending,
          "prompt-decline",
          boardAimDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return actions.length > 0 ? barRow(actions) : null;
  }

  const digHost = pendingDigCastHostMode(pending, state, board.promptDraft);
  if (pendingExilePickIds(pending, state) != null && digHost == null) {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const oneClick = pendingExilePickOneClick(pending);
    const ready = !oneClick && cardPickReady(pending, picked);
    const actions: Html[] = [];
    if (!oneClick) {
      actions.push(submitBarButton("Choose", PromptSubmitted(), tableId == null || !ready));
    }
    const decline = declineAnswer(pending);
    if (decline != null) {
      actions.push(
        pendingBarButton(
          pending,
          "prompt-decline",
          boardAimDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return actions.length > 0 ? barRow(actions) : null;
  }

  if (pendingHandPickIds(pending, state) != null) {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const oneClick = pendingHandPickOneClick(pending);
    const ready = !oneClick && cardPickReady(pending, picked);
    const actions: Html[] = [];
    if (!oneClick) {
      actions.push(submitBarButton(handSubmitLabel(pending.kind), PromptSubmitted(), tableId == null || !ready));
    }
    const decline = declineAnswer(pending);
    if (decline != null) {
      actions.push(
        pendingBarButton(
          pending,
          "prompt-decline",
          boardAimDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return actions.length > 0 ? barRow(actions) : null;
  }

  if (pendingBoardTargetMode(pending, state) != null || digHost != null) {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "card-pick" ? draft.picked : [];
    const oneClick = digHost != null || pendingTargetOneClick(pending);
    const ready = !oneClick && cardPickReady(pending, picked);
    const actions: Html[] = [];
    if (!oneClick) {
      actions.push(submitBarButton("Confirm", PromptSubmitted(), tableId == null || !ready));
    }
    const decline = declineAnswer(pending);
    if (decline != null) {
      actions.push(
        pendingBarButton(
          pending,
          "prompt-decline",
          boardAimDeclineLabel(pending) ?? "Decline",
          decline,
          false,
          tableId == null,
        ),
      );
    }
    return actions.length > 0 ? barRow(actions) : null;
  }

  if (pendingPlayerAimSeats(pending, state) != null) {
    const oneClick = pendingPlayerAimOneClick(pending);
    if (oneClick) return null;
    if (pending.kind !== "choose_target_players") return null;
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    const picked = draft.kind === "player-pick" ? draft.players : [];
    const ready = picked.length >= pending.min && picked.length <= pending.max;
    return barRow([submitBarButton("Confirm", PromptSubmitted(), tableId == null || !ready)]);
  }

  if (pending.kind === "assign_combat_damage" && pendingDamageAssignBlockers(pending, state) != null) {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    return barRow([
      submitBarButton("Assign", PromptSubmitted(), tableId == null || !damageAssignReady(pending, draft, state)),
    ]);
  }

  if (pending.kind === "divide_spell_damage" && pendingDivideSpellObjectIndexes(pending, state) != null) {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    return barRow([
      submitBarButton("Assign", PromptSubmitted(), tableId == null || buildAnswerFromDraft(pending, draft) == null),
    ]);
  }

  if (pending.kind === "divide_counters" && pendingDamageAssignBlockers(pending, state) != null) {
    const draft = board.promptDraft ?? initPromptDraft(pending, state);
    return barRow([
      submitBarButton("Assign", PromptSubmitted(), tableId == null || !damageAssignReady(pending, draft, state)),
    ]);
  }

  return null;
}

function playModeBarActions(pick: NonNullable<BoardModel["playModePick"]>): Html {
  return barStack([
    ...pick.modes.map((mode, index) =>
      barButton(`play-mode-${index}`, messageText(mode.label), PlayModeChosen({ actionId: mode.id }), index === 0),
    ),
    barButton("prompt-cancel", "Cancel", CancelActionClicked(), false),
  ]);
}

function modalModeBarActions(mc: NonNullable<BoardModel["modalCast"]>): Html | null {
  if (mc.chosen != null) return null;

  const choose = mc.action.modal?.choose ?? 1;
  const chooseMax = mc.action.modal?.choose_max ?? choose;
  const multi = chooseMax > 1;
  const picked = multi ? mc.modeDraft : [];
  const ready = multi ? picked.length >= choose && picked.length <= chooseMax : true;

  const modeButtons = mc.modes.map((mode, index) => {
    const available = modeAvailable(mode);
    if (multi) {
      const selected = picked.includes(index);
      return barButton(
        `modal-mode-${index}`,
        `${messageText(mode.label)}${available ? "" : " (no legal target)"}`,
        ModalModeToggled({ index }),
        selected,
        !available,
        selected,
      );
    }

    return barButton(
      `modal-mode-${index}`,
      messageText(mode.label),
      ModalModesChosen({ chosen: [index] }),
      index === 0,
      !available,
    );
  });

  const footer = multi
    ? [
        barButton("modal-cast", "Cast", ModalModesChosen({ chosen: [...picked] }), true, !ready),
        barButton("prompt-cancel", "Cancel", CancelActionClicked(), false),
      ]
    : [barButton("prompt-cancel", "Cancel", CancelActionClicked(), false)];

  return h.div(
    [h.Class("flex max-w-[min(100vw-2rem,24rem)] flex-col items-end gap-sm")],
    [...modeButtons, barRow(footer)],
  );
}

export function simplePromptBarActions(_board: BoardModel, state: VisibleState, tableId: string | null): Html | null {
  if (_board.playModePick != null) return playModeBarActions(_board.playModePick);

  if (_board.modalCast != null) {
    const modalActions = modalModeBarActions(_board.modalCast);
    if (modalActions != null) return modalActions;
  }

  const localBoardAimActions = localBoardAimBarActions(_board, state);
  if (localBoardAimActions != null) return localBoardAimActions;

  const pending = state.pending_choice;
  if (pending == null) return null;

  const pendingBoardAimActions = pendingBoardAimBarActions(_board, state, tableId);
  if (pendingBoardAimActions != null) return pendingBoardAimActions;

  switch (pending.kind) {
    case "may_yes_no":
    case "dance_exile_more":
      return barRow([
        pendingBarButton(pending, "prompt-yes", "Yes", { kind: "may", yes: true }, true, tableId == null),
        pendingBarButton(pending, "prompt-no", "No", { kind: "may", yes: false }, false, tableId == null),
      ]);
    case "pay_cost":
    case "pay_or_counter":
    case "pay_or_controller_draws":
    case "pay_echo_or_sacrifice":
    case "pay_recover_or_exile":
    case "sacrifice_unless_pay":
    case "pay_life_or_enters_tapped":
      return payCostBarActions(_board, pending, tableId);
    case "choose_mode":
      return chooseModeBarActions(pending, tableId);
    case "choose_trigger_modes":
      return chooseTriggerModesBarActions(pending, _board, state);
    case "opponent_chooses_pile":
    case "choose_pile_for_hand":
      return pilePickBarActions(pending, tableId);
    case "choose_countered_spell_destination":
    case "revealed_card_to_battlefield_or_hand":
      return destinationBarActions(pending, tableId);
    default:
      return null;
  }
}
