// The modal-cast state machine (CR 700.2), pulled out of Board so it's decidable in a unit test.
//
// A modal spell chooses `choose..choose_max` distinct modes, and each chosen mode carries its own
// target — the cast's top-level `target` stays empty. Answering them is a walk left to right over
// the chosen indices: a mode that takes no target answers itself, one that does has to be asked.

import type { ModeView, WireModeChoice } from "~/api/generated";

export type ModalStep =
  /** Ask for `mode`'s target (it is `chosen[answers.length]`). */
  | { kind: "ask"; index: number; mode: ModeView }
  /** Every chosen mode is answered; cast with these. */
  | { kind: "submit"; modes: WireModeChoice[] };

/** Auto-answer the untargeted modes at the front of the remaining queue, then say what's next. */
export function advance(modes: ModeView[], chosen: number[], answers: WireModeChoice[]): ModalStep {
  const filled = [...answers];
  while (filled.length < chosen.length) {
    const index = chosen[filled.length];
    if (modes[index].needs_target) return { kind: "ask", index, mode: modes[index] };
    filled.push({ index, target: null });
  }
  return { kind: "submit", modes: filled };
}

/** Whether a mode can be chosen at all: one that wants a target but has none legal right now can't
 * be, and the picker must grey it out rather than let the cast be rejected. */
export const modeAvailable = (mode: ModeView): boolean => !mode.needs_target || mode.targets.length > 0;
