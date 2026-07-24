import { type Html, html } from "foldkit/html";
import { mulliganChrome } from "~/mulligan";
import { gameButtonClass } from "~/ui/buttonClass";
import { cardArt } from "~/ui/card-art";
import type { VisibleState } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import { KeepHandClicked, type Message, MulliganClicked } from "../messages";

const h = html<Message>();

export function mulliganOverlayView(state: VisibleState): Html | null {
  const chrome = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  });
  if (!chrome.show || !chrome.showControls) return null;

  const hand = state.objects.filter(
    (o) => Number(o.zone) === ZONE.Hand && Number(o.owner) === Number(state.viewer),
  );

  return h.div(
    [
      h.DataAttribute("testid", "mulligan-overlay"),
      h.Class(
        "pointer-events-auto fixed inset-0 z-40 flex flex-col items-center justify-center gap-md bg-black/70 px-md py-lg text-snow",
      ),
    ],
    [
      h.div(
        [
          h.Class(
            "flex max-h-[min(70vh,640px)] w-full max-w-[min(96vw,1100px)] flex-col items-center gap-sm rounded-hud border border-vine/50 bg-forest-hud px-md py-md shadow-hud",
          ),
        ],
        [
          h.div([h.Class("text-label uppercase tracking-[0.08em] text-mist")], [chrome.title]),
          h.div([h.Class("text-caption text-snow-mint")], [chrome.status]),
          h.div(
            [
              h.DataAttribute("testid", "mulligan-hand"),
              h.Class("flex w-full flex-wrap justify-center gap-3 overflow-y-auto py-sm"),
            ],
            hand.map((obj) =>
              h.div(
                [
                  h.DataAttribute("testid", `mulligan-face-${obj.id}`),
                  h.Class("pointer-events-none shrink-0"),
                ],
                [
                  obj.print
                    ? cardArt(h, {
                        print: obj.print,
                        size: "large",
                        alt: obj.name,
                        className:
                          "block aspect-[150/209] w-[min(22vw,160px)] rounded-[9px] bg-morph-slate shadow-hand",
                      })
                    : h.div(
                        [
                          h.Class(
                            "flex aspect-[150/209] w-[min(22vw,160px)] items-center justify-center rounded-[9px] bg-morph-slate px-2 text-center text-caption text-snow",
                          ),
                        ],
                        [obj.name],
                      ),
                ],
              ),
            ),
          ),
          h.div(
            [h.Class("flex flex-wrap justify-center gap-sm")],
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
          ),
        ],
      ),
    ],
  );
}

export function mulliganWaitingView(state: VisibleState): Html | null {
  const chrome = mulliganChrome({
    mulliganing: state.mulliganing,
    localSeat: state.viewer,
    players: state.players,
  });
  if (!chrome.show || chrome.showControls) return null;

  return h.div(
    [
      h.DataAttribute("testid", "mulligan-waiting"),
      h.Class(
        "pointer-events-none fixed top-md left-1/2 z-30 max-w-[min(90vw,28rem)] -translate-x-1/2 rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-center text-chip text-seafoam shadow-hud",
      ),
    ],
    [chrome.status],
  );
}
