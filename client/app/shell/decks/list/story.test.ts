import { Submodel } from "foldkit";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import { BindCardArt, CardArtTick } from "../../../../lib/ui/card-art";
import { ModalOpened, OpenDialogAsModal } from "../../../../lib/ui/confirmDialog";
import type { CatalogCard } from "../../../../lib/wire/types";
import { ClosedDeckListMenu, type Message as DeckListMessage, OpenedDeckListMenu } from "./messages";
import { initialDeckListSubmodel } from "./submodel";
import { update } from "./update";
import { BindDeckListContextMenu, BindDeckListContextMenuEscape, view } from "./view";

const listView = Submodel.defineView<ReturnType<typeof initialDeckListSubmodel>, DeckListMessage>((model) =>
  view(model, "alice", null),
);
type SceneListMessage = DeckListMessage | typeof ModalOpened.Type | { readonly _tag?: string } | undefined;

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
    set: "tst",
    subtypes: [],
    summary: "",
    ...overrides,
  };
}

test("deck list chrome and tiles share the wide column classes", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      decks: [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }],
      knownCommanders: {
        atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice", default_print: "atraxa-print" }),
      },
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-search"]')).toHaveClass("max-w-[960px]"),
    Scene.expect(Scene.selector('[data-testid="deck-list-grid"]')).toHaveClass("max-w-[960px]"),
    Scene.expect(Scene.selector('[data-testid="deck-list-grid"]')).toHaveClass(
      "grid-cols-[repeat(auto-fill,minmax(220px,1fr))]",
    ),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
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
      [BindCardArt, CardArtTick()],
      [BindCardArt, CardArtTick()],
      [BindCardArt, CardArtTick()],
    ),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    Scene.type(Scene.selector('[data-testid="deck-list-search"]'), "mirror"),
    Scene.Mount.expectEnded(BindDeckListContextMenu, BindDeckListContextMenu, BindCardArt, BindCardArt),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--9"]')).toExist(),
    Scene.type(Scene.selector('[data-testid="deck-list-search"]'), "zzzz"),
    Scene.Mount.expectEnded(BindDeckListContextMenu, BindCardArt),
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
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
    Scene.click(Scene.selector('[data-testid="deck-list-menu-delete"]')),
    Scene.expect(Scene.selector('[data-testid="confirm-delete-dialog"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-context-menu"]')).not.toExist(),
    Scene.Mount.resolve(OpenDialogAsModal(), ModalOpened()),
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
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});
