import { afterEach, describe, expect, it, vi } from "vitest";
import { colors } from "~/design-tokens.generated";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, PlayerView } from "~/wire/types";
import { gravatarUrl } from "../../domain/gravatar";
import type { RenderCard } from "../geometry/layout";
import { ZONE } from "../geometry/layout";
import { spawnExitFx } from "../motion/exit-fx";
import { spawnFlight } from "../motion/flights";
import {
  applyPublishedFrame,
  type BitmapFrame,
  bitmapFrameNeedsRaf,
  type FlightClockState,
  paintBitmapLayer,
  paintFlightLayer,
  tickFlightClock,
} from "./mount";

const missingGlobal = Symbol("missing-global");
const originalGlobals = new Map<string, unknown>();
const nativeStubGlobal = typeof vi.stubGlobal === "function" ? vi.stubGlobal.bind(vi) : null;
const nativeUnstubAllGlobals = typeof vi.unstubAllGlobals === "function" ? vi.unstubAllGlobals.bind(vi) : null;

function _stubGlobal(name: string, value: unknown): void {
  if (nativeStubGlobal != null) {
    nativeStubGlobal(name, value);
    return;
  }
  if (!originalGlobals.has(name)) {
    const hadOwnGlobal = Object.hasOwn(globalThis, name);
    originalGlobals.set(name, hadOwnGlobal ? Reflect.get(globalThis, name) : missingGlobal);
  }
  Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
}

function unstubAllGlobals(): void {
  if (nativeUnstubAllGlobals != null) {
    nativeUnstubAllGlobals();
    return;
  }
  for (const [name, value] of originalGlobals) {
    if (value === missingGlobal) {
      Reflect.deleteProperty(globalThis, name);
      continue;
    }
    Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
  }
  originalGlobals.clear();
}

if (nativeStubGlobal == null) {
  Object.assign(vi, { stubGlobal: _stubGlobal });
}

afterEach(() => {
  unstubAllGlobals();
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
    fillRect: vi.fn(),
    fillText: vi.fn((text: string, _x: number, y: number) => {
      calls.push(`text:${text}`);
      calls.push(`text:${text}@${y}`);
    }),
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
    label: testMessageRef("Activate"),
    needs_target: false,
    object: objectId,
    section: "battlefield",
    ...overrides,
  };
}

function frame(overrides: Partial<BitmapFrame> = {}): BitmapFrame {
  return {
    width: 800,
    height: 600,
    camera: { panX: 0, panY: 0, zoom: 1 },
    cards: [card()],
    viewer: 0,
    players: [player()],
    priority: 0,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    stagedAttackers: [],
    stagedBlocks: [],
    flights: [],
    hideCardIds: new Set(),
    targetObjects: new Set(),
    pickedObjects: new Set(),
    assignAmounts: new Map(),
    targetPlayers: new Set(),
    pickedPlayers: new Set(),
    aimFrom: null,
    cursor: { x: 0, y: 0 },
    combatDragFrom: null,
    combatDragStroke: null,
    exitFx: [],
    paymentPreviewIds: new Set(),
    ...overrides,
  };
}

