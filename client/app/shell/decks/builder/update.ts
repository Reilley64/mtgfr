import * as Dialog from "@foldkit/ui/dialog";
import * as VirtualList from "@foldkit/ui/virtualList";
import { Effect, Match as M, Option, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command, Navigation } from "foldkit";
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
import { printSearchUrl, searchPrintPage } from "../../../domain/deck-builder/scryfall";
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
  GotDiscardDialogMessage,
  GotPoolGridMessage,
  GotPrintDialogMessage,
  GotPrintGridMessage,
  HydratedBuilderCards,
  type Message,
  NavigatedAwayFromBuilder,
  ReceivedBuilderPrints,
  ReceivedBuilderSearchPage,
  ReceivedDeckForBuilder,
} from "./messages";
import {
  type DeckBuilderSubmodel,
  initialDeckBuilderSubmodel,
  PRINT_GRID_ID,
  poolGridColumns,
  poolGridRowHeightPx,
  printGridRowHeightPx,
  viewportWidthPx,
} from "./submodel";

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

/** Fetches one page of printings. The update re-issues this for `nextPage` until there is none,
 *  so a card with hundreds of printings shows its first 175 while the rest are still arriving. */
export const SearchBuilderPrints = Command.define(
  "SearchBuilderPrints",
  { cardId: S.String, url: S.String },
  ReceivedBuilderPrints,
  BuilderPrintSearchFailed,
)(({ cardId, url }) =>
  searchPrintPage(url).pipe(
    Effect.map(({ nextPage, prints }) => ReceivedBuilderPrints({ cardId, nextPage, prints, url })),
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

function setCount(model: DeckBuilderSubmodel, card: BuilderCatalogCard, count: number): DeckBuilderSubmodel {
  const entries = { ...model.entries };
  if (count <= 0) {
    delete entries[card.id];
    return { ...model, dirty: true, entries };
  }

  entries[card.id] = {
    count: BASICS.has(card.name) ? count : 1,
    print: entries[card.id]?.print ?? (printFor(model, card.id) || card.default_print),
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
  };
}

function saveBody(model: DeckBuilderSubmodel): SaveDeckRequestShape {
  return {
    cards: Object.entries(model.entries).map(
      ([id, entry]): DeckCardEntry => ({ count: entry.count, id, print: entry.print }),
    ),
    commander: model.commander.id,
    commander_print: model.commander.print,
    name: model.name,
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
      return openPrintPicker(closed, { addOnPick: action.addOnPick, cardId: action.cardId });
  }
}

type UpdateReturn = readonly [DeckBuilderSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>];

const toDiscardDialogMessage = (message: Dialog.Message): Message => GotDiscardDialogMessage({ message });
const toPrintDialogMessage = (message: Dialog.Message): Message => GotPrintDialogMessage({ message });
const toPrintGridMessage = (message: VirtualList.Message): Message => GotPrintGridMessage({ message });

const toPoolGridMessage = (message: VirtualList.Message): Message => GotPoolGridMessage({ message });

/** Rows of lookahead before the next page is fetched. Wider than VirtualList's own render overscan
 *  so the request is in flight before the blank rows would show. */
const POOL_PAGE_OVERSCAN_ROWS = 12;

/** Dismisses the discard confirmation. */
function closeDiscardConfirm(model: DeckBuilderSubmodel): UpdateReturn {
  const [discardDialog, commands] = Dialog.close(model.discardDialog);
  return [{ ...model, discardDialog }, Command.mapMessages(commands, toDiscardDialogMessage)];
}

/** Opens the print picker on a card and starts its search. Closes the context menu it came from. */
function openPrintPicker(model: DeckBuilderSubmodel, args: { addOnPick: boolean; cardId: string }): UpdateReturn {
  const [printDialog, commands] = Dialog.open(model.printDialog);
  const url = printSearchUrl(args.cardId);
  return [
    {
      ...model,
      menu: null,
      printDialog,
      // Tile height follows the viewport, and VirtualList fixes the row height at init, so the grid
      // is rebuilt each time the picker opens — also resetting it to the top.
      // ponytail: rotating the device with the picker open misaligns rows until it is reopened.
      printGrid: VirtualList.init({ id: PRINT_GRID_ID, rowHeightPx: printGridRowHeightPx(viewportWidthPx()) }),
      printPicker: { addOnPick: args.addOnPick, cardId: args.cardId, error: false, pendingPage: url, prints: [] },
    },
    [...Command.mapMessages(commands, toPrintDialogMessage), SearchBuilderPrints({ cardId: args.cardId, url })],
  ];
}

/** Asks the catalog for the next page, unless one is already in flight or the pool is complete. */
function nextPoolPage(model: DeckBuilderSubmodel): UpdateReturn {
  if (model.atEnd || model.searching) return [model, []];
  const offset = model.offset + PAGE;
  return [{ ...model, offset, searching: true }, [SearchDeckBuilderCards({ query: model.query, offset })]];
}

/** True once the windowed grid is rendering rows within an overscan of the end of the loaded pool.
 *  This is what pages the catalog: the windowed grid renders no sentinel element at the bottom to
 *  hang an IntersectionObserver on, because the bottom is not in the DOM until you scroll to it. */
function poolWindowNearEnd(model: DeckBuilderSubmodel): boolean {
  const columns = poolGridColumns(model.poolWidth);
  const rowCount = Math.ceil(model.pool.length / columns);
  return Option.match(VirtualList.visibleWindow(model.poolGrid, rowCount, POOL_PAGE_OVERSCAN_ROWS), {
    onNone: () => false,
    onSome: ({ endIndex }) => endIndex >= rowCount,
  });
}

/** Dismisses the print picker; the prints it loaded go with it. */
function closePrintPicker(model: DeckBuilderSubmodel): UpdateReturn {
  const [printDialog, commands] = Dialog.close(model.printDialog);
  return [{ ...model, printDialog, printPicker: null }, Command.mapMessages(commands, toPrintDialogMessage)];
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
      ChangedBuilderQuery: ({ query }) => {
        // New results start at the top. The container element survives the query change, so its
        // scroll position has to be driven back rather than reset in the model alone.
        const [poolGrid, scrollCommands] = VirtualList.scrollToIndex(model.poolGrid, 0);
        return [
          { ...model, atEnd: false, offset: 0, pool: [], poolGrid, query, searching: true },
          [SearchDeckBuilderCards({ query, offset: 0 }), ...Command.mapMessages(scrollCommands, toPoolGridMessage)],
        ];
      },
      ChangedBuilderRoute: ({ editingId }) => enterBuilder(editingId),
      ReceivedBuilderSearchPage: ({ cards, offset, query }) => {
        if (query !== model.query || offset !== model.offset) return [model, []];
        const seen = new Set(model.pool.map((card) => card.id));
        const pool = [...model.pool, ...cards.filter((card) => !seen.has(card.id))];
        const loaded = rememberCards({ ...model, atEnd: cards.length < PAGE, pool, searching: false }, cards);
        // A page that still does not reach past the viewport asks for the next one straight away —
        // nothing else would, since there is no scroll event to hang the request on.
        return poolWindowNearEnd(loaded) ? nextPoolPage(loaded) : [loaded, []];
      },
      BuilderSearchFailed: () => [{ ...model, atEnd: true, searching: false }, []],
      ReceivedDeckForBuilder: ({ deck }) => {
        const ids = [...deck.cards.map((card) => card.id), deck.commander].filter(Boolean);
        return [
          {
            ...model,
            commander: { id: deck.commander, print: deck.commander_print },
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
      OpenedBuilderPrintPicker: ({ addOnPick, cardId }) => openPrintPicker(model, { addOnPick, cardId }),
      // Pages append, and the next one is asked for only once this one is in the model, so a picker
      // that was closed and reopened cannot have a stale chain still feeding it.
      ReceivedBuilderPrints: ({ cardId, nextPage, prints, url }) => {
        const picker = model.printPicker;
        if (picker?.cardId !== cardId || picker.pendingPage !== url) return [model, []];
        return [
          {
            ...model,
            printPicker: { ...picker, error: false, pendingPage: nextPage, prints: [...picker.prints, ...prints] },
          },
          nextPage === null ? [] : [SearchBuilderPrints({ cardId, url: nextPage })],
        ];
      },
      // Pages that already landed stay on screen — a later page failing should not empty the picker.
      // ponytail: the picker then shows a short list with no hint that it is short.
      BuilderPrintSearchFailed: ({ cardId }) => {
        if (model.printPicker?.cardId !== cardId) return [model, []];
        return [{ ...model, printPicker: { ...model.printPicker, error: true, pendingPage: null } }, []];
      },
      PickedBuilderPrint: ({ cardId, print }) => closePrintPicker(pickPrint(model, cardId, print)),
      // Escape, a backdrop click, and Close all reach here as Dialog's Closed out-message.
      GotPrintDialogMessage: ({ message }) => {
        const [printDialog, commands, outMessage] = Dialog.update(model.printDialog, message);
        const withDialog = { ...model, printDialog };
        const mapped = Command.mapMessages(commands, toPrintDialogMessage);
        if (Option.isNone(outMessage) || outMessage.value._tag !== "Closed") return [withDialog, mapped];

        const [dismissed, dismissCommands] = closePrintPicker(withDialog);
        return [dismissed, [...mapped, ...dismissCommands]];
      },
      GotPrintGridMessage: ({ message }) => {
        const [printGrid, commands] = VirtualList.update(model.printGrid, message);
        return [{ ...model, printGrid }, Command.mapMessages(commands, toPrintGridMessage)];
      },
      GotPoolGridMessage: ({ message }) => {
        const [poolGrid, commands] = VirtualList.update(model.poolGrid, message);
        const scrolled = { ...model, poolGrid };
        const [paged, pageCommands] = poolWindowNearEnd(scrolled) ? nextPoolPage(scrolled) : [scrolled, []];
        return [paged, [...Command.mapMessages(commands, toPoolGridMessage), ...pageCommands]];
      },
      MeasuredPoolGrid: ({ width }) => {
        if (width === model.poolWidth) return [model, []];
        // `rowHeightPx` is fixed at init, but it is a plain field: writing it keeps the scroll
        // position and the container measurement that a re-init would have thrown away.
        const measured = {
          ...model,
          poolGrid: { ...model.poolGrid, rowHeightPx: poolGridRowHeightPx(width) },
          poolWidth: width,
        };
        // A wider pool fits more rows, which can leave the first page no longer filling it.
        return poolWindowNearEnd(measured) ? nextPoolPage(measured) : [measured, []];
      },
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
        if (model.menu != null || model.printPicker != null) return [{ ...model, hover: null }, []];
        return [{ ...model, hover: { id, print: printFor(model, id), x, y } }, []];
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
        if (!model.dirty) return [model, [NavigateHome()]];
        const [discardDialog, commands] = Dialog.open(model.discardDialog);
        return [{ ...model, discardDialog }, Command.mapMessages(commands, toDiscardDialogMessage)];
      },
      ConfirmedBuilderDiscard: () => {
        const [closed, commands] = closeDiscardConfirm(model);
        return [closed, [...commands, NavigateHome()]];
      },
      // Escape, a backdrop click, and Cancel all reach here as Dialog's Closed out-message.
      GotDiscardDialogMessage: ({ message }) => {
        const [discardDialog, commands, outMessage] = Dialog.update(model.discardDialog, message);
        const withDialog = { ...model, discardDialog };
        const mapped = Command.mapMessages(commands, toDiscardDialogMessage);
        if (Option.isNone(outMessage) || outMessage.value._tag !== "Closed") return [withDialog, mapped];

        const [cancelled, cancelCommands] = closeDiscardConfirm(withDialog);
        return [cancelled, [...mapped, ...cancelCommands]];
      },
      NavigatedAwayFromBuilder: () => [model, []],
    }),
  );
