// Dropdown chrome — the floating panel and its rows, shared by `@foldkit/ui`'s Menu and Combobox.
// Both render their own trigger, panel, and item elements and accept only `buttonClassName` /
// `itemsClassName` / `ItemConfig.className` strings — so a component function cannot wrap them.
// These return class strings instead, the opposite of the usual "views never assemble class
// strings" rule.
// The recipe is private on purpose — see recipe.ts.

import type { ClassValue } from "../cn";
import { cva } from "./recipe";

/** `shell` is page chrome (headers, lists); `hud` matches the board's translucent prompt frames. */
export type DropdownVariant = "shell" | "hud";

const panelRecipe = cva({
  base: "flex flex-col rounded-hud border p-xs",
  variants: {
    variant: {
      // z-41 is the shell overlay convention (backdrop z-40, panel z-41). It is not optional:
      // Menu/Combobox portal the panel into a root prepended before the app root, and the
      // shell frame is fixed + opaque, so a z-auto panel opens in the DOM but paints
      // underneath the page. A call-site z passed as `extra` still wins through `cn`.
      shell: "z-41 border-vine bg-forest-surface shadow-table",
      hud: "border-vine/50 bg-forest-hud shadow-hud",
    },
  },
  defaultVariants: { variant: "shell" },
});

const itemRecipe = cva({
  base: "cursor-pointer rounded-control border-none bg-transparent px-md py-xs text-left text-snow hover:bg-white/8 focus-visible:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
  variants: {
    variant: { shell: "text-label", hud: "text-body" },
  },
  defaultVariants: { variant: "shell" },
});

/** Shared dropdown panel chrome. The shell variant bakes the shell overlay z (`z-41`) because a
 * portaled panel without it paints under the fixed shell frame; positioning (absolute/fixed,
 * min-width) still differs per site — pass it as `extra`. */
export function menuPanelClass(extra?: ClassValue, variant: DropdownVariant = "shell"): string {
  return panelRecipe({ variant, class: extra });
}

/** A single menu row: transparent, borderless, hover/focus-visible highlight. */
export function menuItemClass(extra?: ClassValue, variant: DropdownVariant = "shell"): string {
  return itemRecipe({ variant, class: extra });
}
