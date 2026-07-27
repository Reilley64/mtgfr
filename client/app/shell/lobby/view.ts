import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import type { DeckCardFlipTick } from "../../deck-card-nav";
import type { BuilderCatalogCard } from "../../domain/deck-builder/cards";
import type { AppChromeMeta } from "../../domain/ui/app-version";
import { buttonClass } from "../../domain/ui/buttonClass";
import type { CardArtTick } from "../../domain/ui/card-art";
import { seatFace } from "../../domain/ui/seat-face";
import { alertClass, fieldClass, panelClass } from "../../domain/ui/surfaces";
import type { DeckSummary } from "../../domain/wire/types";
import type { ClosedAccountMenu, GotAuthMessage, ToggledAccountMenu } from "../../messages";
import { HomeRoute, routePath } from "../../routes";
import { accountChrome } from "../account-chrome/view";
import { type DeckCardModel, renderDeckCard } from "../decks/deck-card";
import { shellFrame } from "../frame/shell-frame";
import {
  ChangedLobbyCode,
  type Message as LobbyMessage,
  RequestedLobbyCopy,
  RequestedLobbyHost,
  RequestedLobbyJoin,
  RequestedLobbyReady,
  RequestedLobbyStart,
} from "./messages";
import type { LobbySlice } from "./submodel";
import { lobbyHost, lobbyReady } from "./update";

export type ViewMessage =
  | LobbyMessage
  | typeof CardArtTick.Type
  | typeof DeckCardFlipTick.Type
  | typeof ClosedAccountMenu.Type
  | typeof GotAuthMessage.Type
  | typeof ToggledAccountMenu.Type;
export type LobbySurface = "entry" | "table";

export type ViewInputs = {
  decks: ReadonlyArray<DeckSummary>;
  decksLoading: boolean;
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>;
  chrome: AppChromeMeta;
  surface: LobbySurface;
  username: string;
  meGravatarHash: string | null;
  accountMenuOpen: boolean;
};

const h = html<ViewMessage>();

const seatDots = ["bg-seat-forest", "bg-seat-island", "bg-seat-mountain", "bg-seat-arcane"];

function humanError(code: string): string {
  const map: Record<string, string> = {
    TableFull: "That table is full.",
    AlreadyStarted: "The game already started.",
    NotHost: "Only the host can start.",
    NeedTwoPlayers: "Need at least two players.",
    NotAllReady: "Waiting for everyone to Ready…",
    UnknownTable: "That table link is stale or expired. Ask the host for a new code.",
    NotSeated: "Claim a seat first.",
    UnknownDeck: "That deck no longer exists.",
    Draining: "Server is restarting — try again in a moment.",
    SeedFailed: "Couldn't start the game — try again.",
    Unreachable: "Couldn't reach the table — try again.",
  };
  return map[code] ?? code;
}

function deckCardModel(
  deck: DeckSummary,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): DeckCardModel {
  const commander = knownCommanders[deck.commander];
  return {
    id: deck.id,
    name: deck.name,
    commander: deck.commander,
    commanderName: commander?.name ?? deck.commander,
    print: deck.commander_print ?? commander?.default_print ?? "",
    colorIdentity: commander?.color_identity ?? [],
  };
}

function deckCardAndBack(
  model: LobbySlice,
  decks: ReadonlyArray<DeckSummary>,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  const deck = model.selectedDeckId == null ? undefined : decks.find((item) => item.id === model.selectedDeckId);
  const card =
    deck == null
      ? h.div(
          [h.Class("rounded-hud bg-glass-dim p-md text-label text-lichen")],
          [decksLoading ? "Loading decks…" : "Deck not found."],
        )
      : renderDeckCard(h, deckCardModel(deck, knownCommanders), {
          mode: "static",
          testId: `lobby-deck-card-${deck.id}`,
        });

  return h.div(
    [h.Class("flex flex-col gap-sm")],
    [
      h.div([h.Class("max-w-[240px]"), h.DataAttribute("testid", "lobby-deck-card")], [card]),
      h.a(
        [h.Href(routePath(HomeRoute())), h.DataAttribute("testid", "lobby-back"), h.Class(buttonClass("ghost"))],
        ["Back"],
      ),
    ],
  );
}

