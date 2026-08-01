/**
 * @vitest-environment happy-dom
 */
import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import { BindCardArt } from "~/ui/card-art";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import { SubmitIntent } from "../../game/intents";
import { emptyCostPicks } from "../action/execution";
import { ZONE } from "../geometry/layout";
import { STACK_EXPAND_COUNT } from "../geometry/stackLayout";
import { type Message, StackCollapseClicked, TargetChosen } from "../messages";
import { spawnFlight } from "../motion/flights";
import { type BoardModel, initialBoardModel, updateBoard } from "../submodel";
import { boardOverlays } from "./overlays";
import { resolveBoardCardArtMounts, resolveBoardOverlayMounts } from "./scene-helpers";

const h = html<Message>();

type ViewModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const overlayView = Submodel.defineView<ViewModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log);
});

function player(): import("~/wire/types").PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
  };
}

function gameState(over: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects: [],
    pending_choice: null,
    players: [player(), { ...player(), player: 1, username: "Bob" }],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...over,
  };
}

function gameFold(state: VisibleState): GameFoldState {
  return {
    seq: 1,
    state,
    log: [],
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

function spellOnStack(
  sourceId: number,
  label: string,
  print: string,
): { stack: VisibleState["stack"]; objects: ObjectView[] } {
  const spell: ObjectView = {
    controller: 0,
    has_haste: false,
    id: sourceId,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: label,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print,
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Stack,
  };
  return {
    objects: [spell],
    stack: [{ controller: 0, kind: "spell", label: testMessageRef(label), source: sourceId }],
  };
}

test("stack overlay renders card art for spells on the stack", () => {
  const { objects, stack } = spellOnStack(42, "Lightning Bolt", "bolt-print");
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(gameState({ objects, stack })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.selector("[data-art-url]")).toExist(),
  );
});

test("spell stack face stays hidden while its stack entrance flight is in progress", () => {
  const { objects, stack } = spellOnStack(42, "Lightning Bolt", "bolt-print");
  const flight = {
    ...spawnFlight({
      id: 42,
      kind: "stack",
      name: "Lightning Bolt",
      print: "bolt-print",
      scale: 0.8,
      targetScale: 1,
      targetX: 100,
      targetY: 40,
      x: 20,
      y: 10,
      fromCardId: 7,
    }),
    phase: "flying" as const,
  };
  const model: ViewModel = {
    board: {
      ...initialBoardModel(),
      flights: new Map([[42, flight]]),
      hideCardIds: new Set([42]),
      ownedIds: new Set([42]),
    },
    fold: gameFold(gameState({ objects, stack })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toBeAbsent(),
  );
});

test("spell stack face stays hidden while a settled stack flight is still in the model", () => {
  const { objects, stack } = spellOnStack(42, "Lightning Bolt", "bolt-print");
  const flight = {
    ...spawnFlight({
      id: 42,
      kind: "stack",
      name: "Lightning Bolt",
      print: "bolt-print",
      scale: 1,
      targetScale: 1,
      targetX: 100,
      targetY: 40,
      x: 100,
      y: 40,
      fromCardId: 7,
    }),
    phase: "settled" as const,
    hold: false,
  };
  const model: ViewModel = {
    board: {
      ...initialBoardModel(),
      flights: new Map([[42, flight]]),
      hideCardIds: new Set([42]),
      ownedIds: new Set([42]),
    },
    fold: gameFold(gameState({ objects, stack })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toBeAbsent(),
  );
});

function abilityDuringSourceFlight(kind: "battlefield" | "from-stack"): ViewModel {
  // Trigger on the stack: entry.source is the permanent id. A battlefield / from-stack flight for
  // that same id puts it in hideCardIds so the resting battlefield face stays hidden — but the
  // ability on the stack is a different resting face and must still show the source's art (not
  // only the effect caption).
  const sourceId = 99;
  const permanent: ObjectView = {
    controller: 0,
    has_haste: false,
    id: sourceId,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 1, 0, 0] },
    marked_damage: 0,
    name: "Elvish Visionary",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "visionary-print",
    summoning_sick: true,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const flight = {
    ...spawnFlight({
      id: sourceId,
      kind,
      name: permanent.name,
      print: permanent.print ?? "",
      scale: 0.8,
      targetScale: 1,
      targetX: 100,
      targetY: 40,
      x: 20,
      y: 10,
    }),
    phase: "flying" as const,
  };
  return {
    board: {
      ...initialBoardModel(),
      flights: new Map([[sourceId, flight]]),
      hideCardIds: new Set([sourceId]),
      ownedIds: new Set([sourceId]),
    },
    fold: gameFold(
      gameState({
        objects: [permanent],
        stack: [
          {
            controller: 0,
            kind: "ability",
            label: testMessageRef("Draw a card"),
            source: sourceId,
          },
        ],
      }),
    ),
    tableId: "T1",
  };
}

test("ability stack face keeps card art while its source permanent is mid-battlefield flight", () => {
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(abilityDuringSourceFlight("battlefield")),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.selector("[data-art-url]")).toExist(),
    Scene.expect(Scene.testId("stack-top-caption")).toContainText("Draw a card"),
  );
});

