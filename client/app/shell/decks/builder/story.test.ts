import * as Dialog from "@foldkit/ui/dialog";
import * as Menu from "@foldkit/ui/menu";
import { Effect, Option } from "effect";
import { Story } from "foldkit";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import type { ScryfallPrint } from "../../../domain/deck-builder/scryfall";
import { client } from "../../../domain/rpc-client";
import { BindCardArt } from "../../../domain/ui/card-art";
import { type CatalogCard, CreateDeck422, type SaveDeckRequest } from "../../../domain/wire/types";
import { update as appUpdate, init } from "../../../main-exports";
import { GotDeckBuilderMessage, UrlChanged } from "../../../messages";
import { RpcClient } from "../../../resources";
import { DeckRoute } from "../../../routes";
import {
  ActivatedBuilderTarget,
  AddedBuilderCard,
  type Message as BuilderMessage,
  ChangedBuilderName,
  ClearedBuilderHover,
  ConfirmedBuilderDiscard,
  DeckSaveFailed,
  GotDiscardDialogMessage,
  MovedBuilderHover,
  NavigatedAwayFromBuilder,
  OpenedBuilderMenu,
  OpenedBuilderPrintPicker,
  PickedBuilderPrint,
  RanBuilderMenuAction,
  ReceivedBuilderPrints,
  ReceivedBuilderSearchPage,
  RemovedBuilderCard,
  RequestedBuilderCancel,
  SetBuilderCommander,
} from "./messages";
import { DISCARD_DIALOG_ID, initialDeckBuilderSubmodel, PRINT_DIALOG_ID } from "./submodel";
import { update as builderUpdate, NavigateHome, SaveDeck, SearchBuilderPrints } from "./update";
import { BindBuilderCardPointer, view as builderView } from "./view";

const emptyChrome = { version: null, faithfulCount: null, oracleTotal: null, coverageHref: null };
const me = { id: 1, email: "alice@example.com", username: "alice" };
const emptyViewInputs = {
  chrome: emptyChrome,
  username: me.username,
  meGravatarHash: null,
  accountMenu: Menu.init({ id: "account-menu" }),
};

const url = (pathname: string, search = "") => ({
  protocol: "http:",
  host: "localhost",
  port: Option.none<string>(),
  pathname,
  search: search === "" ? Option.none<string>() : Option.some(search),
  hash: Option.none<string>(),
});

function appMessage(message: BuilderMessage) {
  return GotDeckBuilderMessage({ message });
}

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

function print(overrides: Partial<ScryfallPrint> = {}): ScryfallPrint {
  return {
    collector_number: "1",
    id: "print-1",
    released_at: "2024-01-01",
    set: "tst",
    set_name: "Test Set",
    ...overrides,
  };
}

test("GotDeckBuilderMessage updates the builder through the parent update", () => {
  const [model] = init();

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ChangedBuilderName({ name: "x" }))),
    Story.model((m) => {
      expect(m.decks.builder.name).toBe("x");
      expect(m.decks.builder.dirty).toBe(true);
    }),
  );
});

test("UrlChanged to DeckRoute resets stale builder state through the parent route entry", () => {
  const [base] = init();
  const staleCard = card({ id: "sol-ring", name: "Sol Ring" });
  const [next, commands] = appUpdate(
    {
      ...base,
      currentPath: "/",
      route: DeckRoute({ id: "old-deck" }),
      sessionLoaded: true,
      session: { me, meGravatarHash: null },
      decks: {
        ...base.decks,
        builder: {
          ...base.decks.builder,
          atEnd: true,
          commander: { id: staleCard.id, print: staleCard.default_print },
          discardDialog: Dialog.init({ id: DISCARD_DIALOG_ID, isOpen: true }),
          dirty: true,
          editingId: "old-deck",
          entries: { [staleCard.id]: { count: 1, print: staleCard.default_print } },
          hover: { id: staleCard.id, print: staleCard.default_print, x: 10, y: 20 },
          known: { [staleCard.id]: staleCard },
          loadingDeck: false,
          menu: { items: [], title: "Old deck", x: 10, y: 20 },
          name: "Old deck",
          offset: 50,
          pool: [staleCard],
          preferredPrint: { [staleCard.id]: staleCard.default_print },
          printPicker: { addOnPick: false, cardId: staleCard.id, error: false, loading: false, prints: [] },
          problems: ["Could not save the deck."],
          query: "mana",
          saving: true,
          searching: false,
        },
      },
    },
    UrlChanged({ url: url("/decks/abc") }),
  );

  expect(next.route).toEqual(DeckRoute({ id: "abc" }));
  expect(next.currentPath).toBe("/decks/abc");
  expect(next.decks.builder).toEqual(initialDeckBuilderSubmodel("abc"));
  expect(commands).toMatchObject([
    { name: "SearchDeckBuilderCards", args: { query: "", offset: 0 } },
    { name: "LoadDeckForBuilder", args: { id: "abc" } },
  ]);
});