function selectedDeckCard(
  deck: DeckSummary | undefined,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  if (deck == null) {
    return h.div(
      [h.Class("rounded-hud bg-glass-dim p-md text-label text-lichen")],
      [decksLoading ? "Loading decks…" : "Deck not found."],
    );
  }

  return renderDeckCard(h, deckCardModel(deck, knownCommanders), {
    mode: "static",
    testId: `lobby-deck-card-${deck.id}`,
  });
}

function entrySurface(
  model: LobbySlice,
  deck: DeckSummary | undefined,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  return h.div(
    [
      h.DataAttribute("testid", "lobby-entry"),
      h.DataAttribute("lobby-entry-motion", "1"),
      h.Class("grid grid-cols-[minmax(0,320px)_minmax(0,1fr)] items-center gap-xl max-w-[min(100%,720px)]"),
    ],
    [
      h.div(
        [h.Class("max-w-[320px]"), h.DataAttribute("testid", "lobby-deck-card")],
        [selectedDeckCard(deck, decksLoading, knownCommanders)],
      ),
      h.div(
        [h.Class("flex flex-col gap-md")],
        [
          h.div(
            [h.Class("flex flex-col gap-xs")],
            [
              h.div([h.Class("font-display font-semibold text-title tracking-[-0.02em]")], ["Ready to play?"]),
              h.div([h.Class("text-label text-lichen")], ["Host a fresh Commander table with this deck."]),
            ],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "lobby-host"),
              h.Disabled(model.submitting),
              h.OnClick(RequestedLobbyHost()),
              h.Class(buttonClass("primary", "w-fit")),
            ],
            ["Host a table"],
          ),
          h.div(
            [h.Class("flex flex-col gap-sm")],
            [
              h.div([h.Class("text-label text-lichen")], ["Have a code?"]),
              h.label([h.For("table-code"), h.Class("sr-only")], ["Table code"]),
              h.div(
                [h.Class("flex flex-wrap items-center gap-sm")],
                [
                  h.input([
                    h.Id("table-code"),
                    h.DataAttribute("testid", "lobby-join-code"),
                    h.Placeholder("Table code"),
                    h.Value(model.code),
                    h.OnInput((code) => ChangedLobbyCode({ code })),
                    h.Autocomplete("off"),
                    h.Spellcheck(false),
                    h.Class(fieldClass("min-w-[10rem] flex-1")),
                  ]),
                  h.button(
                    [
                      h.Type("button"),
                      h.DataAttribute("testid", "lobby-join"),
                      h.Disabled(model.submitting),
                      h.OnClick(RequestedLobbyJoin()),
                      h.Class(buttonClass("ghost")),
                    ],
                    ["Join table"],
                  ),
                ],
              ),
            ],
          ),
          h.a(
            [
              h.Href(routePath(HomeRoute())),
              h.DataAttribute("testid", "lobby-back"),
              h.Class(buttonClass("ghost", "w-fit")),
            ],
            ["Back"],
          ),
        ],
      ),
    ],
  );
}

function entry(
  model: LobbySlice,
  decks: ReadonlyArray<DeckSummary>,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  if (decksLoading && decks.length === 0 && model.selectedDeckId == null) {
    return h.div([h.Class("text-label text-lichen")], ["Loading decks…"]);
  }

  if (!decksLoading && decks.length === 0 && model.selectedDeckId == null) {
    return lobbyEmpty("Build a deck first (Your decks → New deck).");
  }

  if (model.selectedDeckId == null) {
    return lobbyEmpty("Pick a deck to play first (Your decks → Play).");
  }

  const deck = decks.find((item) => item.id === model.selectedDeckId);
  return entrySurface(model, deck, decksLoading, knownCommanders);
}

