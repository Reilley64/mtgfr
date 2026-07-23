// Opening-hand mulligan chrome: Keep / Mulligan while the game is in the
// simultaneous pre-game mulligan phase. Snapshot fields drive visibility;
// lifecycle events are omitted from the wire.

import { type Html, html } from "foldkit/html";
import { type MulliganChrome, mulliganChrome } from "~/mulligan";
import { gameButtonClass } from "~/ui/buttonClass";
import type { VisibleState } from "~/wire/types";
import { KeepHandClicked, type Message, MulliganClicked } from "../messages";
import { HAND_BAR_H } from "./hand";

const h = html<Message>();

export function mulliganBarView(state: VisibleState): Html | null {
  const chrome = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  });
  if (!chrome.show) return null;
  return mulliganBarChrome(chrome);
}

function mulliganBarChrome(chrome: MulliganChrome): Html {
  return h.div(
    [
      h.DataAttribute("testid", "mulligan-bar"),
      h.Style({ bottom: `${HAND_BAR_H + 10}px` }),
      h.Class(
        "pointer-events-auto fixed left-1/2 z-25 flex w-[min(520px,calc(100vw-32px))] -translate-x-1/2 flex-col items-center gap-xs rounded-game bg-forest-floor/95 px-lg py-md text-center shadow-press",
      ),
    ],
    [
      h.div([h.Class("text-label text-mist uppercase tracking-[0.08em]")], [chrome.title]),
      h.div([h.Class("text-caption text-snow-mint")], [chrome.status]),
      chrome.showControls
        ? h.div(
            [h.Class("mt-xs flex flex-wrap justify-center gap-sm")],
            [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "mulligan-keep"),
                  h.OnClick(KeepHandClicked()),
                  h.Class(gameButtonClass("game")),
                ],
                [chrome.keepLabel],
              ),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "mulligan-take"),
                  h.Disabled(!chrome.canMulligan),
                  h.OnClick(MulliganClicked()),
                  h.Class(gameButtonClass("game-quiet")),
                ],
                [chrome.mulliganLabel],
              ),
            ],
          )
        : null,
    ].filter((node): node is Html => node !== null),
  );
}