test("ability stack face uses entry print when the source id is no longer in objects", () => {
  // Evolving Wilds / other sacrifice-as-cost activations: stack.source is the Moved tombstone id,
  // which never appears in VisibleState.objects. Art must come from StackObjectView.print.
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(
      gameState({
        objects: [],
        stack: [
          {
            controller: 0,
            kind: "ability",
            label: testMessageRef("Search your library for a basic land card"),
            source: 77,
            print: "evolving-wilds-print",
            name: "Evolving Wilds",
            card_id: "evolving-wilds-id",
          },
        ],
      }),
    ),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    // BindCardArt only mounts when print+name resolve — proves entry.print was used.
    Scene.expect(Scene.selector("[data-art-url]")).toExist(),
  );
});

test("ability stack face keeps card art while its source permanent is mid from-stack flight", () => {
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(abilityDuringSourceFlight("from-stack")),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.selector("[data-art-url]")).toExist(),
    Scene.expect(Scene.testId("stack-top-caption")).toContainText("Draw a card"),
  );
});

test("stack pile caption lists every declared target", () => {
  const { objects } = spellOnStack(42, "Electrolyze", "electrolyze-print");
  const bear: ObjectView = {
    controller: 1,
    has_haste: false,
    id: 22,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 2, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 1,
    plus_counters: 0,
    power: 2,
    print: "bear-print",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(
      gameState({
        objects: [...objects, bear],
        stack: [
          {
            controller: 0,
            kind: "spell",
            label: testMessageRef("Electrolyze"),
            source: 42,
            targets: [
              { kind: "object", id: 22 },
              { kind: "player", player: 1 },
            ],
          },
        ],
      }),
    ),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-top-caption")).toContainText("Bear"),
    Scene.expect(Scene.testId("stack-top-caption")).toContainText("Bob"),
  );
});

test("staged ghost appears on the stack during arrow targeting", () => {
  const handCard: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 11,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Shock",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "shock-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
  };
  const target: ObjectView = {
    controller: 1,
    has_haste: false,
    id: 22,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 1,
    plus_counters: 0,
    power: 2,
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const castAction: ActionView = {
    id: 9,
    kind: "cast",
    label: testMessageRef("Cast Shock"),
    needs_target: true,
    object: handCard.id,
    section: "hand",
    targets: [{ kind: "object", id: 22 }],
  };
  const model: ViewModel = {
    board: {
      ...initialBoardModel(),
      staged: {
        card: handCard,
        action: castAction,
        picks: emptyCostPicks(),
        preferPick: false,
        playOrigin: { x: 0, y: 0 },
        playOriginScreen: { x: 0, y: 0 },
      },
    },
    fold: gameFold(gameState({ objects: [handCard, target] })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(2),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.testId("stack-staged-hint")).toContainText("Choose a target"),
    Scene.expect(Scene.selector("[data-art-url]")).toExist(),
  );
});

