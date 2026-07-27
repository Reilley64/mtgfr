/**
 * @vitest-environment happy-dom
 *
 * Board overlay surface coverage — every chrome/prompt/overlay panel must appear here
 * (or in a focused sibling Scene test) with a data-testid assertion.
 * See AGENTS.md: "Client UI: every surface gets a Scene test."
 */
import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { beforeAll, expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import { fromProtoWire } from "~/wire/protoMap";
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import type { GameFoldState, LogLine } from "../../game/fold";
import { emptyCostPicks, type ModalCast, type PlayModePick, type XPromptState } from "../action/execution";
import { ZONE } from "../geometry/layout";
import type { Message } from "../messages";
import { type BoardModel, initialBoardModel } from "../submodel";
import { type BoardViewModel, view as boardView } from "../view";
import { boardOverlays } from "./overlays";
import { resolveBoardCardArtMounts, resolveBoardOverlayMounts, resolveLiveBoardMounts } from "./scene-helpers";

/** Preorder `data-testid` walk — later siblings paint above earlier ones under `board-mount`. */
function collectTestIds(node: unknown, out: string[] = []): string[] {
  if (node == null || typeof node !== "object") return out;
  const n = node as { data?: { attrs?: Record<string, string> }; children?: unknown[] };
  const id = n.data?.attrs?.["data-testid"];
  if (typeof id === "string") out.push(id);
  for (const child of n.children ?? []) {
    if (typeof child === "object" && child != null) collectTestIds(child, out);
  }
  return out;
}

function testId(node: unknown): string | null {
  if (node == null || typeof node !== "object") return null;
  const n = node as { data?: { attrs?: Record<string, string> } };
  const id = n.data?.attrs?.["data-testid"];
  return typeof id === "string" ? id : null;
}

function className(node: unknown): string {
  if (node == null || typeof node !== "object") return "";
  const n = node as { data?: { class?: Record<string, boolean> } };
  return Object.entries(n.data?.class ?? {})
    .filter(([, active]) => active)
    .map(([name]) => name)
    .join(" ");
}

function findTestId(node: unknown, id: string): unknown | null {
  if (testId(node) === id) return node;
  if (node == null || typeof node !== "object") return null;
  const n = node as { children?: unknown[] };
  for (const child of n.children ?? []) {
    const found = findTestId(child, id);
    if (found != null) return found;
  }
  return null;
}

function dataAttr(node: unknown, name: string): string | null {
  if (node == null || typeof node !== "object") return null;
  const n = node as { data?: { attrs?: Record<string, string> } };
  const value = n.data?.attrs?.[`data-${name}`];
  return typeof value === "string" ? value : null;
}

function attr(node: unknown, name: string): string | null {
  if (node == null || typeof node !== "object") return null;
  const n = node as { data?: { attrs?: Record<string, string> } };
  const value = n.data?.attrs?.[name];
  return typeof value === "string" ? value : null;
}

function findAttr(node: unknown, name: string, value: string): unknown | null {
  if (attr(node, name) === value) return node;
  if (node == null || typeof node !== "object") return null;
  const n = node as { children?: unknown[] };
  for (const child of n.children ?? []) {
    const found = findAttr(child, name, value);
    if (found != null) return found;
  }
  return null;
}

function findParentOfTestId(node: unknown, id: string): unknown | null {
  if (node == null || typeof node !== "object") return null;
  const n = node as { children?: unknown[] };
  for (const child of n.children ?? []) {
    if (testId(child) === id) return node;
    const parent = findParentOfTestId(child, id);
    if (parent != null) return parent;
  }
  return null;
}

const h = html<Message>();

beforeAll(() => {
  class MockImage {
    onload: (() => void) | null = null;
    onerror: (() => void) | null = null;
    src = "";
    addEventListener(type: string, fn: () => void): void {
      if (type === "load") this.onload = fn;
    }
  }
  // @ts-expect-error test stub
  globalThis.Image = MockImage;
});

type OverlayModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const overlayView = Submodel.defineView<OverlayModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log);
});

const fullBoardView = Submodel.defineView<BoardViewModel, Message>(boardView);

function player(
  seat: number,
  overrides: Partial<VisibleState["players"][number]> = {},
): VisibleState["players"][number] {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: seat,
    username: seat === 0 ? "Alice" : "Bob",
    ...overrides,
  };
}

function cost(overrides: Partial<WireCost> = {}): WireCost {
  return {
    generic: 0,
    colored: [0, 0, 0, 0, 0],
    ...overrides,
  };
}

function action(id: number, overrides: Partial<ActionView> = {}): ActionView {
  return {
    id,
    kind: "cast",
    label: testMessageRef(`Action ${id}`),
    needs_target: false,
    section: "hand",
    ...overrides,
  };
}

function card(id: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "instant" },
    mana_cost: cost({ generic: 1 }),
    marked_damage: 0,
    name: `Card ${id}`,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
    ...overrides,
  };
}

function gameState(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects: [],
    pending_choice: null,
    players: [player(0), player(1)],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...overrides,
  };
}

function gameFold(state: VisibleState | null = gameState(), log: ReadonlyArray<LogLine> = []): GameFoldState {
  return {
    seq: 1,
    state,
    log: [...log],
    reject: null,
    provenance: {
      zoneMoves: new Map(),
      resolvedFromStack: new Set(),
      leftStackToPile: new Set(),
      battlefieldExits: new Map(),
      tokenCreators: new Map(),
      landPlayFrom: new Map(),
      zonePileEntrances: new Map(),
      stackEntrances: new Map(),
      priorStackObjectIds: new Set(),
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false, destroy: false, exile: false },
  };
}

function stagedBoard(overrides: Partial<BoardModel> = {}): BoardModel {
  const spell = card(10, {
    kind: { kind: "sorcery" },
    name: "Shock",
    owner: 0,
    controller: 0,
  });
  return {
    ...initialBoardModel(),
    staged: {
      card: spell,
      action: action(10, {
        object: spell.id,
        label: testMessageRef("Cast Shock"),
        needs_target: true,
        targets: [{ kind: "object", id: 22 }],
      }),
      picks: emptyCostPicks(),
      preferPick: false,
      playOrigin: { x: 0, y: 0 },
      playOriginScreen: { x: 0, y: 0 },
    },
    ...overrides,
  };
}

function overlayModel(
  board: BoardModel = initialBoardModel(),
  state: VisibleState = gameState(),
  log: ReadonlyArray<LogLine> = [],
): OverlayModel {
  return {
    board,
    fold: gameFold(state, log),
    tableId: "T1",
  };
}

function fullBoardModel(
  board: BoardModel = initialBoardModel(),
  state: VisibleState | null = gameState(),
  connected = true,
  log: ReadonlyArray<LogLine> = [],
): BoardViewModel {
  return {
    board,
    fold: gameFold(state, log),
    tableId: "T1",
    connected,
  };
}

function overlayScene(model: OverlayModel, ...steps: readonly unknown[]) {
  Scene.scene<OverlayModel, Message>(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    ...(steps as []),
  );
}

function overlaySceneWithoutMounts(model: OverlayModel, ...steps: readonly unknown[]) {
  Scene.scene<OverlayModel, Message>(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    ...(steps as []),
  );
}

function liveBoardScene(model: BoardViewModel, ...steps: readonly unknown[]) {
  Scene.scene<BoardViewModel, Message>(
    { update: (m) => [m, []], view: fullBoardView },
    Scene.with(model),
    resolveLiveBoardMounts(),
    ...(steps as []),
  );
}

test("smoke scene keeps existing chrome visible", () => {
  const handCard = card(1, {
    name: "Lightning Bolt",
    print: "bolt-print",
    mana_cost: cost({ generic: 1 }),
  });
  const handAction = action(1, {
    label: testMessageRef("Cast Lightning Bolt"),
    object: handCard.id,
    section: "hand",
  });
  const model = overlayModel(
    {
      ...initialBoardModel(),
      hintDismissed: false,
      hintAutoHidden: false,
    },
    gameState({
      actions: [handAction],
      objects: [handCard],
      players: [
        player(0, {
          mana_pool: { any: 0, colored: [1, 0, 0, 0, 0], colorless: 0, either: [], of_colors: [] },
        }),
        player(1),
      ],
    }),
    [{ seq: 1, text: "AUTO Alice casts Lightning Bolt", auto: true }],
  );

  overlayScene(
    model,
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("hand-bar")).toExist(),
    Scene.expect(Scene.testId("board-primary")).toExist(),
    Scene.expect(Scene.testId("board-concede")).toExist(),
    Scene.expect(Scene.testId("board-hint")).toExist(),
    Scene.expect(Scene.testId("board-sound-toggle")).toExist(),
    // Battlefield mana tray is composed in view.ts (under bitmap), not in boardOverlays.
    Scene.expect(Scene.testId("mana-tray")).toBeAbsent(),
    Scene.expect(Scene.testId("board-log")).toExist(),
    Scene.expect(Scene.testId("priority-context-bar")).toExist(),
  );
});