test("CreateDeck422 folds into problems list", () => {
  const [model] = init();

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(DeckSaveFailed({ problems: ["Too many cards"] }))),
    Story.model((m) => {
      expect(m.decks.builder.problems).toEqual(["Too many cards"]);
    }),
  );
});

test("non-basic cards stay singleton while basics can be added and removed", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const island = card({ id: "island", kind: { kind: "land", colors: [1] }, name: "Island" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing, island], offset: 0, query: "" }))),
    Story.message(appMessage(AddedBuilderCard({ card: solRing }))),
    Story.message(appMessage(AddedBuilderCard({ card: solRing }))),
    Story.message(appMessage(AddedBuilderCard({ card: island }))),
    Story.message(appMessage(AddedBuilderCard({ card: island }))),
    Story.message(appMessage(RemovedBuilderCard({ id: "island" }))),
    Story.model((m) => {
      expect(m.decks.builder.entries["sol-ring"]?.count).toBe(1);
      expect(m.decks.builder.entries.island?.count).toBe(1);
    }),
  );
});

test("picking a print for a commander-only card updates commander art", () => {
  const [model] = init();
  const commander = card({
    id: "commander",
    kind: { kind: "creature", power: 2, toughness: 2 },
    legendary: true,
    name: "Commander",
  });
  const alternatePrint = print({ id: "commander-alt-print" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [commander], offset: 0, query: "" }))),
    Story.message(appMessage(SetBuilderCommander({ card: commander }))),
    Story.message(appMessage(OpenedBuilderPrintPicker({ addOnPick: false, cardId: "commander" }))),
    Story.Command.resolve(Dialog.ShowDialog, Dialog.CompletedShowDialog()),
    Story.Command.resolve(
      SearchBuilderPrints,
      ReceivedBuilderPrints({ cardId: "commander", prints: [alternatePrint] }),
    ),
    Story.message(appMessage(PickedBuilderPrint({ cardId: "commander", print: alternatePrint.id }))),
    Story.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Story.model((m) => {
      expect(m.decks.builder.commander.print).toBe(alternatePrint.id);
      expect(m.decks.builder.entries.commander).toBeUndefined();
      expect(m.decks.builder.preferredPrint.commander).toBe(alternatePrint.id);
      expect(m.decks.builder.printPicker).toBeNull();
    }),
  );
});

test("picking a print for a deck row updates preferredPrint and the entry print", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const alternatePrint = print({ id: "sol-ring-alt-print", collector_number: "42" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing], offset: 0, query: "" }))),
    Story.message(appMessage(AddedBuilderCard({ card: solRing }))),
    Story.message(appMessage(OpenedBuilderPrintPicker({ addOnPick: false, cardId: "sol-ring" }))),
    Story.Command.resolve(Dialog.ShowDialog, Dialog.CompletedShowDialog()),
    Story.Command.resolve(SearchBuilderPrints, ReceivedBuilderPrints({ cardId: "sol-ring", prints: [alternatePrint] })),
    Story.message(appMessage(PickedBuilderPrint({ cardId: "sol-ring", print: alternatePrint.id }))),
    Story.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Story.model((m) => {
      expect(m.decks.builder.entries["sol-ring"]?.print).toBe(alternatePrint.id);
      expect(m.decks.builder.entries["sol-ring"]?.count).toBe(1);
      expect(m.decks.builder.preferredPrint["sol-ring"]).toBe(alternatePrint.id);
      expect(m.decks.builder.printPicker).toBeNull();
    }),
  );
});

