// TDD tests for new board model state: inspect pin, pile overlay, concede, result, keyboard shortcuts.

import { Story } from "foldkit";
import { expect, test } from "vitest";
import type { ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { SubmitIntent } from "../game/intents";
import { worldToScreen } from "./geometry/camera";
import { avatarPos, layout } from "./geometry/layout";
import type { Message } from "./messages";
import {
  AltDown,
  AltUp,
  BoardPointerMove,
  ConcedeCancelled,
  ConcedeClicked,
  ConcedeConfirmed,
  InspectAuxHovered,
  InspectCardFetched,
  InspectDismissed,
  InspectFlipFace,
  KeepHandClicked,
  KeyboardEscape,
  KeyboardSpacePressed,
  MulliganClicked,
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

function screenCenterForCard(fold: GameFoldState, id: number) {
  const state = fold.state;
  if (state == null) throw new Error("expected game state");
  const card = layout(state, state.viewer).find((c) => c.id === id);
  if (card == null) throw new Error(`expected card ${id}`);
  return worldToScreen(initialBoardModel().camera, card.x + card.w / 2, card.y + card.h / 2);
}

// ── AltDown / AltUp (hold Alt over a card to pin; release clears) ─

function battlefieldCreature(id: number, name: string, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
    marked_damage: 0,
    name,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "print-1",
    card_id: "card-1",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: 2,
    ...overrides,
  };
}

test("AltDown sets altDown flag", () => {
  const model = update(initialBoardModel(), AltDown());
  expect(model.altDown).toBe(true);
});

test("AltDown pins the face-up card under the cursor (no click)", () => {
  const creature = battlefieldCreature(7, "Sol Ring");
  const fold = gameFold({ objects: [creature] });
  const screen = screenCenterForCard(fold, 7);

  let model = initialBoardModel();
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");
  const [pinned, cmds] = updateBoard(model, AltDown(), fold, "table-1");

  expect(pinned.altDown).toBe(true);
  expect(pinned.inspectPin).toEqual(
    expect.objectContaining({ name: "Sol Ring", objectId: 7, cardId: "card-1", print: "print-1" }),
  );
  expect((cmds[0] as { name?: string } | undefined)?.name).toBe("FetchInspectCard");
});

test("AltDown over a life orb pins that player for commander-damage inspect", () => {
  const fold = gameFold({
    players: [
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 26,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 0,
        username: "Alice",
        commander_damage: [
          { from: 1, amount: 14 },
          { from: 2, amount: 7 },
        ],
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
  });
  const avatar = avatarPos(0, 0, 2);
  const screen = worldToScreen(initialBoardModel().camera, avatar.x, avatar.y);

  let model = initialBoardModel();
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");
  const [pinned, cmds] = updateBoard(model, AltDown(), fold, "table-1");

  expect(pinned.inspectPin).toEqual({ name: "Alice", prepared: false, playerSeat: 0 });
  expect(cmds).toEqual([]);
});

test("pointer move pins the face-up board card while Alt is already held", () => {
  const creature = battlefieldCreature(7, "Sol Ring");
  const fold = gameFold({ objects: [creature] });
  const screen = screenCenterForCard(fold, 7);

  const [pinned, cmds] = updateBoard(
    { ...initialBoardModel(), altDown: true },
    BoardPointerMove({ x: screen.x, y: screen.y }),
    fold,
    "table-1",
  );

  expect(pinned.inspectPin).toEqual(
    expect.objectContaining({ name: "Sol Ring", objectId: 7, cardId: "card-1", print: "print-1" }),
  );
  expect((cmds[0] as { name?: string } | undefined)?.name).toBe("FetchInspectCard");
});

test("AltDown prefers hand aux hover over the battlefield hit under the cursor", () => {
  const creature = battlefieldCreature(7, "Board Bolt");
  const fold = gameFold({ objects: [creature] });
  const screen = screenCenterForCard(fold, 7);

  let model = initialBoardModel();
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");
  [model] = updateBoard(
    model,
    InspectAuxHovered({
      source: "hand",
      card: { name: "Hand Shock", cardId: "shock-id", print: "shock-print" },
    }),
    fold,
    "table-1",
  );
  const [pinned] = updateBoard(model, AltDown(), fold, "table-1");

  expect(pinned.inspectPin).toEqual({
    name: "Hand Shock",
    prepared: false,
    cardId: "shock-id",
    print: "shock-print",
  });
});

test("AltDown during an undecided mulligan clears inspect and stays inert", () => {
  const creature = battlefieldCreature(7, "Board Bolt");
  const fold = gameFold({
    mulliganing: true,
    objects: [creature],
    players: [
      { ...twoPlayerState().players[0], hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
      { ...twoPlayerState().players[1], hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
    ],
  });
  const screen = screenCenterForCard(fold, 7);

  let model: BoardModel = {
    ...initialBoardModel(),
    inspectPin: { name: "Old Pin", prepared: false, cardId: "old-card", print: "old-print" },
    inspectCard: null,
  };
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");
  const [locked, cmds] = updateBoard(model, AltDown(), fold, "table-1");

  expect(locked.altDown).toBe(false);
  expect(locked.inspectPin).toBeNull();
  expect(locked.inspectCard).toBeUndefined();
  expect(cmds).toEqual([]);
});

test("while Alt held, leaving the hand peek keeps hand inspect over a battlefield hit", () => {
  // Hand faces are pointer-events-none except the peek strip; leave clears aux while the
  // cursor is still over hand art. Live Alt re-pin must not steal to the BF card underneath.
  const creature = battlefieldCreature(7, "Board Bolt");
  const fold = gameFold({ objects: [creature] });
  const screen = screenCenterForCard(fold, 7);

  let model: BoardModel = { ...initialBoardModel(), altDown: true };
  [model] = updateBoard(
    model,
    InspectAuxHovered({
      source: "hand",
      card: { name: "Hand Shock", cardId: "shock-id", print: "shock-print" },
    }),
    fold,
    "table-1",
  );
  expect(model.inspectPin).toEqual(
    expect.objectContaining({ name: "Hand Shock", cardId: "shock-id", print: "shock-print" }),
  );

  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");
  [model] = updateBoard(model, InspectAuxHovered({ source: "hand", card: null }), fold, "table-1");
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");

  expect(model.handInspectHover).toEqual({
    name: "Hand Shock",
    cardId: "shock-id",
    print: "shock-print",
  });
  expect(model.inspectPin).toEqual(
    expect.objectContaining({ name: "Hand Shock", cardId: "shock-id", print: "shock-print" }),
  );
});

test("while Alt held, moving above the hand sticky band releases hand inspect to the board", () => {
  const creature = battlefieldCreature(7, "Board Bolt");
  const fold = gameFold({ objects: [creature] });
  const screen = screenCenterForCard(fold, 7);

  let model: BoardModel = { ...initialBoardModel(), altDown: true };
  [model] = updateBoard(
    model,
    InspectAuxHovered({
      source: "hand",
      card: { name: "Hand Shock", cardId: "shock-id", print: "shock-print" },
    }),
    fold,
    "table-1",
  );
  // Park the cursor in the hand sticky band, then leave the peek (aux stays latched).
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");
  [model] = updateBoard(model, InspectAuxHovered({ source: "hand", card: null }), fold, "table-1");
  expect(model.handInspectHover).not.toBeNull();

  // Leaving the raised-face band clears sticky hand hover so BF can win again.
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: 80 }), fold, "table-1");
  expect(model.handInspectHover).toBeNull();
  [model] = updateBoard(model, BoardPointerMove({ x: screen.x, y: screen.y }), fold, "table-1");
  expect(model.inspectPin).toEqual(expect.objectContaining({ name: "Board Bolt", objectId: 7 }));
});

test("aux hover pins hand and stack cards while Alt is already held", () => {
  const fold = gameFold();
  let model: BoardModel = { ...initialBoardModel(), altDown: true };

  let cmds: ReadonlyArray<unknown>;
  [model, cmds] = updateBoard(
    model,
    InspectAuxHovered({
      source: "hand",
      card: { name: "Hand Shock", cardId: "shock-id", print: "shock-print" },
    }),
    fold,
    "table-1",
  );
  expect(model.inspectPin).toEqual({
    name: "Hand Shock",
    prepared: false,
    cardId: "shock-id",
    print: "shock-print",
  });
  expect((cmds[0] as { name?: string } | undefined)?.name).toBe("FetchInspectCard");

  [model, cmds] = updateBoard(
    model,
    InspectAuxHovered({
      source: "stack",
      card: { name: "Stack Bolt", cardId: "bolt-id", print: "bolt-print" },
    }),
    fold,
    "table-1",
  );
  expect(model.inspectPin).toEqual({
    name: "Hand Shock",
    prepared: false,
    cardId: "shock-id",
    print: "shock-print",
  });
  expect(cmds).toEqual([]);

  [model, cmds] = updateBoard(model, InspectAuxHovered({ source: "hand", card: null }), fold, "table-1");
  expect(model.inspectPin).toEqual({
    name: "Stack Bolt",
    prepared: false,
    cardId: "bolt-id",
    print: "bolt-print",
  });
  expect((cmds[0] as { name?: string } | undefined)?.name).toBe("FetchInspectCard");
});

test("AltUp clears altDown and dismisses the inspect pin", () => {
  const model = update(
    {
      ...initialBoardModel(),
      altDown: true,
      inspectPin: { name: "Sol Ring", prepared: false },
      inspectCard: null,
    },
    AltUp(),
  );
  expect(model.altDown).toBe(false);
  expect(model.inspectPin).toBeNull();
  expect(model.inspectCard).toBeUndefined();
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

test("KeepHandClicked submits keep_hand for the viewer", () => {
  const fold = gameFold({
    ...twoPlayerState(),
    mulliganing: true,
    players: [
      { ...twoPlayerState().players[0], hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
      { ...twoPlayerState().players[1], hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
    ],
  });
  const [, cmds] = updateBoard(initialBoardModel(), KeepHandClicked(), fold, "table-1");
  expect(cmds[0]?.name).toBe(SubmitIntent.name);
});

test("MulliganClicked is a no-op when can_mulligan is false", () => {
  const fold = gameFold({
    ...twoPlayerState(),
    mulliganing: true,
    players: [
      { ...twoPlayerState().players[0], hand_kept: false, can_mulligan: false, mulligans_taken: 6 },
      { ...twoPlayerState().players[1], hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
    ],
  });
  const [, cmds] = updateBoard(initialBoardModel(), MulliganClicked(), fold, "table-1");
  expect(cmds).toEqual([]);
});

test("KeyboardSpacePressed is inert while mulliganing", () => {
  const fold = gameFold({ ...twoPlayerState(), mulliganing: true });
  const [, cmds] = updateBoard(initialBoardModel(), KeyboardSpacePressed(), fold, "table-1");
  expect(cmds).toEqual([]);
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
