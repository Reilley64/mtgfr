import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import { emptyCostPicks, type ModalCast, type PlayModePick, type XPromptState } from "./action/execution";
import { ZONE } from "./geometry/layout";
import { promptPresentation } from "./promptPresentation";
import { initialBoardModel } from "./submodel";

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

function state(overrides: Partial<VisibleState> = {}): VisibleState {
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

function cost(overrides: Partial<WireCost> = {}): WireCost {
  return {
    generic: 0,
    colored: [0, 0, 0, 0, 0],
    ...overrides,
  };
}

function card(id: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: cost({ generic: 1 }),
    marked_damage: 0,
    name: `Card ${id}`,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function action(id: number, overrides: Partial<ActionView> = {}): ActionView {
  return {
    id,
    kind: "cast",
    label: testMessageRef(`Action ${id}`),
    needs_target: false,
    object: id,
    section: "hand",
    ...overrides,
  };
}

function xPrompt(): XPromptState {
  return {
    action: action(12, { label: testMessageRef("Comet Storm"), has_x: true, max_x: 3, min_x: 0 }),
    target: null,
    picks: emptyCostPicks(),
    modes: [],
    name: "Comet Storm",
    minX: 0,
    maxX: 3,
    draftX: 1,
    xCost: cost({ generic: 1, has_x: true, x_symbols: 1 }),
  };
}

function playModePick(): PlayModePick {
  const valleyRannet = card(42, { name: "Valley Rannet", zone: ZONE.Hand });
  return {
    card: valleyRannet,
    modes: [
      action(1, { kind: "cast", object: valleyRannet.id, label: testMessageRef("Valley Rannet") }),
      action(2, {
        kind: "activate_hand_ability",
        object: valleyRannet.id,
        label: testMessageRef("Discard: Mountain"),
      }),
    ],
    dropSeed: { x: 0, y: 0 },
    screenOrigin: { x: 400, y: 200 },
  };
}

function modalCastWaiting(): ModalCast {
  return {
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
}

describe("promptPresentation", () => {
  it("returns none with no prompt", () => {
    expect(promptPresentation(initialBoardModel(), state())).toEqual({ mode: "none" });
  });

  it("classifies may_yes_no as simple non-aim", () => {
    const presentation = promptPresentation(
      initialBoardModel(),
      state({
        pending_choice: {
          kind: "may_yes_no",
          label: testMessageRef("Draw?"),
          player: 0,
          source: 1,
        },
      }),
    );

    expect(presentation).toEqual({ mode: "simple", source: "pending", boardAim: false });
  });

  it("classifies search_library as modal", () => {
    const presentation = promptPresentation(
      initialBoardModel(),
      state({
        pending_choice: {
          kind: "search_library",
          player: 0,
          items: [{ id: 1, label: "Forest" }],
        },
      }),
    );

    expect(presentation).toEqual({ mode: "modal", source: "pending" });
  });

  it("classifies on-board choose_target as simple boardAim", () => {
    const bear = card(7, { name: "Bear" });
    const presentation = promptPresentation(
      initialBoardModel(),
      state({
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
    );

    expect(presentation).toEqual({ mode: "simple", source: "pending", boardAim: true });
  });

  it("returns none for another seat's pending_choice", () => {
    const presentation = promptPresentation(
      initialBoardModel(),
      state({
        pending_choice: {
          kind: "may_yes_no",
          label: testMessageRef("Draw?"),
          player: 1,
          source: 1,
        },
      }),
    );

    expect(presentation).toEqual({ mode: "none" });
  });

  it("classifies xPrompt as modal", () => {
    const presentation = promptPresentation({ ...initialBoardModel(), xPrompt: xPrompt() }, state());
    expect(presentation).toEqual({ mode: "modal", source: "local" });
  });

  it("classifies playModePick as simple", () => {
    const presentation = promptPresentation({ ...initialBoardModel(), playModePick: playModePick() }, state());
    expect(presentation).toEqual({ mode: "simple", source: "local", boardAim: false });
  });

  it("classifies modalCast waiting as simple boardAim", () => {
    const presentation = promptPresentation({ ...initialBoardModel(), modalCast: modalCastWaiting() }, state());
    expect(presentation).toEqual({ mode: "simple", source: "local", boardAim: true });
  });

  it("classifies on-board sacrificePick as simple boardAim", () => {
    const sacrificeBody = card(55, { name: "Token" });
    const presentation = promptPresentation(
      {
        ...initialBoardModel(),
        sacrificePick: {
          action: action(14, {
            kind: "activate",
            label: testMessageRef("Village Rites"),
            sacrifice_choices: [55],
            object: 14,
          }),
          card: card(14, { name: "Village Rites", kind: { kind: "instant" }, zone: ZONE.Hand }),
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
      },
      state({ objects: [sacrificeBody] }),
    );

    expect(presentation).toEqual({ mode: "simple", source: "local", boardAim: true });
  });

  it("classifies local discardPick hand aim as simple boardAim", () => {
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
    const presentation = promptPresentation(
      {
        ...initialBoardModel(),
        discardPick: {
          action: action(50, {
            kind: "cast",
            label: testMessageRef("Cast"),
            discard_choices: [11],
            object: 10,
            section: "hand",
          }),
          card: caster,
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
      },
      state({ objects: [caster, fodder] }),
    );

    expect(presentation).toEqual({ mode: "simple", source: "local", boardAim: true });
  });

  it("classifies local gyExilePick pile aim as simple boardAim", () => {
    const caster = card(10, {
      name: "Caster",
      zone: ZONE.Hand,
      kind: { kind: "instant" },
    });
    const graveyardCard = card(8, {
      name: "Corpse",
      zone: ZONE.Graveyard,
    });
    const presentation = promptPresentation(
      {
        ...initialBoardModel(),
        gyExilePick: {
          action: action(50, {
            kind: "cast",
            label: testMessageRef("Cast"),
            graveyard_exile_choices: [8],
            graveyard_exile_min: 1,
            graveyard_exile_max: 1,
            object: 10,
            section: "hand",
          }),
          card: caster,
          dropSeed: { x: 0, y: 0 },
          screenOrigin: { x: 0, y: 0 },
          picks: emptyCostPicks(),
        },
      },
      state({ objects: [caster, graveyardCard] }),
    );

    expect(presentation).toEqual({ mode: "simple", source: "local", boardAim: true });
  });

  it("classifies staged off-board target pick as modal", () => {
    const corpse = card(22, {
      name: "Corpse",
      zone: ZONE.Graveyard,
      owner: 0,
      controller: 0,
    });
    const spell = card(10, {
      kind: { kind: "sorcery" },
      name: "Reanimate",
      owner: 0,
      controller: 0,
      zone: ZONE.Hand,
    });
    const presentation = promptPresentation(
      {
        ...initialBoardModel(),
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
      },
      state({ objects: [spell, corpse] }),
    );

    expect(presentation).toEqual({ mode: "modal", source: "local" });
  });

  it("keeps pure staged on-board targeting as none", () => {
    const target = card(22, {
      controller: 1,
      owner: 1,
      name: "Bear",
    });
    const spell = card(10, {
      kind: { kind: "sorcery" },
      name: "Shock",
      owner: 0,
      controller: 0,
      zone: ZONE.Hand,
    });
    const presentation = promptPresentation(
      {
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
      },
      state({ objects: [target] }),
    );

    expect(presentation).toEqual({ mode: "none" });
  });

  it("classifies staged preferPick on-board targeting as modal after a cost dialog", () => {
    const target = card(22, {
      controller: 1,
      owner: 1,
      name: "Bear",
    });
    const spell = card(10, {
      kind: { kind: "sorcery" },
      name: "Shock",
      owner: 0,
      controller: 0,
      zone: ZONE.Hand,
    });
    const presentation = promptPresentation(
      {
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
          preferPick: true,
          playOrigin: { x: 0, y: 0 },
          playOriginScreen: { x: 0, y: 0 },
        },
      },
      state({ objects: [target] }),
    );

    expect(presentation).toEqual({ mode: "modal", source: "local" });
  });

  // PendingChoiceView.kind is exhaustive in typed tests, so this off-board choose_target case
  // covers the same modal fallback contract future unknown kinds should preserve at runtime.
  it("classifies off-board choose_target as modal fallback", () => {
    const exiledCard = card(22, {
      controller: 1,
      owner: 1,
      name: "Exiled Card",
      zone: ZONE.Exile,
    });
    const presentation = promptPresentation(
      initialBoardModel(),
      state({
        objects: [exiledCard],
        pending_choice: {
          kind: "choose_target",
          label: testMessageRef("Target creature card"),
          min: 1,
          max: 1,
          player: 0,
          source: 1,
          items: [{ id: 22, label: "Exiled Card" }],
        },
      }),
    );

    expect(presentation).toEqual({ mode: "modal", source: "pending" });
  });
});
