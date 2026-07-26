/**
 * Shell surface coverage — every auth / decks / lobby / 404 panel must appear here
 * (or in a focused sibling Scene test) with a data-testid or unique-copy assertion.
 * See AGENTS.md: "Client UI: every surface gets a Scene test."
 */
import { Scene } from "foldkit/test";
import { describe, it } from "vitest";
import { BindCardArt, CardArtTick } from "../../lib/ui/card-art";
import { ModalOpened, OpenDialogAsModal } from "../../lib/ui/confirmDialog";
import type { CatalogCard } from "../../lib/wire/types";
import { BindDeckCardFlip, DeckCardFlipTick } from "../deck-card-nav";
import { init, update } from "../main-exports";
import type { Model as AppModel } from "../model";
import {
  HomeRoute,
  LeaderboardRoute,
  LoginRoute,
  NewDeckRoute,
  NotFoundRoute,
  PlayRoute,
  routePath,
  TableRoute,
} from "../routes";
import { view } from "../view";
import { BindAccountMenuEscape } from "./account-chrome/escape";
import { ClosedAccountMenu } from "./account-chrome/messages";
import { ClearedBuilderHover } from "./decks/builder/messages";
import { initialDeckBuilderSubmodel } from "./decks/builder/submodel";
import { BindBuilderCardPointer } from "./decks/builder/view";
import { ClosedDeckListMenu } from "./decks/list/messages";
import { BindDeckListContextMenu, BindDeckListContextMenuEscape } from "./decks/list/view";
import { initialLobbySlice } from "./lobby/submodel";

const me = { id: 1, email: "alice@example.com", username: "alice" };

const atraxa = card({
  color_identity: [2, 4, 5],
  cost: { colored: [0, 0, 1, 1, 1], generic: 4 },
  default_print: "atraxa-print",
  id: "atraxa",
  kind: { kind: "creature", power: 4, toughness: 4 },
  legendary: true,
  name: "Atraxa, Praetors' Voice",
  oracle: "Flying, vigilance, deathtouch, lifelink",
  set: "c16",
  subtypes: ["Angel", "Horror"],
});

const solRing = card({
  default_print: "sol-ring-print",
  id: "sol-ring",
  name: "Sol Ring",
});

const deck = {
  commander: "atraxa",
  commander_print: "atraxa-print",
  id: 1,
  name: "Superfriends",
};

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
    summary: [],
    ...overrides,
  };
}

function loginModel(overrides: Partial<AppModel> = {}): AppModel {
  const [model] = init();
  return {
    ...model,
    route: LoginRoute(),
    portraitGate: { open: false },
    sessionLoaded: true,
    session: { me: null, meGravatarHash: null },
    ...overrides,
  };
}

function authedModel(route: AppModel["route"], overrides: Partial<AppModel> = {}): AppModel {
  const [model] = init();
  return {
    ...model,
    route,
    portraitGate: { open: false },
    sessionLoaded: true,
    session: { me, meGravatarHash: "ff8d9819fc0e12bf0d24892e45987e249a28dce836a85cad60e28eaaa8c6d976" },
    ...overrides,
  };
}

