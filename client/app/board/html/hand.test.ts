import { describe, expect, it } from "vitest";
import { testHtml } from "~/test-html";
import type { Message } from "../messages";

const h = testHtml<Message>();

import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import { HAND_DESIGN_VIEWPORT, handMetrics, handView } from "./hand";

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
    is_token: false,
    legendary: false,
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
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
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
  return handView(
    {
      viewport: HAND_DESIGN_VIEWPORT,
      state: visible,
      hiddenId: null,
      flyingIds: new Set(),
      hiddenIds: new Set(),
      handDrag: null,
    },
    h,
  );
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

function collectTestIds(node: unknown): string[] {
  const ids: string[] = [];
  const walk = (current: unknown) => {
    const id = testId(current);
    if (id != null) ids.push(id);
    if (current == null || typeof current !== "object") return;
    const n = current as { children?: unknown[] };
    for (const child of n.children ?? []) walk(child);
  };
  walk(node);
  return ids;
}

function attr(node: unknown, name: string): string | undefined {
  if (node == null || typeof node !== "object") return undefined;
  const n = node as { data?: { attrs?: Record<string, string> } };
  const value = n.data?.attrs?.[name];
  return typeof value === "string" ? value : undefined;
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
      is_token: false,
      legendary: false,
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
    const tree = handView(
      {
        viewport: HAND_DESIGN_VIEWPORT,
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
      },
      h,
    );
    const root = findTestId(tree, "hand-tile-42");
    expect(attr(root, "data-drag-source")).toBe("true");
    const face = findTestId(tree, "hand-card-face-42");
    expect(face).not.toBeNull();
    expect(treeHasClass(face, "group-data-[drag-source=true]/hand-tile:opacity-25")).toBe(true);
  });
});

describe("handView discard pick accessibility", () => {
  it("names discard-selectable hit targets for assistive tech", () => {
    const a = object(42, { name: "Lightning Bolt" });
    const tree = handView(
      {
        viewport: HAND_DESIGN_VIEWPORT,
        state: state({ objects: [a], actions: [] }),
        hiddenId: null,
        flyingIds: new Set(),
        hiddenIds: new Set(),
        handDrag: null,
        discardCostIds: new Set([42]),
      },
      h,
    );

    const hit = findTestId(tree, "hand-card-42");
    expect(hit).not.toBeNull();
    expect(attr(hit, "aria-label")).toBe("Lightning Bolt (discard)");
  });
});