function flightClockState(overrides: Partial<FlightClockState> = {}): FlightClockState {
  return {
    liveFlights: [],
    liveExitFx: [],
    lastRestingSnapshot: null,
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
          card({ id: 4, markedDamage: 3, name: "Damaged Bear", x: 370 }),
        ],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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
    expect(calls).toContain("text:3");
    expect(calls).toContain("fill:#e8b24a");
    expect(calls).toContain(`fill:${colors.damageCrimson}`);
    expect(calls).not.toContain("text:Runeclaw Bear");
    expect(calls).not.toContain("text:Test Walker");
    expect(calls).not.toContain("text:Counter Bear");
    expect(calls).not.toContain("text:Damaged Bear");
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
          blocked_attackers: [],
        },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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

  it("does not paint an arrow for a blocked attacker with no living blocker after blockers declare", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;

    paintBitmapLayer(
      canvas,
      frame({
        players: [player(), player({ player: 1, username: "Bob" })],
        combat: {
          attackers: [{ attacker: 1, defender: 1 }],
          blocks: [],
          attackers_declared: true,
          blockers_declared: [1],
          blocked_attackers: [1],
        },
      }),
      { get: vi.fn(() => undefined) },
    );

    expect(calls).not.toContain("arrow");
  });

  it("paints stack target arrows above resting permanents (not under card art)", () => {
    // Stack→target arrows used to live only on the Foldkit Canvas under the Mount bitmap,
    // so Island Blue arrows disappeared under permanent faces. Mount layer 4 must paint them.
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
          attackers: [],
          blocks: [],
          attackers_declared: false,
          blockers_declared: [],
          blocked_attackers: [],
        },
        stagedAttackers: [],
        stagedBlocks: [],
        stack: [
          {
            controller: 0,
            kind: "spell",
            label: testMessageRef("Lightning Bolt"),
            source: 9,
            target: { kind: "object", id: 1 },
          },
        ],
        stackPresentation: "pile",
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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
    expect(calls.indexOf("arrow")).toBeGreaterThan(calls.indexOf("avatar"));
  });

  it("paints Cmd N on life orbs from max commander_damage", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [],
        viewer: 0,
        players: [
          player({
            commander_damage: [
              { from: 1, amount: 7 },
              { from: 2, amount: 14 },
            ],
          }),
          player({ player: 1, username: "Bob" }),
        ],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      { get: vi.fn(() => undefined) },
    );

    expect(calls).toContain("text:Cmd 14");
    expect(calls).not.toContain("text:Cmd 0");
    expect(calls.filter((c) => /^text:Cmd \d+$/.test(c))).toHaveLength(1);
  });

  it("mirrors flipped opponent label paint away from their card row", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;

    paintBitmapLayer(
      canvas,
      frame({
        cards: [],
        players: [
          player(),
          player({
            player: 1,
            username: "Bob",
            hand_count: 8,
            life: 41,
            commander_damage: [{ from: 0, amount: 9 }],
          }),
        ],
      }),
      { get: vi.fn(() => undefined) },
    );

    expect(calls).toContain("text:41@-96");
    expect(calls).toContain("text:Hand 8@-19");
    expect(calls).toContain("text:Cmd 9@-128");
  });

  it("paints Gravatar face images with life below the circle", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const hash = "abc123";
    const image = { label: "gravatar" } as unknown as HTMLImageElement;
    const cache = {
      get: vi.fn((url: string) => (url === gravatarUrl(hash) ? image : undefined)),
    };

    paintBitmapLayer(
      canvas,
      {
        width: 800,
        height: 600,
        camera: { panX: 0, panY: 0, zoom: 1 },
        cards: [],
        viewer: 0,
        players: [player({ gravatar_hash: hash })],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(cache.get).toHaveBeenCalledWith(gravatarUrl(hash));
    expect(calls).toContain("image:gravatar");
    expect(calls).toContain("text:40@956");
  });

  // Poison is a lose condition (CR 704.5c) and rad drives a mill clock — both belong on the orb.
  it("stacks poison and rad chips under the Cmd chip", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;

    paintBitmapLayer(
      canvas,
      frame({
        cards: [],
        players: [player({ commander_damage: [{ from: 1, amount: 3 }], poison: 4, rad: 1 })],
      }),
      { get: vi.fn(() => undefined) },
    );

    expect(calls).toContain("text:Cmd 3");
    expect(calls).toContain("text:Poison 4");
    expect(calls).toContain("text:Rad 1");
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
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [{ attacker: 1, defender: 1 }],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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
      combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
      stagedAttackers: [],
      stagedBlocks: [],
      flights: [],
      hideCardIds: new Set<number>(),
      targetObjects: new Set<number>(),
      pickedObjects: new Set<number>(),
      assignAmounts: new Map<number, number>(),
      targetPlayers: new Set<number>(),
      pickedPlayers: new Set<number>(),
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

  it("does not outline a summoning-sick creature for a tap activate", () => {
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
      cards: [card({ id: 7, pt: "", name: "Zimone, Quandrix Prodigy", summoningSick: true })],
      viewer: 0,
      players: [player()],
      priority: 0,
      combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
      stagedAttackers: [],
      stagedBlocks: [],
      flights: [],
      hideCardIds: new Set<number>(),
      targetObjects: new Set<number>(),
      pickedObjects: new Set<number>(),
      assignAmounts: new Map<number, number>(),
      targetPlayers: new Set<number>(),
      pickedPlayers: new Set<number>(),
      aimFrom: null,
      cursor: { x: 0, y: 0 },
      combatDragFrom: null,
      combatDragStroke: null,
      paymentPreviewIds: new Set<number>(),
      actions: [battlefieldAction(7, { taps_self: true })],
    };

    paintBitmapLayer(canvas, frame, cache);

    expect(calls).not.toContain("stroke:#EAFFF0");
    expect(calls).toContain("stroke:#1a1a1a");
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
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set([22]),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set<number>(),
        pickedPlayers: new Set<number>(),
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

  it("paints assign-amount badges on blockers during combat damage draft", () => {
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
        cards: [card({ id: 22 }), card({ id: 99, x: 200, y: 200, name: "Elf" })],
        viewer: 0,
        players: [player()],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set([22, 99]),
        pickedObjects: new Set([22]),
        assignAmounts: new Map([
          [22, 3],
          [99, 0],
        ]),
        targetPlayers: new Set<number>(),
        pickedPlayers: new Set<number>(),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls).toContain(`fill:${colors.damageCrimson}`);
  });

  it("paints a solid Priority Gold ring on picked player-aim seats", () => {
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
        cards: [],
        viewer: 0,
        players: [player(), player({ player: 1, username: "Bob" })],
        priority: 0,
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
        stagedAttackers: [],
        stagedBlocks: [],
        flights: [],
        hideCardIds: new Set(),
        targetObjects: new Set(),
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set([1]),
        pickedPlayers: new Set([1]),
        aimFrom: null,
        cursor: { x: 0, y: 0 },
        combatDragFrom: null,
        combatDragStroke: null,
        paymentPreviewIds: new Set(),
      },
      cache,
    );

    expect(calls).toContain(`stroke:${colors.priorityGold}`);
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
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
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
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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
        combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
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
        pickedObjects: new Set(),
        assignAmounts: new Map(),
        targetPlayers: new Set(),
        pickedPlayers: new Set(),
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

  it("paints exit FX art on the animated layer after flights", () => {
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
      get: vi.fn((url: string) => (url.includes("exit-print") ? image("exit") : undefined)),
    };

    paintFlightLayer(
      canvas,
      frame({
        cards: [],
        flights: [],
        exitFx: [
          {
            ...spawnExitFx({
              id: 77,
              kind: "destroy",
              name: "Ash Bear",
              print: "exit-print",
              x: 100,
              y: 100,
              scale: 1,
            }),
            progress: 0.4,
          },
        ],
      }),
      cache,
    );

    expect(calls).toContain("image:exit");
  });
});

