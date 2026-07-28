// Top-left sound toggle — visible for everyone on the stream (Solid board.tsx).

import { type Html, html } from "foldkit/html";
import { button } from "~/ui/button";
import { type Message, SoundToggled } from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();

export function soundToggleView(board: BoardModel): Html {
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
