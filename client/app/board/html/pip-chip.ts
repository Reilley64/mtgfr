// Mana pip chip: opaque colored plate + mana-font glyph, sized in px. Shared by the hand cost
// strip, the activation menu cost chip, and color-pick prompts — one renderer, one pip ink.

import type { Html, HtmlBuilder } from "foldkit/html";
import { costPipPlate } from "~/costPips";

export function pipChip<Msg>(
  h: HtmlBuilder<Msg>,
  opts: { ms: string; code: string; sizePx: number; extraClass?: string; testId?: string },
): Html {
  return h.span(
    [
      h.Class(
        [
          // The pip ink is a one-off near-black (no token); it lives here and nowhere else.
          "inline-flex size-(--sz) shrink-0 items-center justify-center rounded-full bg-(--plate) text-[#111] text-[length:var(--fsz)] shadow-[0_1px_2px_rgb(0_0_0/0.9)]",
          opts.extraClass ?? "",
        ]
          .filter((v) => v !== "")
          .join(" "),
      ),
      h.Style({
        "--sz": `${opts.sizePx}px`,
        "--fsz": `${Math.round(opts.sizePx * 0.82)}px`,
        "--plate": costPipPlate(opts.code),
      }),
      ...(opts.testId != null ? [h.DataAttribute("testid", opts.testId)] : []),
    ],
    [h.i([h.Class(`ms ms-${opts.ms}`)], [])],
  );
}
