import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command, Navigation } from "foldkit";
import { proxyArtUrlError } from "../../../domain/card-art/proxy-url";
import {
  BASICS,
  type BuilderCatalogCard,
  canBeCommander,
  DECK_SIZE,
  deckCount,
  PAGE,
} from "../../../domain/deck-builder/cards";
import { lookupCardsByIds } from "../../../domain/deck-builder/lookup-cards";
import { commanderMenuItems, poolMenuItems, rowMenuItems } from "../../../domain/deck-builder/menu";
import { commanderPrintForRow, reconcileEntries } from "../../../domain/deck-builder/print";
import { searchPrints } from "../../../domain/deck-builder/scryfall";
import {
  type DeckCardEntry,
  SaveDeckRequest,
  type SaveDeckRequest as SaveDeckRequestShape,
} from "../../../domain/wire/types";
import { RpcClient } from "../../../resources";
import {
  type BuilderMenuActionSchema,
  BuilderPrintSearchFailed,
  BuilderSearchFailed,
  DeckBuilderLoadFailed,
  DeckSaved,
  DeckSaveFailed,
  HydratedBuilderCards,
  type Message,
  NavigatedAwayFromBuilder,
  ReceivedBuilderPrints,
  ReceivedBuilderSearchPage,
  ReceivedDeckForBuilder,
} from "./messages";
import { type DeckBuilderSubmodel, initialDeckBuilderSubmodel } from "./submodel";

export const NavigateHome = Command.define(
  "NavigateHome",
  NavigatedAwayFromBuilder,
)(Navigation.replaceUrl("/").pipe(Effect.as(NavigatedAwayFromBuilder())));

export const SearchDeckBuilderCards = Command.define(
  "SearchDeckBuilderCards",
  { offset: S.Number, query: S.String },
  ReceivedBuilderSearchPage,
  BuilderSearchFailed,
)(({ offset, query }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.searchCards({ q: query, limit: PAGE, offset }).pipe(
      Effect.map((cards) => ReceivedBuilderSearchPage({ cards, offset, query })),
      Effect.catch(() => Effect.succeed(BuilderSearchFailed())),
    );
  }),
);

export const LoadDeckForBuilder = Command.define(
  "LoadDeckForBuilder",
  { id: S.String },
  ReceivedDeckForBuilder,
  DeckBuilderLoadFailed,
)(({ id }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.getDeck(id).pipe(
      Effect.map((deck) => ReceivedDeckForBuilder({ deck })),
      Effect.catch(() => Effect.succeed(DeckBuilderLoadFailed({ message: "Could not load that deck." }))),
    );
  }),
);

export const HydrateBuilderCards = Command.define(
  "HydrateBuilderCards",
  { ids: S.Array(S.String) },
  HydratedBuilderCards,
)(({ ids }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* lookupCardsByIds(rpc, ids).pipe(
      Effect.map((cards) => HydratedBuilderCards({ cards })),
      Effect.catch(() => Effect.succeed(HydratedBuilderCards({ cards: [] }))),
    );
  }),
);

export const SearchBuilderPrints = Command.define(
  "SearchBuilderPrints",
  { cardId: S.String },
  ReceivedBuilderPrints,
  BuilderPrintSearchFailed,
)(({ cardId }) =>
  Effect.tryPromise(() => searchPrints(cardId)).pipe(
    Effect.map((prints) => ReceivedBuilderPrints({ cardId, prints })),
    Effect.catch(() => Effect.succeed(BuilderPrintSearchFailed({ cardId }))),
  ),
);

export const SaveDeck = Command.define(
  "SaveDeck",
  { body: SaveDeckRequest, id: S.NullOr(S.String) },
  DeckSaved,
  DeckSaveFailed,
)(({ body, id }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;

    if (id !== null) {
      return yield* rpc.updateDeck(id, body).pipe(
        Effect.tap(() => Navigation.replaceUrl("/")),
        Effect.as(DeckSaved()),
        Effect.catchTag("UpdateDeck422", (err) =>
          Effect.succeed(DeckSaveFailed({ problems: [...err.cause.problems] })),
        ),
        Effect.catch(() => Effect.succeed(DeckSaveFailed({ problems: ["Could not save the deck."] }))),
      );
    }

    return yield* rpc.createDeck(body).pipe(
      Effect.tap(() => Navigation.replaceUrl("/")),
      Effect.as(DeckSaved()),
      Effect.catchTag("CreateDeck422", (err) => Effect.succeed(DeckSaveFailed({ problems: [...err.cause.problems] }))),
      Effect.catch(() => Effect.succeed(DeckSaveFailed({ problems: ["Could not save the deck."] }))),
    );
  }),
);

