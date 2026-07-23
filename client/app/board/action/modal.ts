// The modal-cast state machine (CR 700.2), pure so update can decide it in a unit test.
//
// A modal spell chooses `choose..choose_max` distinct modes, and each chosen mode carries its own
// target — the cast's top-level `target` stays empty. Answering them is a walk left to right over
// the chosen indices: a mode that takes no target answers itself, one that does has to be asked.

import type { ModeView, WireModeChoice } from "~/wire/types";

export type ModalStep = { kind: "ask"; index: number; mode: ModeView } | { kind: "submit"; modes: WireModeChoice[] };

export function advance(modes: ModeView[], chosen: number[], answers: WireModeChoice[]): ModalStep {
  const filled = [...answers];
  while (filled.length < chosen.length) {
    const index = chosen[filled.length];
    if (modes[index].needs_target) return { kind: "ask", index, mode: modes[index] };
    filled.push({ index, target: null });
  }
  return { kind: "submit", modes: filled };
}

export const modeAvailable = (mode: ModeView): boolean => !mode.needs_target || mode.targets.length > 0;