describe("handView rendered face", () => {
  function findWithAttr(node: unknown, name: string): unknown | null {
    if (attr(node, name) != null) return node;
    if (node == null || typeof node !== "object") return null;
    const n = node as { children?: unknown[] };
    for (const child of n.children ?? []) {
      const found = findWithAttr(child, name);
      if (found != null) return found;
    }
    return null;
  }

  it("draws the rendered card face, not the printed image", () => {
    const bolt = object(42, { name: "Lightning Bolt", print: "lea-161" });
    const tree = renderHand(state({ objects: [bolt], actions: [] }));

    const host = findWithAttr(findTestId(tree, "hand-card-face-42"), "data-face");
    expect(host).not.toBeNull();
    expect(JSON.parse(attr(host, "data-face") ?? "{}")).toMatchObject({ name: "Lightning Bolt", print: "lea-161" });
    expect(attr(host, "data-face-variant")).toBe("full");
  });

  it("draws the snapshot's printed words — including the deck printing's flavor", () => {
    const bolt = object(42, { name: "Lightning Bolt", print: "lea-161", card_id: "bolt" });
    const tree = handView(
      {
        viewport: HAND_DESIGN_VIEWPORT,
        state: state({ objects: [bolt], actions: [] }),
        hiddenId: null,
        flyingIds: new Set(),
        hiddenIds: new Set(),
        handDrag: null,
        cardText: new Map([
          [
            "bolt",
            {
              card_id: "bolt",
              type_line: "Instant",
              oracle: "Deals 3 damage to any target.",
              flavor: "The sparkmage shrieked.",
            },
          ],
        ]),
      },
      h,
    );

    const host = findWithAttr(findTestId(tree, "hand-card-face-42"), "data-face");
    expect(JSON.parse(attr(host, "data-face") ?? "{}")).toMatchObject({
      typeLine: "Instant",
      oracle: "Deals 3 damage to any target.",
      flavor: "The sparkmage shrieked.",
    });
  });

  it("tucks the cost pips over the card's top-right corner", () => {
    const bolt = object(42, { name: "Lightning Bolt", print: "lea-161" });
    const tree = renderHand(state({ objects: [bolt], actions: [] }));
    const pips = findTestId(tree, "hand-cost-pips");
    const metrics = handMetrics(HAND_DESIGN_VIEWPORT);

    // The row no longer clears the frame: its bottom edge sits below the card's top edge, and it
    // holds off the right border so the disks land inside it.
    const top = Number.parseFloat(styleValue(pips, "top") ?? "0");
    expect(top).toBeGreaterThan(-metrics.pipRowH);
    expect(top + metrics.pipRowH).toBeGreaterThan(0);
    expect(Number.parseFloat(styleValue(pips, "paddingRight") ?? "0")).toBeGreaterThan(0);
  });

  it("draws a graveyard bar tile's card face, not its action label", () => {
    const pest = object(62, { zone: ZONE.Graveyard, name: "Teacher's Pest", print: "snc-99" });
    const tree = renderHand(
      state({
        objects: [pest],
        actions: [action(62, { object: 62, section: "graveyard", label: testMessageRef("Cast Teacher's Pest") })],
      }),
    );

    const host = findWithAttr(findTestId(tree, "hand-card-face-62"), "data-face");
    expect(JSON.parse(attr(host, "data-face") ?? "{}")).toMatchObject({ name: "Teacher's Pest" });
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

  it("shows a graveyard-section activate on the graveyard bar (Teacher's Pest self-return)", () => {
    const pest = object(62, {
      zone: ZONE.Graveyard,
      name: "Teacher's Pest",
      kind: { kind: "creature", power: 1, toughness: 1 },
    });
    const tree = renderHand(
      state({
        objects: [pest],
        actions: [
          action(62, {
            object: pest.id,
            section: "graveyard",
            kind: "activate",
            label: testMessageRef("Return this card from your graveyard to the battlefield tapped"),
          }),
        ],
      }),
    );

    const face = findTestId(tree, "hand-card-face-62");
    expect(face).not.toBeNull();
    expect(className(face)).toContain("ring-playable-border");
    expect(className(face)).toContain("outline-graveyard-outline");
  });

  it("keeps commander gold on an unplayable command-zone commander", () => {
    const commander = object(9, {
      zone: ZONE.Command,
      is_commander: true,
      is_token: false,
      legendary: false,
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
      is_token: false,
      legendary: false,
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
  it("fades the drag source and leaves the ghost to the flight canvas", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const cast = action(7, { object: 42 });
    const tree = handView(
      {
        viewport: HAND_DESIGN_VIEWPORT,
        state: state({ objects: [castable], actions: [cast] }),
        hiddenId: null,
        flyingIds: new Set(),
        hiddenIds: new Set(),
        handDrag: {
          action: cast,
          name: "Lightning Bolt",
          print: "bolt-print",
          manaCost: cost(),
          zone: "hand",
          x: 10,
          y: 10,
        },
      },
      h,
    );
    const source = findTestId(tree, "hand-card-face-42");
    expect(className(source)).not.toContain("ring-playable-border");
    expect(treeHasClass(source, "group-data-[drag-source=true]/hand-tile:opacity-25")).toBe(true);
    expect(findTestId(tree, "hand-drag-ghost")).toBeNull();
  });

  it("does not render an HTML drag ghost for name-only cards", () => {
    const castable = object(42, { name: "Lightning Bolt", print: "" });
    const cast = action(7, { object: 42 });
    const tree = handView(
      {
        viewport: HAND_DESIGN_VIEWPORT,
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
      },
      h,
    );
    expect(findTestId(tree, "hand-drag-ghost")).toBeNull();
  });

  it("does not render an HTML command-zone drag ghost", () => {
    const commander = object(9, {
      zone: ZONE.Command,
      is_commander: true,
      is_token: false,
      legendary: false,
      name: "Zimone, Quandrix Prodigy",
    });
    const cast = action(9, { object: 9, section: "command", kind: "cast" });
    const tree = handView(
      {
        viewport: HAND_DESIGN_VIEWPORT,
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
      },
      h,
    );
    expect(findTestId(tree, "hand-drag-ghost")).toBeNull();
    const source = findTestId(tree, "hand-card-face-9");
    expect(className(source)).not.toContain("ring-playable-border");
    expect(treeHasClass(source, "group-data-[drag-source=true]/hand-tile:opacity-25")).toBe(true);
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

describe("handView multi-action cards", () => {
  it("renders one hand tile when cast and two hand abilities are legal", () => {
    const card = object(42, { name: "Valley Rannet", kind: { kind: "creature", power: 6, toughness: 3 } });
    const tree = renderHand(
      state({
        objects: [card],
        actions: [
          action(1, { object: 42, kind: "cast" }),
          action(2, { object: 42, kind: "activate_hand_ability", label: testMessageRef("Discard: Mountain") }),
          action(3, { object: 42, kind: "activate_hand_ability", label: testMessageRef("Discard: Forest") }),
        ],
      }),
    );
    expect(findTestId(tree, "hand-card-42")).not.toBeNull();
    expect(findTestId(tree, "hand-card-face-42")).not.toBeNull();
    const faces = collectTestIds(tree).filter((id) => id.startsWith("hand-card-face-"));
    expect(faces).toEqual(["hand-card-face-42"]);
  });

  it("omits Discard caption when multiple modes are legal", () => {
    const card = object(42, { name: "Valley Rannet" });
    const tree = renderHand(
      state({
        objects: [card],
        actions: [
          action(2, { object: 42, kind: "activate_hand_ability" }),
          action(3, { object: 42, kind: "activate_hand_ability" }),
        ],
      }),
    );
    const hit = findTestId(tree, "hand-card-42");
    expect(className(hit)).not.toContain("Discard");
    expect(findTestId(tree, "hand-caption-42")).toBeNull();
    expect(attr(hit, "aria-label")).toBe("Valley Rannet");
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

  it("does not elevate z for discard-selected without hover; hover still brings to front", () => {
    const a = object(42, { name: "Lightning Bolt" });
    const tree = handView(
      {
        viewport: HAND_DESIGN_VIEWPORT,
        state: state({ objects: [a], actions: [] }),
        hiddenId: null,
        flyingIds: new Set(),
        hiddenIds: new Set(),
        handDrag: null,
        discardCostIds: new Set([42]),
        discardSelectedIds: new Set([42]),
      },
      h,
    );
    const root = findTestId(tree, "hand-tile-42");
    expect(root).not.toBeNull();
    // Selection alone must not add a selected z class; hover elevate stays available.
    expect(treeHasClass(root, "[z-index:var(--hand-z)]")).toBe(true);
    expect(treeHasClass(root, "hover:[z-index:50]")).toBe(true);
    expect(styleValue(root, "--hand-z")).toBe("1");
    expect(styleValue(root, "z-index")).toBeUndefined();
    expect(className(root)).not.toContain("z-30");
    expect(treeHasClass(root, "z-50")).toBe(false); // bare z-50 without hover: prefix
    const face = findTestId(tree, "hand-card-face-42");
    expect(className(face)).toContain("group-data-[selected=true]/hand-tile:ring-llanowar");
    expect(attr(root, "data-selected")).toBe("true");
    expect(attr(root, "data-selectable")).toBe("true");
    expect(treeHasClass(findTestId(tree, "hand-tile-42"), "z-30")).toBe(false);
  });
});

describe("hand tile art chrome attributes", () => {
  it("drives hover-brighten and drag-fade from tile data attributes", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const tree = renderHand(state({ objects: [castable], actions: [action(7, { object: 42 })] }));

    const root = findTestId(tree, "hand-tile-42");
    expect(attr(root, "data-playable")).toBe("true");
    expect(attr(root, "data-drag-source")).toBe("false");

    const face = findTestId(tree, "hand-card-face-42");
    expect(treeHasClass(face, "group-hover/hand-tile:group-data-[playable=true]/hand-tile:brightness-110")).toBe(true);
    expect(treeHasClass(face, "group-data-[drag-source=true]/hand-tile:opacity-25")).toBe(true);
    // No ternary leftovers: the fade must arrive as a variant, never as bare opacity-25.
    expect(className(face)).not.toContain("opacity-25");
  });

  it("marks unplayable tiles data-playable=false so hover does not brighten", () => {
    const uncastable = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [uncastable], actions: [] }));

    expect(attr(findTestId(tree, "hand-tile-43"), "data-playable")).toBe("false");
  });
});