describe("bitmapFrameNeedsRaf", () => {
  it("idles while no bitmap animation is active", () => {
    expect(bitmapFrameNeedsRaf({ flights: [], exitFx: [] })).toBe(false);
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
        exitFx: [],
      }),
    ).toBe(true);
  });

  it("requests frames while exit FX are active", () => {
    expect(
      bitmapFrameNeedsRaf({
        flights: [],
        exitFx: [
          spawnExitFx({
            id: 9,
            kind: "exile",
            name: "Void Bear",
            print: "exit-print",
            x: 0,
            y: 0,
            scale: 1,
          }),
        ],
      }),
    ).toBe(true);
  });
});

describe("flight clock helpers", () => {
  it("pose-only flight tick does not request resting paint", () => {
    const flight = spawnFlight({
      id: 3,
      print: "p",
      name: "Bolt",
      x: 0,
      y: 0,
      scale: 1,
      targetX: 100,
      targetY: 0,
      targetScale: 1,
      kind: "battlefield",
    });
    const publishedFrame = frame({ flights: [flight] });
    let state = flightClockState({ liveFlights: [flight] });

    const published = applyPublishedFrame(state, publishedFrame);

    expect(published.paintResting).toBe(true);
    expect(published.paintFlight).toBe(true);
    state = published.state;

    const tick = tickFlightClock(state, published.frame, 16, 16, false);

    expect(tick.paintFlight).toBe(true);
    expect(tick.sync).toBeNull();
    expect(tick.frame.flights[0]?.x).toBeGreaterThan(0);
    expect(tick.frame.flights[0]?.phase).toBe("flying");

    const republish = applyPublishedFrame(tick.state, frame({ flights: [flight] }));

    expect(republish.paintResting).toBe(false);
    expect(republish.frame.flights[0]?.x).not.toBe(0);
  });

  it("preserves active exit FX in sync payloads while flights settle", () => {
    const flight = spawnFlight({
      id: 3,
      print: "p",
      name: "Bolt",
      x: 0,
      y: 0,
      scale: 1,
      targetX: 0,
      targetY: 0,
      targetScale: 1,
      kind: "battlefield",
    });
    const exitFx = spawnExitFx({
      id: 7,
      kind: "destroy",
      name: "Grizzly Bears",
      print: "print-id",
      x: 80,
      y: 60,
      scale: 1,
    });
    const publishedFrame = frame({ flights: [flight], exitFx: [exitFx] });
    const state = flightClockState({ liveFlights: [flight] });

    const published = applyPublishedFrame(state, publishedFrame);
    const tick = tickFlightClock(published.state, published.frame, 16, 16, false);

    expect(tick.sync).toEqual({
      flights: [{ ...flight, x: 0, y: 0, scale: 1, phase: "settled" }],
      exitFx: [{ ...exitFx, progress: 16 / 550 }],
      now: 16,
    });
  });

  it("strips exit FX before publish-time paint under reduced motion", () => {
    const calls: string[] = [];
    vi.stubGlobal("window", { devicePixelRatio: 1 });
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => ({ matches: true })),
    );
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => mockCtx(calls)),
      style: {},
    } as unknown as HTMLCanvasElement;
    const cache = {
      get: vi.fn(() => ({ label: "exit" }) as unknown as HTMLImageElement),
    };
    const exitFx = spawnExitFx({
      id: 11,
      kind: "exile",
      name: "Void Bear",
      print: "exit-print",
      x: 80,
      y: 60,
      scale: 1,
    });

    const published = applyPublishedFrame(flightClockState(), frame({ cards: [], flights: [], exitFx: [exitFx] }));

    paintFlightLayer(canvas, published.frame, cache);

    expect(published.state.liveExitFx).toEqual([]);
    expect(published.frame.exitFx).toEqual([]);
    expect(bitmapFrameNeedsRaf(published.frame)).toBe(false);
    expect(calls).not.toContain("image:exit");
    expect(Reflect.get(published, "sync")).toMatchObject({ flights: [], exitFx: [] });
  });

  it("does not sync exit FX pose-only ticks but syncs completed membership changes", () => {
    const activeExitFx = spawnExitFx({
      id: 7,
      kind: "destroy",
      name: "Grizzly Bears",
      print: "print-id",
      x: 80,
      y: 60,
      scale: 1,
    });
    const activeTick = tickFlightClock(
      flightClockState({ liveExitFx: [activeExitFx] }),
      frame({ flights: [], exitFx: [activeExitFx] }),
      16,
      16,
      false,
    );

    expect(activeTick.frame.exitFx).toEqual([{ ...activeExitFx, progress: 16 / 550 }]);
    expect(activeTick.sync).toBeNull();

    const completingExitFx = { ...activeExitFx, progress: 0.95 };
    const completedTick = tickFlightClock(
      flightClockState({ liveExitFx: [completingExitFx] }),
      frame({ flights: [], exitFx: [completingExitFx] }),
      32,
      32,
      false,
    );

    expect(completedTick.frame.exitFx).toEqual([]);
    expect(completedTick.sync).toEqual({ flights: [], exitFx: [], now: 32 });
  });

  it("steps exit FX forward and drops completed entries from the sync payload", () => {
    const exitFx = {
      ...spawnExitFx({
        id: 7,
        kind: "destroy",
        name: "Grizzly Bears",
        print: "print-id",
        x: 80,
        y: 60,
        scale: 1,
      }),
      progress: 0.95,
    };
    const publishedFrame = frame({ flights: [], exitFx: [exitFx] });
    const state = flightClockState();

    const published = applyPublishedFrame(state, publishedFrame);
    const tick = tickFlightClock(published.state, published.frame, 32, 32, false);

    expect(tick.frame.exitFx).toEqual([]);
    expect(tick.sync).toEqual({ flights: [], exitFx: [], now: 32 });
  });
});