export function enterBuilder(
  editingId: string | null,
): readonly [DeckBuilderSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const commands: Array<FoldkitCommand.Command<Message, never, RpcClient>> = [
    SearchDeckBuilderCards({ query: "", offset: 0 }),
  ];
  if (editingId !== null) commands.push(LoadDeckForBuilder({ id: editingId }));
  return [initialDeckBuilderSubmodel(editingId), commands];
}

function rememberCards(model: DeckBuilderSubmodel, cards: ReadonlyArray<BuilderCatalogCard>): DeckBuilderSubmodel {
  if (cards.length === 0) return model;

  const known = { ...model.known };
  const preferredPrint = { ...model.preferredPrint };
  for (const card of cards) {
    known[card.id] = card;
    if (!(card.id in preferredPrint)) preferredPrint[card.id] = card.default_print;
  }
  return { ...model, known, preferredPrint };
}

function printFor(model: DeckBuilderSubmodel, cardId: string): string {
  const preferred = model.preferredPrint[cardId];
  if (preferred) return preferred;
  if (model.commander.id === cardId && model.commander.print) return model.commander.print;
  const entry = model.entries[cardId];
  if (entry?.print) return entry.print;
  return model.known[cardId]?.default_print ?? "";
}

function proxyArtFor(model: DeckBuilderSubmodel, cardId: string): string | undefined {
  const entryProxyArt = model.entries[cardId]?.proxyArtUrl;
  if (entryProxyArt) return entryProxyArt;
  if (model.commander.id === cardId) return model.commander.proxyArtUrl;
  return undefined;
}

function proxyArtTargetKind(model: DeckBuilderSubmodel, cardId: string): "deck" | "commander" | null {
  if (model.entries[cardId] != null) return "deck";
  if (model.commander.id === cardId) return "commander";
  return null;
}

function setCount(model: DeckBuilderSubmodel, card: BuilderCatalogCard, count: number): DeckBuilderSubmodel {
  const entries = { ...model.entries };
  if (count <= 0) {
    delete entries[card.id];
    return { ...model, dirty: true, entries };
  }

  entries[card.id] = {
    count: BASICS.has(card.name) ? count : 1,
    print: entries[card.id]?.print ?? (printFor(model, card.id) || card.default_print),
    ...(entries[card.id]?.proxyArtUrl ? { proxyArtUrl: entries[card.id]?.proxyArtUrl } : {}),
  };
  return rememberCards({ ...model, dirty: true, entries }, [card]);
}

function addN(model: DeckBuilderSubmodel, card: BuilderCatalogCard, count: number): DeckBuilderSubmodel {
  return setCount(model, card, (model.entries[card.id]?.count ?? 0) + count);
}

function removeN(model: DeckBuilderSubmodel, card: BuilderCatalogCard, count: number): DeckBuilderSubmodel {
  return setCount(model, card, (model.entries[card.id]?.count ?? 0) - count);
}

function addOneWithPrint(model: DeckBuilderSubmodel, card: BuilderCatalogCard, print: string): DeckBuilderSubmodel {
  const entries = { ...model.entries };
  entries[card.id] = {
    count: BASICS.has(card.name) ? (entries[card.id]?.count ?? 0) + 1 : 1,
    print,
    ...(entries[card.id]?.proxyArtUrl ? { proxyArtUrl: entries[card.id]?.proxyArtUrl } : {}),
  };
  return rememberCards({ ...model, entries }, [card]);
}

function pickPrint(model: DeckBuilderSubmodel, cardId: string, print: string): DeckBuilderSubmodel {
  const entries = { ...model.entries };
  const picker = model.printPicker;
  const card = model.known[cardId];

  if (entries[cardId] != null) entries[cardId] = { ...entries[cardId], print };

  const withAddedCard =
    picker?.addOnPick === true && card != null ? addOneWithPrint({ ...model, entries }, card, print) : null;
  const nextEntries = withAddedCard?.entries ?? entries;
  const commanderPrint = commanderPrintForRow(model.commander.id, cardId, print);

  return {
    ...(withAddedCard ?? model),
    commander: commanderPrint == null ? model.commander : { ...model.commander, print: commanderPrint },
    dirty: true,
    entries: nextEntries,
    preferredPrint: { ...model.preferredPrint, [cardId]: print },
    printPicker: null,
  };
}

