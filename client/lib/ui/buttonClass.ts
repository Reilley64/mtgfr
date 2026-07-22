// DESIGN.md §5 button vocabulary for Foldkit views (ported from Solid `atoms/button.tsx`).

import { cn } from "../cn";

const VARIANTS = {
  primary:
    "rounded-control border-none bg-llanowar px-lg py-sm text-button text-snow-mint transition-colors duration-150 ease-state disabled:opacity-50",
  ghost:
    "rounded-control border border-vine bg-transparent px-lg py-sm text-button text-mist transition-colors duration-150 ease-state disabled:opacity-50",
  danger:
    "rounded-control border border-burn-red bg-transparent px-lg py-sm text-button text-burn-red transition-colors duration-150 ease-state disabled:opacity-50",
  link: "border-none bg-transparent p-0 font-[inherit] text-vine underline",
  game: "min-w-[132px] rounded-game border-none bg-llanowar-deep px-[26px] py-[11px] text-game text-snow-mint shadow-press transition-[background,transform,box-shadow] duration-150 ease-state hover:enabled:bg-llanowar active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
  "game-quiet":
    "min-w-0 rounded-game border-none bg-tapped-out px-lg py-[7px] text-label text-mist shadow-press transition-[background,transform,box-shadow] duration-150 ease-state hover:enabled:bg-quiet-hover active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
  "game-yielded":
    "min-w-0 rounded-game border-none bg-yielded px-lg py-[7px] text-label text-yielded-ink shadow-press transition-[background,transform,box-shadow] duration-150 ease-state hover:enabled:bg-yielded-hover active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
} as const;

export type ButtonVariant = keyof typeof VARIANTS;
export type GameButtonVariant = "game" | "game-quiet" | "game-yielded";

export function buttonClass(
  variant: ButtonVariant = "primary",
  ...extra: Array<string | false | null | undefined>
): string {
  return cn("cursor-pointer", VARIANTS[variant], ...extra);
}

/** @deprecated Prefer `buttonClass("game" | …)`. Kept for board call sites. */
export function gameButtonClass(
  variant: GameButtonVariant,
  ...extra: Array<string | false | null | undefined>
): string {
  return buttonClass(variant, ...extra);
}