test("top-left toolbar keeps legend toggle and sound as in-flow siblings", () => {
  overlayScene(
    overlayModel({ ...initialBoardModel(), hintDismissed: true, hintAutoHidden: true }),
    Scene.tap((sim) => {
      const toolbar = findParentOfTestId(sim.html, "board-legend-toggle");
      expect(toolbar).not.toBeNull();
      expect(className(toolbar)).toContain("fixed top-md left-md");
      expect(className(toolbar)).toContain("flex items-center gap-xs");

      const children = (toolbar as { children?: unknown[] }).children ?? [];
      expect(children.map(testId)).toEqual(expect.arrayContaining(["board-legend-toggle", "board-sound-toggle"]));
      expect(className(children.find((child) => testId(child) === "board-legend-toggle"))).not.toContain("left-md");
    }),
    Scene.expect(Scene.testId("board-concede")).toHaveClass("right-md"),
    Scene.expect(Scene.testId("board-concede")).not.toHaveClass("left-md"),
  );
});

test("turn chrome renders banner and label", () => {
  overlayScene(
    overlayModel(),
    Scene.expect(Scene.testId("board-turn-banner")).toExist(),
    Scene.expect(Scene.testId("board-turn-label")).toContainText("Your turn"),
  );
});

test("stack context renders resolve stack affordance and top caption", () => {
  const state = gameState({
    stack: [{ controller: 1, kind: "ability", label: testMessageRef("Ward 2"), source: 99 }],
  });
  overlayScene(
    overlayModel(initialBoardModel(), state),
    Scene.expect(Scene.testId("board-stack-yield")).toExist(),
    Scene.expect(Scene.testId("stack-top-caption")).toExist(),
  );
});

test("armed stack yield state renders separately", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        stack: [{ controller: 1, kind: "spell", label: testMessageRef("Bolt"), source: 77 }],
        yielded: true,
      }),
    ),
    Scene.expect(Scene.testId("board-stack-yield-armed")).toExist(),
  );
});

test("active player sees the end-turn affordance", () => {
  overlayScene(overlayModel(), Scene.expect(Scene.testId("board-end-turn")).toExist());
});

test("end turn is hidden when a goaded creature must attack", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        step: 5, // declare attackers
        actions: [
          {
            id: 1,
            kind: "declare_attackers",
            label: testMessageRef("Declare attackers"),
            needs_target: false,
            section: "combat",
            declare_for: [0],
            required_attacks: [{ attacker: 7, defender: 1 }],
          },
        ],
      }),
    ),
    Scene.expect(Scene.testId("board-end-turn")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toContainText("Attack (1)"),
  );
});

test("non-active player sees the turn-yield rocker", () => {
  overlayScene(
    overlayModel(initialBoardModel(), gameState({ active_player: 1 })),
    Scene.expect(Scene.testId("board-turn-yield")).toExist(),
  );
});

test("staged targeting shows cancel affordance and staged hint", () => {
  const target = card(22, {
    controller: 1,
    owner: 1,
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
    name: "Bear",
  });
  const board = stagedBoard();
  if (board.staged == null) throw new Error("expected staged board fixture");
  overlayScene(
    overlayModel(
      {
        ...board,
        staged: {
          ...board.staged,
          action: action(10, {
            object: 10,
            label: {
              key: "effect.life_gain",
              params: [{ name: "amount", int_value: 1 }],
              children: [],
            },
            needs_target: true,
            targets: [{ kind: "object", id: 22 }],
          }),
        },
      },
      gameState({ objects: [target] }),
    ),
    Scene.expect(Scene.testId("board-cancel-target")).toExist(),
    Scene.expect(Scene.testId("board-staged-hint")).toHaveText("Gain 1 life: click a highlighted card"),
  );
});

test("staged preferPick on-board target pick uses a center modal", () => {
  const target = card(22, {
    controller: 1,
    owner: 1,
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
    name: "Bear",
    print: "bear-print",
  });
  const spell = card(10, {
    kind: { kind: "sorcery" },
    name: "Shock",
    owner: 0,
    controller: 0,
  });
  overlayScene(
    overlayModel(
      stagedBoard({
        staged: {
          card: spell,
          action: action(10, {
            object: spell.id,
            label: testMessageRef("Cast Shock"),
            needs_target: true,
            targets: [{ kind: "object", id: 22 }],
          }),
          picks: emptyCostPicks(),
          preferPick: true,
          playOrigin: { x: 0, y: 0 },
          playOriginScreen: { x: 0, y: 0 },
        },
      }),
      gameState({ objects: [target] }),
    ),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("target-pick-modal")).toExist(),
    Scene.expect(Scene.testId("target-pick-0")).toExist(),
    Scene.expect(Scene.testId("target-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("board-cancel-target")).toBeAbsent(),
  );
});

test("board reject surface renders when local reject text is set", () => {
  overlayScene(
    overlayModel({ ...initialBoardModel(), reject: "Choose a legal target" }),
    Scene.expect(Scene.testId("board-reject")).toContainText("Choose a legal target"),
  );
});

test("hand surfaces render cost pips and fade the drag source without an HTML ghost", () => {
  const handCard = card(42, {
    name: "Lightning Bolt",
    print: "bolt-print",
    mana_cost: cost({ generic: 1 }),
  });
  const castAction = action(7, {
    label: testMessageRef("Cast Lightning Bolt"),
    object: handCard.id,
    section: "hand",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        handDrag: {
          action: castAction,
          name: "Lightning Bolt",
          print: "bolt-print",
          manaCost: handCard.mana_cost,
          kind: "instant",
          x: 200,
          y: 300,
        },
      },
      gameState({ actions: [castAction], objects: [handCard] }),
    ),
    resolveBoardCardArtMounts(1),
    Scene.expect(Scene.testId("hand-cost-pips")).toExist(),
    Scene.expect(Scene.testId("hand-drag-ghost")).not.toExist(),
  );
});

test("inspect overlay renders from a pinned inspect card", () => {
  overlayScene(
    overlayModel({
      ...initialBoardModel(),
      inspectPin: { name: "Sol Ring", prepared: false, print: "sol-ring-print" },
      inspectCard: {
        id: "sol-ring",
        name: "Sol Ring",
        oracle: "{T}: Add {C}{C}.",
        approximates: null,
        back: null,
        color_identity: [],
        cost: cost({ generic: 1 }),
        default_print: "sol-ring-print",
        keywords: [],
        kind: { kind: "artifact" },
        legendary: false,
        otags: [],
        set: "",
        sets: ["soc"],
        subtypes: [],
        summary: [],
      },
    }),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
  );
});

test("inspect overlay shows marked damage for a damaged battlefield permanent", () => {
  const bear = card(42, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
    name: "Grizzly Bears",
    print: "bears-print",
    marked_damage: 3,
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        inspectPin: {
          name: "Grizzly Bears",
          objectId: 42,
          prepared: false,
          print: "bears-print",
        },
        inspectCard: {
          id: "grizzly-bears",
          name: "Grizzly Bears",
          oracle: "",
          approximates: null,
          back: null,
          color_identity: [],
          cost: cost({ generic: 1, colored: [0, 1, 0, 0, 0] }),
          default_print: "bears-print",
          keywords: [],
          kind: { kind: "creature", power: 2, toughness: 2 },
          legendary: false,
          otags: [],
          set: "",
          sets: ["soc"],
          subtypes: ["Bear"],
          summary: [],
        },
      },
      gameState({ objects: [bear] }),
    ),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.testId("inspect-marked-damage")).toHaveText("Marked damage: 3"),
  );
});

test("inspect overlay shows per-commander damage breakdown for a player pin", () => {
  const atraxa = card(9, {
    owner: 1,
    controller: 1,
    is_commander: true,
    name: "Atraxa, Praetors' Voice",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 4, toughness: 4 },
    power: 4,
    toughness: 4,
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        inspectPin: { name: "Alice", prepared: false, playerSeat: 0 },
      },
      gameState({
        players: [
          player(0, {
            username: "Alice",
            life: 26,
            commander_damage: [
              { from: 1, amount: 14 },
              { from: 2, amount: 7 },
            ],
          }),
          player(1, { username: "Bob" }),
          player(2, { username: "Carol" }),
        ],
        objects: [atraxa],
      }),
    ),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.testId("inspect-player-life")).toHaveText("Life: 26"),
    Scene.expect(Scene.testId("inspect-commander-damage")).toExist(),
    Scene.expect(Scene.testId("inspect-commander-damage-1")).toHaveText("Bob — Atraxa, Praetors' Voice: 14 / 21"),
    Scene.expect(Scene.testId("inspect-commander-damage-2")).toHaveText("Carol: 7 / 21"),
  );
});

test("inspect overlay omits commander-damage block when the seat has none", () => {
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        inspectPin: { name: "Alice", prepared: false, playerSeat: 0 },
      },
      gameState({
        players: [player(0, { username: "Alice", life: 40 }), player(1, { username: "Bob" })],
      }),
    ),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.testId("inspect-player-life")).toHaveText("Life: 40"),
    Scene.expect(Scene.testId("inspect-commander-damage")).toBeAbsent(),
  );
});

