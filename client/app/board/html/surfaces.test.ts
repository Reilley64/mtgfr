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
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import type { GameFoldState, LogLine } from "../../game/fold";
import { emptyCostPicks, type ModalCast, type XPromptState } from "../action/execution";
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
    label: `Action ${id}`,
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
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
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
      tokenCreators: new Map(),
      landPlayFrom: new Map(),
      zonePileEntrances: new Map(),
      stackEntrances: new Map(),
      priorStackObjectIds: new Set(),
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false },
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
        label: "Cast Shock",
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
    label: "Cast Lightning Bolt",
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
    stack: [{ controller: 1, kind: "ability", label: "Ward 2", source: 99 }],
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
      gameState({ stack: [{ controller: 1, kind: "spell", label: "Bolt", source: 77 }], yielded: true }),
    ),
    Scene.expect(Scene.testId("board-stack-yield-armed")).toExist(),
  );
});

test("active player sees the end-turn affordance", () => {
  overlayScene(overlayModel(), Scene.expect(Scene.testId("board-end-turn")).toExist());
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
  overlayScene(
    overlayModel(stagedBoard(), gameState({ objects: [target] })),
    Scene.expect(Scene.testId("board-cancel-target")).toExist(),
    Scene.expect(Scene.testId("board-staged-hint")).toContainText("Cast Shock"),
  );
});

test("board reject surface renders when local reject text is set", () => {
  overlayScene(
    overlayModel({ ...initialBoardModel(), reject: "Choose a legal target" }),
    Scene.expect(Scene.testId("board-reject")).toContainText("Choose a legal target"),
  );
});

test("hand surfaces render cost pips and a drag ghost", () => {
  const handCard = card(42, {
    name: "Lightning Bolt",
    print: "bolt-print",
    mana_cost: cost({ generic: 1 }),
  });
  const castAction = action(7, {
    label: "Cast Lightning Bolt",
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
    resolveBoardCardArtMounts(2),
    Scene.expect(Scene.testId("hand-cost-pips")).toExist(),
    Scene.expect(Scene.testId("hand-drag-ghost")).toExist(),
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
        set: "soc",
        subtypes: [],
        summary: "Mana rock",
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
          set: "soc",
          subtypes: ["Bear"],
          summary: "Bear",
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

test("x prompt shows stepper controls and a live cost preview", () => {
  const xPrompt: XPromptState = {
    action: action(12, { label: "Comet Storm", has_x: true, max_x: 3, min_x: 0 }),
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
    Scene.expect(Scene.testId("x-prompt")).toExist(),
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

test("modal mode picker renders before modes are chosen", () => {
  const modalCast: ModalCast = {
    action: action(13, {
      label: "Cryptic Command",
      modal: {
        choose: 2,
        choose_max: 2,
        modes: [
          { label: "Counter target spell", needs_target: false, targets: [] },
          { label: "Draw a card", needs_target: false, targets: [] },
        ],
      },
    }),
    modes: [
      { label: "Counter target spell", needs_target: false, targets: [] },
      { label: "Draw a card", needs_target: false, targets: [] },
    ],
    picks: emptyCostPicks(),
    chosen: null,
    answers: [],
    modeDraft: [],
  };
  overlayScene(
    overlayModel({ ...initialBoardModel(), modalCast }),
    Scene.expect(Scene.testId("modal-mode-picker")).toExist(),
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
    Scene.expect(Scene.testId("pending-choice")).toExist(),
    Scene.expect(Scene.testId("prompt-number-value")).toHaveText("0"),
    Scene.expect(Scene.testId("prompt-number-min")).toExist(),
    Scene.expect(Scene.testId("prompt-number-max")).toExist(),
    Scene.expect(Scene.testId("prompt-number-0")).not.toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toExist(),
  );
});

test("choose_card_name prompt shows a Card name placeholder field", () => {
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
    Scene.expect(Scene.testId("pending-choice")).toExist(),
    Scene.expect(Scene.placeholder("Card name")).toExist(),
    Scene.expect(Scene.testId("prompt-name-input")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
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
    Scene.expect(Scene.testId("pending-choice")).toExist(),
    Scene.expect(Scene.testId("prompt-damage-assigned")).toHaveText("assigned 2 / 5"),
    Scene.expect(Scene.testId("prompt-damage-overflow")).toHaveText("to defender: 3"),
    Scene.expect(Scene.testId("prompt-submit")).not.toBeDisabled(),
    Scene.expect(Scene.testId("prompt-damage-20-value")).toHaveText("2"),
    Scene.expect(Scene.testId("prompt-damage-20-inc")).toExist(),
    Scene.expect(Scene.testId("prompt-damage-20-dec")).toExist(),
    Scene.expect(Scene.selector('input[type="number"]')).not.toExist(),
  );
});

test("sacrifice pick prompt renders as a board surface", () => {
  const sacrificeAction = action(14, {
    kind: "activate",
    label: "Village Rites",
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
    Scene.expect(Scene.testId("sacrifice-pick")).toExist(),
  );
});

test("full board view mounts the bitmap layer", () => {
  liveBoardScene(
    fullBoardModel(initialBoardModel(), gameState()),
    Scene.expect(Scene.testId("board-bitmap-layer")).toExist(),
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
