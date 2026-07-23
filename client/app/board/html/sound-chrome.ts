// Top-left sound toggle — visible for everyone on the stream (Solid board.tsx).

import { type Html, html } from "foldkit/html";
import { buttonClass } from "~/ui/buttonClass";
import { type Message, SoundToggled } from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();

export function soundToggleView(board: BoardModel): Html {
  const on = board.soundOn;
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", "board-sound-toggle"),
      h.Attribute("aria-label", on ? "Mute sound" : "Unmute sound"),
      h.Attribute("aria-pressed", on ? "true" : "false"),
      h.OnClick(SoundToggled()),
      h.Class(buttonClass("ghost", "pointer-events-auto px-md py-xs text-caption")),
    ],
    [on ? "Sound" : "Muted"],
  );
}