test("pile overlay renders with its close control", () => {
  const graveyardCard = card(60, {
    owner: 1,
    controller: 1,
    zone: ZONE.Graveyard,
    name: "Dead Weight",
    print: "dead-weight-print",
  });
  overlayScene(
    overlayModel(
      { ...initialBoardModel(), pileExpand: { zone: ZONE.Graveyard, owner: 1 } },
      gameState({ objects: [graveyardCard] }),
    ),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("pile-overlay")).toExist(),
    Scene.expect(Scene.testId("pile-overlay-close")).toExist(),
  );
});

test("concede confirmation dialog renders both actions", () => {
  overlayScene(
    overlayModel({ ...initialBoardModel(), confirmConcede: true }),
    Scene.expect(Scene.testId("concede-dialog")).toExist(),
    Scene.expect(Scene.testId("concede-cancel")).toExist(),
    Scene.expect(Scene.testId("concede-confirm")).toExist(),
  );
});

test("result overlay renders watch and leave actions", () => {
  overlaySceneWithoutMounts(
    overlayModel(
      initialBoardModel(),
      gameState({
        players: [player(0, { lost: true }), player(1)],
      }),
    ),
    Scene.expect(Scene.testId("result-overlay")).toExist(),
    Scene.expect(Scene.testId("result-watch")).toExist(),
    Scene.expect(Scene.testId("result-leave")).toExist(),
  );
});

