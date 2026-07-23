// Class-list composition: `cn(base, cond && "override")`.
//
// Two jobs, from two libraries. `clsx` drops falsy entries, so a condition reads as a condition
// rather than a `? "x" : ""` ternary. `tailwind-merge` then resolves utilities that set the same
// CSS property, last one winning — which is what lets a conditional *override* the base instead of
// piling up beside it and leaving the winner to CSS source order.
//
// tailwind-merge knows stock Tailwind's scales, not the ones DESIGN.md gives us. Unconfigured, it
// classifies `text-caption` as a colour and drops it from `text-caption text-burn-red`. Hence
// `THEME_SCALES` — guarded against drifting from global.css by cn.test.ts.

import { type ClassValue, clsx } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

export type { ClassValue };

/** The @theme namespaces whose keys tailwind-merge cannot infer. Colours need no entry: an unknown
 * `text-*`/`bg-*` value falls back to the colour group, which is already right. */
export const THEME_SCALES = {
  text: ["title", "body", "button", "label", "caption", "chip", "display", "game", "micro"],
  radius: ["panel", "modal", "hud", "control", "focus", "game"],
  spacing: ["xs", "sm", "md", "lg", "xl", "xxl"],
} as const;

const mergeTw = extendTailwindMerge({
  extend: {
    theme: {
      text: [...THEME_SCALES.text],
      radius: [...THEME_SCALES.radius],
      spacing: [...THEME_SCALES.spacing],
    },
  },
});

export { mergeTw };

/** Compose a class list. Later entries override earlier ones for the same CSS property. */
export const cn = (...classes: ClassValue[]): string => mergeTw(clsx(classes));
