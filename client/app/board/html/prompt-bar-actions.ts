import { type Html, html } from "foldkit/html";
import { type AnswerInput, choiceIntent, initPromptDraft } from "~/choice";
import { priorityPrimaryClass } from "~/priorityContextChrome";
import { gameButtonClass } from "~/ui/buttonClass";
import type { MessageRef, PendingChoiceView, VisibleState } from "~/wire/types";
import { costText } from "~/xCost";
import { formatMessage } from "../../domain/i18n/message";
import { modeAvailable } from "../action/modal";
import {
  CancelActionClicked,
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

  const pending = state.pending_choice;
  if (pending == null) return null;

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
