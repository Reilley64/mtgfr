// Surface recipes for Foldkit views. Token values come from design.tokens.json via generated theme in app/domain/ui/.

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

/** Glass + vine input field chrome. */
export function fieldClass(...extra: Array<string | false | null | undefined>): string {
  return cn("rounded-control border border-vine bg-glass px-md py-sm text-body text-snow", ...extra);
}

/** Inline shell alert / legality stack. Call sites still set `role="alert"`. */
export function alertClass(...extra: Array<string | false | null | undefined>): string {
  return cn("flex flex-col gap-[3px] text-label text-reconnect-rust", ...extra);
}

/** Fixed bottom-left API/build badge (Solid AppVersion silhouette). */
export function appVersionClass(): string {
  return "pointer-events-none fixed bottom-md left-md z-10 text-label text-lichen/70";
}
