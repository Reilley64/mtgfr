// Menu chrome. `@foldkit/ui`'s Menu renders its own trigger button, items container, and item
// elements, and accepts only `buttonClassName` / `itemsClassName` / `ItemConfig.className`
// strings — so a component function cannot wrap it. These return class strings instead, the
// opposite of the usual "views never assemble class strings" rule.
// The recipe is private on purpose — see recipe.ts.

import type { ClassValue } from "../cn";
import { cva } from "./recipe";

const panelRecipe = cva({
  base: "flex flex-col rounded-hud border border-vine bg-forest-surface p-xs shadow-table",
});

const itemRecipe = cva({
  base: "cursor-pointer rounded-control border-none bg-transparent px-md py-xs text-left text-label text-snow hover:bg-white/8 focus-visible:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
});

/** Shared dropdown panel chrome. Positioning (absolute/fixed, z-index, min-width) differs per
 * site — pass it as `extra`. */
export function menuPanelClass(extra?: ClassValue): string {
  return panelRecipe({ class: extra });
}

/** A single menu row: transparent, borderless, hover/focus-visible highlight. */
export function menuItemClass(extra?: ClassValue): string {
  return itemRecipe({ class: extra });
}
