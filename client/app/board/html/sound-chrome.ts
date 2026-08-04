// Top-left sound toggle — visible for everyone on the stream (Solid board.tsx).

import type { Html, HtmlBuilder } from "foldkit/html";
import { button } from "~/ui/button";
import { type Message, SoundToggled } from "../messages";
import type { BoardModel } from "../submodel";

export function soundToggleView(board: BoardModel, h: HtmlBuilder<Message>): Html {
  const on = board.soundOn;
  return button(
    h,
    {
      testId: "board-sound-toggle",
      ariaLabel: on ? "Mute sound" : "Unmute sound",
      onClick: SoundToggled(),
      variant: "ghost",
      class: "pointer-events-auto px-md py-xs text-caption",
      attrs: [h.Attribute("aria-pressed", on ? "true" : "false")],
    },
    [on ? "Sound" : "Muted"],
  );
}