test("legal stack face is highlighted and click submits take_action", () => {
  const { objects, stack } = spellOnStack(42, "Lightning Bolt", "bolt-print");
  const counter: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 7,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 2, colored: [0, 1, 0, 0, 0] },
    marked_damage: 0,
    name: "Counterspell",
    needs_target: true,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "counter-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
  };
  const castAction: ActionView = {
    id: 3,
    kind: "cast",
    label: testMessageRef("Cast Counterspell"),
    needs_target: true,
    object: counter.id,
    section: "hand",
    targets: [{ kind: "object", id: 42 }],
  };
  const board: BoardModel = {
    ...initialBoardModel(),
    staged: {
      card: counter,
      action: castAction,
      picks: emptyCostPicks(),
      preferPick: false,
      playOrigin: { x: 0, y: 0 },
      playOriginScreen: { x: 0, y: 0 },
    },
  };
  const fold = gameFold(gameState({ objects: [...objects, counter], stack }));
  Scene.scene(
    {
      update: (m, message: Message) => {
        const [nextBoard] = updateBoard(m.board, message, m.fold, m.tableId);
        return [{ ...m, board: nextBoard }, []];
      },
      view: overlayView,
    },
    Scene.with({ board, fold, tableId: "T1" }),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(3),
    Scene.expect(Scene.selector('[data-legal-target="true"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="stack-face-0"][data-legal-target="true"]')).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toHaveAttr("role", "button"),
    Scene.expect(Scene.testId("stack-face-0")).toHaveAccessibleName("Target: Lightning Bolt"),
    Scene.expect(Scene.testId("target-pick")).toBeAbsent(),
    // Keyboard path: Enter on a focused legal target picks it, same as click.
    Scene.keydown(Scene.testId("stack-face-0"), "Enter"),
    Scene.expect(Scene.selector('[data-legal-target="true"]')).not.toExist(),
    Scene.Mount.expectEnded(BindCardArt),
  );
  const [nextBoard, commands] = updateBoard(board, TargetChosen({ target: { kind: "object", id: 42 } }), fold, "T1");
  expect(nextBoard.staged).toBeNull();
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
});

test("stack overlay hidden when stack is empty and nothing is staged", () => {
  const model: ViewModel = { board: initialBoardModel(), fold: gameFold(gameState()), tableId: "T1" };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toBeAbsent(),
  );
});

test("pending choose_target shows source card art on the stack while aiming (Innkeeper's Talent)", () => {
  // Trigger placement pauses on ChooseTarget before the ability is pushed (Placement::Paused).
  // Arrow aim uses the stack origin — the source permanent's art must ghost there, same as a
  // staged cast. Innkeeper's Talent's begin-combat +1/+1 is the reproducing case.
  const talent: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 0,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "enchantment" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 1] },
    marked_damage: 0,
    name: "Innkeeper's Talent",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "innkeepers-talent-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Battlefield,
  };
  const bear: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 1,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 1] },
    marked_damage: 0,
    name: "Grizzly Bear",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "bear-print",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(
      gameState({
        objects: [talent, bear],
        stack: [],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Put a +1/+1 counter on target creature you control"),
          min: 1,
          max: 1,
          player: 0,
          source: talent.id,
          items: [{ id: bear.id, label: "Grizzly Bear", print: "bear-print" }],
        },
      }),
    ),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.testId("stack-staged-hint")).toContainText("Choose a target"),
    Scene.expect(Scene.selector("[data-art-url]")).toExist(),
  );
});

test("pending proliferate shows source card art on the stack after the ability left", () => {
  // Abilities leave the stack before effects run (CR 608). Contagion Engine / Cankerbloom / etc.
  // pause on proliferate with an empty stack — ghost the permanent's art at the aim origin.
  const engine: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 0,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "artifact" },
    mana_cost: { generic: 6, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Contagion Engine",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "contagion-engine-print",
    summoning_sick: false,
    tapped: true,
    toughness: 0,
    zone: ZONE.Battlefield,
  };
  const infected: ObjectView = {
    controller: 1,
    has_haste: false,
    id: 1,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 1] },
    marked_damage: 0,
    name: "Infected Bear",
    needs_target: false,
    owner: 1,
    plus_counters: 1,
    power: 2,
    print: "bear-print",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(
      gameState({
        objects: [engine, infected],
        stack: [],
        pending_choice: {
          kind: "proliferate",
          player: 0,
          source: engine.id,
          items: [{ id: infected.id, label: "Infected Bear", print: "bear-print" }],
        },
      }),
    ),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.testId("stack-staged-hint")).toContainText("Choose a target"),
    Scene.expect(Scene.selector("[data-art-url]")).toExist(),
  );
});

