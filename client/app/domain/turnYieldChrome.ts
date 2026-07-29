import { cn } from "./cn";

/**
 * Arena-style turn-yield rocker classes.
 * Armed state uses amber earth (`yielded`), never priority gold — The Gold Means Act Rule.
 * Sized to quiet companions so the primary keeps silhouette hierarchy.
 *
 * State is attribute-driven: the rocker button already carries `role="switch"` +
 * `aria-checked`, so the chrome keys off ARIA (`aria-checked:` on the rocker,
 * `group-aria-checked:` inside the `group/yield`) instead of a parallel JS class
 * ternary — the accessible state and the visual state cannot drift apart.
 */
export function turnYieldRockerClass(): string {
  return cn(
    "group/yield flex h-[36px] items-center rounded-game border border-white/12 bg-forest-hud px-sm",
    "transition-colors duration-150 ease-state",
    "aria-checked:border-yielded/60 aria-checked:bg-yielded/15",
  );
}

export function turnYieldTrackClass(): string {
  return cn(
    "relative h-[20px] w-[36px] shrink-0 rounded-full bg-tapped-out transition-colors duration-150 ease-state",
    "group-aria-checked:bg-yielded",
  );
}

export function turnYieldThumbClass(): string {
  return cn(
    "absolute top-[2px] left-[2px] flex size-[16px] items-center justify-center rounded-full",
    "bg-snow font-bold text-forest-floor text-micro leading-none shadow-press",
    "transition-transform duration-150 ease-state",
    "group-aria-checked:translate-x-[16px] group-aria-checked:bg-forest-floor group-aria-checked:text-yielded-ink",
  );
}