test("catalog and decklist are independent overscroll-contained scroll hosts", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
    preferredPrint: { "sol-ring": solRing.default_print },
    printPicker: null,
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolve(BindCardArt, ClearedBuilderHover() as never),
    // Fill the contained shell stage — h-dvh under the header made the whole shell page scroll.
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toHaveClass("h-full"),
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toHaveClass("min-h-0"),
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toHaveClass("flex-1"),
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toHaveClass("grid-rows-[minmax(0,1fr)]"),
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toHaveClass("overflow-hidden"),
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).not.toHaveClass("h-dvh"),
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).not.toHaveClass("min-h-screen"),
    Scene.expect(Scene.selector('[data-testid="builder-pool-scroll"]')).toHaveClass("overflow-y-auto"),
    Scene.expect(Scene.selector('[data-testid="builder-pool-scroll"]')).toHaveClass("overscroll-contain"),
    Scene.expect(Scene.selector('[data-testid="builder-decklist-scroll"]')).toHaveClass("overflow-y-auto"),
    Scene.expect(Scene.selector('[data-testid="builder-decklist-scroll"]')).toHaveClass("overscroll-contain"),
    // The <dialog> element stays mounted so Dialog can open it; a closed picker paints nothing.
    Scene.expect(Scene.selector('[data-testid="builder-print-picker-backdrop"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-print-picker-scroll"]')).not.toExist(),
  );
});

test("print picker freezes catalog and decklist scroll while print grid stays scrollable", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const alternatePrint = print({ collector_number: "42", id: "sol-ring-alt-print", set: "rex", set_name: "Rex" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
    preferredPrint: { "sol-ring": solRing.default_print },
    printDialog: Dialog.init({ id: PRINT_DIALOG_ID, isOpen: true }),
    printPicker: { addOnPick: false, cardId: "sol-ring", error: false, loading: false, prints: [alternatePrint] },
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolveAll(
      [BindCardArt, ClearedBuilderHover() as never],
      [BindCardArt, ClearedBuilderHover() as never],
    ),
    Scene.expect(Scene.selector('[data-testid="builder-print-picker"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-pool-scroll"]')).toHaveClass("overflow-hidden"),
    Scene.expect(Scene.selector('[data-testid="builder-pool-scroll"]')).not.toHaveClass("overflow-y-auto"),
    Scene.expect(Scene.selector('[data-testid="builder-decklist-scroll"]')).toHaveClass("overflow-hidden"),
    Scene.expect(Scene.selector('[data-testid="builder-decklist-scroll"]')).not.toHaveClass("overflow-y-auto"),
    Scene.expect(Scene.selector('[data-testid="builder-print-picker-scroll"]')).toHaveClass("overflow-y-auto"),
    Scene.expect(Scene.selector('[data-testid="builder-print-picker-scroll"]')).toHaveClass("overscroll-contain"),
  );
});

test("clicking the dimmed page behind the print picker dismisses it and unfreezes the builder", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const alternatePrint = print({ collector_number: "42", id: "sol-ring-alt-print", set: "rex", set_name: "Rex" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
    preferredPrint: { "sol-ring": solRing.default_print },
    printDialog: Dialog.init({ id: PRINT_DIALOG_ID, isOpen: true }),
    printPicker: { addOnPick: false, cardId: "sol-ring", error: false, loading: false, prints: [alternatePrint] },
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolveAll(
      [BindCardArt, ClearedBuilderHover() as never],
      [BindCardArt, ClearedBuilderHover() as never],
    ),
    Scene.click(Scene.testId("builder-print-picker-backdrop")),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.expect(Scene.selector('[data-testid="print-tile-sol-ring-alt-print"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-pool-scroll"]')).toHaveClass("overflow-y-auto"),
    Scene.expect(Scene.selector('[data-testid="builder-decklist-scroll"]')).toHaveClass("overflow-y-auto"),
    // The print tiles' art mounts end with the picker.
    Scene.Mount.expectEnded(BindCardArt),
  );
});

