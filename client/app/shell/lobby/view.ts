import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import type { DeckCardFlipTick } from "../../deck-card-nav";
import { cn } from "../../domain/cn";
import type { BuilderCatalogCard } from "../../domain/deck-builder/cards";
import type { AppChromeMeta } from "../../domain/ui/app-version";
import { buttonClass } from "../../domain/ui/buttonClass";
import { type CardArtTick, cardArt } from "../../domain/ui/card-art";
import { seatFace } from "../../domain/ui/seat-face";
import { fieldClass, panelClass } from "../../domain/ui/surfaces";
import type { DeckSummary } from "../../domain/wire/types";
import type { ClosedAccountMenu, GotAuthMessage, ToggledAccountMenu } from "../../messages";
import { HomeRoute, routePath } from "../../routes";
import { accountChrome } from "../account-chrome/view";
import { type DeckCardModel, renderDeckCard } from "../decks/deck-card";
import { shellFrame } from "../frame/shell-frame";
import {
  ChangedLobbyCode,
  type Message as LobbyMessage,
  RequestedLobbyCancelJoin,
  RequestedLobbyCopy,
  RequestedLobbyHost,
  RequestedLobbyJoin,
  RequestedLobbyOpenJoin,
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

function joinCardClass(): string {
  return cn(
    "flex min-h-full flex-col gap-sm rounded-hud border border-dashed border-vine bg-glass-dim p-md text-left",
    "hover:bg-white/8 disabled:opacity-60",
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

function chooseEntry(
  model: LobbySlice,
  deck: DeckSummary | undefined,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  return h.div(
    [h.Class("flex flex-col gap-lg")],
    [
      h.div(
        [
          h.DataAttribute("testid", "lobby-entry-choose"),
          h.DataAttribute("lobby-entry-motion", "1"),
          h.Class("grid grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)] gap-lg"),
        ],
        [
          h.div(
            [h.Class("flex flex-col gap-md")],
            [
              h.div(
                [h.Class("max-w-[280px]"), h.DataAttribute("testid", "lobby-deck-card")],
                [selectedDeckCard(deck, decksLoading, knownCommanders)],
              ),
              h.div(
                [h.Class("flex flex-col gap-xs")],
                [
                  h.div([h.Class("font-semibold text-title")], ["Ready to play?"]),
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
            ],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "lobby-open-join"),
              h.Disabled(model.submitting),
              h.OnClick(RequestedLobbyOpenJoin()),
              h.Class(joinCardClass()),
            ],
            [
              h.div(
                [
                  h.Class(
                    "flex aspect-[137/100] w-full items-center justify-center rounded-hud border border-dashed border-vine-dim bg-glass text-display text-lichen",
                  ),
                ],
                ["#"],
              ),
              h.div([h.Class("font-semibold")], ["Join a table"]),
              h.div([h.Class("text-label text-lichen")], ["enter a code"]),
            ],
          ),
        ],
      ),
      h.a(
        [h.Href(routePath(HomeRoute())), h.DataAttribute("testid", "lobby-back"), h.Class(buttonClass("ghost"))],
        ["Back"],
      ),
    ],
  );
}

function bringingArt(
  deck: DeckSummary | undefined,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  if (deck == null) {
    return h.div([h.Class("size-10 bg-glass")], []);
  }

  const print = deck.commander_print ?? knownCommanders[deck.commander]?.default_print ?? "";
  if (print === "") {
    return h.div([h.Class("size-10 bg-glass")], []);
  }

  return cardArt(h, {
    print,
    size: "art_crop",
    alt: "",
    className: "size-10 object-cover",
  });
}

function joinEntry(
  model: LobbySlice,
  deck: DeckSummary | undefined,
  decksLoading: boolean,
  knownCommanders: Readonly<Record<string, BuilderCatalogCard>>,
): Html {
  const deckName = deck?.name ?? (decksLoading ? "Loading decks…" : "Deck not found.");

  return h.div(
    [
      h.DataAttribute("testid", "lobby-entry-join"),
      h.DataAttribute("lobby-entry-motion", "1"),
      h.Class("flex flex-col gap-md"),
    ],
    [
      h.div(
        [
          h.DataAttribute("testid", "lobby-bringing"),
          h.Class("flex items-center gap-sm border-b border-vine-dim pb-sm"),
        ],
        [
          h.div([h.Class("size-10 shrink-0 overflow-hidden rounded-control")], [bringingArt(deck, knownCommanders)]),
          h.div(
            [h.Class("min-w-0")],
            [
              h.div([h.Class("text-label text-lichen")], ["Bringing"]),
              h.div([h.Class("truncate font-semibold")], [deckName]),
            ],
          ),
        ],
      ),
      h.div([h.Class("font-semibold text-title")], ["Join a table"]),
      h.div([h.Class("text-label text-lichen")], ["Paste the code your host shared"]),
      h.label([h.For("table-code"), h.Class("sr-only")], ["Table code"]),
      h.input([
        h.Id("table-code"),
        h.DataAttribute("testid", "lobby-join-code"),
        h.Placeholder("Table code"),
        h.Value(model.code),
        h.OnInput((code) => ChangedLobbyCode({ code })),
        h.Autocomplete("off"),
        h.Spellcheck(false),
        h.Class(fieldClass("w-full")),
      ]),
      h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "lobby-join"),
          h.Disabled(model.submitting),
          h.OnClick(RequestedLobbyJoin()),
          h.Class(buttonClass("primary")),
        ],
        ["Join table"],
      ),
      h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "lobby-join-cancel"),
          h.Disabled(model.submitting),
          h.OnClick(RequestedLobbyCancelJoin()),
          h.Class(buttonClass("ghost")),
        ],
        ["Cancel"],
      ),
      h.a(
        [h.Href(routePath(HomeRoute())), h.DataAttribute("testid", "lobby-back"), h.Class(buttonClass("ghost"))],
        ["Back"],
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
    return h.div([h.Class("text-caution-amber text-label")], ["Build a deck first (Your decks → New deck)."]);
  }

  if (model.selectedDeckId == null) {
    return h.div([h.Class("text-caution-amber text-label")], ["Pick a deck to play first (Your decks → Play)."]);
  }

  const deck = decks.find((item) => item.id === model.selectedDeckId);
  if (model.entryMode === "choose") {
    return chooseEntry(model, deck, decksLoading, knownCommanders);
  }

  return joinEntry(model, deck, decksLoading, knownCommanders);
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
    return h.div([h.Class("text-caution-amber text-label")], ["Build a deck first (Your decks → New deck)."]);
  }

  return h.div([h.Class("text-caution-amber text-label")], ["Pick a deck to play first (Your decks → Play)."]);
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
    stage: h.div(
      [h.Class("flex justify-center py-xxl")],
      [
        h.section(
          [
            h.DataAttribute("testid", "lobby"),
            h.DataAttribute("ui", "panel"),
            h.Class(panelClass("max-w-[min(100%-2rem,640px)]")),
          ],
          [
            body,
            model.error == null
              ? null
              : h.div(
                  [h.Role("alert"), h.DataAttribute("testid", "lobby-error"), h.Class("text-burn-red text-caption")],
                  [humanError(model.error)],
                ),
          ],
        ),
      ],
    ),
  });
});