describe("shell surface scenes", () => {
  it("renders auth login surfaces from the app view", () => {
    Scene.scene(
      { update, view },
      Scene.with(loginModel({ apiVersion: "1.2.3" })),
      Scene.expect(Scene.selector('[data-testid="auth-panel"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-form"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-email"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-password"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-submit"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="app-version"]')).toExist(),
      Scene.expect(Scene.text("API 1.2.3")).toExist(),
      Scene.expect(Scene.text("edh.reilley.dev")).toExist(),
      Scene.expect(Scene.text("mtgfr")).not.toExist(),
    );
  });

  it("renders auth signup surfaces and auth errors", () => {
    const [model] = init();

    Scene.scene(
      { update, view },
      Scene.with(
        loginModel({
          auth: {
            ...model.auth,
            error: "Email already in use.",
            mode: "signup",
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="auth-username"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-toggle-mode"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-error"]')).toExist(),
      Scene.expect(Scene.text("Email already in use.")).toExist(),
    );
  });

  it("renders deck list chrome and tiles", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              decks: [deck],
              knownCommanders: { atraxa },
              loading: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="decks-page"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="header-leaderboard-link"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="account-menu-trigger"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="seat-face-0"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-list-search"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="delete-deck-1"]')).not.toExist(),
      Scene.expect(Scene.selector('[data-testid="account-gravatar-link"]')).not.toExist(),
      Scene.expect(Scene.text("Sign out")).not.toExist(),
      Scene.expect(Scene.text("Your decks")).toExist(),
      Scene.expect(Scene.text("Superfriends")).toExist(),
      Scene.expect(Scene.selector(`[data-testid="deck-list-new-deck"][href="${routePath(NewDeckRoute())}"]`)).toExist(),
      Scene.expect(Scene.text("New deck")).toExist(),
      Scene.expect(
        Scene.selector(`[data-testid="deck-list-header"] a[href="${routePath(NewDeckRoute())}"]`),
      ).not.toExist(),
      Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("opens the account menu from the home avatar", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              decks: [deck],
              knownCommanders: { atraxa },
              loading: false,
              accountMenuOpen: true,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="account-menu"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="account-menu-username"]')).toExist(),
      Scene.expect(Scene.text("alice")).toExist(),
      Scene.expect(Scene.selector('[data-testid="account-gravatar-link"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="account-menu-sign-out"]')).toExist(),
      Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindAccountMenuEscape(), ClosedAccountMenu()),
      Scene.Mount.expectEnded(BindAccountMenuEscape),
    );
  });

  it("renders the deck delete confirmation dialog", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              confirmingDeleteId: 1,
              decks: [deck],
              knownCommanders: { atraxa },
              loading: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="confirm-delete-dialog"]')).toExist(),
      Scene.expect(Scene.text('Delete "Superfriends"?')).toExist(),
      Scene.Mount.resolve(OpenDialogAsModal(), ModalOpened()),
      Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("shows a New deck create tile when the list is empty", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              decks: [],
              loading: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.text("No decks yet — build one to get started.")).not.toExist(),
      Scene.expect(Scene.selector(`[data-testid="deck-list-new-deck"][href="${routePath(NewDeckRoute())}"]`)).toExist(),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("renders deck list loading copy", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              loading: true,
            },
          },
        }),
      ),
      Scene.expect(Scene.text("Loading decks…")).toExist(),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("renders leaderboard rows with usernames and ratings", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(LeaderboardRoute(), {
          leaderboard: {
            entries: [
              { rank: 1, rating: 1200, user_id: 1, username: "alice" },
              { rank: 2, rating: 1175, user_id: 2, username: "bruno" },
            ],
            accountMenuOpen: false,
            error: null,
            status: "ready",
            total: 2,
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="leaderboard-page"]')).toExist(),
      Scene.expectAll(Scene.all.selector('[data-testid="leaderboard-row"]')).toHaveCount(2),
      Scene.expect(Scene.text("#1")).toExist(),
      Scene.expect(Scene.text("alice")).toExist(),
      Scene.expect(Scene.text("1200")).toExist(),
      Scene.expect(Scene.text("#2")).toExist(),
      Scene.expect(Scene.text("bruno")).toExist(),
      Scene.expect(Scene.text("1175")).toExist(),
      Scene.expect(Scene.text("Play")).toExist(),
      Scene.expect(Scene.selector('[data-testid="account-menu-trigger"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="header-leaderboard-link"]')).not.toExist(),
      Scene.expect(Scene.text("Signed in as alice")).not.toExist(),
    );
  });

  it("opens the account menu on the leaderboard", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(LeaderboardRoute(), {
          leaderboard: {
            entries: [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }],
            accountMenuOpen: true,
            error: null,
            status: "ready",
            total: 1,
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="account-menu"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="account-menu-sign-out"]')).toExist(),
      Scene.Mount.resolve(BindAccountMenuEscape(), ClosedAccountMenu()),
      Scene.Mount.expectEnded(BindAccountMenuEscape),
    );
  });

  it("hides load more while the leaderboard shows a retry error", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(LeaderboardRoute(), {
          leaderboard: {
            entries: [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }],
            accountMenuOpen: false,
            error: "Could not load the leaderboard.",
            status: "error",
            total: 2,
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="leaderboard-page"]')).toExist(),
      Scene.expectAll(Scene.all.selector('[data-testid="leaderboard-row"]')).toHaveCount(1),
      Scene.expect(Scene.text("#1")).toExist(),
      Scene.expect(Scene.text("Could not load the leaderboard.")).toExist(),
      Scene.expect(Scene.text("Try again")).toExist(),
      Scene.expect(Scene.text("Load more")).not.toExist(),
    );
  });

  it("renders deck builder chrome, problems, and builder mounts", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(NewDeckRoute(), {
          decks: {
            ...init()[0].decks,
            builder: {
              ...initialDeckBuilderSubmodel(),
              atEnd: true,
              confirmingDiscard: true,
              known: { "sol-ring": solRing },
              pool: [solRing],
              preferredPrint: { "sol-ring": "sol-ring-print" },
              problems: ["Choose a commander first."],
              searching: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-name"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="save-deck"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="builder-cancel"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="builder-pool-hint"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-problems"]')).toExist(),
      Scene.expect(Scene.text("Choose a commander first.")).toExist(),
      Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "pool" }), ClearedBuilderHover()),
      Scene.Mount.resolve(OpenDialogAsModal(), ModalOpened()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
    );
  });

  it("renders lobby entry choose destinations with decks", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(PlayRoute({ deckId: "1" }), {
          decks: {
            ...init()[0].decks,
            list: { ...init()[0].decks.list, decks: [deck], knownCommanders: { atraxa }, loading: false },
          },
          lobby: { ...initialLobbySlice(), selectedDeckId: 1 },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="lobby-entry-choose"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-host"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-open-join"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-deck-card"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-deck-card-1"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-join-code"]')).toBeAbsent(),
      Scene.expect(Scene.text("Lobby")).toExist(),
      Scene.expect(Scene.text("edh.reilley.dev")).toExist(),
      Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
    );
  });

  it("keeps a play deck route in the lobby while the deck list error is visible", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(PlayRoute({ deckId: "1" }), {
          currentPath: "/play/1",
          decks: {
            ...init()[0].decks,
            list: { ...init()[0].decks.list, decks: [], error: "Could not load decks.", loading: false },
          },
          lobby: { ...initialLobbySlice(), selectedDeckId: 1 },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).toExist(),
      Scene.expect(Scene.text("Deck not found.")).toExist(),
      Scene.expect(Scene.text("Not found")).not.toExist(),
      Scene.expect(Scene.text("No Foldkit route for /play/1.")).not.toExist(),
    );
  });

  it("renders lobby table chrome, seats, and errors", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(TableRoute({ deckId: "1", table: "ABC123" }), {
          decks: {
            ...init()[0].decks,
            list: { ...init()[0].decks.list, decks: [deck], knownCommanders: { atraxa }, loading: false },
          },
          lobby: {
            ...initialLobbySlice(),
            error: "UnknownTable",
            selectedDeckId: 1,
            tableId: "ABC123",
            view: {
              error: null,
              seats: [
                {
                  claimed: true,
                  deck_id: 1,
                  deck_name: "Superfriends",
                  gravatar_hash: "ff8d9819fc0e12bf0d24892e45987e249a28dce836a85cad60e28eaaa8c6d976",
                  is_host: true,
                  is_you: true,
                  player: 0,
                  ready: false,
                  username: "alice",
                },
              ],
              start_error: "NeedTwoPlayers",
              started: false,
              table_id: "ABC123",
              you: 0,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="lobby-table-code"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-copy-code"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-seats"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-seat-0"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="seat-face-0"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-start-error"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-start-error"].text-caution-amber')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-error"]')).toExist(),
      Scene.expect(Scene.text("Need at least two players.")).toExist(),
      Scene.expect(Scene.text("That table link is stale or expired. Ask the host for a new code.")).toExist(),
    );
  });

  it("renders the app not-found route", () => {
    Scene.scene(
      { update, view },
      Scene.with(authedModel(NotFoundRoute({ path: "/missing" }))),
      Scene.expect(Scene.text("Not found")).toExist(),
      Scene.expect(Scene.text("No Foldkit route for /missing.")).toExist(),
    );
  });

  // The board-mount placeholder is unreachable through routeBody today:
  // PlayRoute/TableRoute only call boardMount when model.game?.active === true,
  // and boardMount immediately renders the board submodel whenever model.game exists.
});
