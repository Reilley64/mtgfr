import { type Html, html } from "foldkit/html";
import { type AnswerInput, choiceIntent } from "~/choice";
import { priorityPrimaryClass } from "~/priorityContextChrome";
import { gameButtonClass } from "~/ui/buttonClass";
import type { PendingChoiceView, VisibleState } from "~/wire/types";
import { type Message, PendingChoiceAnswered } from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();

type SimpleYesNoPending = Extract<PendingChoiceView, { kind: "may_yes_no" | "dance_exile_more" }>;

function simpleYesNoPending(state: VisibleState): SimpleYesNoPending | null {
  const pending = state.pending_choice;
  if (pending == null) return null;

  switch (pending.kind) {
    case "may_yes_no":
    case "dance_exile_more":
      return pending;
    default:
      return null;
  }
}

function simplePromptButton(
  pending: SimpleYesNoPending,
  testId: "prompt-yes" | "prompt-no",
  label: "Yes" | "No",
  answer: AnswerInput,
  primary: boolean,
  disabled: boolean,
): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", testId),
      h.Disabled(disabled),
      h.OnClick(PendingChoiceAnswered({ intent: choiceIntent(pending, answer) })),
      h.Class(gameButtonClass(primary ? "game" : "game-quiet", primary ? priorityPrimaryClass(true) : null)),
    ],
    [label],
  );
}

export function simplePromptBarActions(_board: BoardModel, state: VisibleState, tableId: string | null): Html | null {
  const pending = simpleYesNoPending(state);
  if (pending == null) return null;

  return h.div(
    [h.Class("flex flex-row-reverse flex-wrap items-center justify-end gap-sm")],
    [
      simplePromptButton(pending, "prompt-yes", "Yes", { kind: "may", yes: true }, true, tableId == null),
      simplePromptButton(pending, "prompt-no", "No", { kind: "may", yes: false }, false, tableId == null),
    ],
  );
}