function saveBody(model: DeckBuilderSubmodel): SaveDeckRequestShape {
  return {
    cards: Object.entries(model.entries).map(
      ([id, entry]): DeckCardEntry => ({
        count: entry.count,
        id,
        print: entry.print,
        ...(entry.proxyArtUrl ? { proxy_art_url: entry.proxyArtUrl } : {}),
      }),
    ),
    commander: model.commander.id,
    commander_print: model.commander.print,
    ...(model.commander.proxyArtUrl ? { commander_proxy_art_url: model.commander.proxyArtUrl } : {}),
    name: model.name,
  };
}

function setProxyArt(model: DeckBuilderSubmodel, cardId: string, proxyArtUrl: string): DeckBuilderSubmodel {
  const target = proxyArtTargetKind(model, cardId);
  if (target == null) return { ...model, proxyArtPicker: null };

  const normalized = proxyArtUrl.trim();
  if (!normalized) return { ...model, proxyArtPicker: null };

  if (target === "deck") {
    const entry = model.entries[cardId];
    if (entry == null) return { ...model, proxyArtPicker: null };
    return {
      ...model,
      dirty: true,
      entries: { ...model.entries, [cardId]: { ...entry, proxyArtUrl: normalized } },
      hover: model.hover?.id === cardId ? { ...model.hover, proxyArtUrl: normalized } : model.hover,
      proxyArtPicker: null,
    };
  }

  return {
    ...model,
    commander: { ...model.commander, proxyArtUrl: normalized },
    dirty: true,
    hover: model.hover?.id === cardId ? { ...model.hover, proxyArtUrl: normalized } : model.hover,
    proxyArtPicker: null,
  };
}

function clearProxyArt(model: DeckBuilderSubmodel, cardId: string): DeckBuilderSubmodel {
  const target = proxyArtTargetKind(model, cardId);
  if (target == null) return { ...model, proxyArtPicker: null };

  if (target === "deck") {
    const entry = model.entries[cardId];
    if (entry == null) return { ...model, proxyArtPicker: null };
    const { proxyArtUrl: _proxyArtUrl, ...rest } = entry;
    return {
      ...model,
      dirty: true,
      entries: { ...model.entries, [cardId]: rest },
      hover: model.hover?.id === cardId ? { ...model.hover, proxyArtUrl: undefined } : model.hover,
      proxyArtPicker: null,
    };
  }

  const { proxyArtUrl: _proxyArtUrl, ...commander } = model.commander;
  return {
    ...model,
    commander,
    dirty: true,
    hover: model.hover?.id === cardId ? { ...model.hover, proxyArtUrl: undefined } : model.hover,
    proxyArtPicker: null,
  };
}

function resolveCard(model: DeckBuilderSubmodel, cardId: string): BuilderCatalogCard | undefined {
  return model.known[cardId] ?? model.pool.find((card) => card.id === cardId);
}

function openMenu(
  model: DeckBuilderSubmodel,
  args: { cardId: string; kind: "pool" | "deck" | "commander"; x: number; y: number },
): DeckBuilderSubmodel {
  const card = resolveCard(model, args.cardId);
  const title = card?.name ?? args.cardId;
  const total = deckCount(model.entries);
  const items =
    args.kind === "pool"
      ? card
        ? poolMenuItems({ card, inDeck: model.entries[args.cardId] != null, total })
        : []
      : args.kind === "deck"
        ? rowMenuItems({ card, total })
        : commanderMenuItems({ cardId: args.cardId });

  return {
    ...model,
    hover: null,
    menu: { items, title, x: args.x, y: args.y },
  };
}

