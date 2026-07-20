import { createMemo, createSignal, Show } from "solid-js";
import { Dynamic } from "solid-js/web";
import { Button, Field, Modal } from "~/components/atoms";
import { costText, FORMS, PROMPT_ROW, PROMPT_TITLE } from "~/components/molecules/prompt-forms";
import { type AnswerInput, choiceIntent, choiceShowKey, myChoice } from "~/lib/choice";
import { isFullscreenPrompt } from "~/lib/promptForm";
import { clampX, costWithChosenX } from "~/lib/xCost";
import type { PendingChoiceView, VisibleState, WireCost, WireIntent } from "~/wire/types";

export { choiceShowKey, myChoice } from "~/lib/choice";

// True when the event target is itself a focusable interactive control (a text field, a button, or
// a `role="button"` tile like a Hand bar card) — anything that already owns Enter/Space natively, so
// the global pass-priority shortcut only fires when *nothing* more specific is focused to claim it.
export function isInteractiveControl(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  const tag = el?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || tag === "BUTTON") return true;
  return el?.getAttribute("role") === "button";
}

function PromptModal(props: { pc: PendingChoiceView; state: VisibleState; onAnswer: (i: WireIntent) => void }) {
  const answer = (a: AnswerInput) => props.onAnswer(choiceIntent(props.pc, a));
  const Form = () => FORMS[props.pc.kind];
  const body = (
    <Show when={Form()} fallback={<div>Unhandled choice: {props.pc.kind}</div>}>
      <Dynamic component={Form()} pc={props.pc} state={props.state} onAnswer={answer} />
    </Show>
  );
  return (
    <Show when={!isFullscreenPrompt(props.pc.kind)} fallback={body}>
      <Modal class="fixed top-[45%] left-1/2 z-30 -translate-x-1/2 -translate-y-1/2">{body}</Modal>
    </Show>
  );
}

/** Choose a value for a cast's `{X}` (CR 601.2b). Client-local like staged targeting — the chosen
 * value rides the cast intent's `x`; the engine verifies the whole payment and rejects an
 * unaffordable X, so no affordability math here. */
export function XPromptModal(props: {
  name: string;
  minX: number;
  maxX: number;
  xCost: WireCost;
  onSubmit: (x: number) => void;
  onCancel: () => void;
}) {
  const min = () => props.minX;
  const max = () => props.maxX;
  const [x, setX] = createSignal(clampX(props.maxX, min(), max()));
  const setClamped = (value: number) => setX(clampX(value, min(), max()));
  const preview = () => costText(costWithChosenX(props.xCost, x()));
  return (
    <Modal
      class="fixed top-[45%] left-1/2 z-30 -translate-x-1/2 -translate-y-1/2"
      onKeyDown={(event) => {
        if (event.key === "Escape") props.onCancel();
      }}
    >
      <div class={PROMPT_TITLE}>Choose X for {props.name}</div>
      <div class="mb-sm text-[var(--mist)]">Pay {preview()}</div>
      <div class={PROMPT_ROW}>
        <Button type="button" variant="ghost" onClick={() => setClamped(min())}>
          Min
        </Button>
        <Button type="button" disabled={x() <= min()} onClick={() => setClamped(x() - 1)}>
          −
        </Button>
        <Field
          ref={(el) => setTimeout(() => el.select())}
          type="number"
          min={String(min())}
          max={String(max())}
          value={x()}
          onInput={(e) => setClamped(Number(e.currentTarget.value))}
          onKeyDown={(e) => e.key === "Enter" && props.onSubmit(x())}
          class="w-[70px]"
        />
        <Button type="button" disabled={x() >= max()} onClick={() => setClamped(x() + 1)}>
          +
        </Button>
        <Button type="button" variant="ghost" onClick={() => setClamped(max())}>
          Max
        </Button>
        <Button type="button" onClick={() => props.onSubmit(x())}>
          Cast
        </Button>
        <Button type="button" onClick={props.onCancel} variant="ghost">
          Cancel
        </Button>
      </div>
    </Modal>
  );
}

/** Pending-choice host: renders the engine's prompt modal for the viewer's seat. */
export function PromptHost(props: { me: number; state: VisibleState; onAnswer: (i: WireIntent) => void }) {
  // Key by kind:player (memo) so Solid remounts when the choice *type* changes, without wiping
  // in-progress picks on every same-kind delta. Read helpers in JSX so `props.state` stays tracked.
  const gate = createMemo(() => choiceShowKey(props.state, props.me));
  return (
    <Show when={gate()} keyed>
      {(_key) => (
        <Show when={myChoice(props.state, props.me)}>
          {(pc) => <PromptModal pc={pc()} state={props.state} onAnswer={props.onAnswer} />}
        </Show>
      )}
    </Show>
  );
}
