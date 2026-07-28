// Text field component. ARIA wiring and the native value/type/autofocus bindings come from
// @foldkit/ui; markup and classes come from here.
// The recipe is private on purpose — a styled thing that is not a field should not be reachable.

import * as Input from "@foldkit/ui/input";
import type { Attribute, html as createHtml, Html } from "foldkit/html";
import type { ClassValue } from "../cn";
import { cva } from "./recipe";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

const recipe = cva({
  variants: {
    variant: {
      field: "rounded-control border border-vine bg-glass px-md py-sm text-body text-snow",
      hud: "shrink-0 rounded-hud bg-glass px-3 py-1 text-body text-snow",
    },
  },
  defaultVariants: { variant: "field" },
});

/** `field` is shell chrome (panels, forms, search bars); `hud` is the board's prompt chrome. */
export type InputVariant = "field" | "hud";

/** The same chrome as a class string, for `@foldkit/ui` primitives that render their own input
 * element and take only an `inputClassName` (Combobox). */
export function inputClass(extra?: ClassValue, variant: InputVariant = "field"): string {
  return recipe({ variant, class: extra });
}

export type InputProps<Msg> = {
  /** Labels and the field's own aria-describedby are wired off this, so it is required. */
  id: string;
  variant?: InputVariant;
  value?: string;
  onInput?: (value: string) => Msg;
  type?: string;
  placeholder?: string;
  autofocus?: boolean;
  testId?: string;
  ariaLabel?: string;
  class?: ClassValue;
  attrs?: ReadonlyArray<Attribute<Msg>>;
};

export function input<Msg>(h: HtmlFactory<Msg>, props: InputProps<Msg>): Html {
  return Input.view<Msg>({
    id: props.id,
    value: props.value,
    onInput: props.onInput,
    type: props.type,
    placeholder: props.placeholder,
    isAutofocus: props.autofocus,
    toView: (a) => {
      const extra: Array<Attribute<Msg>> = [h.Class(recipe({ variant: props.variant, class: props.class }))];
      if (props.testId != null) extra.push(h.DataAttribute("testid", props.testId));
      if (props.ariaLabel != null) extra.push(h.AriaLabel(props.ariaLabel));
      return h.input([...a.input, ...extra, ...(props.attrs ?? [])]);
    },
  });
}
