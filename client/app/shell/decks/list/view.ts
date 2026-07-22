import { Effect, Queue, Schema as S, Stream } from "effect";
import { type Html, html } from "foldkit/html";
import * as Mount from "foldkit/mount";
import { cardHoverPreviewView } from "../../../../lib/deck-builder/card-hover-preview";
import { appVersionBadge } from "../../../../lib/ui/app-version";
import { buttonClass } from "../../../../lib/ui/buttonClass";
import { cardArt } from "../../../../lib/ui/card-art";
import { confirmDialog } from "../../../../lib/ui/confirmDialog";
import { feltClass, listRowClass } from "../../../../lib/ui/surfaces";
import type { Message } from "../../../messages";
import { RequestedLogout } from "../../../messages";
import { DeckRoute, NewDeckRoute, PlayRoute, routePath } from "../../../routes";
import {
  AskedDeckDelete,
  CancelledDeckDelete,
  ClearedDeckListHover,
  MovedDeckListHover,
  RequestedDeckDelete,
} from "./messages";
import type { DeckListSubmodel } from "./submodel";

const h = html<Message>();

export const BindDeckListCommanderHover = Mount.defineStream(
  "BindDeckListCommanderHover",
  { cardId: S.String, print: S.String },
  MovedDeckListHover,
  ClearedDeckListHover,
)(
  (args) => (element) =>
    Stream.callback<typeof MovedDeckListHover.Type | typeof ClearedDeckListHover.Type>((queue) =>
      Effect.gen(function* () {
        yield* Effect.acquireRelease(
          Effect.sync(() => {
            const onMouseMove = (event: Event) => {
              if (!(event instanceof MouseEvent)) return;
              Queue.offerUnsafe(
                queue,
                MovedDeckListHover({ id: args.cardId, print: args.print, x: event.clientX, y: event.clientY }),
              );
            };
            const onMouseLeave = () => {
              Queue.offerUnsafe(queue, ClearedDeckListHover());
            };
            element.addEventListener("mousemove", onMouseMove);
            element.addEventListener("mouseleave", onMouseLeave);
            return () => {
              element.removeEventListener("mousemove", onMouseMove);
              element.removeEventListener("mouseleave", onMouseLeave);
            };
          }),
          (teardown) => Effect.sync(teardown),
        );
        return yield* Effect.never;
      }),
    ),
);

function commanderName(model: DeckListSubmodel, id: string): string {
  return model.knownCommanders[id]?.name ?? id;
}

function commanderPrint(model: DeckListSubmodel, deck: DeckListSubmodel["decks"][number]): string {
  return deck.commander_print ?? model.knownCommanders[deck.commander]?.default_print ?? "";
}

function hoverPreview(model: DeckListSubmodel): Html | null {
  const hover = model.hover;
  if (hover == null) return null;
  return cardHoverPreviewView(h, {
    hover,
    card: model.knownCommanders[hover.id],
    testId: "deck-list-hover-preview",
  });
}

export function view(model: DeckListSubmodel, username: string, apiVersion: string | null): Html {
  return h.main(
    [
      h.Class(
        feltClass(
          "h-full overflow-y-auto p-xxl pt-[max(1.5rem,env(safe-area-inset-top))] pr-[max(1.5rem,env(safe-area-inset-right))] pb-[max(1.5rem,env(safe-area-inset-bottom))] pl-[max(1.5rem,env(safe-area-inset-left))]",
        ),
      ),
      h.DataAttribute("testid", "decks-page"),
    ],
    [
      model.confirmingDeleteId != null
        ? confirmDialog(h, {
            title: `Delete "${model.decks.find((d) => d.id === model.confirmingDeleteId)?.name ?? ""}"?`,
            body: "This deck and its card list are gone for good.",
            confirmLabel: "Delete deck",
            danger: true,
            onConfirm: RequestedDeckDelete({ id: model.confirmingDeleteId }),
            onCancel: CancelledDeckDelete(),
            testId: "confirm-delete-dialog",
          })
        : null,
      h.div(
        [h.Class("mx-auto mb-5 flex max-w-[720px] flex-wrap items-center justify-between gap-md")],
        [
          h.h1([h.Class("m-0 text-title")], ["Your decks"]),
          h.div(
            [h.Class("flex flex-wrap items-center gap-md")],
            [
              h.span([h.Class("text-label text-lichen")], [username]),
              h.button([h.Type("button"), h.OnClick(RequestedLogout()), h.Class(buttonClass("ghost"))], ["Sign out"]),
              h.a([h.Href(routePath(NewDeckRoute())), h.Class(buttonClass("primary"))], ["New deck"]),
            ],
          ),
        ],
      ),
      h.section(
        [h.Class("mx-auto flex max-w-[720px] flex-col gap-md")],
        [
          model.error == null
            ? null
            : h.div([h.Role("alert"), h.Class("text-label text-reconnect-rust")], [model.error]),
          model.loading ? h.div([h.Class("text-label text-lichen")], ["Loading decks…"]) : null,
          !model.loading && model.decks.length === 0
            ? h.div([h.Class("text-label text-lichen")], ["No decks yet — build one to get started."])
            : null,
          ...model.decks.map((deck) =>
            h.article(
              [h.Class(listRowClass("flex flex-wrap items-center gap-md rounded-hud p-md"))],
              [
                commanderPrint(model, deck) === ""
                  ? h.div([h.Class("size-[56px] shrink-0 rounded-control bg-glass")], [])
                  : cardArt(h, {
                      print: commanderPrint(model, deck),
                      size: "art_crop",
                      alt: "",
                      className: "size-[56px] shrink-0 rounded-control object-cover",
                    }),
                h.div(
                  [h.Class("min-w-0 flex-1")],
                  [
                    h.div(
                      [h.Class("font-semibold")],
                      [
                        deck.name,
                        deck.id < 0
                          ? h.span(
                              [
                                h.Class(
                                  "ml-sm rounded-full bg-lichen/14 px-[7px] py-px align-middle text-chip text-lichen",
                                ),
                              ],
                              ["Precon"],
                            )
                          : null,
                      ],
                    ),
                    h.span(
                      [
                        h.Class("text-label text-lichen"),
                        h.OnMount(
                          BindDeckListCommanderHover({
                            cardId: deck.commander,
                            print: commanderPrint(model, deck),
                          }),
                        ),
                      ],
                      [commanderName(model, deck.commander)],
                    ),
                  ],
                ),
                h.div(
                  [h.Class("flex flex-wrap gap-sm")],
                  [
                    h.a(
                      [h.Href(`${routePath(PlayRoute())}?deck=${deck.id}`), h.Class(buttonClass("primary"))],
                      ["Play"],
                    ),
                    deck.id < 0
                      ? null
                      : h.a(
                          [h.Href(routePath(DeckRoute({ id: String(deck.id) }))), h.Class(buttonClass("ghost"))],
                          ["Edit"],
                        ),
                    deck.id < 0
                      ? null
                      : h.button(
                          [
                            h.Type("button"),
                            h.DataAttribute("testid", `delete-deck-${deck.id}`),
                            h.OnClick(AskedDeckDelete({ id: deck.id })),
                            h.Class(buttonClass("ghost")),
                          ],
                          ["Delete"],
                        ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
      appVersionBadge(h, apiVersion),
      hoverPreview(model),
    ],
  );
}
