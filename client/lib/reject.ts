import { formatMessage } from "./i18n/message";
import type { MessageRef } from "./wire/types";

const legacyRejectKeys: Readonly<Record<string, string>> = {
  CannotActivate: "reject.cannot_activate",
  CannotDiscardCost: "reject.cannot_discard_cost",
  CannotExileCost: "reject.cannot_exile_cost",
  CannotPayCost: "reject.cannot_pay_cost",
  CannotProduceMana: "reject.cannot_produce_mana",
  ChoicePending: "reject.choice_pending",
  EngineError: "reject.engine_error",
  GameNotStarted: "reject.game_not_started",
  IllegalChoice: "reject.illegal_choice",
  IllegalDeclaration: "reject.illegal_declaration",
  IllegalMode: "reject.illegal_mode",
  IllegalTarget: "reject.illegal_target",
  NotCastable: "reject.not_castable",
  NotYourPriority: "reject.not_your_priority",
  NotYourSeat: "reject.not_seated",
  UnknownAction: "reject.unknown_action",
  UnknownObject: "reject.unknown_object",
  UnknownTable: "reject.unknown_table",
  WrongTiming: "reject.wrong_timing",
};

/** Deprecated local reject-name adapter; server acks now carry MessageRef directly. */
export function humanReason(reason: MessageRef | string): string {
  if (typeof reason !== "string") return formatMessage(reason);

  const key = legacyRejectKeys[reason] ?? reason;
  if (!key.startsWith("reject.")) return reason;
  return formatMessage({ key, params: [], children: [] });
}
