import * as Dialog from "@foldkit/ui/dialog";
import * as Menu from "@foldkit/ui/menu";
import { Story, Submodel } from "foldkit";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { BindDeckCardFlip, DeckCardFlipTick } from "../../../deck-card-nav";
import { BindCardArt, CardArtTick } from "../../../domain/ui/card-art";
import type { CatalogCard } from "../../../domain/wire/types";
import { update as appUpdate, init } from "../../../main-exports";
import { GotDeckListMessage } from "../../../messages";
import {
  ClosedDeckListMenu,
  DeckDeleted,
  type Message as DeckListMessage,
  OpenedDeckListMenu,
  ReceivedDeckListCommanders,
  ReceivedDecks,
  RequestedDecksRefresh,
} from "./messages";
import { DELETE_DIALOG_ID, initialDeckListSubmodel } from "./submodel";
import { DeleteDeck, FetchDecks, LookupDeckListCommanders, update } from "./update";
import { BindDeckListContextMenu, BindDeckListContextMenuEscape, view } from "./view";

const emptyChrome = { version: null, faithfulCount: null, oracleTotal: null, coverageHref: null };
const accountMenu = Menu.init({ id: "account-menu" });

const listView = Submodel.defineView<ReturnType<typeof initialDeckListSubmodel>, DeckListMessage>((model) =>
  view(model, { username: "alice", meGravatarHash: null, chrome: emptyChrome, accountMenu }),
);
type SceneListMessage = DeckListMessage | { readonly _tag?: string } | undefined;

function isDeckListMessage(message: SceneListMessage): message is DeckListMessage {
  switch (message?._tag) {
    case "RequestedDecksRefresh":
    case "ReceivedDecks":
    case "DecksLoadFailed":
    case "ReceivedDeckListCommanders":
    case "ChangedDeckListSearch":
    case "OpenedDeckListMenu":
    case "ClosedDeckListMenu":
    case "AskedDeckDelete":
    case "CancelledDeckDelete":
    case "RequestedDeckDelete":
    case "GotConfirmDialogMessage":
    case "DeckDeleted":
    case "DeckDeleteFailed":
      return true;
    default:
      return false;
  }
}

const listUpdate = (model: ReturnType<typeof initialDeckListSubmodel>, message: SceneListMessage) => {
  if (!isDeckListMessage(message)) return [model, []] as const;
  return update(model, message);
};
const listProgram = { update: listUpdate, view: listView };

function card(overrides: Partial<CatalogCard> = {}): CatalogCard {
  return {
    color_identity: [],
    cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
    default_print: `${overrides.id ?? "card"}-print`,
    id: "card",
    keywords: [],
    kind: { kind: "artifact" },
    legendary: false,
    name: "Card",
    otags: [],
    set: "",
    sets: ["tst"],
    subtypes: [],
    summary: [],
    ...overrides,
  };
}

test("GotDeckListMessage updates the deck list through the parent update", () => {
  const [model] = init();

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(GotDeckListMessage({ message: RequestedDecksRefresh() })),
    Story.model((m) => {
      expect(m.decks.list.loading).toBe(true);
      expect(m.decks.list.error).toBeNull();
    }),
    Story.Command.resolve(FetchDecks, ReceivedDecks({ decks: [] })),
    Story.Command.resolve(LookupDeckListCommanders({ ids: [] }), ReceivedDeckListCommanders({ cards: [] })),
  );
});

test("deck list chrome and tiles use the shell stage width", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      decks: [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }],
      knownCommanders: {
        atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice", default_print: "atraxa-print" }),
      },
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-search"]')).toHaveClass("w-full"),
    Scene.expect(Scene.selector('[data-testid="deck-list-search"]')).not.toHaveClass("max-w-[960px]"),
    Scene.expect(Scene.selector('[data-testid="deck-list-grid"]')).toHaveClass("w-full"),
    Scene.expect(Scene.selector('[data-testid="deck-list-grid"]')).not.toHaveClass("max-w-[960px]"),
    Scene.expect(Scene.selector('[data-testid="deck-list-grid"]')).toHaveClass(
      "grid-cols-[repeat(auto-fill,minmax(220px,1fr))]",
    ),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
  );
});

test("empty deck list points players to deck creation", () => {
  Scene.scene(
    listProgram,
    Scene.with({ ...initialDeckListSubmodel(), decks: [], loading: false }),
    Scene.expect(Scene.selector('[data-testid="deck-list-empty"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-empty"] a[href="/decks/new"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-new-deck"][href="/decks/new"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
  );
});

test("deck list does not render a hover preview", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      knownCommanders: {
        atraxa: card({
          id: "atraxa",
          name: "Atraxa, Praetors' Voice",
          color_identity: [2, 4, 5],
          default_print: "atraxa-print",
          legendary: true,
          kind: { kind: "creature", power: 4, toughness: 4 },
        }),
      },
      decks: [{ commander: "atraxa", commander_print: "atraxa-print", id: 1, name: "Superfriends" }],
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-hover-preview"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
  );
});

