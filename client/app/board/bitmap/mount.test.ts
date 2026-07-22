import { afterEach, describe, expect, it, vi } from "vitest";
import type { ActionView, PlayerView } from "~/wire/types";
import type { RenderCard } from "../geometry/layout";
import { ZONE } from "../geometry/layout";
import { spawnFlight } from "../motion/flights";
import { bitmapFrameNeedsRaf, paintBitmapLayer, paintFlightLayer } from "./mount";

afterEach(() => {
  vi.unstubAllGlobals();
});

function player(overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
    ...overrides,
  };
}

function card(overrides: Partial<RenderCard> = {}): RenderCard {
  return {
    cardId: "card",
    cluster: 0,
    clusterMembers: [],
    controller: 0,
    counters: 0,
    faceDown: false,
    fanAngle: 0,
    goaded: false,
    h: 134,
    hasHaste: false,
    id: 1,
    isCommander: false,
    keywords: [],
    kind: "creature",
    markedDamage: 0,
    name: "Grizzly Bears",
    owner: 0,
    pile: 0,
    prepared: false,
    print: "resting-print",
    pt: "2/2",
    summoningSick: false,
    tapped: false,
    tapsForMana: false,
    w: 96,
    x: 10,
    y: 20,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function mockCtx(calls: string[]): CanvasRenderingContext2D {
  const state = { fillStyle: "", strokeStyle: "" };
  const ctx = {
    arc: vi.fn(() => calls.push("avatar")),
    beginPath: vi.fn(),
    clearRect: vi.fn(() => calls.push("clear")),
    clip: vi.fn(),
    closePath: vi.fn(),
    drawImage: vi.fn((image: { label?: string }) => calls.push(`image:${image.label ?? "unknown"}`)),
    fill: vi.fn(() => calls.push(`fill:${state.fillStyle}`)),
    fillText: vi.fn((text: string) => calls.push(`text:${text}`)),
    lineTo: vi.fn(),
    measureText: vi.fn(() => ({ width: 0 })),
    moveTo: vi.fn(),
    quadraticCurveTo: vi.fn(() => calls.push("arrow")),
    restore: vi.fn(),
    rotate: vi.fn(),
    roundRect: vi.fn(),
    save: vi.fn(),
    setLineDash: vi.fn((dash: number[]) => {
      if (dash.join(",") === "2,6") calls.push("target-highlight");
    }),
    setTransform: vi.fn(),
    stroke: vi.fn(() => {
      calls.push("stroke");
      calls.push(`stroke:${state.strokeStyle}`);
    }),
    strokeText: vi.fn(),
    translate: vi.fn(),
  } as unknown as CanvasRenderingContext2D;
  Object.defineProperty(ctx, "fillStyle", {
    get: () => state.fillStyle,
    set: (value) => {
      state.fillStyle = String(value);
    },
  });
  Object.defineProperty(ctx, "strokeStyle", {
    get: () => state.strokeStyle,
    set: (value) => {
      state.strokeStyle = String(value);
    },
  });
  return ctx;
}

function battlefieldAction(objectId: number, overrides: Partial<ActionView> = {}): ActionView {
  return {
    id: objectId + 100,
    kind: "activate",
    label: "Activate",
    needs_target: false,
    object: objectId,
    section: "battlefield",
    ...overrides,
  };
}

describe("paintBitmapLayer", () => {
  it("paints battlefield permanent chrome on the resting layer without under-card labels", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const image = (label: string) => ({ label }) as unknown as HTMLImageElement;
    const cache = { get: vi.fn(() => image("resting")) };

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [
          card({ name: "Runeclaw Bear", pt: "2/2", summoningSick: true }),
          card({ id: 2, kind: "planeswalker", name: "Test Walker", pt: "4", x: 130 }),
          card({ id: 3, counters: 1, name: "Counter Bear", x: 250 }),
        ],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        targetPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls).toContain("text:2/2");
    expect(calls).toContain("text:4");
    expect(calls).toContain("text:+1");
    expect(calls).toContain("fill:#e8b24a");
    expect(calls).not.toContain("text:Runeclaw Bear");
    expect(calls).not.toContain("text:Test Walker");
    expect(calls).not.toContain("text:Counter Bear");
  });

  it("layers resting art below avatars and committed combat arrows", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const image = (label: string) => ({ label }) as unknown as HTMLImageElement;
    const cache = {
      get: vi.fn((url: string) => {
        if (url.includes("resting-print")) return image("resting");
        return undefined;
      }),
    };

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [card()],
        viewer: 0,
        players: [player(), player({ player: 1, username: "Bob" })],
        priority: 0,
        combat: {
          attackers: [{ attacker: 1, defender: 1 }],
          blocks: [],
          attackers_declared: true,
          blockers_declared: [],
        },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        targetPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls.indexOf("image:resting")).toBeGreaterThan(calls.indexOf("clear"));
    expect(calls.indexOf("avatar")).toBeGreaterThan(calls.indexOf("image:resting"));
    expect(calls.indexOf("arrow")).toBeGreaterThan(calls.indexOf("avatar"));
  });

  it("paints staged declare-attackers arrows above resting cards", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const image = (label: string) => ({ label }) as unknown as HTMLImageElement;
    const cache = {
      get: vi.fn((url: string) => (url.includes("resting-print") ? image("resting") : undefined)),
    };

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [card()],
        viewer: 0,
        players: [player(), player({ player: 1, username: "Bob" })],
        priority: 0,
        // Nothing committed yet — the arrow only exists in staging.
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
        stagedAttackers: [{ attacker: 1, defender: 1 }],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        targetPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls.includes("arrow")).toBe(true);
    expect(calls.indexOf("arrow")).toBeGreaterThan(calls.indexOf("image:resting"));
  });

  it("paints a combat drag arrow while dragging a creature", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const cache = { get: vi.fn(() => undefined) };

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [card()],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        targetPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: { x: 100, y: 100 },
        combatDragStroke: "#ff6b6b",
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls.some((call) => call === "arrow")).toBe(true);
  });

  it("paints auto-tap glyphs on previewed lands", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const ctx = mockCtx(calls);
    const strokeText = vi.fn(() => calls.push("auto-tap-glyph"));
    Object.assign(ctx, { strokeText });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => ctx),
      style: {},
    } as unknown as HTMLCanvasElement;
    const cache = { get: vi.fn(() => undefined) };

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [card({ id: 5, kind: "land", pt: "" })],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        targetPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set([5]),
      },
      cache,
    );

    expect(calls).toContain("auto-tap-glyph");
  });

  it("outlines only battlefield permanents with playable actions and leaves tap-only lands undimmed", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const cache = { get: vi.fn(() => undefined) };
    const frame = {
      width: 800,
      height: 600,
      camera: { panX: 0, panY: 0, zoom: 1 },
      cards: [
        card({ id: 7, pt: "", name: "Timberwatch Elf" }),
        card({ id: 8, kind: "land", name: "Forest", pt: "", tapsForMana: true, x: 130 }),
      ],
      viewer: 0,
      players: [player()],
      priority: 0,
      combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
      stagedAttackers: [],
      stagedBlocks: [],
      flights: [],
      hideCardIds: new Set<number>(),
      targetObjects: new Set<number>(),
      targetPlayers: new Set<number>(),
      aimFrom: null,
      cursor: { x: 0, y: 0 },
      combatDragFrom: null,
      combatDragStroke: null,
      paymentPreviewIds: new Set<number>(),
      actions: [battlefieldAction(7)],
    };

    paintBitmapLayer(canvas, frame, cache);

    expect(calls).toContain("stroke:#EAFFF0");
    expect(calls).toContain("stroke:#1a1a1a");
    expect(calls).not.toContain("fill:rgba(0,0,0,0.45)");
  });

  it("paints target highlights and an aim arrow while staged spell targeting", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const cache = { get: vi.fn(() => undefined) };

    paintBitmapLayer(
      canvas,
      {
        width: 1440,
        height: 900,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [card({ id: 22 }), card({ id: 99, x: 200, y: 200, name: "Forest", kind: "land", pt: "" })],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set([22]),
        targetPlayers: new Set<number>(),
        aimFrom: { x: 1300, y: 450 },
        cursor: { x: 500, y: 300 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls.some((call) => call === "target-highlight")).toBe(true);
    expect(calls.filter((call) => call === "stroke").length).toBeGreaterThan(0);
    expect(calls.indexOf("target-highlight")).toBeGreaterThan(calls.indexOf("image:unknown"));
  });

  it("does not paint flights on the resting-permanent layer", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const image = (label: string) => ({ label }) as unknown as HTMLImageElement;
    const cache = { get: vi.fn((url: string) => (url.includes("flight-print") ? image("flight") : undefined)) };

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [
          spawnFlight({
            id: 99,
            kind: "battlefield",
            name: "Flight",
            print: "flight-print",
            scale: 1,
            targetScale: 1,
            targetX: 200,
            targetY: 200,
            x: 100,
            y: 100,
          }),
        ],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        targetPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls.includes("image:flight")).toBe(false);
  });
});

