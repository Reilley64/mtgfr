import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import { handView } from "./hand";

function cost(overrides: Partial<WireCost> = {}): WireCost {
  return {
    generic: 1,
    colored: [0, 0, 0, 0, 0],
    ...overrides,
  };
}

function object(id: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "instant" },
    mana_cost: cost(),
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

function action(id: number, overrides: Partial<ActionView> = {}): ActionView {
  return {
    id,
    kind: "cast",
    label: testMessageRef(`Cast ${id}`),
    needs_target: false,
    object: id,
    section: "hand",
    ...overrides,
  };
}

function state(overrides: Partial<VisibleState> = {}): VisibleState {
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
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...overrides,
  };
}

function renderHand(visible: VisibleState): unknown {
  return handView({
    state: visible,
    hiddenId: null,
    flyingIds: new Set(),
    hiddenIds: new Set(),
    handDrag: null,
  });
}

function testId(node: unknown): string | null {
  if (node == null || typeof node !== "object") return null;
  const n = node as { data?: { attrs?: Record<string, string> } };
  const id = n.data?.attrs?.["data-testid"];
  return typeof id === "string" ? id : null;
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

function className(node: unknown): string {
  if (node == null || typeof node !== "object") return "";
  const n = node as { data?: { class?: Record<string, boolean> } };
  return Object.entries(n.data?.class ?? {})
    .filter(([, active]) => active)
    .map(([name]) => name)
    .join(" ");
}

function styleValue(node: unknown, name: string): string | undefined {
  if (node == null || typeof node !== "object") return undefined;
  const n = node as { data?: { style?: Record<string, string> } };
  const value = n.data?.style?.[name];
  return typeof value === "string" ? value : undefined;
}

function treeHasClass(node: unknown, token: string): boolean {
  if (className(node).split(/\s+/).includes(token)) return true;
  if (node == null || typeof node !== "object") return false;
  const n = node as { children?: unknown[] };
  return (n.children ?? []).some((child) => treeHasClass(child, token));
}

describe("handView unplayable brightness", () => {
  it("does not darken unplayable hand tiles (borders carry castability)", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const uncastable = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [castable, uncastable], actions: [action(7, { object: 42 })] }));

    const unplayableFace = findTestId(tree, "hand-card-face-43");
    expect(unplayableFace).not.toBeNull();
    expect(treeHasClass(unplayableFace, "brightness-[0.55]")).toBe(false);
    expect(className(unplayableFace)).not.toContain("ring-playable-border");
  });

  it("does not darken unplayable command tiles", () => {
    const commander = object(9, {
      name: "Atraxa",
      zone: ZONE.Command,
      is_commander: true,
      kind: { kind: "creature", power: 4, toughness: 4 },
    });
    const tree = renderHand(state({ objects: [commander], actions: [] }));
    const face = findTestId(tree, "hand-card-face-9");
    expect(face).not.toBeNull();
    expect(treeHasClass(face, "brightness-[0.55]")).toBe(false);
  });

  it("still fades the drag-source hand tile", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const cast = action(7, { object: 42 });
    const tree = handView({
      state: state({ objects: [castable], actions: [cast] }),
      hiddenId: null,
      flyingIds: new Set(),
      hiddenIds: new Set(),
      handDrag: {
        action: cast,
        name: "Lightning Bolt",
        print: "",
        manaCost: cost(),
        x: 10,
        y: 10,
      },
    });
    const face = findTestId(tree, "hand-card-face-42");
    expect(face).not.toBeNull();
    expect(treeHasClass(face, "opacity-25")).toBe(true);
  });
});

describe("handView playable outlines", () => {
  it("adds the playable border to castable hand tiles only", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const uncastable = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [castable, uncastable], actions: [action(7, { object: 42 })] }));

    expect(className(findTestId(tree, "hand-card-face-42"))).toContain("ring-playable-border");
    expect(className(findTestId(tree, "hand-card-face-43"))).not.toContain("ring-playable-border");
  });

  it("layers mint playable ring with gy/exile outline halos on playable zone bar tiles", () => {
    const graveyardCard = object(60, { zone: ZONE.Graveyard });
    const exileCard = object(61, { zone: ZONE.Exile });
    const tree = renderHand(
      state({
        objects: [graveyardCard, exileCard],
        actions: [
          action(60, { object: graveyardCard.id, section: "graveyard" }),
          action(61, { object: exileCard.id, section: "exile" }),
        ],
      }),
    );

    const gy = className(findTestId(tree, "hand-card-face-60"));
    const exile = className(findTestId(tree, "hand-card-face-61"));
    expect(gy).toContain("ring-playable-border");
    expect(gy).toContain("outline-graveyard-outline");
    expect(exile).toContain("ring-playable-border");
    expect(exile).toContain("outline-exile-outline");
  });

  it("keeps commander gold on an unplayable command-zone commander", () => {
    const commander = object(9, {
      zone: ZONE.Command,
      is_commander: true,
      name: "Zimone, Quandrix Prodigy",
    });
    const tree = renderHand(state({ objects: [commander], actions: [] }));
    const face = className(findTestId(tree, "hand-card-face-9"));
    expect(face).toContain("ring-commander-gold");
    expect(face).not.toContain("ring-playable-border");
  });

  it("omits unplayable gy/exile cards from the action bar", () => {
    const gy = object(60, { zone: ZONE.Graveyard, name: "Grizzly Bears" });
    const exile = object(61, { zone: ZONE.Exile, name: "Sol Ring" });
    const tree = renderHand(state({ objects: [gy, exile], actions: [] }));

    expect(findTestId(tree, "hand-card-face-60")).toBeNull();
    expect(findTestId(tree, "hand-card-face-61")).toBeNull();
  });

  it("layers mint playable ring with outer commander-gold outline when castable", () => {
    const commander = object(9, {
      zone: ZONE.Command,
      is_commander: true,
      name: "Zimone, Quandrix Prodigy",
    });
    const tree = renderHand(
      state({
        objects: [commander],
        actions: [action(9, { object: 9, section: "command", kind: "cast" })],
      }),
    );
    const face = className(findTestId(tree, "hand-card-face-9"));
    expect(face).toContain("ring-playable-border");
    expect(face).toContain("outline-commander-gold");
  });
});