test("tile Play href uses /play/:deckId and search filters tiles", () => {
  const knownCommanders = {
    atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice", color_identity: [0, 1, 2, 4] }),
    breena: card({ id: "breena", name: "Breena, the Demagogue" }),
    riku: card({ id: "riku", name: "Riku of Two Reflections" }),
  };
  const decks = [
    { id: 1, name: "Superfriends", commander: "atraxa", commander_print: "" },
    { id: -1, name: "Silverquill Influence", commander: "breena", commander_print: "" },
    { id: -9, name: "Mirror Mastery", commander: "riku", commander_print: "" },
  ];

  Scene.scene(
    listProgram,
    Scene.with({ ...initialDeckListSubmodel(), decks, knownCommanders }),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"][href="/play/1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--9"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--1"]')).toExist(),
    Scene.expectAll(Scene.all.selector('[data-testid^="deck-tile-"]')).toHaveCount(3),
    Scene.expect(Scene.nth(Scene.all.selector('[data-testid^="deck-tile-"]'), 0)).toHaveAttr(
      "data-testid",
      "deck-tile-1",
    ),
    Scene.expect(Scene.nth(Scene.all.selector('[data-testid^="deck-tile-"]'), 1)).toHaveAttr(
      "data-testid",
      "deck-tile--9",
    ),
    Scene.expect(Scene.nth(Scene.all.selector('[data-testid^="deck-tile-"]'), 2)).toHaveAttr(
      "data-testid",
      "deck-tile--1",
    ),
    Scene.Mount.resolveAll(
      [BindDeckListContextMenu, ClosedDeckListMenu()],
      [BindDeckListContextMenu, ClosedDeckListMenu()],
      [BindDeckListContextMenu, ClosedDeckListMenu()],
      [BindDeckCardFlip, DeckCardFlipTick()],
      [BindDeckCardFlip, DeckCardFlipTick()],
      [BindDeckCardFlip, DeckCardFlipTick()],
      [BindCardArt, CardArtTick()],
      [BindCardArt, CardArtTick()],
      [BindCardArt, CardArtTick()],
    ),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    Scene.type(Scene.selector('[data-testid="deck-list-search"]'), "mirror"),
    Scene.Mount.expectEnded(
      BindDeckListContextMenu,
      BindDeckListContextMenu,
      BindDeckCardFlip,
      BindDeckCardFlip,
      BindCardArt,
      BindCardArt,
    ),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--9"]')).toExist(),
    Scene.type(Scene.selector('[data-testid="deck-list-search"]'), "zzzz"),
    Scene.Mount.expectEnded(BindDeckListContextMenu, BindDeckCardFlip, BindCardArt),
    Scene.expect(Scene.selector('[data-testid="deck-list-filter-empty"]')).toExist(),
    Scene.expect(Scene.text("No decks match.")).toExist(),
  );
});

test("owned deck context menu offers Edit and Delete", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      contextMenu: { deckId: 1, x: 40, y: 50 },
      decks: [
        { id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" },
        { id: -1, name: "Silverquill Influence", commander: "breena", commander_print: "" },
      ],
      knownCommanders: {
        atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice" }),
        breena: card({ id: "breena", name: "Breena, the Demagogue" }),
      },
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-context-menu"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-menu-edit"][href="/decks/1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-menu-delete"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: -1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: -1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("menu Delete opens the confirm dialog", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      contextMenu: { deckId: 1, x: 40, y: 50 },
      decks: [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }],
      knownCommanders: { atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice" }) },
    }),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), OpenedDeckListMenu({ deckId: 1, x: 40, y: 50 })),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.click(Scene.selector('[data-testid="deck-list-menu-delete"]')),
    Scene.Command.resolve(Dialog.ShowDialog, Dialog.CompletedShowDialog()),
    Scene.expect(Scene.selector('[data-testid="confirm-delete-dialog"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-context-menu"]')).not.toExist(),
  );
});

test("Escape closes the context menu", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      contextMenu: { deckId: 1, x: 40, y: 50 },
      decks: [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }],
      knownCommanders: { atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice" }) },
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-context-menu"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), OpenedDeckListMenu({ deckId: 1, x: 40, y: 50 })),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    Scene.expect(Scene.selector('[data-testid="deck-list-context-menu"]')).not.toExist(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

/** One deck, with its delete prompt already up — what `AskedDeckDelete` leaves behind. */
const askingToDelete = {
  ...initialDeckListSubmodel(),
  confirmDialog: Dialog.init({ id: DELETE_DIALOG_ID, isOpen: true }),
  confirmingDeleteId: 1,
  decks: [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }],
  knownCommanders: { atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice" }) },
};

/** Mounts the one deck tile plus the list-level Escape binding put in the tree on every render. */
const resolveDeckListMounts = (times: number) =>
  Array.from({ length: times }, () => [
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  ]).flat();

test("backing out of the delete prompt leaves the deck where it was", () => {
  Scene.scene(
    listProgram,
    Scene.with(askingToDelete),
    ...resolveDeckListMounts(1),
    Scene.click(Scene.selector('[data-testid="confirm-cancel"]')),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.expect(Scene.selector('[data-testid="confirm-title"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
  );
});

test("clicking the dimmed page behind the prompt cancels the delete", () => {
  Scene.scene(
    listProgram,
    Scene.with(askingToDelete),
    ...resolveDeckListMounts(1),
    Scene.click(Scene.selector('[data-testid="confirm-backdrop"]')),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.expect(Scene.selector('[data-testid="confirm-title"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
  );
});

test("confirming the prompt deletes the deck and the tile is gone on reload", () => {
  Scene.scene(
    listProgram,
    Scene.with(askingToDelete),
    ...resolveDeckListMounts(1),
    Scene.click(Scene.selector('[data-testid="confirm-ok"]')),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.Command.resolve(DeleteDeck({ id: 1 }), DeckDeleted()),
    Scene.Command.resolve(FetchDecks, ReceivedDecks({ decks: [] })),
    Scene.Command.resolve(LookupDeckListCommanders({ ids: [] }), ReceivedDeckListCommanders({ cards: [] })),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-empty"]')).toExist(),
    // The tile's own mounts go away with it.
    Scene.Mount.expectEnded(BindDeckListContextMenu),
    Scene.Mount.expectEnded(BindDeckCardFlip),
    Scene.Mount.expectEnded(BindCardArt),
  );
});