function seats(model: LobbySlice): Html {
  return h.div(
    [h.Class("flex flex-col gap-sm"), h.DataAttribute("testid", "lobby-seats")],
    (model.view?.seats ?? []).map((seat) =>
      h.div(
        [
          h.Class(
            "grid grid-cols-[auto_auto_minmax(7rem,11rem)_minmax(0,1fr)_auto] items-center gap-sm rounded-hud bg-glass-dim px-md py-sm",
          ),
          h.DataAttribute("testid", `lobby-seat-${seat.player}`),
          h.DataAttribute("claimed", seat.claimed ? "1" : "0"),
        ],
        [
          h.span([h.Class(`size-2.5 shrink-0 rounded-full ${seatDots[seat.player] ?? "bg-fog"}`)], []),
          seatFace(h, {
            seat: seat.player,
            username: seat.username,
            gravatarHash: seat.gravatar_hash ?? null,
          }),
          h.span(
            [h.Class(seat.claimed ? "min-w-0 font-semibold" : "min-w-0 text-lichen")],
            [seat.claimed ? (seat.username ?? `Seat ${seat.player + 1}`) : `Seat ${seat.player + 1}`],
          ),
          h.span(
            [h.Class(seat.claimed ? "min-w-0 text-mist" : "min-w-0 text-lichen")],
            [seat.claimed ? (seat.deck_name ?? "—") : "open"],
          ),
          h.span(
            [h.Class("flex items-center justify-end gap-xs")],
            [
              seat.is_host ? h.span([h.Class("text-label text-lichen")], ["Host"]) : null,
              seat.claimed && seat.ready
                ? h.span(
                    [
                      h.DataAttribute("testid", `lobby-seat-${seat.player}-ready`),
                      h.Class(
                        "inline-block rounded-full bg-llanowar/25 px-sm py-0.5 font-semibold text-caption text-ready-sprout",
                      ),
                    ],
                    ["Ready"],
                  )
                : null,
              seat.claimed && !seat.ready ? h.span([h.Class("text-label text-lichen")], ["Waiting…"]) : null,
              seat.is_you ? h.span([h.Class("text-label text-lichen")], ["(you)"]) : null,
            ],
          ),
        ],
      ),
    ),
  );
}

function claimSeat(
  model: LobbySlice,
  decks: ReadonlyArray<DeckSummary>,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  if (decksLoading && model.selectedDeckId == null) {
    return h.div([h.Class("text-label text-lichen")], ["Loading decks…"]);
  }

  if (model.selectedDeckId != null) {
    return h.div(
      [h.Class("flex flex-col gap-md")],
      [
        deckCardAndBack(model, decks, decksLoading, knownCommanders),
        h.button(
          [
            h.Type("button"),
            h.DataAttribute("testid", "lobby-claim"),
            h.Disabled(model.submitting),
            h.OnClick(RequestedLobbyJoin()),
            h.Class(buttonClass("primary")),
          ],
          ["Claim a seat"],
        ),
      ],
    );
  }

  if (decks.length === 0) {
    return lobbyEmpty("Build a deck first (Your decks → New deck).");
  }

  return lobbyEmpty("Pick a deck to play first (Your decks → Play).");
}

function lobbyEmpty(message: string): Html {
  return h.div([h.Class("text-caution-amber text-label"), h.DataAttribute("testid", "lobby-empty")], [message]);
}