test("the print picker's Close button dismisses it", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const alternatePrint = print({ collector_number: "42", id: "sol-ring-alt-print", set: "rex", set_name: "Rex" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
    preferredPrint: { "sol-ring": solRing.default_print },
    printDialog: Dialog.init({ id: PRINT_DIALOG_ID, isOpen: true }),
    printPicker: { addOnPick: false, cardId: "sol-ring", error: false, loading: false, prints: [alternatePrint] },
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolveAll(
      [BindCardArt, ClearedBuilderHover() as never],
      [BindCardArt, ClearedBuilderHover() as never],
    ),
    Scene.click(Scene.testId("close-print-picker")),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.expect(Scene.text("Choose printing")).not.toExist(),
    // The print tiles' art mounts end with the picker.
    Scene.Mount.expectEnded(BindCardArt),
  );
});

test("print selection renders a Scryfall tile picker instead of a UUID input", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const alternatePrint = print({ collector_number: "42", id: "sol-ring-alt-print", set: "rex", set_name: "Rex" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
    preferredPrint: { "sol-ring": solRing.default_print },
    printDialog: Dialog.init({ id: PRINT_DIALOG_ID, isOpen: true }),
    printPicker: { addOnPick: false, cardId: "sol-ring", error: false, loading: false, prints: [alternatePrint] },
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolveAll(
      [BindCardArt, ClearedBuilderHover() as never],
      [BindCardArt, ClearedBuilderHover() as never],
    ),
    Scene.expect(Scene.selector('[data-testid="print-sol-ring"]')).not.toExist(),
    Scene.expect(Scene.text("Choose printing")).toExist(),
    Scene.expect(Scene.selector('[data-testid="print-tile-sol-ring-alt-print"]')).toExist(),
    Scene.expect(Scene.text("REX")).toExist(),
    Scene.expect(Scene.text("#42")).toExist(),
  );
});

test("opening a pool context menu builds expected items and clears hover", () => {
  const [model] = init();
  const island = card({ id: "island", kind: { kind: "land", colors: [1] }, name: "Island" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [island], offset: 0, query: "" }))),
    Story.message(appMessage(MovedBuilderHover({ id: "island", x: 10, y: 20 }))),
    Story.message(appMessage(OpenedBuilderMenu({ cardId: "island", kind: "pool", x: 40, y: 50 }))),
    Story.model((m) => {
      expect(m.decks.builder.hover).toBeNull();
      expect(m.decks.builder.menu?.title).toBe("Island");
      expect(m.decks.builder.menu?.items.map((item) => item.label)).toEqual([
        "Add One",
        "Add Two",
        "Add Five",
        "Fill deck",
        "Choose print",
      ]);
    }),
  );
});

test("activated pool target adds a card; deck target removes one", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing], offset: 0, query: "" }))),
    Story.message(appMessage(ActivatedBuilderTarget({ cardId: "sol-ring", kind: "pool" }))),
    Story.message(appMessage(ActivatedBuilderTarget({ cardId: "sol-ring", kind: "deck" }))),
    Story.model((m) => {
      expect(m.decks.builder.entries["sol-ring"]).toBeUndefined();
    }),
  );
});

test("removing two different deck cards clears both entries", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const manaCrypt = card({ id: "mana-crypt", name: "Mana Crypt" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing, manaCrypt], offset: 0, query: "" }))),
    Story.message(appMessage(ActivatedBuilderTarget({ cardId: "sol-ring", kind: "pool" }))),
    Story.message(appMessage(ActivatedBuilderTarget({ cardId: "mana-crypt", kind: "pool" }))),
    Story.message(appMessage(ActivatedBuilderTarget({ cardId: "sol-ring", kind: "deck" }))),
    Story.message(appMessage(ActivatedBuilderTarget({ cardId: "mana-crypt", kind: "deck" }))),
    Story.model((m) => {
      expect(m.decks.builder.entries["sol-ring"]).toBeUndefined();
      expect(m.decks.builder.entries["mana-crypt"]).toBeUndefined();
    }),
  );
});

