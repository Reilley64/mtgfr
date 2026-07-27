import { Effect, Option } from "effect";
import { Story } from "foldkit";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import type { ScryfallPrint } from "../../../domain/deck-builder/scryfall";
import { client } from "../../../domain/rpc-client";
import { BindCardArt } from "../../../domain/ui/card-art";
import { OpenDialogAsModal } from "../../../domain/ui/confirmDialog";
import { type CatalogCard, CreateDeck422, type SaveDeckRequest } from "../../../domain/wire/types";
import { update as appUpdate, init } from "../../../main-exports";
import { GotDeckBuilderMessage, UrlChanged } from "../../../messages";
import { RpcClient } from "../../../resources";
import { DeckRoute } from "../../../routes";
import {
  ActivatedBuilderTarget,
  AddedBuilderCard,
  type Message as BuilderMessage,
  CancelledBuilderDiscard,
  ChangedBuilderName,
  ChangedBuilderProxyArtUrl,
  ClearedBuilderHover,
  ClearedBuilderProxyArt,
  ClosedBuilderProxyArtPicker,
  ConfirmedBuilderDiscard,
  DeckSaveFailed,
  MovedBuilderHover,
  NavigatedAwayFromBuilder,
  OpenedBuilderMenu,
  OpenedBuilderPrintPicker,
  OpenedBuilderProxyArtPicker,
  PickedBuilderPrint,
  RanBuilderMenuAction,
  ReceivedBuilderPrints,
  ReceivedBuilderSearchPage,
  ReceivedDeckForBuilder,
  RemovedBuilderCard,
  RequestedBuilderCancel,
  SetBuilderCommander,
  SubmittedBuilderProxyArt,
  SubmittedDeckSave,
} from "./messages";
import { initialDeckBuilderSubmodel } from "./submodel";
import { update as builderUpdate, NavigateHome, SaveDeck, SearchBuilderPrints } from "./update";
import { BindBuilderCardPointer, view as builderView } from "./view";

const emptyChrome = { version: null, faithfulCount: null, oracleTotal: null, coverageHref: null };
const me = { id: 1, email: "alice@example.com", username: "alice" };
const emptyViewInputs = {
  chrome: emptyChrome,
  username: me.username,
  meGravatarHash: null,
  accountMenuOpen: false,
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
          confirmingDiscard: true,
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
          proxyArtPicker: null,
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
    Story.Command.resolve(
      SearchBuilderPrints,
      ReceivedBuilderPrints({ cardId: "commander", prints: [alternatePrint] }),
    ),
    Story.message(appMessage(PickedBuilderPrint({ cardId: "commander", print: alternatePrint.id }))),
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
    Story.Command.resolve(SearchBuilderPrints, ReceivedBuilderPrints({ cardId: "sol-ring", prints: [alternatePrint] })),
    Story.message(appMessage(PickedBuilderPrint({ cardId: "sol-ring", print: alternatePrint.id }))),
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
    Scene.expect(Scene.selector('[data-testid="builder-print-picker"]')).not.toExist(),
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
    printPicker: { addOnPick: false, cardId: "sol-ring", error: false, loading: false, prints: [alternatePrint] },
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolve(OpenDialogAsModal(), ClearedBuilderHover()),
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

test("print selection renders a Scryfall tile picker instead of a UUID input", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const alternatePrint = print({ collector_number: "42", id: "sol-ring-alt-print", set: "rex", set_name: "Rex" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
    preferredPrint: { "sol-ring": solRing.default_print },
    printPicker: { addOnPick: false, cardId: "sol-ring", error: false, loading: false, prints: [alternatePrint] },
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolve(OpenDialogAsModal(), ClearedBuilderHover()),
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
    Story.message(appMessage(MovedBuilderHover({ id: "island", kind: "pool", x: 10, y: 20 }))),
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
  const html = builderView(model, emptyViewInputs);
  expect(html).not.toBeNull();
  if (html == null) return;
  for (const id of ["mana-crypt", "sol-ring"] as const) {
    const row = Scene.selector(`[data-testid="deck-row-${id}"]`)(html);
    expect(Option.isSome(row)).toBe(true);
    if (Option.isNone(row)) return;
    expect(row.value.key).toBe(id);
  }
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

test("commander proxy art save does not write into the deck row when both share the same card id", () => {
  const [model] = init();
  const commander = card({
    id: "atraxa",
    kind: { kind: "creature", power: 4, toughness: 4 },
    legendary: true,
    name: "Atraxa, Praetors' Voice",
  });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [commander], offset: 0, query: "" }))),
    Story.message(appMessage(AddedBuilderCard({ card: commander }))),
    Story.message(appMessage(SetBuilderCommander({ card: commander }))),
    Story.message(
      appMessage(
        RanBuilderMenuAction({
          action: { kind: "setProxyArt", cardId: "atraxa", target: "commander" },
        }),
      ),
    ),
    Story.message(appMessage(ChangedBuilderProxyArtUrl({ url: "https://example.com/commander.png" }))),
    Story.message(appMessage(SubmittedBuilderProxyArt())),
    Story.model((m) => {
      expect(m.decks.builder.commander.proxyArtUrl).toBe("https://example.com/commander.png");
      expect(m.decks.builder.entries.atraxa?.proxyArtUrl).toBeUndefined();
      expect(m.decks.builder.proxyArtPicker).toBeNull();
    }),
  );
});

