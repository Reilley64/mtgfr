import { type Html, html } from "foldkit/html";
import { appVersionBadge } from "../../../lib/ui/app-version";
import { buttonClass } from "../../../lib/ui/buttonClass";
import { feltClass, fieldClass, panelClass } from "../../../lib/ui/surfaces";
import type { DeckSummary } from "../../../lib/wire/types";
import {
  ChangedLobbyCode,
  ChangedLobbyDeck,
  type Message,
  RequestedLobbyCopy,
  RequestedLobbyHost,
  RequestedLobbyJoin,
  RequestedLobbyReady,
  RequestedLobbyStart,
} from "./messages";
import type { LobbySlice } from "./submodel";
import { lobbyHost, lobbyReady } from "./update";

const h = html<Message>();

const seatDots = ["bg-seat-forest", "bg-seat-island", "bg-seat-mountain", "bg-seat-arcane"];

function humanError(code: string): string {
  const map: Record<string, string> = {
    TableFull: "That table is full.",
    AlreadyStarted: "The game already started.",
    NotHost: "Only the host can start.",
    NeedTwoPlayers: "Need at least two players.",
    NotAllReady: "Everyone must ready up first.",
    UnknownTable: "No such table.",
    NotSeated: "Claim a seat first.",
    UnknownDeck: "That deck no longer exists.",
    Draining: "Server is restarting — try again in a moment.",
    SeedFailed: "Couldn't start the game — try again.",
    Unreachable: "Couldn't reach the table — try again.",
  };
  return map[code] ?? code;
}

function pickedDeckName(model: LobbySlice, decks: ReadonlyArray<DeckSummary>): string {
  return decks.find((deck) => deck.id === model.selectedDeckId)?.name ?? "your deck";
}

function deckPicker(model: LobbySlice, decks: ReadonlyArray<DeckSummary>): Html {
  const selected = model.selectedDeckId ?? decks[0]?.id ?? "";

  return h.select(
    [
      h.Id("lobby-deck"),
      h.DataAttribute("testid", "lobby-deck"),
      h.Value(String(selected)),
      h.OnInput((value) => ChangedLobbyDeck({ deckId: Number(value) })),
      h.Class(fieldClass("min-w-0 flex-1")),
    ],
    decks.map((deck) => h.option([h.Value(String(deck.id)), h.Selected(deck.id === selected)], [deck.name])),
  );
}

function entry(model: LobbySlice, decks: ReadonlyArray<DeckSummary>): Html {
  // Deck is chosen on Your decks (Play) — host/join only, never a deck picker.
  if (model.selectedDeckId == null) {
    return h.div([h.Class("text-caution-amber text-label")], ["Pick a deck to play first (Your decks → Play)."]);
  }

  return h.div(
    [h.Class("flex flex-col gap-md")],
    [
      h.div(
        [h.Class("flex items-center gap-sm")],
        [
          h.span(
            [h.Class("text-label text-lichen"), h.DataAttribute("testid", "lobby-bring")],
            ["Bring: ", h.b([], [pickedDeckName(model, decks)])],
          ),
        ],
      ),
      h.div(
        [h.Class("flex items-center gap-sm")],
        [
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "lobby-host"),
              h.Disabled(model.submitting),
              h.OnClick(RequestedLobbyHost()),
              h.Class(buttonClass("primary")),
            ],
            ["Host a table"],
          ),
        ],
      ),
      h.div(
        [h.Class("flex flex-wrap items-center gap-sm")],
        [
          h.label([h.For("table-code"), h.Class("sr-only")], ["Table code"]),
          h.input([
            h.Id("table-code"),
            h.DataAttribute("testid", "lobby-join-code"),
            h.Placeholder("Table code"),
            h.Value(model.code),
            h.OnInput((code) => ChangedLobbyCode({ code })),
            h.Autocomplete("off"),
            h.Spellcheck(false),
            h.Class(fieldClass("min-w-0 flex-1")),
          ]),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "lobby-join"),
              h.Disabled(model.submitting),
              h.OnClick(RequestedLobbyJoin()),
              h.Class(buttonClass("primary")),
            ],
            ["Join"],
          ),
        ],
      ),
    ],
  );
}

function seats(model: LobbySlice): Html {
  return h.div(
    [h.Class("flex flex-col gap-sm"), h.DataAttribute("testid", "lobby-seats")],
    (model.view?.seats ?? []).map((seat) =>
      h.div(
        [
          h.Class(
            "grid grid-cols-[auto_minmax(7rem,11rem)_minmax(0,1fr)_auto] items-center gap-sm rounded-hud bg-glass-dim px-md py-sm",
          ),
          h.DataAttribute("testid", `lobby-seat-${seat.player}`),
          h.DataAttribute("claimed", seat.claimed ? "1" : "0"),
        ],
        [
          h.span([h.Class(`size-2.5 shrink-0 rounded-full ${seatDots[seat.player] ?? "bg-fog"}`)], []),
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

function claimSeat(model: LobbySlice, decks: ReadonlyArray<DeckSummary>, decksLoading: boolean): Html {
  if (decksLoading && model.selectedDeckId == null) {
    return h.div([h.Class("text-label text-lichen")], ["Loading decks…"]);
  }

  // Deck already chosen (Play → Host/Join, or ?deck= on the share link): claim only — no picker.
  if (model.selectedDeckId != null) {
    return h.div(
      [h.Class("flex flex-wrap items-center gap-sm")],
      [
        h.span(
          [h.Class("text-label text-lichen"), h.DataAttribute("testid", "lobby-bring")],
          ["Bring: ", h.b([], [pickedDeckName(model, decks)])],
        ),
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

  // Share-link arrival without a deck: pick once, then claim (only place the picker appears).
  if (decks.length === 0) {
    return h.div([h.Class("text-caution-amber text-label")], ["Build a deck first (Your decks → New deck)."]);
  }

  return h.div(
    [h.Class("flex flex-wrap items-center gap-sm")],
    [
      deckPicker(model, decks),
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

function tableLobby(model: LobbySlice, decks: ReadonlyArray<DeckSummary>, decksLoading: boolean): Html {
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
              h.Class("select-text font-bold text-display tracking-[0.06em]"),
            ],
            [model.tableId ?? ""],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "lobby-copy-code"),
              h.OnClick(RequestedLobbyCopy()),
              h.Class(buttonClass("primary")),
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
                    [h.DataAttribute("testid", "lobby-start-error"), h.Class("text-caption text-lichen")],
                    [humanError(startError)],
                  ),
            ],
          )
        : claimSeat(model, decks, decksLoading),
    ],
  );
}

export function view(
  model: LobbySlice,
  decks: ReadonlyArray<DeckSummary>,
  decksLoading: boolean,
  apiVersion: string | null,
): Html {
  return h.main(
    [h.Class(feltClass("fixed inset-0 overflow-y-auto"))],
    [
      h.div(
        [h.Class("flex min-h-full items-center justify-center p-xxl")],
        [
          h.section(
            [
              h.DataAttribute("testid", "lobby"),
              h.DataAttribute("ui", "panel"),
              h.Class(panelClass("max-w-[min(100%-2rem,560px)]")),
            ],
            [
              h.div(
                [h.Class("flex flex-col gap-xs")],
                [
                  h.div([h.Class("m-0 text-display tracking-[-0.02em]")], ["mtgfr"]),
                  h.h1([h.Class("m-0 text-lichen text-title")], ["Lobby"]),
                ],
              ),
              model.tableId == null ? entry(model, decks) : tableLobby(model, decks, decksLoading),
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
      appVersionBadge(h, apiVersion),
    ],
  );
}