function runMenuAction(
  model: DeckBuilderSubmodel,
  action: BuilderMenuActionSchema,
): readonly [DeckBuilderSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const closed = { ...model, menu: null };

  switch (action.kind) {
    case "add": {
      const card = resolveCard(closed, action.cardId);
      if (card == null) return [closed, []];
      return [addN(closed, card, action.count), []];
    }
    case "remove": {
      const card = resolveCard(closed, action.cardId);
      if (card == null) return [closed, []];
      return [removeN(closed, card, action.count), []];
    }
    case "fill": {
      const card = resolveCard(closed, action.cardId);
      if (card == null || action.count <= 0) return [closed, []];
      return [addN(closed, card, action.count), []];
    }
    case "setCommander": {
      const card = resolveCard(closed, action.cardId);
      if (card == null || !canBeCommander(card)) return [closed, []];
      return [
        rememberCards(
          {
            ...closed,
            commander: { id: card.id, print: printFor(closed, card.id) || card.default_print },
            dirty: true,
          },
          [card],
        ),
        [],
      ];
    }
    case "choosePrint":
      return [
        {
          ...closed,
          printPicker: {
            addOnPick: action.addOnPick,
            cardId: action.cardId,
            error: false,
            loading: true,
            prints: [],
          },
        },
        [SearchBuilderPrints({ cardId: action.cardId })],
      ];
    case "setProxyArt":
      return [
        {
          ...closed,
          hover: null,
          proxyArtPicker: {
            cardId: action.cardId,
            error: null,
            url: proxyArtFor(closed, action.cardId) ?? "",
          },
        },
        [],
      ];
  }
}