test("decklist rows are keyed by card id so pointer mounts remount after remove", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const manaCrypt = card({ id: "mana-crypt", name: "Mana Crypt" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: {
      "mana-crypt": { count: 1, print: manaCrypt.default_print },
      "sol-ring": { count: 1, print: solRing.default_print },
    },
    known: { "mana-crypt": manaCrypt, "sol-ring": solRing },
    preferredPrint: {
      "mana-crypt": manaCrypt.default_print,
      "sol-ring": solRing.default_print,
    },
  };

  // Keys force snabbdom to destroy/recreate rows; without them BindBuilderCardPointer
  // keeps the removed cardId after the first click (Mount args are mount-time only).
  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.tap((sim) => {
      for (const id of ["mana-crypt", "sol-ring"] as const) {
        const row = Scene.selector(`[data-testid="deck-row-${id}"]`)(sim.html);
        expect(Option.isSome(row)).toBe(true);
        if (Option.isNone(row)) return;
        expect(row.value.key).toBe(id);
      }
    }),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "mana-crypt", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolve(BindCardArt, ClearedBuilderHover() as never),
    Scene.Mount.resolve(BindCardArt, ClearedBuilderHover() as never),
  );
});

test("choose-print menu action opens the print picker without adding a copy", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing], offset: 0, query: "" }))),
    Story.message(appMessage(AddedBuilderCard({ card: solRing }))),
    Story.message(
      appMessage(
        RanBuilderMenuAction({
          action: { kind: "choosePrint", cardId: "sol-ring", addOnPick: false },
        }),
      ),
    ),
    Story.Command.resolve(Dialog.ShowDialog, Dialog.CompletedShowDialog()),
    Story.Command.resolve(SearchBuilderPrints, ReceivedBuilderPrints({ cardId: "sol-ring", prints: [] })),
    Story.model((m) => {
      expect(m.decks.builder.menu).toBeNull();
      expect(m.decks.builder.printPicker).toEqual({
        addOnPick: false,
        cardId: "sol-ring",
        error: false,
        loading: false,
        prints: [],
      });
      expect(m.decks.builder.entries["sol-ring"]?.count).toBe(1);
    }),
  );
});

test("pool cards do not set a native title tooltip on hover", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    pool: [solRing],
    preferredPrint: { "sol-ring": solRing.default_print },
    searching: false,
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.expect(Scene.selector('[data-testid="pool-card-sol-ring"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pool-card-sol-ring"][title]')).toBeAbsent(),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "pool" }), ClearedBuilderHover()),
    Scene.Mount.resolve(BindCardArt, ClearedBuilderHover() as never),
  );
});

test("hover preview and context menu render when present in the model", () => {
  const solRing = card({
    id: "sol-ring",
    name: "Sol Ring",
    oracle: "{1}: Untap target artifact.",
  });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    hover: { id: "sol-ring", print: solRing.default_print, x: 120, y: 80 },
    known: { "sol-ring": solRing },
    menu: {
      items: [{ label: "Add One", action: { kind: "add" as const, cardId: "sol-ring", count: 1 } }],
      title: "Sol Ring",
      x: 200,
      y: 100,
    },
    pool: [solRing],
    preferredPrint: { "sol-ring": solRing.default_print },
    searching: false,
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.expect(Scene.selector('[data-testid="builder-hover-preview"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-context-menu"]')).toExist(),
    Scene.expect(Scene.text("Add One")).toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-pool-hint"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="add-sol-ring"]')).not.toExist(),
    Scene.expect(Scene.text("Choose print")).not.toExist(),
    // Acknowledge the continuous pointer Mount without asserting its stream events.
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "pool" }), ClearedBuilderHover()),
    Scene.Mount.resolve(BindCardArt, ClearedBuilderHover() as never),
    Scene.Mount.expectEnded(BindCardArt),
  );
});