test("pending choose_target does not duplicate a spell already on the stack", () => {
  // Post-cast spell targeting: the spell is already a stack entry — ghost must not double it.
  const bolt: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 42,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Lightning Bolt",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "bolt-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Stack,
  };
  const bear: ObjectView = {
    controller: 1,
    has_haste: false,
    id: 7,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 1] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 1,
    plus_counters: 0,
    power: 2,
    print: "bear-print",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(
      gameState({
        objects: [bolt, bear],
        stack: [
          {
            controller: 0,
            kind: "spell",
            label: testMessageRef("Lightning Bolt"),
            source: bolt.id,
            print: "bolt-print",
            name: "Lightning Bolt",
          },
        ],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Lightning Bolt"),
          min: 1,
          max: 1,
          player: 0,
          source: bolt.id,
          items: [{ id: bear.id, label: "Bear", print: "bear-print" }],
        },
      }),
    ),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.testId("stack-face-1")).toBeAbsent(),
    Scene.expect(Scene.testId("stack-staged-hint")).toBeAbsent(),
  );
});

test("a second trigger from one permanent gets its own top face while aiming", () => {
  // Simultaneous triggers off one source (CR 603.3b): the engine places them one at a time and
  // pauses on `choose_target` before pushing the next, so the ability you are targeting is always
  // the top face. Ability entries carry the *source permanent's* id, so the already-placed first
  // trigger must not be mistaken for the one being targeted.
  const veyran: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 3,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 2, colored: [0, 0, 1, 1, 0] },
    marked_damage: 0,
    name: "Veyran, Voice of Duality",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "veyran-print",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const bear: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 7,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 1] },
    marked_damage: 0,
    name: "Grizzly Bear",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "bear-print",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(
      gameState({
        objects: [veyran, bear],
        stack: [{ controller: 0, kind: "ability", label: testMessageRef("Draw a card"), source: veyran.id }],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Target creature gets +1/+1"),
          min: 1,
          max: 1,
          player: 0,
          source: veyran.id,
          items: [{ id: bear.id, label: "Grizzly Bear", print: "bear-print" }],
        },
      }),
    ),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(2),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.testId("stack-face-1")).toHaveAttr("data-staged", "true"),
    Scene.expect(Scene.testId("stack-staged-hint")).toContainText("Choose a target"),
  );
});

test("expand button appears for a tall stack and opens strip view", () => {
  const objects: ObjectView[] = [];
  const stack: VisibleState["stack"] = [];
  for (let i = 0; i < STACK_EXPAND_COUNT; i++) {
    const id = 100 + i;
    objects.push({
      controller: 0,
      has_haste: false,
      id,
      is_commander: false,
      is_token: false,
      legendary: false,
      kind: { kind: "instant" },
      mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
      marked_damage: 0,
      name: `Spell ${i}`,
      needs_target: false,
      owner: 0,
      plus_counters: 0,
      power: 0,
      print: `print-${i}`,
      summoning_sick: false,
      tapped: false,
      toughness: 0,
      zone: ZONE.Stack,
    });
    stack.push({ controller: 0, kind: "spell", label: testMessageRef(`Spell ${i}`), source: id });
  }
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(gameState({ objects, stack })),
    tableId: "T1",
  };
  Scene.scene(
    {
      update: (m, msg: Message) => {
        const [board] = updateBoard(m.board, msg, m.fold, m.tableId);
        return [{ ...m, board }, []];
      },
      view: overlayView,
    },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(STACK_EXPAND_COUNT),
    Scene.expect(Scene.testId("stack-expand")).toExist(),
    Scene.click(Scene.testId("stack-expand")),
    Scene.expect(Scene.testId("stack-overlay-expanded")).toExist(),
  );
});

test("StackCollapseClicked collapses expanded stack", () => {
  const board = { ...initialBoardModel(), stackExpand: true };
  const next = updateBoard(board, StackCollapseClicked(), gameFold(gameState()), "T1")[0];
  expect(next.stackExpand).toBe(false);
});

test("hold bar renders when stack_hold_remaining_ms is positive", () => {
  const { objects, stack } = spellOnStack(42, "Bolt", "bolt-print");
  const model: ViewModel = {
    board: { ...initialBoardModel(), stackHoldPeak: 2000 },
    fold: gameFold(gameState({ objects, stack, stack_hold_remaining_ms: 1500 })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("stack-hold-bar")).toExist(),
    Scene.expect(Scene.selector('[data-testid="stack-hold-bar"].opacity-0')).not.toExist(),
  );
});
