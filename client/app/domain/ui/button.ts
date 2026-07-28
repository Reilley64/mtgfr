// Button component. Behaviour and ARIA come from @foldkit/ui; markup and classes come from here.
// The recipe is private on purpose — a styled thing that is not a button should not be reachable.

import * as Button from "@foldkit/ui/button";
import type { Attribute, html as createHtml, Html } from "foldkit/html";
import type { ClassValue } from "../cn";
import { cva } from "./recipe";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;
// `foldkit/html`'s public surface re-exports Attribute/Html/etc but not the `Child` type alias
// itself, so its definition (Html | string) is inlined here rather than imported.
type Child = Html | string;

const recipe = cva({
  base: "cursor-pointer",
  variants: {
    variant: {
      primary:
        "rounded-control border-none bg-llanowar px-lg py-sm text-button text-snow-mint transition-colors duration-150 ease-state disabled:opacity-50",
      ghost:
        "rounded-control border border-vine bg-transparent px-lg py-sm text-button text-snow-mint transition-colors duration-150 ease-state disabled:opacity-50",
      danger:
        "rounded-control border border-burn-red bg-transparent px-lg py-sm text-button text-burn-red transition-colors duration-150 ease-state disabled:opacity-50",
      link: "border-none bg-transparent p-0 font-[inherit] text-vine underline",
      game: "min-w-[132px] rounded-game border-none bg-llanowar-deep px-[26px] py-[11px] text-game text-snow-mint shadow-press transition-[background,transform,box-shadow] duration-150 ease-state hover:enabled:bg-llanowar active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
      "game-quiet":
        "min-w-0 rounded-game border-none bg-tapped-out px-lg py-[7px] text-label text-mist shadow-press transition-[background,transform,box-shadow] duration-150 ease-state hover:enabled:bg-quiet-hover active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
      "game-yielded":
        "min-w-0 rounded-game border-none bg-yielded px-lg py-[7px] text-label text-yielded-ink shadow-press transition-[background,transform,box-shadow] duration-150 ease-state hover:enabled:bg-yielded-hover active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
    },
  },
  defaultVariants: { variant: "primary" },
});

export type ButtonVariant = "primary" | "ghost" | "danger" | "link" | "game" | "game-quiet" | "game-yielded";

type SharedProps<Msg> = {
  variant?: ButtonVariant;
  onClick?: Msg;
  testId?: string;
  ariaLabel?: string;
  class?: ClassValue;
  attrs?: ReadonlyArray<Attribute<Msg>>;
};

// `as: "a"` and `as: "button"` (or omitted) are mutually exclusive shapes, not one shape with
// optional fields — an anchor has no `disabled`/`type` and a button has no `href`. The union
// rejects `{ as: "a", disabled: true }` and a missing anchor `href` wherever excess-property
// checking applies: object literals passed inline, and values assigned or returned into a
// ButtonProps-annotated position. It does NOT catch props built through an unannotated
// intermediate (`const p = { ...anchorProps, disabled: true }`), where structural assignability
// lets the extra field through to be silently ignored at render.
export type ButtonProps<Msg> = SharedProps<Msg> &
  ({ as?: "button"; type?: "button" | "submit" | "reset"; disabled?: boolean } | { as: "a"; href: string });

function shared<Msg>(h: HtmlFactory<Msg>, props: ButtonProps<Msg>): Array<Attribute<Msg>> {
  const className = recipe({ variant: props.variant, class: props.class });
  const out: Array<Attribute<Msg>> = [h.Class(className)];
  if (props.testId != null) out.push(h.DataAttribute("testid", props.testId));
  if (props.ariaLabel != null) out.push(h.AriaLabel(props.ariaLabel));
  return [...out, ...(props.attrs ?? [])];
}

export function button<Msg>(h: HtmlFactory<Msg>, props: ButtonProps<Msg>, children: ReadonlyArray<Child>): Html {
  // An anchor is not a button: @foldkit/ui's button emits `type`, which is invalid on <a>.
  // Link-styled navigation therefore renders directly instead of through Button.view.
  if (props.as === "a") {
    // Button.view wires onClick for the button branch; an anchor bypasses Button.view entirely
    // (see above), so onClick has to be attached here or it silently does nothing.
    const onClick = props.onClick != null ? [h.OnClick(props.onClick)] : [];
    return h.a([h.Href(props.href), ...onClick, ...shared(h, props)], children);
  }

  return Button.view<Msg>({
    onClick: props.onClick,
    isDisabled: props.disabled,
    type: props.type ?? "button",
    // Button.view only marks disabled via aria-disabled/data-disabled; set the native `disabled`
    // prop too so the browser actually blocks focus/click and form submission.
    toView: (a) => h.button([...a.button, h.Disabled(props.disabled ?? false), ...shared(h, props)], children),
  });
}