test("set-proxy-art menu action opens the dialog, locks scroll, and saves proxy art for a deck row", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
    menu: {
      items: [{ label: "Set proxy art…", action: { kind: "setProxyArt" as const, cardId: "sol-ring", target: "entry" } }],
      title: "Sol Ring",
      x: 40,
      y: 50,
    },
    preferredPrint: { "sol-ring": solRing.default_print },
    searching: false,
  };

  Scene.scene(
    { update: builderUpdate, view: (nextModel) => builderView(nextModel, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolve(BindCardArt, ClearedBuilderHover() as never),
    Scene.click(Scene.selector('[data-testid="builder-menu-item-0"]')),
    Scene.Mount.resolve(OpenDialogAsModal(), ClearedBuilderHover()),
    Scene.expect(Scene.selector('[data-testid="builder-proxy-art-picker"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-pool-scroll"]')).toHaveClass("overflow-hidden"),
    Scene.expect(Scene.selector('[data-testid="builder-decklist-scroll"]')).toHaveClass("overflow-hidden"),
    Scene.type(Scene.selector('[data-testid="builder-proxy-art-url"]'), "https://example.com/a.png"),
    Scene.expect(Scene.selector('[data-testid="builder-proxy-art-save"]')).toBeEnabled(),
    Scene.click(Scene.selector('[data-testid="builder-proxy-art-save"]')),
    Scene.Mount.expectEnded(OpenDialogAsModal()),
    Scene.expect(Scene.selector('[data-testid="builder-proxy-art-picker"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-deck-proxy-chip-sol-ring"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-row-sol-ring"] [data-art-url^="/api/card-art/proxy?url="]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-pool-scroll"]')).toHaveClass("overflow-y-auto"),
    Scene.expect(Scene.selector('[data-testid="builder-decklist-scroll"]')).toHaveClass("overflow-y-auto"),
  );
});

test("proxy art dialog validates the url and clear removes the existing override", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    entries: {
      "sol-ring": {
        count: 1,
        print: solRing.default_print,
        proxyArtUrl: "https://example.com/old.png",
      },
    },
    known: { "sol-ring": solRing },
    menu: {
      items: [{ label: "Set proxy art…", action: { kind: "setProxyArt" as const, cardId: "sol-ring", target: "entry" } }],
      title: "Sol Ring",
      x: 40,
      y: 50,
    },
    preferredPrint: { "sol-ring": solRing.default_print },
    searching: false,
  };

  Scene.scene(
    { update: builderUpdate, view: (nextModel) => builderView(nextModel, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "deck" }), ClearedBuilderHover()),
    Scene.Mount.resolve(BindCardArt, ClearedBuilderHover() as never),
    Scene.click(Scene.selector('[data-testid="builder-menu-item-0"]')),
    Scene.Mount.resolve(OpenDialogAsModal(), ClearedBuilderHover()),
    Scene.expect(Scene.selector('[data-testid="builder-proxy-art-url"]')).toHaveAttr("value", "https://example.com/old.png"),
    Scene.type(Scene.selector('[data-testid="builder-proxy-art-url"]'), "http://example.com/a.png"),
    Scene.expect(Scene.selector('[data-testid="builder-proxy-art-error"]')).toContainText("https"),
    Scene.expect(Scene.selector('[data-testid="builder-proxy-art-save"]')).toBeDisabled(),
    Scene.click(Scene.selector('[data-testid="builder-proxy-art-clear"]')),
    Scene.Mount.expectEnded(OpenDialogAsModal()),
    Scene.expect(Scene.selector('[data-testid="builder-proxy-art-picker"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="builder-deck-proxy-chip-sol-ring"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-row-sol-ring"] [data-art-url^="/api/card-art/proxy?url="]')).not.toExist(),
  );
});

