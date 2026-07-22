// DESIGN.md §5 surface recipes — Foldkit ports of Solid `atoms/surfaces.tsx` + `field.tsx`.

import { cn } from "../cn";

export function panelClass(...extra: Array<string | false | null | undefined>): string {
  return cn(
    "flex w-full min-w-0 max-w-[min(100%-2rem,420px)] flex-col gap-lg rounded-panel border border-vine",
    "bg-forest-surface p-xxl text-snow shadow-table",
    ...extra,
  );
}

export function modalClass(...extra: Array<string | false | null | undefined>): string {
  return cn("rounded-modal border border-vine bg-forest-surface p-xl text-body text-snow shadow-table", ...extra);
}

export function listRowClass(...extra: Array<string | false | null | undefined>): string {
  return cn("border border-vine-dim bg-glass-dim text-snow hover:bg-white/8", ...extra);
}

export function feltClass(...extra: Array<string | false | null | undefined>): string {
  return cn("bg-forest-floor font-sans text-body text-snow", ...extra);
}

/** Glass + vine input from DESIGN.md §5. */
export function fieldClass(...extra: Array<string | false | null | undefined>): string {
  return cn("rounded-control border border-vine bg-glass px-md py-sm text-body text-snow", ...extra);
}

/** Fixed bottom-left API/build badge (Solid AppVersion silhouette). */
export function appVersionClass(): string {
  return "pointer-events-none fixed bottom-md left-md z-10 text-label text-lichen/70";
}