describe("paintFlightLayer", () => {
  it("clears and paints in-flight card art above the hand", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const image = (label: string) => ({ label }) as unknown as HTMLImageElement;
    const cache = { get: vi.fn((url: string) => (url.includes("flight-print") ? image("flight") : undefined)) };

    paintFlightLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [card()],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [
          spawnFlight({
            id: 99,
            kind: "battlefield",
            name: "Flight",
            print: "flight-print",
            scale: 1,
            targetScale: 1,
            targetX: 200,
            targetY: 200,
            x: 100,
            y: 100,
          }),
        ],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        targetPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    // Flight layer paints only flights — no resting permanents leak onto it.
    expect(calls.includes("image:flight")).toBe(true);
    expect(calls.includes("image:resting")).toBe(false);
    expect(calls.indexOf("image:flight")).toBeGreaterThan(calls.indexOf("clear"));
  });
});

describe("bitmapFrameNeedsRaf", () => {
  it("idles while no bitmap animation is active", () => {
    expect(bitmapFrameNeedsRaf({ flights: [] })).toBe(false);
  });

  it("requests frames while flights are active", () => {
    expect(
      bitmapFrameNeedsRaf({
        flights: [
          spawnFlight({
            id: 1,
            kind: "battlefield",
            name: "",
            print: "",
            scale: 1,
            targetScale: 1,
            targetX: 0,
            targetY: 0,
            x: 0,
            y: 0,
          }),
        ],
      }),
    ).toBe(true);
  });
});