test("save command folds CreateDeck422 into problems list", async () => {
  const body: SaveDeckRequest = { cards: [], commander: "", commander_print: "", name: "New deck" };
  const failingClient = {
    ...client,
    createDeck: () => Effect.fail(new CreateDeck422({ cause: { problems: ["Too many cards"] } })),
  };

  const message = await Effect.runPromise(
    SaveDeck({ body, id: null }).effect.pipe(Effect.provideService(RpcClient, failingClient)),
  );

  expect(message).toEqual(DeckSaveFailed({ problems: ["Too many cards"] }));
});

test("Cancel on a clean builder does not open the discard confirm", () => {
  const [model] = init();

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(RequestedBuilderCancel())),
    Story.Command.resolve(NavigateHome, NavigatedAwayFromBuilder()),
    Story.model((m) => {
      expect(m.decks.builder.discardDialog.isOpen).toBe(false);
    }),
  );
});

test("Cancel on a dirty builder opens the discard confirm", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing], offset: 0, query: "" }))),
    Story.message(appMessage(AddedBuilderCard({ card: solRing }))),
    Story.message(appMessage(RequestedBuilderCancel())),
    Story.Command.resolve(Dialog.ShowDialog, Dialog.CompletedShowDialog()),
    Story.model((m) => {
      expect(m.decks.builder.dirty).toBe(true);
      expect(m.decks.builder.discardDialog.isOpen).toBe(true);
    }),
  );
});

test("dismissing the discard confirm keeps the edits and stays in the builder", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing], offset: 0, query: "" }))),
    Story.message(appMessage(AddedBuilderCard({ card: solRing }))),
    Story.message(appMessage(RequestedBuilderCancel())),
    Story.Command.resolve(Dialog.ShowDialog, Dialog.CompletedShowDialog()),
    Story.message(appMessage(GotDiscardDialogMessage({ message: Dialog.RequestedClose() }))),
    Story.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Story.model((m) => {
      expect(m.decks.builder.discardDialog.isOpen).toBe(false);
      expect(m.decks.builder.dirty).toBe(true);
    }),
  );
});

test("ConfirmedBuilderDiscard is handled without throwing", () => {
  const [model] = init();

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ConfirmedBuilderDiscard())),
    Story.Command.resolve(NavigateHome, NavigatedAwayFromBuilder()),
    Story.model((m) => {
      expect(m.decks.builder.discardDialog.isOpen).toBe(false);
    }),
  );
});

test("Cancel button renders in builder view", () => {
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    searching: false,
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.expect(Scene.selector('[data-testid="builder-cancel"]')).toExist(),
  );
});

/** A dirty builder with its discard prompt already up — what `RequestedBuilderCancel` leaves behind. */
const askingToDiscard = {
  ...initialDeckBuilderSubmodel(),
  atEnd: true,
  dirty: true,
  discardDialog: Dialog.init({ id: DISCARD_DIALOG_ID, isOpen: true }),
  searching: false,
};

const builderProgram = {
  update: builderUpdate,
  view: (model: typeof askingToDiscard) => builderView(model, emptyViewInputs),
};

test("backing out of the discard prompt keeps the edits", () => {
  Scene.scene(
    builderProgram,
    Scene.with(askingToDiscard),
    Scene.click(Scene.selector('[data-testid="confirm-cancel"]')),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.expect(Scene.selector('[data-testid="confirm-title"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toExist(),
  );
});

test("confirming the discard prompt leaves the builder", () => {
  Scene.scene(
    builderProgram,
    Scene.with(askingToDiscard),
    Scene.click(Scene.selector('[data-testid="confirm-ok"]')),
    Scene.Command.resolve(Dialog.CloseDialog, Dialog.CompletedCloseDialog()),
    Scene.Command.resolve(NavigateHome, NavigatedAwayFromBuilder()),
    Scene.expect(Scene.selector('[data-testid="confirm-title"]')).not.toExist(),
  );
});

test("the discard prompt asks before throwing edits away", () => {
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    discardDialog: Dialog.init({ id: DISCARD_DIALOG_ID, isOpen: true }),
    searching: false,
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.expect(Scene.selector('[data-testid="builder-discard-confirm"]')).toExist(),
    Scene.expect(Scene.text("Discard changes?")).toExist(),
  );
});
