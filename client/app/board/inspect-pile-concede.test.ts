// TDD tests for new board model state: inspect pin, pile overlay, concede, result, keyboard shortcuts.

import { Story } from "foldkit";
import { expect, test } from "vitest";
import type { VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import type { Message } from "./messages";
import {
  AltDown,
  AltUp,
  ConcedeCancelled,
  ConcedeClicked,
  ConcedeConfirmed,
  InspectCardFetched,
  InspectDismissed,
  InspectFlipFace,
  KeyboardEscape,
  KeyboardSpacePressed,
  PileExpanded,
  PileOverlayClosed,
  RadialWedgeArmed,
  ResultSeen,
} from "./messages";
import { type BoardModel, initialBoardModel, updateBoard } from "./submodel";

function twoPlayerState(): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 0,
        username: "Alice",
      },
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 1,
        username: "Bob",
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

function gameFold(overrides: Partial<VisibleState> = {}): GameFoldState {
  return {
    seq: 1,
    state: { ...twoPlayerState(), ...overrides },
    log: [],
    reject: null,
    provenance: {
      zoneMoves: new Map(),
      resolvedFromStack: new Set(),
      leftStackToPile: new Set(),
      tokenCreators: new Map(),
      landPlayFrom: new Map(),
      zonePileEntrances: new Map(),
      stackEntrances: new Map(),
      priorStackObjectIds: new Set(),
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false },
  };
}

function update(model: BoardModel, message: Message): BoardModel {
  const [next] = updateBoard(model, message, gameFold(), "table-1");
  return next;
}

// ── AltDown / AltUp ────────────────────────────────────────────────────────────

test("AltDown sets altDown flag", () => {
  const model = update(initialBoardModel(), AltDown());
  expect(model.altDown).toBe(true);
});

test("AltUp clears altDown flag", () => {
  const model = update({ ...initialBoardModel(), altDown: true }, AltUp());
  expect(model.altDown).toBe(false);
});

// ── Inspect ────────────────────────────────────────────────────────────────────

test("InspectCardFetched stores catalog card", () => {
  const card = {
    id: "card-1",
    name: "Test",
    oracle: "Do stuff.",
    approximates: null,
    back: null,
    color_identity: [],
    cost: { generic: 0, colored: [0, 0, 0, 0, 0], has_x: false, x_symbols: 0 },
    default_print: "print-1",
    keywords: [],
    kind: { kind: "instant" as const },
    legendary: false,
    otags: [],
    set: "soc",
    subtypes: [],
    summary: "Do stuff.",
  } as unknown as import("~/wire/types").CatalogCard;
  const model = update(
    { ...initialBoardModel(), inspectPin: { name: "Test", prepared: false } },
    InspectCardFetched({ card }),
  );
  expect(model.inspectCard).toEqual(card);
});

test("InspectCardFetched with null clears pending state", () => {
  const model = update(
    { ...initialBoardModel(), inspectPin: { name: "Fog", prepared: false } },
    InspectCardFetched({ card: null }),
  );
  expect(model.inspectCard).toBeNull();
});

test("InspectFlipFace toggles from front to back", () => {
  const model = update({ ...initialBoardModel(), inspectFace: "front" }, InspectFlipFace());
  expect(model.inspectFace).toBe("back");
});

test("InspectFlipFace toggles from back to front", () => {
  const model = update({ ...initialBoardModel(), inspectFace: "back" }, InspectFlipFace());
  expect(model.inspectFace).toBe("front");
});

test("InspectDismissed clears pin, card, and altDown", () => {
  const start: BoardModel = {
    ...initialBoardModel(),
    altDown: true,
    inspectPin: { name: "Sol Ring", prepared: false },
    inspectCard: null,
  };
  const model = update(start, InspectDismissed());
  expect(model.inspectPin).toBeNull();
  expect(model.inspectCard).toBeUndefined();
  expect(model.altDown).toBe(false);
});

// ── Pile overlay ───────────────────────────────────────────────────────────────

test("PileExpanded stores zone + owner", () => {
  const model = update(initialBoardModel(), PileExpanded({ zone: 4, owner: 1 }));
  expect(model.pileExpand).toEqual({ zone: 4, owner: 1 });
});

test("PileOverlayClosed clears pileExpand", () => {
  const model = update({ ...initialBoardModel(), pileExpand: { zone: 4, owner: 1 } }, PileOverlayClosed());
  expect(model.pileExpand).toBeNull();
});

// ── Concede ─────────────────────────────────────────────────────────────────────

test("ConcedeClicked sets confirmConcede", () => {
  const model = update(initialBoardModel(), ConcedeClicked());
  expect(model.confirmConcede).toBe(true);
});

test("ConcedeCancelled clears confirmConcede", () => {
  const model = update({ ...initialBoardModel(), confirmConcede: true }, ConcedeCancelled());
  expect(model.confirmConcede).toBe(false);
});

test("ConcedeConfirmed clears confirmConcede and submits intent", () => {
  const [resultModel, cmds] = updateBoard(
    { ...initialBoardModel(), confirmConcede: true },
    ConcedeConfirmed(),
    gameFold(),
    "table-1",
  );
  expect(resultModel.confirmConcede).toBe(false);
  expect(cmds.length).toBeGreaterThan(0);
});

// ── Game result ────────────────────────────────────────────────────────────────

test("ResultSeen sets resultSeen flag", () => {
  const model = update(initialBoardModel(), ResultSeen());
  expect(model.resultSeen).toBe(true);
});

// ── Keyboard escape ────────────────────────────────────────────────────────────

test("KeyboardEscape dismisses inspect when inspect is open", () => {
  const model = update(
    { ...initialBoardModel(), inspectPin: { name: "Sol Ring", prepared: false }, altDown: true },
    KeyboardEscape(),
  );
  expect(model.inspectPin).toBeNull();
  expect(model.altDown).toBe(false);
});

test("KeyboardEscape dismisses radial when radial is selected (no inspect)", () => {
  const model = update({ ...initialBoardModel(), selectedId: 42 }, KeyboardEscape());
  expect(model.selectedId).toBeNull();
});

test("KeyboardEscape clears action + pile when nothing else is open", () => {
  const start: BoardModel = {
    ...initialBoardModel(),
    pileExpand: { zone: 4, owner: 0 },
    reject: "Nope",
  };
  const model = update(start, KeyboardEscape());
  expect(model.pileExpand).toBeNull();
  expect(model.reject).toBeNull();
});

// ── Keyboard space ─────────────────────────────────────────────────────────────

test("KeyboardSpacePressed submits pass_priority intent", () => {
  const [, cmds] = updateBoard(initialBoardModel(), KeyboardSpacePressed(), gameFold(), "table-1");
  expect(cmds.length).toBeGreaterThan(0);
});

// ── Radial not disrupted by new state ──────────────────────────────────────────

test("RadialWedgeArmed still sets radial press with new state", () => {
  Story.story(
    (model: BoardModel, message: Message) => updateBoard(model, message, gameFold(), null),
    Story.with(initialBoardModel()),
    Story.message(RadialWedgeArmed({ index: 2 })),
    Story.model((model) => {
      expect(model.radialPress.armed).toBe(2);
    }),
  );
});
