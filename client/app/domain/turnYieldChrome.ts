import { cn } from "./cn";

/**
 * Arena-style rocker classes, shared by both turn-yield toggles. The two never render at once
 * (End Turn shows only to the active seat, until-my-turn only to the others), so they share the
 * `group/yield` name.
 *
 * Tone picks the armed hue: `yield` is amber earth (`yielded`), `end-turn` is island blue —
 * neither is priority gold, per The Gold Means Act Rule.
 *
 * State is attribute-driven: the rocker button already carries `role="switch"` +
 * `aria-checked`, so the chrome keys off ARIA (`aria-checked:` on the rocker,
 * `group-aria-checked/yield:` on track/thumb inside the named `group/yield`) instead of
 * a parallel JS class ternary — the accessible state and the visual state cannot drift
 * apart. The variant must name the group: bare `group-aria-checked:` compiles to
 * `:where(.group)[aria-checked] *`, which a `group/yield` parent never matches.
 */
export type YieldTone = "yield" | "end-turn";

// Literal class strings per tone — Tailwind scans source text, so these cannot be interpolated.
const TONE = {
  yield: {
    rocker: "aria-checked:border-yielded/60 aria-checked:bg-yielded/15",
    track: "group-aria-checked/yield:bg-yielded",
    thumb: "group-aria-checked/yield:bg-forest-floor group-aria-checked/yield:text-yielded-ink",
  },
  "end-turn": {
    rocker:
      "aria-checked:border-island-blue/60 aria-checked:bg-island-blue/15 aria-checked:shadow-[0_0_12px_rgba(74,158,255,0.45)]",
    track: "group-aria-checked/yield:bg-island-blue",
    thumb: "group-aria-checked/yield:bg-forest-floor group-aria-checked/yield:text-island-blue",
  },
} as const;

export function turnYieldRockerClass(tone: YieldTone = "yield"): string {
  return cn(
    "group/yield flex h-[36px] items-center gap-xs rounded-game border border-white/12 bg-forest-hud px-sm",
    "transition-colors duration-150 ease-state",
    TONE[tone].rocker,
  );
}

export function turnYieldTrackClass(tone: YieldTone = "yield"): string {
  return cn(
    "relative h-[20px] w-[36px] shrink-0 rounded-full bg-tapped-out transition-colors duration-150 ease-state",
    TONE[tone].track,
  );
}

export function turnYieldThumbClass(tone: YieldTone = "yield"): string {
  return cn(
    "absolute top-[2px] left-[2px] flex size-[16px] items-center justify-center rounded-full",
    "bg-snow font-bold text-forest-floor text-micro leading-none shadow-press",
    "transition-transform duration-150 ease-state",
    "group-aria-checked/yield:translate-x-[16px]",
    TONE[tone].thumb,
  );
}

/** Arena's hover label: collapsed to zero width until the rocker is hovered or keyboard-focused,
 * then it slides open to the left of the track. Stays in the DOM so the accessible name and the
 * visible name are the same string. */
export function turnYieldLabelClass(): string {
  return cn(
    "max-w-0 overflow-hidden whitespace-nowrap text-caption text-snow/85 opacity-0",
    "transition-all duration-150 ease-state",
    "group-hover/yield:max-w-[160px] group-hover/yield:opacity-100",
    "group-focus-visible/yield:max-w-[160px] group-focus-visible/yield:opacity-100",
  );
}
