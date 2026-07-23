import { describe, expect, it } from "vitest";
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
    label: `Cast ${id}`,
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
      kind: { kind: "creature" },
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

  it("layers playable border with gy/exile zone halos on playable zone bar tiles", () => {
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
    expect(gy).toContain("--color-graveyard-outline");
    expect(exile).toContain("ring-playable-border");
    expect(exile).toContain("--color-exile-outline");
  });

  it("layers playable border with commander gold on a castable command-zone commander", () => {
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
    expect(face).toContain("--color-commander-gold");
  });
});
