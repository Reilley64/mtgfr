import { cn } from "./cn";

/**
 * Arena-style turn-yield rocker classes.
 * Armed state uses amber earth (`yielded`), never priority gold — The Gold Means Act Rule.
 * Sized to quiet companions so the primary keeps silhouette hierarchy.
 */
export function turnYieldRockerClass(yielded: boolean): string {
  return cn(
    "flex h-[36px] items-center rounded-game border border-white/12 bg-forest-hud px-sm",
    "transition-colors duration-150 ease-state",
    yielded && "border-yielded/60 bg-yielded/15",
  );
}

export function turnYieldTrackClass(yielded: boolean): string {
  return cn(
    "relative h-[20px] w-[36px] shrink-0 rounded-full transition-colors duration-150 ease-state",
    yielded ? "bg-yielded" : "bg-tapped-out",
  );
}

export function turnYieldThumbClass(yielded: boolean): string {
  return cn(
    "absolute top-[2px] left-[2px] flex size-[16px] items-center justify-center rounded-full",
    "bg-snow font-bold text-forest-floor text-micro leading-none shadow-press",
    "transition-transform duration-150 ease-state",
    yielded && "translate-x-[16px] bg-forest-floor text-yielded-ink",
  );
}