describe("handView drag chrome", () => {
  it("moves playable border from source to ghost while dragging", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const cast = action(7, { object: 42 });
    const tree = handView({
      state: state({ objects: [castable], actions: [cast] }),
      hiddenId: null,
      flyingIds: new Set(),
      hiddenIds: new Set(),
      handDrag: {
        action: cast,
        name: "Lightning Bolt",
        print: "",
        manaCost: cost(),
        zone: "hand",
        x: 10,
        y: 10,
      },
    });
    const source = findTestId(tree, "hand-card-face-42");
    expect(className(source)).not.toContain("ring-playable-border");
    expect(treeHasClass(source, "opacity-25")).toBe(true);
    const ghost = findTestId(tree, "hand-drag-ghost");
    expect(ghost).not.toBeNull();
    expect(treeHasClass(ghost, "ring-playable-border")).toBe(true);
  });

  it("uses command playable aura classes on a command-zone drag ghost", () => {
    const commander = object(9, {
      zone: ZONE.Command,
      is_commander: true,
      name: "Zimone, Quandrix Prodigy",
    });
    const cast = action(9, { object: 9, section: "command", kind: "cast" });
    const tree = handView({
      state: state({ objects: [commander], actions: [cast] }),
      hiddenId: null,
      flyingIds: new Set(),
      hiddenIds: new Set(),
      handDrag: {
        action: cast,
        name: commander.name,
        print: "",
        manaCost: commander.mana_cost,
        zone: "command",
        x: 10,
        y: 10,
      },
    });
    const ghost = findTestId(tree, "hand-drag-ghost");
    expect(ghost).not.toBeNull();
    expect(treeHasClass(ghost, "ring-playable-border")).toBe(true);
    expect(treeHasClass(ghost, "outline-commander-gold")).toBe(true);
  });

  it("uses not-allowed on unplayable and grab on playable hit strips", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const uncastable = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [castable, uncastable], actions: [action(7, { object: 42 })] }));
    const playableHit = findTestId(tree, "hand-card-42");
    const unplayableHit = findTestId(tree, "hand-card-43");
    expect(className(playableHit)).toContain("cursor-grab");
    expect(className(playableHit)).not.toContain("cursor-not-allowed");
    expect(className(unplayableHit)).toContain("cursor-not-allowed");
    expect(className(unplayableHit)).not.toContain("cursor-grab");
  });
});

describe("handView hover stacking", () => {
  it("keeps resting hand z overridable from the tile root", () => {
    const a = object(42, { name: "Lightning Bolt" });
    const b = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [a, b], actions: [action(7, { object: 42 })] }));

    const root = findTestId(tree, "hand-tile-42");
    expect(root).not.toBeNull();
    expect(treeHasClass(root, "group/hand-tile")).toBe(true);
    expect(treeHasClass(root, "[z-index:var(--hand-z)]")).toBe(true);
    expect(treeHasClass(root, "hover:[z-index:50]")).toBe(true);
    expect(styleValue(root, "--hand-z")).toBe("1");
    expect(styleValue(root, "z-index")).toBeUndefined();

    const face = findTestId(tree, "hand-card-face-42");
    expect(className(face)).not.toContain("group-hover/hand-tile:z-30");
    // Face may still live under a wrapper — assert the tree no longer uses face hover z-30:
    expect(treeHasClass(tree, "group-hover/hand-tile:z-30")).toBe(false);
  });

  it("does not elevate z for discard-selected without relying on selection z", () => {
    const a = object(42, { name: "Lightning Bolt" });
    const tree = handView({
      state: state({ objects: [a], actions: [] }),
      hiddenId: null,
      flyingIds: new Set(),
      hiddenIds: new Set(),
      handDrag: null,
      discardCostIds: new Set([42]),
      discardSelectedIds: new Set([42]),
    });
    const root = findTestId(tree, "hand-tile-42");
    expect(root).not.toBeNull();
    // Root still has hover elevate available, but selection alone must not add a selected z class.
    expect(treeHasClass(root, "[z-index:var(--hand-z)]")).toBe(true);
    expect(treeHasClass(root, "hover:[z-index:50]")).toBe(true);
    expect(styleValue(root, "--hand-z")).toBe("1");
    expect(styleValue(root, "z-index")).toBeUndefined();
    expect(className(root)).not.toContain("z-30");
    expect(treeHasClass(root, "z-50")).toBe(false); // bare z-50 without hover: prefix
    const face = findTestId(tree, "hand-card-face-42");
    expect(className(face)).toContain("ring-llanowar");
    // Face raise for selection must not use elevated z-30:
    expect(treeHasClass(findTestId(tree, "hand-tile-42"), "z-30")).toBe(false);
  });
});