function tableLobby(
  model: LobbySlice,
  decks: ReadonlyArray<DeckSummary>,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  const joined = model.view?.you != null;
  const startError = model.view?.start_error ?? null;

  return h.div(
    [h.Class("flex flex-col gap-md")],
    [
      h.div(
        [h.Class("flex flex-wrap items-center gap-md")],
        [
          h.span([h.Class("text-label text-lichen")], ["Table code"]),
          h.span(
            [
              h.DataAttribute("testid", "lobby-table-code"),
              h.Class("select-text font-display text-display tracking-[0.06em]"),
            ],
            [model.tableId ?? ""],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "lobby-copy-code"),
              h.OnClick(RequestedLobbyCopy()),
              h.Class(buttonClass("ghost")),
            ],
            [model.copied ? "Copied" : "Copy code"],
          ),
        ],
      ),
      model.clipboardFallback
        ? h.input([
            h.Id("share-code"),
            h.Readonly(true),
            h.Value(model.tableId ?? ""),
            h.Class(fieldClass("w-[120px] text-chip tracking-[0.06em]")),
          ])
        : null,
      seats(model),
      !joined && model.view != null && !model.view.started
        ? h.div(
            [
              h.DataAttribute("testid", "lobby-watch-note"),
              h.Class("rounded-hud border border-vine-dim bg-glass-dim px-md py-sm text-caption text-lichen"),
            ],
            ["Stay on this table link: if you don't claim a seat before the host starts, you'll enter spectator view."],
          )
        : null,
      joined
        ? h.div(
            [h.Class("flex flex-wrap items-center gap-sm")],
            [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "lobby-ready"),
                  h.Disabled(model.submitting),
                  h.OnClick(RequestedLobbyReady({ ready: !lobbyReady(model) })),
                  h.Class(buttonClass("primary")),
                ],
                [lobbyReady(model) ? "Unready" : "Ready up"],
              ),
              lobbyHost(model)
                ? h.button(
                    [
                      h.Type("button"),
                      h.DataAttribute("testid", "lobby-start"),
                      h.Disabled(startError !== null || model.submitting),
                      h.OnClick(RequestedLobbyStart()),
                      h.Class(buttonClass("primary")),
                    ],
                    ["Start game"],
                  )
                : null,
              startError == null
                ? null
                : h.span(
                    [h.DataAttribute("testid", "lobby-start-error"), h.Class("text-caption text-caution-amber")],
                    [humanError(startError)],
                  ),
            ],
          )
        : claimSeat(model, decks, decksLoading, knownCommanders),
    ],
  );
}

export const view = Submodel.defineView<LobbySlice, ViewMessage, ViewInputs>((model, viewInputs): Html => {
  const { accountMenuOpen, chrome, decks, decksLoading, knownCommanders, meGravatarHash, surface, username } =
    viewInputs;
  // PlayRoute always paints entry — even after Host sets tableId and queues
  // Redirect — so we do not flash claim-seat / table chrome before navigation.
  const body =
    surface === "entry"
      ? entry(model, decks, decksLoading, knownCommanders)
      : tableLobby(model, decks, decksLoading, knownCommanders);

  const error =
    model.error == null
      ? null
      : h.div(
          [h.Role("alert"), h.DataAttribute("testid", "lobby-error"), h.Class(alertClass("text-burn-red"))],
          [humanError(model.error)],
        );

  const stage =
    surface === "entry"
      ? h.div(
          [h.Class("flex justify-center py-xxl"), h.DataAttribute("testid", "lobby")],
          [h.div([h.Class("w-full max-w-[min(100%-2rem,720px)]")], [body, error])],
        )
      : h.div(
          [h.Class("flex justify-center py-xxl")],
          [
            h.section(
              [
                h.DataAttribute("testid", "lobby"),
                h.DataAttribute("ui", "panel"),
                h.Class(panelClass("max-w-[min(100%-2rem,640px)]")),
              ],
              [body, error],
            ),
          ],
        );

  return shellFrame(h, {
    atmosphere: "shell",
    title: "Lobby",
    chrome,
    trailing: accountChrome(h, {
      username,
      gravatarHash: meGravatarHash,
      menuOpen: accountMenuOpen,
      showLeaderboardLink: true,
    }),
    stage,
  });
});
