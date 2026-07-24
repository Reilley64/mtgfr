import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import { lookupCardsByIds } from "../../../../lib/deck-builder/lookup-cards";
import { RpcClient } from "../../../resources";
import {
  DeckDeleted,
  DeckDeleteFailed,
  DecksLoadFailed,
  type Message,
  ReceivedDeckListCommanders,
  ReceivedDecks,
} from "./messages";

import type { DeckListSubmodel } from "./submodel";
import { deckListContextMenuAllowed } from "./visible";

export const FetchDecks = Command.define(
  "FetchDecks",
  ReceivedDecks,
  DecksLoadFailed,
)(
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.listDecks().pipe(
      Effect.map((decks) => ReceivedDecks({ decks })),
      Effect.catch(() => Effect.succeed(DecksLoadFailed({ message: "Could not load decks." }))),
    );
  }),
);

export const LookupDeckListCommanders = Command.define(
  "LookupDeckListCommanders",
  { ids: S.Array(S.String) },
  ReceivedDeckListCommanders,
)(({ ids }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* lookupCardsByIds(rpc, ids).pipe(
      Effect.map((cards) => ReceivedDeckListCommanders({ cards })),
      Effect.catch(() => Effect.succeed(ReceivedDeckListCommanders({ cards: [] }))),
    );
  }),
);

export const DeleteDeck = Command.define(
  "DeleteDeck",
  { id: S.Number },
  DeckDeleted,
  DeckDeleteFailed,
)(({ id }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.deleteDeck(String(id)).pipe(
      Effect.as(DeckDeleted()),
      Effect.catch(() => Effect.succeed(DeckDeleteFailed({ message: "Couldn't delete that deck — try again." }))),
    );
  }),
);

export function loadDeckList(
  model: DeckListSubmodel,
): readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  return [{ ...model, error: null, loading: true }, [FetchDecks()]];
}

export const update = (
  model: DeckListSubmodel,
  message: Message,
): readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
    M.tagsExhaustive({
      RequestedDecksRefresh: () => loadDeckList(model),
      ReceivedDecks: ({ decks }) => {
        const ids = [...new Set(decks.map((deck) => deck.commander).filter(Boolean))];
        return [{ ...model, decks: [...decks], error: null, loading: false }, [LookupDeckListCommanders({ ids })]];
      },
      DecksLoadFailed: ({ message }) => [{ ...model, error: message, loading: false }, []],
      ReceivedDeckListCommanders: ({ cards }) => [
        { ...model, knownCommanders: Object.fromEntries(cards.map((card) => [card.id, card])) },
        [],
      ],
      MovedDeckListHover: ({ id, print, x, y }) => [{ ...model, hover: { id, print, x, y } }, []],
      ClearedDeckListHover: () => [{ ...model, hover: null }, []],
      ChangedDeckListSearch: ({ query }) => [{ ...model, searchQuery: query }, []],
      OpenedDeckListMenu: ({ deckId, x, y }) => {
        if (!deckListContextMenuAllowed(deckId)) return [model, []];
        return [{ ...model, contextMenu: { deckId, x, y } }, []];
      },
      ClosedDeckListMenu: () => [{ ...model, contextMenu: null }, []],
      AskedDeckDelete: ({ id }) => [{ ...model, confirmingDeleteId: id, error: null, contextMenu: null }, []],
      CancelledDeckDelete: () => [{ ...model, confirmingDeleteId: null }, []],
      RequestedDeckDelete: ({ id }) => [{ ...model, confirmingDeleteId: null }, [DeleteDeck({ id })]],
      DeckDeleted: () => loadDeckList(model),
      DeckDeleteFailed: ({ message }) => [{ ...model, error: message }, []],
    }),
  );
