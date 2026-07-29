// Prompt card face: art (or a name plate when no print resolves) at the Magic card aspect.
// One renderer for the card-pick grids in prompts and the mulligan opening hand.

import { type html as createHtml, type Html } from "foldkit/html";
import { cardArt } from "~/ui/card-art";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

/** `sm` 120px pick tiles, `md` 150px large picks, `fluid` the mulligan hand's vw-capped tiles. */
export type PromptCardFaceSize = "sm" | "md" | "fluid";

const FACE: Record<PromptCardFaceSize, { art: string; plate: string }> = {
  sm: {
    art: "block aspect-[150/209] w-[120px] rounded-[6px] bg-morph-slate",
    plate: "flex aspect-[150/209] w-[120px] items-center justify-center rounded-[6px] bg-morph-slate px-2 text-caption text-snow",
  },
  md: {
    art: "block aspect-[150/209] w-[150px] rounded-[9px] bg-morph-slate",
    plate: "flex aspect-[150/209] w-[150px] items-center justify-center rounded-[9px] bg-morph-slate px-2 text-body text-snow",
  },
  fluid: {
    art: "block aspect-[150/209] w-[min(22vw,160px)] rounded-[9px] bg-morph-slate shadow-hand",
    plate: "flex aspect-[150/209] w-[min(22vw,160px)] items-center justify-center rounded-[9px] bg-morph-slate px-2 text-center text-caption text-snow",
  },
};

export function promptCardFace<Msg>(
  h: HtmlFactory<Msg>,
  opts: { print: string; label: string; size: PromptCardFaceSize; testId?: string; alt?: string },
): Html {
  const face = FACE[opts.size];
  return opts.print
    ? cardArt(h, {
        print: opts.print,
        size: "large",
        alt: opts.alt ?? "",
        className: face.art,
        ...(opts.testId != null ? { testId: opts.testId } : {}),
      })
    : h.div(
        [h.Class(face.plate), ...(opts.testId != null ? [h.DataAttribute("testid", opts.testId)] : [])],
        [opts.label],
      );
}