test("deck load and save round-trip proxy art urls for rows and commander", () => {
  const [loaded] = builderUpdate(
    initialDeckBuilderSubmodel("7"),
    ReceivedDeckForBuilder({
      deck: {
        cards: [
          {
            count: 1,
            id: "sol-ring",
            print: "sol-ring-print",
            proxy_art_url: "https://example.com/sol-ring.png",
          },
        ],
        commander: "atraxa",
        commander_print: "atraxa-print",
        commander_proxy_art_url: "https://example.com/atraxa.png",
        id: 7,
        name: "Proxy Friends",
      },
    }),
  );

  expect(loaded.entries["sol-ring"]?.proxyArtUrl).toBe("https://example.com/sol-ring.png");
  expect(loaded.commander.proxyArtUrl).toBe("https://example.com/atraxa.png");

  const [saving, commands] = builderUpdate(loaded, SubmittedDeckSave());
  expect(saving.saving).toBe(true);
  expect(commands).toMatchObject([
    {
      name: "SaveDeck",
      args: {
        id: "7",
        body: {
          cards: [
            {
              count: 1,
              id: "sol-ring",
              print: "sol-ring-print",
              proxy_art_url: "https://example.com/sol-ring.png",
            },
          ],
          commander: "atraxa",
          commander_print: "atraxa-print",
          commander_proxy_art_url: "https://example.com/atraxa.png",
          name: "Proxy Friends",
        },
      },
    },
  ]);
});

test("proxy art messages validate, save, clear, and close the dialog state", () => {
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });
  const model = {
    ...initialDeckBuilderSubmodel(),
    entries: { "sol-ring": { count: 1, print: solRing.default_print } },
    known: { "sol-ring": solRing },
  };

  const [opened] = builderUpdate(model, OpenedBuilderProxyArtPicker({ cardId: "sol-ring", target: "entry" }));
  expect(opened.proxyArtPicker).toEqual({ cardId: "sol-ring", error: null, target: "entry", url: "" });

  const [invalid] = builderUpdate(opened, ChangedBuilderProxyArtUrl({ url: "http://example.com/a.png" }));
  expect(invalid.proxyArtPicker?.error).toContain("https");

  const [valid] = builderUpdate(opened, ChangedBuilderProxyArtUrl({ url: "https://example.com/a.png" }));
  expect(valid.proxyArtPicker?.error).toBeNull();

  const [saved] = builderUpdate(valid, SubmittedBuilderProxyArt());
  expect(saved.entries["sol-ring"]?.proxyArtUrl).toBe("https://example.com/a.png");
  expect(saved.proxyArtPicker).toBeNull();

  const [reopened] = builderUpdate(saved, OpenedBuilderProxyArtPicker({ cardId: "sol-ring", target: "entry" }));
  const [cleared] = builderUpdate(reopened, ClearedBuilderProxyArt());
  expect(cleared.entries["sol-ring"]?.proxyArtUrl).toBeUndefined();
  expect(cleared.proxyArtPicker).toBeNull();

  const [closed] = builderUpdate(opened, ClosedBuilderProxyArtPicker());
  expect(closed.proxyArtPicker).toBeNull();
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
      expect(m.decks.builder.confirmingDiscard).toBe(false);
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
    Story.model((m) => {
      expect(m.decks.builder.dirty).toBe(true);
      expect(m.decks.builder.confirmingDiscard).toBe(true);
    }),
  );
});

test("CancelledBuilderDiscard closes the discard confirm without navigating", () => {
  const [model] = init();
  const solRing = card({ id: "sol-ring", name: "Sol Ring" });

  Story.story(
    appUpdate,
    Story.with(model),
    Story.message(appMessage(ReceivedBuilderSearchPage({ cards: [solRing], offset: 0, query: "" }))),
    Story.message(appMessage(AddedBuilderCard({ card: solRing }))),
    Story.message(appMessage(RequestedBuilderCancel())),
    Story.message(appMessage(CancelledBuilderDiscard())),
    Story.model((m) => {
      expect(m.decks.builder.confirmingDiscard).toBe(false);
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
      expect(m.decks.builder.confirmingDiscard).toBe(false);
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

test("discard confirm dialog renders when confirmingDiscard is true", () => {
  const model = {
    ...initialDeckBuilderSubmodel(),
    atEnd: true,
    confirmingDiscard: true,
    searching: false,
  };

  Scene.scene(
    { update: builderUpdate, view: (model) => builderView(model, emptyViewInputs) },
    Scene.with(model),
    Scene.Mount.resolve(OpenDialogAsModal(), ClearedBuilderHover()),
    Scene.expect(Scene.selector('[data-testid="builder-discard-confirm"]')).toExist(),
    Scene.expect(Scene.text("Discard changes?")).toExist(),
  );
});