export const update = (
  model: DeckBuilderSubmodel,
  message: Message,
): readonly [DeckBuilderSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<
      readonly [DeckBuilderSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]
    >(),
    M.tagsExhaustive({
      ChangedBuilderName: ({ name }) => [{ ...model, dirty: true, name }, []],
      ChangedBuilderQuery: ({ query }) => [
        { ...model, atEnd: false, offset: 0, pool: [], query, searching: true },
        [SearchDeckBuilderCards({ query, offset: 0 })],
      ],
      ChangedBuilderRoute: ({ editingId }) => enterBuilder(editingId),
      RequestedNextBuilderPage: () => {
        if (model.atEnd || model.searching) return [model, []];
        const offset = model.offset + PAGE;
        return [{ ...model, offset, searching: true }, [SearchDeckBuilderCards({ query: model.query, offset })]];
      },
      ReceivedBuilderSearchPage: ({ cards, offset, query }) => {
        if (query !== model.query || offset !== model.offset) return [model, []];
        const seen = new Set(model.pool.map((card) => card.id));
        const pool = [...model.pool, ...cards.filter((card) => !seen.has(card.id))];
        return [rememberCards({ ...model, atEnd: cards.length < PAGE, pool, searching: false }, cards), []];
      },
      BuilderSearchFailed: () => [{ ...model, atEnd: true, searching: false }, []],
      ReceivedDeckForBuilder: ({ deck }) => {
        const ids = [...deck.cards.map((card) => card.id), deck.commander].filter(Boolean);
        return [
          {
            ...model,
            commander: {
              id: deck.commander,
              print: deck.commander_print,
              ...(deck.commander_proxy_art_url ? { proxyArtUrl: deck.commander_proxy_art_url } : {}),
            },
            dirty: false,
            entries: reconcileEntries(deck.cards),
            loadingDeck: false,
            name: deck.name,
            preferredPrint: {
              ...model.preferredPrint,
              ...Object.fromEntries(deck.cards.map((card) => [card.id, card.print])),
              [deck.commander]: deck.commander_print,
            },
          },
          [HydrateBuilderCards({ ids })],
        ];
      },
      DeckBuilderLoadFailed: ({ message }) => [{ ...model, loadingDeck: false, problems: [message] }, []],
      HydratedBuilderCards: ({ cards }) => [rememberCards(model, cards), []],
      AddedBuilderCard: ({ card }) => [addN(model, card, 1), []],
      RemovedBuilderCard: ({ id }) => {
        const card = resolveCard(model, id);
        if (card == null) return [model, []];
        return [removeN(model, card, 1), []];
      },
      SetBuilderCommander: ({ card }) => {
        if (card == null) return [{ ...model, commander: { id: "", print: "" }, dirty: true }, []];
        if (!canBeCommander(card)) return [model, []];
        return [
          rememberCards(
            {
              ...model,
              commander: { id: card.id, print: printFor(model, card.id) || card.default_print },
              dirty: true,
            },
            [card],
          ),
          [],
        ];
      },
      OpenedBuilderPrintPicker: ({ addOnPick, cardId }) => [
        { ...model, menu: null, printPicker: { addOnPick, cardId, error: false, loading: true, prints: [] } },
        [SearchBuilderPrints({ cardId })],
      ],
      ReceivedBuilderPrints: ({ cardId, prints }) => {
        if (model.printPicker?.cardId !== cardId) return [model, []];
        return [
          {
            ...model,
            printPicker: { ...model.printPicker, error: false, loading: false, prints: [...prints] },
          },
          [],
        ];
      },
      BuilderPrintSearchFailed: ({ cardId }) => {
        if (model.printPicker?.cardId !== cardId) return [model, []];
        return [{ ...model, printPicker: { ...model.printPicker, error: true, loading: false, prints: [] } }, []];
      },
      PickedBuilderPrint: ({ cardId, print }) => [pickPrint(model, cardId, print), []],
      ClosedBuilderPrintPicker: () => [{ ...model, printPicker: null }, []],
      OpenedBuilderProxyArtPicker: ({ cardId }) => [
        {
          ...model,
          hover: null,
          menu: null,
          proxyArtPicker: {
            cardId,
            error: null,
            url: proxyArtFor(model, cardId) ?? "",
          },
        },
        [],
      ],
      ChangedBuilderProxyArtUrl: ({ url }) => {
        if (model.proxyArtPicker == null) return [model, []];
        return [
          {
            ...model,
            proxyArtPicker: {
              ...model.proxyArtPicker,
              error: proxyArtUrlError(url),
              url,
            },
          },
          [],
        ];
      },
      SubmittedBuilderProxyArt: () => {
        const picker = model.proxyArtPicker;
        if (picker == null) return [model, []];
        if (picker.error != null || picker.url.trim() === "") return [model, []];
        return [setProxyArt(model, picker.cardId, picker.url), []];
      },
      ClearedBuilderProxyArt: () => {
        const picker = model.proxyArtPicker;
        if (picker == null) return [model, []];
        return [clearProxyArt(model, picker.cardId), []];
      },
      ClosedBuilderProxyArtPicker: () => [{ ...model, proxyArtPicker: null }, []],
      SubmittedDeckSave: () => {
        if (model.saving) return [model, []];
        if (deckCount(model.entries) > DECK_SIZE) {
          return [
            { ...model, problems: [`Deck has ${deckCount(model.entries)} cards; remove extras before saving.`] },
            [],
          ];
        }
        return [{ ...model, problems: [], saving: true }, [SaveDeck({ id: model.editingId, body: saveBody(model) })]];
      },
      DeckSaved: () => [{ ...model, dirty: false, saving: false }, []],
      DeckSaveFailed: ({ problems }) => [{ ...model, problems: [...problems], saving: false }, []],
      MovedBuilderHover: ({ id, x, y }) => {
        if (model.menu != null || model.printPicker != null || model.proxyArtPicker != null) {
          return [{ ...model, hover: null }, []];
        }
        return [{ ...model, hover: { id, print: printFor(model, id), proxyArtUrl: proxyArtFor(model, id), x, y } }, []];
      },
      ClearedBuilderHover: () => [{ ...model, hover: null }, []],
      OpenedBuilderMenu: ({ cardId, kind, x, y }) => [openMenu(model, { cardId, kind, x, y }), []],
      ClosedBuilderMenu: () => [{ ...model, menu: null }, []],
      RanBuilderMenuAction: ({ action }) => runMenuAction(model, action),
      ActivatedBuilderTarget: ({ cardId, kind }) => {
        if (kind === "pool") {
          const card = resolveCard(model, cardId);
          if (card == null) return [model, []];
          return [addN(model, card, 1), []];
        }
        if (kind === "deck") {
          const card = resolveCard(model, cardId);
          if (card == null) return [model, []];
          return [{ ...removeN(model, card, 1), hover: null }, []];
        }
        return [{ ...model, commander: { id: "", print: "" }, dirty: true }, []];
      },
      RequestedBuilderCancel: () => {
        if (model.dirty) return [{ ...model, confirmingDiscard: true }, []];
        return [model, [NavigateHome()]];
      },
      ConfirmedBuilderDiscard: () => [{ ...model, confirmingDiscard: false }, [NavigateHome()]],
      CancelledBuilderDiscard: () => [{ ...model, confirmingDiscard: false }, []],
      NavigatedAwayFromBuilder: () => [model, []],
    }),
  );