test("x prompt uses a center modal with a live cost preview", () => {
  const xPrompt: XPromptState = {
    action: action(12, { label: testMessageRef("Comet Storm"), has_x: true, max_x: 3, min_x: 0 }),
    target: null,
    picks: emptyCostPicks(),
    modes: [],
    name: "Comet Storm",
    minX: 0,
    maxX: 3,
    draftX: 3,
    xCost: cost({ generic: 1, has_x: true, x_symbols: 1 }),
  };
  overlayScene(
    overlayModel({ ...initialBoardModel(), xPrompt }),
    Scene.expect(Scene.testId("x-prompt-modal")).toExist(),
    Scene.expect(Scene.testId("x-prompt-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("x-prompt-preview")).toHaveText("Pay {4}"),
    Scene.expect(Scene.testId("x-prompt-value")).toHaveText("3"),
    Scene.expect(Scene.testId("x-prompt-inc")).toBeDisabled(),
    Scene.expect(Scene.testId("x-prompt-min")).toExist(),
    Scene.expect(Scene.testId("x-prompt-dec")).toExist(),
    Scene.expect(Scene.testId("x-prompt-max")).toExist(),
    Scene.expect(Scene.testId("x-prompt-confirm")).toExist(),
    Scene.expect(Scene.testId("x-prompt-0")).not.toExist(),
  );
});

test("off-board staged target pick uses a center modal", () => {
  const corpse = card(22, {
    name: "Corpse",
    zone: ZONE.Graveyard,
    kind: { kind: "creature", power: 2, toughness: 2 },
    print: "corpse-print",
  });
  const spell = card(10, {
    kind: { kind: "sorcery" },
    name: "Reanimate",
    owner: 0,
    controller: 0,
  });
  overlayScene(
    overlayModel(
      stagedBoard({
        staged: {
          card: spell,
          action: action(10, {
            object: spell.id,
            label: testMessageRef("Cast Reanimate"),
            needs_target: true,
            targets: [{ kind: "object", id: 22 }],
          }),
          picks: emptyCostPicks(),
          preferPick: false,
          playOrigin: { x: 0, y: 0 },
          playOriginScreen: { x: 0, y: 0 },
        },
      }),
      gameState({ objects: [spell, corpse] }),
    ),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("target-pick-modal")).toExist(),
    Scene.expect(Scene.testId("target-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("target-pick")).toBeAbsent(),
    Scene.expect(Scene.testId("target-pick-0")).toExist(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("modal mode picker renders before modes are chosen", () => {
  const modalCast: ModalCast = {
    action: action(13, {
      label: testMessageRef("Cryptic Command"),
      modal: {
        choose: 2,
        choose_max: 2,
        modes: [
          { label: testMessageRef("Counter target spell"), needs_target: false, targets: [] },
          { label: testMessageRef("Draw a card"), needs_target: false, targets: [] },
        ],
      },
    }),
    modes: [
      { label: testMessageRef("Counter target spell"), needs_target: false, targets: [] },
      { label: testMessageRef("Draw a card"), needs_target: false, targets: [] },
    ],
    picks: emptyCostPicks(),
    chosen: null,
    answers: [],
    modeDraft: [],
  };
  overlayScene(
    overlayModel({ ...initialBoardModel(), modalCast }),
    Scene.expect(Scene.testId("modal-mode-aim")).toExist(),
    Scene.expect(Scene.testId("modal-mode-picker")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="modal-mode-0"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="modal-mode-1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="modal-cast"]')).toBeDisabled(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-cancel"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="modal-mode-aim"] [data-testid="modal-mode-0"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="modal-mode-aim"] [data-testid="modal-cast"]')).toBeAbsent(),
  );
});

test("modal mode picker exposes aria-pressed for selected multi-mode rows", () => {
  const modalCast: ModalCast = {
    action: action(13, {
      label: testMessageRef("Cryptic Command"),
      modal: {
        choose: 2,
        choose_max: 2,
        modes: [
          { label: testMessageRef("Counter target spell"), needs_target: false, targets: [] },
          { label: testMessageRef("Draw a card"), needs_target: false, targets: [] },
        ],
      },
    }),
    modes: [
      { label: testMessageRef("Counter target spell"), needs_target: false, targets: [] },
      { label: testMessageRef("Draw a card"), needs_target: false, targets: [] },
    ],
    picks: emptyCostPicks(),
    chosen: null,
    answers: [],
    modeDraft: [1],
  };
  overlayScene(
    overlayModel({ ...initialBoardModel(), modalCast }),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="modal-mode-1"]')).toHaveAttr(
      "aria-pressed",
      "true",
    ),
  );
});

test("playModePick shows docked play-mode-aim with one button per mode", () => {
  const valleyRannet = card(42, { name: "Valley Rannet" });
  const playModePick: PlayModePick = {
    card: valleyRannet,
    modes: [
      action(1, { kind: "cast", object: valleyRannet.id, label: testMessageRef("Valley Rannet") }),
      action(2, {
        kind: "activate_hand_ability",
        object: valleyRannet.id,
        label: testMessageRef("Discard: Mountain"),
      }),
      action(3, {
        kind: "activate_hand_ability",
        object: valleyRannet.id,
        label: testMessageRef("Discard: Forest"),
      }),
    ],
    dropSeed: { x: 0, y: 0 },
    screenOrigin: { x: 400, y: 200 },
  };

  overlayScene(
    overlayModel({ ...initialBoardModel(), playModePick }),
    Scene.expect(Scene.testId("play-mode-aim")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="play-mode-0"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="play-mode-1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="play-mode-2"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-cancel"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="play-mode-aim"] [data-testid="play-mode-0"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="play-mode-aim"] [data-testid="prompt-cancel"]')).toBeAbsent(),
  );
});

test("modal waiting chrome docks when modes are chosen and a target is still needed", () => {
  const modalCast: ModalCast = {
    action: action(13, {
      label: testMessageRef("Fact or Fiction"),
      modal: {
        choose: 1,
        choose_max: 1,
        modes: [{ label: testMessageRef("Counter"), needs_target: true, targets: [] }],
      },
    }),
    modes: [{ label: testMessageRef("Counter"), needs_target: true, targets: [] }],
    picks: emptyCostPicks(),
    chosen: [0],
    answers: [],
    modeDraft: [],
  };
  overlayScene(
    overlayModel({ ...initialBoardModel(), modalCast }),
    Scene.expect(Scene.testId("modal-waiting-aim")).toExist(),
    Scene.expect(Scene.testId("modal-waiting")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-cancel"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="modal-waiting-aim"] [data-testid="prompt-cancel"]')).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("join-forces mana prompt shows a stepper instead of per-amount buttons", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "pay_any_amount_of_mana",
          max: 20,
          player: 0,
          source: 3,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-join-forces-modal")).toExist(),
    Scene.expect(Scene.testId("pending-join-forces-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-number-value")).toHaveText("0"),
    Scene.expect(Scene.testId("prompt-number-min")).toExist(),
    Scene.expect(Scene.testId("prompt-number-max")).toExist(),
    Scene.expect(Scene.testId("prompt-number-0")).not.toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toExist(),
  );
});

test("may_draw_up_to uses a center modal with number buttons", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "may_draw_up_to",
          label: testMessageRef("You may draw up to 3"),
          max: 3,
          player: 0,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-draw-count-modal")).toExist(),
    Scene.expect(Scene.testId("pending-draw-count-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-number-0")).toExist(),
    Scene.expect(Scene.testId("prompt-number-3")).toExist(),
  );
});

test("choose_target_players off-board list uses a center player-pick modal", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        players: [player(0), player(1), player(2)],
        pending_choice: {
          kind: "choose_target_players",
          label: testMessageRef("Choose opponents"),
          min: 1,
          max: 2,
          player: 0,
          source: 1,
          items: [
            { id: 0, label: "Player 2" },
            { id: 1, label: "Player 3" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-player-pick-modal")).toExist(),
    Scene.expect(Scene.testId("pending-player-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-player-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-player-1")).toExist(),
    Scene.expect(Scene.testId("prompt-player-2")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
  );
});

test("choose_splitting_opponent off-board list uses a center player-pick modal", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        players: [player(0), player(1), player(2)],
        pending_choice: {
          kind: "choose_splitting_opponent",
          label: testMessageRef("Choose an opponent"),
          player: 0,
          source: 1,
          items: [
            { id: 0, label: "Player 2" },
            { id: 1, label: "Player 3" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-player-pick-modal")).toExist(),
    Scene.expect(Scene.testId("pending-player-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-player-1")).toExist(),
  );
});

test("on-board choose_target_players Confirm lives in the primary bar", () => {
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "player-pick", players: [1] },
      },
      gameState({
        players: [player(0), player(1), player(2)],
        pending_choice: {
          kind: "choose_target_players",
          label: testMessageRef("Choose opponents"),
          min: 1,
          max: 2,
          player: 0,
          source: 1,
          items: [
            { id: 0, player: 1, label: "Player 2" },
            { id: 0, player: 2, label: "Player 3" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-player-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-player-aim"] [data-testid="prompt-submit"]')).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("choose_trigger_modes shows docked pending-trigger-modes-aim", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "choose_trigger_modes",
          choose: 1,
          modes: [
            { label: testMessageRef("Draw a card"), needs_target: false, targets: [] },
            { label: testMessageRef("Gain 1 life"), needs_target: false, targets: [] },
          ],
          optional: false,
          player: 0,
          source: 1,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-trigger-modes-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-mode-choice-0")).toExist(),
    Scene.expect(Scene.testId("prompt-mode-choice-1")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toBeDisabled(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-cancel"]')).toExist(),
    Scene.expect(
      Scene.selector('[data-testid="pending-trigger-modes-aim"] [data-testid="prompt-submit"]'),
    ).toBeAbsent(),
    Scene.expect(
      Scene.selector('[data-testid="pending-trigger-modes-aim"] [data-testid="prompt-cancel"]'),
    ).toBeAbsent(),
  );
});

test("choose_pile_for_hand shows docked pending-pile-aim", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "choose_pile_for_hand",
          pile_a: [{ id: 1, label: "Pile A card" }],
          pile_b: [{ id: 2, label: "Pile B card" }],
          player: 0,
          source: 8,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-pile-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-pile-0"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-pile-1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-pile-aim"] [data-testid="prompt-pile-0"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-pile-aim"] [data-testid="prompt-pile-1"]')).toBeAbsent(),
  );
});

test("opponent_chooses_pile shows docked pending-pile-aim", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "opponent_chooses_pile",
          pile_a: [{ id: 3, label: "Alpha" }],
          pile_b: [{ id: 4, label: "Beta" }],
          player: 0,
          source: 9,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-pile-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-pile-0")).toExist(),
  );
});

test("on-board choose_target aims instead of showing a card grid", () => {
  const bear = card(7, {
    name: "Bear",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
  });
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [bear],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Target creature"),
          min: 1,
          max: 1,
          player: 0,
          source: 1,
          items: [{ id: 7, label: "Bear" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-target-aim")).toHaveText("Target creature"),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-card-7")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-target-aim"] [data-testid="prompt-submit"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-target-aim"] [data-testid="prompt-decline"]')).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-submit")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-decline")).toBeAbsent(),
  );
});

test("legend rule aim asks which legendary permanent to keep", () => {
  const a = card(7, {
    name: "Legendary Test Creature",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
  });
  const b = card(8, {
    name: "Legendary Test Creature",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
  });
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [a, b],
        pending_choice: {
          kind: "choose_legendary_keep",
          name: "Legendary Test Creature",
          player: 0,
          items: [
            { id: 7, label: "Legendary Test Creature" },
            { id: 8, label: "Legendary Test Creature" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-target-aim")).toHaveText(
      "Legend rule — choose which Legendary Test Creature to keep",
    ),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("on-board sacrifice_edict shows aim chrome instead of card grid", () => {
  const bear = card(7, {
    name: "Bear",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
  });
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [bear],
        pending_choice: {
          kind: "sacrifice_edict",
          player: 0,
          source: 1,
          items: [{ id: 7, label: "Bear" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-target-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-card-7")).toBeAbsent(),
  );
});

test("multi on-board choose_target shows Confirm count chrome", () => {
  const a = card(1, {
    name: "A",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
  });
  const b = card(2, {
    name: "B",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
  });
  overlayScene(
    overlayModel(
      { ...initialBoardModel(), promptDraft: { kind: "card-pick", picked: [1], filter: "" } },
      gameState({
        objects: [a, b],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Target creatures"),
          min: 1,
          max: 2,
          player: 0,
          source: 1,
          items: [
            { id: 1, label: "A" },
            { id: 2, label: "B" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-target-aim")).toExist(),
    Scene.expect(Scene.testId("pending-target-count")).toHaveText("1 / 2 selected"),
    Scene.expect(Scene.testId("prompt-submit")).toBeEnabled(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("optional on-board choose_target Decline lives in the primary bar", () => {
  const bear = card(7, {
    name: "Bear",
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
  });
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [bear],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Target creature"),
          min: 0,
          max: 1,
          player: 0,
          source: 1,
          items: [{ id: 7, label: "Bear" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-target-aim")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-decline"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-target-aim"] [data-testid="prompt-decline"]')).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("scry uses a center modal with Top and Bottom arrange lanes", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "scry",
          player: 0,
          items: [
            { id: 1, label: "Island" },
            { id: 2, label: "Forest" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-arrange-modal")).toExist(),
    Scene.expect(Scene.testId("pending-arrange-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-arrange-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-arrange-top")).toExist(),
    Scene.expect(Scene.testId("prompt-arrange-bottom")).toExist(),
    Scene.expect(Scene.testId("prompt-arrange-bottom-label")).toHaveText("Bottom of library"),
    Scene.expect(Scene.testId("prompt-card-1")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toExist(),
  );
});

test("order_triggers uses a center modal with drag rows and arrow controls", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "order_triggers",
          count: 2,
          labels: [testMessageRef("ETB draw"), testMessageRef("ETB treasure")],
          player: 0,
          source: 4,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-order-modal")).toExist(),
    Scene.expect(Scene.testId("pending-order-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-order-list")).toExist(),
    Scene.expect(Scene.selector('[data-testid="prompt-order-0"][draggable="true"]')).toExist(),
    Scene.expect(Scene.testId("prompt-order-pick-0")).toHaveText("ETB draw"),
    Scene.expect(Scene.testId("prompt-order-up-0")).toExist(),
    Scene.expect(Scene.testId("prompt-order-down-1")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toExist(),
  );
});

test("choose_countered_spell_destination aim shows docked Top and Bottom", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "choose_countered_spell_destination",
          player: 0,
          spell: 5,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-destination-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(
      Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-destination-top"]'),
    ).toExist(),
    Scene.expect(
      Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-destination-bottom"]'),
    ).toExist(),
    Scene.expect(
      Scene.selector('[data-testid="pending-destination-aim"] [data-testid="prompt-destination-top"]'),
    ).toBeAbsent(),
    Scene.expect(
      Scene.selector('[data-testid="pending-destination-aim"] [data-testid="prompt-destination-bottom"]'),
    ).toBeAbsent(),
  );
});

test("revealed_card_to_battlefield_or_hand aim shows face and destination buttons", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "revealed_card_to_battlefield_or_hand",
          player: 0,
          item: { id: 17, label: "Beast" },
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-revealed-destination-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-revealed-face")).toHaveText("Beast"),
    Scene.expect(
      Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-destination-battlefield"]'),
    ).toExist(),
    Scene.expect(
      Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-destination-hand"]'),
    ).toExist(),
    Scene.expect(
      Scene.selector('[data-testid="pending-revealed-destination-aim"] [data-testid="prompt-destination-battlefield"]'),
    ).toBeAbsent(),
    Scene.expect(
      Scene.selector('[data-testid="pending-revealed-destination-aim"] [data-testid="prompt-destination-hand"]'),
    ).toBeAbsent(),
  );
});

test("partition_revealed uses a center modal with Pile A and Pile B lanes", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "partition_revealed",
          player: 0,
          source: 9,
          items: [
            { id: 1, label: "A" },
            { id: 2, label: "B" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-partition-modal")).toExist(),
    Scene.expect(Scene.testId("pending-partition-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-partition-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toExist(),
  );
});

test("distribute_top uses a center modal with Hand Bottom Exile lanes", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "distribute_top",
          player: 0,
          to_hand: 1,
          to_bottom: 1,
          to_exile_may_play: 1,
          items: [
            { id: 1, label: "A" },
            { id: 2, label: "B" },
            { id: 3, label: "C" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-distribute-modal")).toExist(),
    Scene.expect(Scene.testId("pending-distribute-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-distribute-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toExist(),
  );
});

test("select_from_top uses a center modal with Take and Bottom lanes", () => {
  overlayScene(
    overlayModel(
      { ...initialBoardModel(), promptDraft: { kind: "card-pick", picked: [1], filter: "" } },
      gameState({
        pending_choice: {
          kind: "select_from_top",
          up_to: 2,
          player: 0,
          items: [
            { id: 1, label: "Sol Ring" },
            { id: 2, label: "Forest" },
            { id: 3, label: "Island" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-select-top-modal")).toExist(),
    Scene.expect(Scene.testId("pending-select-top-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-select-top-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-select-top-take-label")).toHaveText("Take (1 / 2)"),
    Scene.expect(Scene.testId("prompt-select-top-rest-label")).toHaveText("Bottom of library"),
    Scene.expect(Scene.testId("prompt-card-1")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toExist(),
  );
});

test("pay_cost aim shows docked Pay and decline", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "pay_cost",
          can_pay: true,
          cost: { colored: [0, 0, 0, 1, 0], generic: 2 },
          label: testMessageRef("Create a Fungus Beast"),
          player: 0,
          source: 1,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-pay-cost-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-pay"]')).toHaveText(
      "Pay {2}{R}",
    ),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-decline"]')).toHaveText(
      "Don't pay",
    ),
    Scene.expect(Scene.selector('[data-testid="pending-pay-cost-aim"] [data-testid="prompt-pay"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-pay-cost-aim"] [data-testid="prompt-decline"]')).toBeAbsent(),
  );
});

test("may_yes_no puts Yes/No in the primary bar with a bottom coach", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "may_yes_no",
          label: testMessageRef("Scry?"),
          player: 0,
          source: 3,
        },
      }),
    ),
    Scene.expect(Scene.testId("priority-context-bar")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-yes"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-no"]')).toExist(),
    Scene.expect(Scene.testId("pending-yes-no-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-yes-no-aim"] [data-testid="prompt-yes"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-yes-no-aim"] [data-testid="prompt-no"]')).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("may_yes_no hides idle priority actions", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        active_player: 1,
        pending_choice: {
          kind: "may_yes_no",
          label: testMessageRef("Draw a card?"),
          player: 0,
          source: 1,
        },
      }),
    ),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("board-end-turn")).toBeAbsent(),
    Scene.expect(Scene.testId("board-pass")).toBeAbsent(),
    Scene.expect(Scene.testId("board-turn-yield")).toBeAbsent(),
  );
});

test("may_yes_no may-reveal land label surfaces from MessageRef", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "may_yes_no",
          label: {
            key: "effect.choice_may_reveal_land_from_hand",
            params: [],
            children: [],
          },
          player: 0,
          source: 7,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-yes-no-aim")).toExist(),
    Scene.expect(Scene.testId("pending-yes-no-aim")).toContainText(
      "You may reveal a matching land card from your hand",
    ),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-yes"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-no"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-yes-no-aim"] [data-testid="prompt-yes"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-yes-no-aim"] [data-testid="prompt-no"]')).toBeAbsent(),
  );
});

test("pay_cost with discard shows count and disabled Pay until picked", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [card(11, { zone: ZONE.Hand, name: "Fodder" })],
        pending_choice: {
          kind: "pay_cost",
          can_pay: true,
          cost: { colored: [], generic: 1 },
          discard_count: 1,
          discard_choices: [11],
          label: testMessageRef("Pay 1 and discard a card"),
          player: 0,
          source: 1,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-pay-cost-aim")).toExist(),
    Scene.expect(Scene.testId("pending-pay-discard-count")).toHaveText("0 / 1 selected"),
    Scene.expect(Scene.testId("prompt-pay")).toBeDisabled(),
    Scene.expect(Scene.testId("prompt-decline")).toHaveText("Don't pay"),
  );
});

test("choose_color uses a center modal and hides primary actions", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "choose_color",
          player: 0,
          source: 1,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-color-modal")).toExist(),
    Scene.expect(Scene.testId("pending-color-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-color-0")).toExist(),
    Scene.expect(Scene.testId("prompt-color-pip-4")).toExist(),
  );
});

test("choose_mana_color uses a center modal and hides primary actions", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "choose_mana_color",
          amount: 1,
          player: 0,
          source: 2,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-color-modal")).toExist(),
    Scene.expect(Scene.testId("pending-color-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-color-1")).toExist(),
  );
});

test("choose_mode aim shows docked mode buttons instead of center modal", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "choose_mode",
          labels: [testMessageRef("Draw a card"), testMessageRef("Create a token")],
          player: 0,
          source: 1,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-mode-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-mode-0"]')).toHaveText(
      "Draw a card",
    ),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-mode-1"]')).toHaveText(
      "Create a token",
    ),
    Scene.expect(Scene.selector('[data-testid="pending-mode-aim"] [data-testid="prompt-mode-0"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="pending-mode-aim"] [data-testid="prompt-mode-1"]')).toBeAbsent(),
  );
});

test("non-decider sees waiting banner instead of pending-choice controls", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        viewer: 1,
        pending_choice: {
          kind: "may_yes_no",
          label: testMessageRef("Scry?"),
          player: 0,
          source: 3,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-yes-no-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toHaveText("Waiting for Alice…"),
  );
});

test("library search uses a center modal and hides primary actions", () => {
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "card-pick", picked: [], filter: "sol" },
      },
      gameState({
        pending_choice: {
          kind: "search_library",
          player: 0,
          items: [
            { id: 1, label: "Sol Ring" },
            { id: 2, label: "Forest" },
            { id: 3, label: "Forest" },
            { id: 4, label: "Sol Talisman" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-library-modal")).toExist(),
    Scene.expect(Scene.testId("pending-library-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("pick-title")).toHaveText("Search your library"),
    Scene.expect(Scene.testId("pick-card-filter")).toExist(),
    Scene.expect(Scene.placeholder("Filter by name…")).toExist(),
    Scene.expect(Scene.testId("pick-card-scroll")).toExist(),
    Scene.expect(Scene.testId("prompt-card-1")).toExist(),
    Scene.expect(Scene.testId("prompt-card-4")).toExist(),
    Scene.expect(Scene.testId("prompt-card-2")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-decline")).toHaveText("Fail to find"),
  );
});

test("library search hides idle priority actions", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "search_library",
          player: 0,
          items: [
            { id: 1, label: "Sol Ring" },
            { id: 2, label: "Forest" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("board-end-turn")).toBeAbsent(),
    Scene.expect(Scene.testId("board-pass")).toBeAbsent(),
    Scene.expect(Scene.testId("board-stack-yield")).toBeAbsent(),
  );
});

test("off-board card pick uses a center card-pick modal", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "proliferate",
          player: 0,
          source: 1,
          items: [
            { id: 10, label: "Bear" },
            { id: 11, label: "Elf" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-card-pick-modal")).toExist(),
    Scene.expect(Scene.testId("pending-card-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-target-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("pick-title")).toHaveText("Proliferate — choose any number"),
    Scene.expect(Scene.testId("pick-card-scroll")).toExist(),
    Scene.expect(Scene.testId("prompt-card-10")).toExist(),
    Scene.expect(Scene.testId("prompt-card-11")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toHaveText("Proliferate"),
  );
});

test("choose_target player buttons use a center player-pick modal", () => {
  const yard = card(99, {
    name: "Yard Card",
    owner: 1,
    controller: 1,
    zone: ZONE.Graveyard,
  });
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        players: [player(0), player(1), player(2)],
        objects: [yard],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Choose a target"),
          min: 1,
          max: 1,
          player: 0,
          source: 1,
          items: [
            { id: 0, label: "Alice", player: 0 },
            { id: 1, label: "Bob", player: 1 },
            { id: 99, label: "Yard Card" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-player-pick-modal")).toExist(),
    Scene.expect(Scene.testId("pending-player-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-target-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-player-0")).toExist(),
    Scene.expect(Scene.testId("prompt-player-1")).toExist(),
  );
});

test("choose_card_name uses a center modal with a Card name placeholder field", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "choose_card_name",
          player: 0,
          source: 9,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-card-name-modal")).toExist(),
    Scene.expect(Scene.testId("pending-card-name-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.placeholder("Card name")).toExist(),
    Scene.expect(Scene.testId("prompt-name-input")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
  );
});

test("choose_card_name center modal lists matching catalog suggestions", () => {
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "string", value: "Sol" },
        cardNameSuggestions: { query: "Sol", names: ["Sol Ring", "Sol Talisman"] },
      },
      gameState({
        pending_choice: {
          kind: "choose_card_name",
          player: 0,
          source: 9,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-card-name-modal")).toExist(),
    Scene.expect(Scene.testId("pending-card-name-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-name-suggestions")).toExist(),
    Scene.expect(Scene.testId("prompt-name-suggestion-0")).toHaveText("Sol Ring"),
    Scene.expect(Scene.testId("prompt-name-suggestion-1")).toHaveText("Sol Talisman"),
  );
});

test("choose_creature_type uses a center modal and filters options by name", () => {
  overlayScene(
    overlayModel(
      { ...initialBoardModel(), promptOptionFilter: "cler" },
      gameState({
        pending_choice: {
          kind: "choose_creature_type",
          options: ["Wizard", "Cleric", "Elf"],
          player: 0,
          source: 1,
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-creature-type-modal")).toExist(),
    Scene.expect(Scene.testId("pending-creature-type-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-type-filter")).toExist(),
    Scene.expect(Scene.placeholder("Filter types…")).toExist(),
    Scene.expect(Scene.testId("prompt-type-scroll")).toExist(),
    Scene.expect(Scene.testId("prompt-string-1")).toExist(),
    Scene.expect(Scene.testId("prompt-string-0")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-string-2")).toBeAbsent(),
  );
});

test("trample combat damage assign shows overflow to defender and enables Assign under power", () => {
  const attacker = card(9, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 5, toughness: 5 },
    power: 5,
    toughness: 5,
    name: "Trampler",
    keywords: ["trample"],
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "damage", amounts: { 20: 2, 21: 0 } },
      },
      gameState({
        objects: [attacker],
        pending_choice: {
          kind: "assign_combat_damage",
          player: 0,
          source: 9,
          items: [
            { id: 20, label: "Bear" },
            { id: 21, label: "Elf" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-damage-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-damage-assigned")).toHaveText("assigned 2 / 5"),
    Scene.expect(Scene.testId("prompt-damage-overflow")).toHaveText("to defender: 3"),
    Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
    Scene.expect(Scene.testId("prompt-damage-20-value")).toHaveText("2"),
    Scene.expect(Scene.testId("prompt-damage-20-inc")).toExist(),
    Scene.expect(Scene.testId("prompt-damage-20-dec")).toExist(),
    Scene.expect(Scene.selector('input[type="number"]')).not.toExist(),
  );
});

test("divide_spell_damage on-board aim shows coach when targets are battlefield", () => {
  const bear = card(21, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
    name: "Bear",
  });
  const elf = card(22, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 1, toughness: 1 },
    power: 1,
    toughness: 1,
    name: "Elf",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "divide", amounts: { 0: 2, 1: 1 } },
      },
      gameState({
        objects: [bear, elf],
        pending_choice: {
          kind: "divide_spell_damage",
          player: 0,
          spell: 99,
          total: 3,
          items: [
            { id: 21, label: "Bear" },
            { id: 22, label: "Elf" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-divide-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-damage-assigned")).toHaveText("assigned 3 / 3"),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-divide-aim"] [data-testid="prompt-submit"]')).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-damage-0-inc")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-damage-1-inc")).toBeAbsent(),
  );
});

test("on-board assign_combat_damage hides steppers when blockers are battlefield", () => {
  const attacker = card(9, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 4, toughness: 4 },
    power: 4,
    toughness: 4,
    name: "Atk",
  });
  const bear = card(20, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
    name: "Bear",
  });
  const elf = card(21, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 1, toughness: 1 },
    power: 1,
    toughness: 1,
    name: "Elf",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "damage", amounts: { 20: 3, 21: 1 } },
      },
      gameState({
        objects: [attacker, bear, elf],
        pending_choice: {
          kind: "assign_combat_damage",
          player: 0,
          source: 9,
          items: [
            { id: 20, label: "Bear" },
            { id: 21, label: "Elf" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-damage-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-damage-assigned")).toHaveText("assigned 4 / 4"),
    Scene.expect(Scene.testId("prompt-damage-20-inc")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-damage-21-inc")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-damage-aim"] [data-testid="prompt-submit"]')).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("divide_counters on-board aim shows coach when targets are battlefield", () => {
  const wolf = card(12, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 2, toughness: 2 },
    power: 2,
    toughness: 2,
    name: "Wolf",
  });
  const cat = card(13, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 1, toughness: 1 },
    power: 1,
    toughness: 1,
    name: "Cat",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "damage", amounts: { 12: 1, 13: 1 } },
      },
      gameState({
        objects: [wolf, cat],
        pending_choice: {
          kind: "divide_counters",
          player: 0,
          spell: 77,
          total: 2,
          items: [
            { id: 12, label: "Wolf" },
            { id: 13, label: "Cat" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-divide-counters-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-damage-assigned")).toHaveText("assigned 2 / 2"),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toExist(),
    Scene.expect(
      Scene.selector('[data-testid="pending-divide-counters-aim"] [data-testid="prompt-submit"]'),
    ).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("sacrifice pick prompt renders as a board surface", () => {
  const sacrificeAction = action(14, {
    kind: "activate",
    label: testMessageRef("Village Rites"),
    sacrifice_choices: [55],
    object: 14,
    section: "hand",
  });
  const sacrificeBody = card(55, {
    zone: ZONE.Battlefield,
    kind: { kind: "creature", power: 1, toughness: 1 },
    power: 1,
    toughness: 1,
    name: "Token",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        sacrificePick: {
          action: sacrificeAction,
          card: card(14, { name: "Village Rites", kind: { kind: "instant" } }),
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
      },
      gameState({ objects: [sacrificeBody] }),
    ),
    Scene.expect(Scene.testId("sacrifice-cost-aim")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-cancel"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="sacrifice-cost-aim"] [data-testid="prompt-cancel"]')).toBeAbsent(),
    Scene.expect(Scene.testId("sacrifice-pick")).toBeAbsent(),
    Scene.expect(Scene.testId("sacrifice-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("sacrifice-pick-55")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("off-board sacrifice cost uses a center sacrifice-pick modal", () => {
  const sacrificeAction = action(14, {
    kind: "activate",
    label: testMessageRef("Village Rites"),
    sacrifice_choices: [55],
    object: 14,
    section: "hand",
  });
  const offBoard = card(55, {
    zone: ZONE.Hand,
    kind: { kind: "creature", power: 1, toughness: 1 },
    name: "Fodder",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        sacrificePick: {
          action: sacrificeAction,
          card: card(14, { name: "Village Rites", kind: { kind: "instant" } }),
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
      },
      gameState({ objects: [offBoard] }),
    ),
    Scene.expect(Scene.testId("sacrifice-pick-modal")).toExist(),
    Scene.expect(Scene.testId("sacrifice-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("sacrifice-pick")).toBeAbsent(),
    Scene.expect(Scene.testId("sacrifice-cost-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("sacrifice-pick-55")).toHaveText("Fodder"),
  );
});

test("discard cost aim shows coach when choices are in hand", () => {
  const caster = card(10, {
    name: "Caster",
    zone: ZONE.Hand,
    kind: { kind: "instant" },
  });
  const fodder = card(11, {
    name: "Island",
    zone: ZONE.Hand,
    kind: { kind: "land", colors: [0, 1, 0, 0, 0] },
  });
  const castAction = action(50, {
    kind: "cast",
    label: testMessageRef("Cast"),
    discard_choices: [11],
    object: 10,
    section: "hand",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        discardPick: {
          action: castAction,
          card: caster,
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
      },
      gameState({ objects: [caster, fodder] }),
    ),
    Scene.expect(Scene.testId("discard-cost-aim")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-cancel"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="discard-cost-aim"] [data-testid="prompt-submit"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="discard-cost-aim"] [data-testid="prompt-cancel"]')).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
    Scene.expect(Scene.testId("discard-cost-count")).toHaveText("0 / 1 selected"),
    Scene.expect(Scene.testId("discard-pick")).toBeAbsent(),
    Scene.expect(Scene.testId("discard-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("selected discard-cost hand card paints Llanowar selected chrome", () => {
  const caster = card(10, {
    name: "Caster",
    zone: ZONE.Hand,
    kind: { kind: "instant" },
  });
  const fodder = card(11, {
    name: "Island",
    zone: ZONE.Hand,
    kind: { kind: "land", colors: [0, 1, 0, 0, 0] },
  });
  const castAction = action(50, {
    kind: "cast",
    label: testMessageRef("Cast"),
    discard_choices: [11],
    object: 10,
    section: "hand",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        discardPick: {
          action: castAction,
          card: caster,
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: { ...emptyCostPicks(), discard_cost: [11] },
        },
      },
      gameState({ objects: [caster, fodder] }),
    ),
    Scene.expect(Scene.testId("hand-card-11")).toExist(),
    Scene.expect(Scene.testId("hand-card-face-11")).toExist(),
    Scene.tap((sim) => {
      const face = findTestId(sim.html, "hand-card-face-11");
      const tile = findTestId(sim.html, "hand-tile-11");
      expect(dataAttr(tile, "selected")).toBe("true");
      expect(dataAttr(tile, "selectable")).toBe("true");
      expect(className(face)).toContain("group-data-[selected=true]/hand-tile:ring-llanowar");
    }),
  );
});

test("discard cost aim enables confirm when one card selected", () => {
  const caster = card(10, {
    name: "Caster",
    zone: ZONE.Hand,
    kind: { kind: "instant" },
  });
  const fodder = card(11, {
    name: "Island",
    zone: ZONE.Hand,
    kind: { kind: "land", colors: [0, 1, 0, 0, 0] },
  });
  const castAction = action(50, {
    kind: "cast",
    label: testMessageRef("Cast"),
    discard_choices: [11],
    object: 10,
    section: "hand",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        discardPick: {
          action: castAction,
          card: caster,
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: { ...emptyCostPicks(), discard_cost: [11] },
        },
      },
      gameState({ objects: [caster, fodder] }),
    ),
    Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
    Scene.expect(Scene.testId("discard-cost-count")).toHaveText("1 / 1 selected"),
  );
});

test("off-board discard cost uses a center discard-pick modal", () => {
  const caster = card(10, {
    name: "Caster",
    zone: ZONE.Hand,
    kind: { kind: "instant" },
  });
  const castAction = action(50, {
    kind: "cast",
    label: testMessageRef("Cast"),
    discard_choices: [11],
    object: 10,
    section: "hand",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        discardPick: {
          action: castAction,
          card: caster,
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
      },
      gameState({ objects: [caster] }),
    ),
    Scene.expect(Scene.testId("discard-pick-modal")).toExist(),
    Scene.expect(Scene.testId("discard-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("discard-pick")).toBeAbsent(),
    Scene.expect(Scene.testId("discard-cost-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("discard-pick-11")).toHaveText("#11"),
  );
});

test("pending discard aim shows coach when cards are in hand", () => {
  const a = card(11, {
    name: "A",
    zone: ZONE.Hand,
    kind: { kind: "instant" },
  });
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [a],
        pending_choice: {
          kind: "discard",
          player: 0,
          count: 1,
          items: [{ id: 11, label: "A" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-discard-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("pending discard aim shows Confirm and count for single-card discard", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [card(11, { zone: ZONE.Hand, name: "A" })],
        pending_choice: {
          kind: "discard",
          player: 0,
          count: 1,
          items: [{ id: 11, label: "A" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-discard-aim")).toExist(),
    Scene.expect(Scene.testId("pending-discard-count")).toHaveText("0 / 1 selected"),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
    Scene.expect(Scene.testId("prompt-submit")).toHaveText("Discard"),
  );
});

test("selected pending discard hand card paints Llanowar selected chrome", () => {
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "card-pick", picked: [11], filter: "" },
      },
      gameState({
        objects: [
          card(11, {
            name: "Island",
            zone: ZONE.Hand,
            kind: { kind: "land", colors: [0, 1, 0, 0, 0] },
          }),
        ],
        pending_choice: {
          kind: "discard",
          player: 0,
          count: 1,
          items: [{ id: 11, label: "Island" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("hand-card-face-11")).toExist(),
    Scene.tap((sim) => {
      const face = findTestId(sim.html, "hand-card-face-11");
      const tile = findTestId(sim.html, "hand-tile-11");
      expect(dataAttr(tile, "selected")).toBe("true");
      expect(className(face)).toContain("group-data-[selected=true]/hand-tile:ring-llanowar");
    }),
  );
});

test("may_discard aim shows Continue enabled with zero picks", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [card(11, { zone: ZONE.Hand, name: "A" })],
        pending_choice: {
          kind: "may_discard",
          player: 0,
          source: 1,
          items: [{ id: 11, label: "A" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-discard-aim")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
    Scene.expect(Scene.testId("prompt-submit")).toHaveText("Continue"),
  );
});

test("selected may_discard hand card paints Llanowar selected chrome", () => {
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "card-pick", picked: [11], filter: "" },
      },
      gameState({
        objects: [
          card(11, {
            name: "Island",
            zone: ZONE.Hand,
            kind: { kind: "land", colors: [0, 1, 0, 0, 0] },
          }),
        ],
        pending_choice: {
          kind: "may_discard",
          player: 0,
          source: 1,
          items: [{ id: 11, label: "Island" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("hand-card-face-11")).toExist(),
    Scene.tap((sim) => {
      const face = findTestId(sim.html, "hand-card-face-11");
      const tile = findTestId(sim.html, "hand-tile-11");
      expect(dataAttr(tile, "selected")).toBe("true");
      expect(className(face)).toContain("group-data-[selected=true]/hand-tile:ring-llanowar");
    }),
  );
});

test("pending exile aim shows coach when choose_exiled cards share a pile", () => {
  const exiled = card(30, {
    name: "Exiled",
    zone: ZONE.Exile,
    kind: { kind: "instant" },
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        pileExpand: { zone: ZONE.Exile, owner: 0 },
      },
      gameState({
        objects: [exiled],
        pending_choice: {
          kind: "choose_exiled_with_card",
          player: 0,
          source: 1,
          items: [{ id: 30, label: "Exiled" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-exile-aim")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-decline"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-exile-aim"] [data-testid="prompt-decline"]')).toBeAbsent(),
    Scene.expect(Scene.testId("pile-card-30")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("pending gy aim shows coach for cumulative upkeep when cards share a pile", () => {
  const gy = card(8, {
    name: "Fodder",
    zone: ZONE.Graveyard,
    kind: { kind: "creature", power: 1, toughness: 1 },
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        pileExpand: { zone: ZONE.Graveyard, owner: 0 },
      },
      gameState({
        objects: [gy],
        pending_choice: {
          kind: "pay_cumulative_upkeep_or_sacrifice",
          player: 0,
          source: 1,
          count: 1,
          items: [{ id: 8, label: "Fodder" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-gy-aim")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-decline"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-gy-aim"] [data-testid="prompt-decline"]')).toBeAbsent(),
    Scene.expect(Scene.testId("pile-card-8")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("pending gy aim shows coach when exile_from_graveyard cards share a pile", () => {
  const gy = card(8, {
    name: "Fodder",
    zone: ZONE.Graveyard,
    kind: { kind: "creature", power: 1, toughness: 1 },
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        pileExpand: { zone: ZONE.Graveyard, owner: 0 },
      },
      gameState({
        objects: [gy],
        pending_choice: {
          kind: "exile_from_graveyard",
          player: 0,
          source: 1,
          items: [{ id: 8, label: "Fodder" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-gy-aim")).toExist(),
    Scene.expect(Scene.testId("pile-card-8")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("pending gy aim shows Decline only for optional may_return_from_graveyard", () => {
  const gy = card(8, {
    name: "Reanimate me",
    zone: ZONE.Graveyard,
    kind: { kind: "creature", power: 1, toughness: 1 },
  });

  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        pileExpand: { zone: ZONE.Graveyard, owner: 0 },
      },
      gameState({
        objects: [gy],
        pending_choice: {
          kind: "may_return_from_graveyard",
          player: 0,
          source: 1,
          mandatory: false,
          items: [{ id: 8, label: "Reanimate me" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-gy-aim")).toExist(),
    Scene.expect(Scene.testId("prompt-decline")).toExist(),
  );

  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        pileExpand: { zone: ZONE.Graveyard, owner: 0 },
      },
      gameState({
        objects: [gy],
        pending_choice: {
          kind: "may_return_from_graveyard",
          player: 0,
          source: 1,
          mandatory: true,
          items: [{ id: 8, label: "Reanimate me" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-gy-aim")).toExist(),
    Scene.expect(Scene.testId("prompt-decline")).toBeAbsent(),
  );
});

test("pending gy aim shows Exile and Don't exile for may_exile_discarded_to_play", () => {
  const bolt = card(8, {
    name: "Lightning Bolt",
    zone: ZONE.Graveyard,
    kind: { kind: "instant" },
  });

  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        pileExpand: { zone: ZONE.Graveyard, owner: 0 },
      },
      gameState({
        objects: [bolt],
        pending_choice:
          fromProtoWire<VisibleState>({
            pendingChoice: {
              choice: {
                case: "mayExileDiscardedToPlay",
                value: {
                  player: 0,
                  source: 1,
                  items: [{ id: 8, label: "Lightning Bolt" }],
                },
              },
            },
          }).pending_choice ?? null,
      }),
    ),
    Scene.expect(Scene.testId("pending-gy-aim")).toExist(),
    Scene.expect(Scene.testId("pile-card-8")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toHaveText("Exile"),
    Scene.expect(Scene.testId("prompt-decline")).toHaveText("Don't exile"),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("pending revealed aim shows coach for opponent_chooses_revealed_to_graveyard", () => {
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        pending_choice: {
          kind: "opponent_chooses_revealed_to_graveyard",
          player: 0,
          source: 1,
          items: [
            { id: 21, label: "Island" },
            { id: 22, label: "Swamp" },
          ],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-revealed-aim")).toExist(),
    Scene.expect(Scene.testId("prompt-card-21")).toExist(),
    Scene.expect(Scene.testId("prompt-card-22")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-decline"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-revealed-aim"] [data-testid="prompt-decline"]')).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("pending gy aim shows coach for choose_target when cards share a pile", () => {
  const gy = card(8, {
    name: "Reanimate me",
    zone: ZONE.Graveyard,
    kind: { kind: "creature", power: 1, toughness: 1 },
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        pileExpand: { zone: ZONE.Graveyard, owner: 0 },
      },
      gameState({
        objects: [gy],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Target creature card in a graveyard"),
          min: 1,
          max: 1,
          player: 0,
          source: 1,
          items: [{ id: 8, label: "Reanimate me" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-gy-aim")).toExist(),
    Scene.expect(Scene.testId("pile-card-8")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("gy exile cost aim shows coach when choices share a graveyard", () => {
  const caster = card(10, {
    name: "Caster",
    zone: ZONE.Hand,
    kind: { kind: "instant" },
  });
  const gy = card(8, {
    name: "Fodder",
    zone: ZONE.Graveyard,
    kind: { kind: "creature", power: 1, toughness: 1 },
  });
  const castAction = action(50, {
    kind: "cast",
    label: testMessageRef("Cast"),
    graveyard_exile_choices: [8],
    graveyard_exile_min: 1,
    graveyard_exile_max: 1,
    object: 10,
    section: "hand",
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        gyExilePick: {
          action: castAction,
          card: caster,
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
        pileExpand: { zone: ZONE.Graveyard, owner: 0 },
      },
      gameState({ objects: [caster, gy] }),
    ),
    Scene.expect(Scene.testId("gy-exile-cost-aim")).toExist(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-cancel"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="gy-exile-cost-aim"] [data-testid="prompt-cancel"]')).toBeAbsent(),
    Scene.expect(Scene.testId("pile-overlay")).toExist(),
    Scene.expect(Scene.testId("pile-card-8")).toExist(),
    Scene.expect(Scene.testId("gy-exile-pick")).toBeAbsent(),
    Scene.expect(Scene.testId("gy-exile-pick-aim")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("pending face-down cast aim shows coach when creatures are in hand", () => {
  const bear = card(22, {
    name: "Bear",
    zone: ZONE.Hand,
    kind: { kind: "creature", power: 2, toughness: 2 },
  });
  overlayScene(
    overlayModel(
      initialBoardModel(),
      gameState({
        objects: [bear],
        pending_choice: {
          kind: "cast_creature_face_down",
          player: 0,
          items: [{ id: 22, label: "Bear" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-hand-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("pending put-from-hand aim shows coach when cards are in hand", () => {
  const forest = card(20, {
    name: "Forest",
    zone: ZONE.Hand,
    kind: { kind: "land", colors: [0, 0, 0, 0, 1] },
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "card-pick", picked: [20], filter: "" },
      },
      gameState({
        objects: [forest],
        pending_choice: {
          kind: "put_land_from_hand",
          player: 0,
          items: [{ id: 20, label: "Forest" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-hand-aim")).toExist(),
    Scene.expect(Scene.testId("pending-hand-count")).toHaveText("1 / 1 selected"),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toBeEnabled(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-decline"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-hand-aim"] [data-testid="prompt-submit"]')).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("put_creature_from_hand uses select then Confirm like discard", () => {
  const angel = card(21, {
    name: "Angel",
    zone: ZONE.Hand,
    kind: { kind: "creature", power: 4, toughness: 4 },
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "card-pick", picked: [21], filter: "" },
      },
      gameState({
        objects: [angel],
        pending_choice: {
          kind: "put_creature_from_hand",
          player: 0,
          items: [{ id: 21, label: "Angel" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("pending-hand-aim")).toExist(),
    Scene.expect(Scene.testId("pending-hand-count")).toHaveText("1 / 1 selected"),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-submit"]')).toBeEnabled(),
    Scene.expect(Scene.selector('[data-testid="priority-context-bar"] [data-testid="prompt-decline"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="pending-hand-aim"] [data-testid="prompt-submit"]')).toBeAbsent(),
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
  );
});

test("selected put_creature_from_hand card paints Llanowar selected chrome like discard", () => {
  const angel = card(21, {
    name: "Angel",
    zone: ZONE.Hand,
    kind: { kind: "creature", power: 4, toughness: 4 },
  });
  const bolt = card(22, {
    name: "Lightning Bolt",
    zone: ZONE.Hand,
    kind: { kind: "instant" },
  });
  overlayScene(
    overlayModel(
      {
        ...initialBoardModel(),
        promptDraft: { kind: "card-pick", picked: [21], filter: "" },
      },
      gameState({
        objects: [angel, bolt],
        actions: [
          action(50, {
            kind: "cast",
            label: testMessageRef("Cast"),
            object: 22,
            section: "hand",
          }),
        ],
        pending_choice: {
          kind: "put_creature_from_hand",
          player: 0,
          items: [{ id: 21, label: "Angel" }],
        },
      }),
    ),
    Scene.expect(Scene.testId("hand-card-face-21")).toExist(),
    Scene.expect(Scene.testId("hand-card-face-22")).toExist(),
    Scene.tap((sim) => {
      const selectedFace = findTestId(sim.html, "hand-card-face-21");
      const selectedTile = findTestId(sim.html, "hand-tile-21");
      expect(dataAttr(selectedTile, "selected")).toBe("true");
      expect(dataAttr(selectedTile, "selectable")).toBe("true");
      expect(className(selectedFace)).toContain("group-data-[selected=true]/hand-tile:ring-llanowar");

      const invalidFace = findTestId(sim.html, "hand-card-face-22");
      const invalidTile = findTestId(sim.html, "hand-tile-22");
      expect(dataAttr(invalidTile, "selected")).toBeNull();
      expect(dataAttr(invalidTile, "selectable")).toBeNull();
      expect(className(invalidFace)).not.toContain("ring-island-blue");
      expect(className(invalidFace)).not.toContain("group-data-[selected=true]/hand-tile:ring-llanowar");
    }),
  );
});

test("full board view mounts the bitmap layer", () => {
  liveBoardScene(
    fullBoardModel(initialBoardModel(), gameState()),
    Scene.expect(Scene.testId("board-bitmap-layer")).toExist(),
  );
});

test("full board view mounts the camera gesture host", () => {
  liveBoardScene(
    fullBoardModel(initialBoardModel(), gameState()),
    Scene.expect(Scene.testId("board-camera-gesture-mount")).toExist(),
  );
});

test("board root disables native text selection", () => {
  liveBoardScene(
    fullBoardModel(initialBoardModel(), gameState()),
    Scene.expect(Scene.testId("board-mount")).toHaveClass("select-none"),
  );
});

test("full board view mounts the flight layer above the hand bar", () => {
  liveBoardScene(
    fullBoardModel(initialBoardModel(), gameState()),
    Scene.expect(Scene.testId("board-flight-layer")).toExist(),
    // z-30 sits above the hand bar (z-20) and below prompts (z-40).
    Scene.expect(Scene.testId("board-flight-layer")).toHaveClass("z-30"),
  );
});

test("tiny board HUD close controls keep coarse pointer hit targets", () => {
  const stack = Array.from({ length: 6 }, (_, index) => ({
    controller: index % 2,
    kind: "spell" as const,
    label: testMessageRef(`Spell ${index}`),
    source: 100 + index,
  }));
  overlayScene(
    overlayModel({ ...initialBoardModel(), legendOpen: true, stackExpand: true }, gameState({ stack })),
    resolveBoardCardArtMounts(0),
    Scene.tap((sim) => {
      expect(className(findAttr(sim.html, "aria-label", "Dismiss hint"))).toContain("hit-quiet");
      expect(className(findAttr(sim.html, "aria-label", "Close legend"))).toContain("hit-quiet");
      expect(className(findTestId(sim.html, "stack-collapse"))).toContain("hit-quiet");
    }),
  );
});

test("full board view renders battlefield mana tray when a pool has mana", () => {
  liveBoardScene(
    fullBoardModel(
      initialBoardModel(),
      gameState({
        players: [
          player(0, {
            mana_pool: { any: 0, colored: [1, 0, 0, 0, 0], colorless: 0, either: [], of_colors: [] },
          }),
          player(1),
        ],
      }),
    ),
    Scene.expect(Scene.testId("mana-tray")).toExist(),
    Scene.expect(Scene.testId("mana-tray")).toHaveClass("pointer-events-none"),
  );
});

test("mana tray precedes bitmap layer in board-mount composition (under permanents)", () => {
  liveBoardScene(
    fullBoardModel(
      initialBoardModel(),
      gameState({
        players: [
          player(0, {
            mana_pool: { any: 0, colored: [1, 0, 0, 0, 0], colorless: 0, either: [], of_colors: [] },
          }),
          player(1),
        ],
      }),
    ),
    Scene.tap((sim) => {
      const ids = collectTestIds(sim.html);
      const tray = ids.indexOf("mana-tray");
      const bitmap = ids.indexOf("board-bitmap-layer");
      const flight = ids.indexOf("board-flight-layer");
      expect(tray).toBeGreaterThan(-1);
      expect(bitmap).toBeGreaterThan(tray);
      expect(flight).toBeGreaterThan(bitmap);
    }),
    Scene.expect(Scene.testId("mana-tray")).toExist(),
    Scene.expect(Scene.testId("board-bitmap-layer")).toExist(),
    Scene.expect(Scene.testId("board-flight-layer")).toHaveClass("z-30"),
  );
});
