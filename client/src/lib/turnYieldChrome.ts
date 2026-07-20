import { cn } from "~/lib/cn";

/**
 * Arena-style turn-yield rocker classes.
 * Armed state uses amber earth (`yielded`), never priority gold — The Gold Means Act Rule.
 */
export function turnYieldRockerClass(yielded: boolean): string {
  return cn(
    "flex h-[42px] items-center rounded-game border border-white/12 bg-forest-hud px-md",
    "transition-colors duration-150 ease-state",
    yielded && "border-yielded/50",
  );
}

export function turnYieldTrackClass(yielded: boolean): string {
  return cn(
    "relative h-[22px] w-[40px] shrink-0 rounded-full transition-colors duration-150 ease-state",
    yielded ? "bg-yielded" : "bg-tapped-out",
  );
}

export function turnYieldThumbClass(yielded: boolean): string {
  return cn(
    "absolute top-[2px] left-[2px] flex size-[18px] items-center justify-center rounded-full",
    "bg-snow font-bold text-forest-floor text-micro leading-none shadow-press",
    "transition-transform duration-150 ease-state",
    yielded && "translate-x-[18px] bg-forest-floor text-yielded-ink",
  );
}
