import * as Dialog from "@foldkit/ui/dialog";
import { Effect, Match as M, Option, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import { lookupCardsByIds } from "../../../domain/deck-builder/lookup-cards";
import { RpcClient } from "../../../resources";
import {
  DeckDeleted,
  DeckDeleteFailed,
  DecksLoadFailed,
  GotConfirmDialogMessage,
  type Message,
  ReceivedDeckListCommanders,
  ReceivedDecks,
} from "./messages";

import type { DeckListSubmodel } from "./submodel";
import { deckListContextMenuAllowed } from "./visible";

export const FetchDecks = Command.define("FetchDecks", {
  messages: [ReceivedDecks, DecksLoadFailed],
  execute: Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.listDecks().pipe(
      Effect.map((decks) => ReceivedDecks({ decks })),
      Effect.catch(() => Effect.succeed(DecksLoadFailed({ message: "Could not load decks." }))),
    );
  }),
});

export const LookupDeckListCommanders = Command.define("LookupDeckListCommanders", {
  args: { ids: S.Array(S.String) },
  messages: [ReceivedDeckListCommanders],
  execute: ({ ids }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* lookupCardsByIds(rpc, ids).pipe(
        Effect.map((cards) => ReceivedDeckListCommanders({ cards })),
        Effect.catch(() => Effect.succeed(ReceivedDeckListCommanders({ cards: [] }))),
      );
    }),
});

export const DeleteDeck = Command.define("DeleteDeck", {
  args: { id: S.Number },
  messages: [DeckDeleted, DeckDeleteFailed],
  execute: ({ id }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.deleteDeck(String(id)).pipe(
        Effect.as(DeckDeleted()),
        Effect.catch(() => Effect.succeed(DeckDeleteFailed({ message: "Couldn't delete that deck — try again." }))),
      );
    }),
});

export function loadDeckList(
  model: DeckListSubmodel,
): readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  return [{ ...model, error: null, loading: true }, [FetchDecks()]];
}

type UpdateReturn = readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>];

const toConfirmDialogMessage = (message: Dialog.Message): Message => GotConfirmDialogMessage({ message });

/** Dismisses the delete confirmation and forgets which deck it was asking about. */
function closeDeleteConfirm(model: DeckListSubmodel): UpdateReturn {
  const [confirmDialog, commands] = Dialog.close(model.confirmDialog);
  return [{ ...model, confirmDialog, confirmingDeleteId: null }, Command.mapMessages(commands, toConfirmDialogMessage)];
}

function enterDeckListRoute(model: DeckListSubmodel): UpdateReturn {
  const [closed, closeCommands] = closeDeleteConfirm(model);
  return [
    {
      ...closed,
      contextMenu: null,
      error: null,
      loading: true,
    },
    [...closeCommands, FetchDecks()],
  ];
}

export const update = (
  model: DeckListSubmodel,
  message: Message,
): readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [DeckListSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
    M.tagsExhaustive({
      ChangedDeckListRoute: () => enterDeckListRoute(model),
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
      ChangedDeckListSearch: ({ query }) => [{ ...model, searchQuery: query }, []],
      OpenedDeckListMenu: ({ deckId, x, y }) => {
        if (!deckListContextMenuAllowed(deckId)) return [model, []];
        return [{ ...model, contextMenu: { deckId, x, y } }, []];
      },
      ClosedDeckListMenu: () => [{ ...model, contextMenu: null }, []],
      AskedDeckDelete: ({ id }) => {
        const [confirmDialog, commands] = Dialog.open(model.confirmDialog);
        return [
          { ...model, confirmDialog, confirmingDeleteId: id, error: null, contextMenu: null },
          Command.mapMessages(commands, toConfirmDialogMessage),
        ];
      },
      RequestedDeckDelete: () => {
        const id = model.confirmingDeleteId;
        const [closed, commands] = closeDeleteConfirm(model);
        if (id == null) return [closed, commands];
        return [closed, [...commands, DeleteDeck({ id })]];
      },
      // Escape, a backdrop click, and Cancel all reach here as Dialog's Closed out-message; the
      // deck being confirmed is forgotten with the same handler that a programmatic cancel uses.
      GotConfirmDialogMessage: ({ message }) => {
        const [confirmDialog, commands, outMessage] = Dialog.update(model.confirmDialog, message);
        const withDialog = { ...model, confirmDialog };
        const mapped = Command.mapMessages(commands, toConfirmDialogMessage);
        if (Option.isNone(outMessage) || outMessage.value._tag !== "Closed") return [withDialog, mapped];

        const [cancelled, cancelCommands] = closeDeleteConfirm(withDialog);
        return [cancelled, [...mapped, ...cancelCommands]];
      },
      DeckDeleted: () => loadDeckList(model),
      DeckDeleteFailed: ({ message }) => [{ ...model, error: message }, []],
    }),
  );
