import { createRoot, createSignal } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import {
  avatarWorldFor,
  CLUSTER_LONG_PRESS_MS,
  densityHoverFromHit,
  hitLogicalCard,
  seedEntrances,
  useTableSurface,
} from "~/controllers/tableSurface";
import type { RenderCard } from "~/layout";
import { CARD_H, CARD_W, ZONE } from "~/layout";
import { STACK_PEEK, STACK_VERTICAL_RESERVED, stackAimOrigin, stackPeekFor } from "~/lib/boardDraw";
import { screenToWorld, worldToScreen } from "~/lib/camera";
import { fitCamera } from "~/lib/interaction";

const logical = (id: number, x: number, y: number, extra: Partial<RenderCard> = {}): RenderCard =>
  ({
    id,
    name: `Card ${id}`,
    x,
    y,
    w: CARD_W,
    h: CARD_H,
    zone: ZONE.Battlefield,
    tapped: false,
    faceDown: false,
    prepared: false,
    controller: 0,
    owner: 0,
    kind: "land",
    ...extra,
  }) as RenderCard;

const combatCtx = (aimSeats: readonly number[] = []) => ({
  combatStep: true,
  me: 0,
  aimSeats,
});

describe("densityHoverFromHit", () => {
  it("fans when the hit is a permanent cluster face without raising the face", () => {
    const cluster = logical(10, 0, 0, { cluster: 3, clusterMembers: [10, 11, 12] });
    expect(densityHoverFromHit([cluster], cluster, null)).toEqual({
      hoverId: null,
      fannedClusterId: 10,
    });
  });

  it("keeps the fan when the hit is a cluster member", () => {
    const cluster = logical(10, 0, 0, { cluster: 3, clusterMembers: [10, 11, 12] });
    const member = logical(11, 50, 0);
    expect(densityHoverFromHit([cluster], member, 10)).toEqual({
      hoverId: 11,
      fannedClusterId: 10,
    });
  });

  it("clears the fan when hovering a different card", () => {
    const cluster = logical(10, 0, 0, { cluster: 3, clusterMembers: [10, 11, 12] });
    const other = logical(2, 200, 0);
    expect(densityHoverFromHit([cluster, other], other, 10)).toEqual({
      hoverId: 2,
      fannedClusterId: null,
    });
  });

  it("keeps the fan while a selected member pins it, even with no hit", () => {
    const cluster = logical(10, 0, 0, { cluster: 3, clusterMembers: [10, 11, 12] });
    expect(densityHoverFromHit([cluster], null, 10, 11)).toEqual({
      hoverId: null,
      fannedClusterId: 10,
    });
  });

  it("keeps the fan when hovering away if a selected member still pins it", () => {
    const cluster = logical(10, 0, 0, { cluster: 3, clusterMembers: [10, 11, 12] });
    const other = logical(2, 200, 0);
    expect(densityHoverFromHit([cluster, other], other, 10, 11)).toEqual({
      hoverId: 2,
      fannedClusterId: 10,
    });
  });
});

describe("hitLogicalCard", () => {
  it("returns the card from the logical list under the screen point", () => {
    const cam = fitCamera({ x: 800, y: 600 }, 2, 210);
    const cards = [logical(7, 100, 100)];
    const screen = worldToScreen(cam, 100 + CARD_W / 2, 100 + CARD_H / 2);
    expect(hitLogicalCard(cam, cards, screen.x, screen.y)?.id).toBe(7);
  });

  it("misses when the cursor is off the card", () => {
    const cam = fitCamera({ x: 800, y: 600 }, 2, 210);
    const cards = [logical(7, 100, 100)];
    const screen = worldToScreen(cam, -500, -500);
    expect(hitLogicalCard(cam, cards, screen.x, screen.y)).toBeNull();
  });
});

describe("avatarWorldFor", () => {
  it("places each requested seat", () => {
    const worlds = avatarWorldFor([0, 1], 0, 2);
    expect(Object.keys(worlds).map(Number).sort()).toEqual([0, 1]);
    expect(worlds[0]).toEqual(expect.objectContaining({ x: expect.any(Number), y: expect.any(Number) }));
  });
});

describe("useTableSurface", () => {
  it("auto-fits until the player pans, then leaves the camera alone", () => {
    createRoot((dispose) => {
      const [count, setCount] = createSignal(2);
      const [cards] = createSignal<RenderCard[]>([]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: count,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });

      const fitted2 = fitCamera({ x: 800, y: 600 }, 2, 210);
      expect(surface.camera()).toEqual(fitted2);

      surface.pan(40, 0);
      const afterPan = surface.camera();
      expect(afterPan.panX).toBe(fitted2.panX + 40);

      setCount(4);
      // Auto-fit would have changed zoom/pan for 4 seats; userMoved blocks that.
      expect(surface.camera()).toEqual(afterPan);

      dispose();
    });
  });

  it("hitCard reads the cards accessor (logical layout)", () => {
    createRoot((dispose) => {
      const card = logical(3, 50, 50);
      const [cards, setCards] = createSignal<RenderCard[]>([card]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });
      const cam = surface.camera();
      const screen = worldToScreen(cam, 50 + CARD_W / 2, 50 + CARD_H / 2);
      expect(surface.hitCard(screen.x, screen.y)?.id).toBe(3);

      setCards([]);
      expect(surface.hitCard(screen.x, screen.y)).toBeNull();
      dispose();
    });
  });

  it("pointer click on a card emits click; miss clears selection", () => {
    createRoot((dispose) => {
      const card = logical(3, 50, 50);
      const [cards] = createSignal<RenderCard[]>([card]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });
      const screen = worldToScreen(surface.camera(), 50 + CARD_W / 2, 50 + CARD_H / 2);
      surface.pointerDown(screen.x, screen.y, combatCtx());
      expect(surface.pointerUp(screen.x, screen.y)).toEqual({ kind: "click", card });

      surface.pointerDown(0, 0, combatCtx());
      expect(surface.pointerUp(0, 0)).toEqual({ kind: "clear-selection" });
      dispose();
    });
  });

  it("pointer pan moves the camera and still clears selection on release", () => {
    createRoot((dispose) => {
      const [cards] = createSignal<RenderCard[]>([]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });
      const before = surface.camera();
      surface.pointerDown(100, 100, combatCtx());
      surface.pointerMove(140, 100);
      expect(surface.camera().panX).toBe(before.panX + 40);
      expect(surface.pointerUp(140, 100)).toEqual({ kind: "clear-selection" });
      dispose();
    });
  });

  it("aimSeats press+release emits aim-seat", () => {
    createRoot((dispose) => {
      const [cards] = createSignal<RenderCard[]>([]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });
      const world = avatarWorldFor([1], 0, 2)[1];
      const screen = worldToScreen(surface.camera(), world.x, world.y);
      surface.pointerDown(screen.x, screen.y, combatCtx([1]));
      expect(surface.pointerUp(screen.x, screen.y)).toEqual({ kind: "aim-seat", seat: 1 });
      dispose();
    });
  });

  it("aimSeats press released off the seat is a no-op (does not clear selection)", () => {
    createRoot((dispose) => {
      const [cards] = createSignal<RenderCard[]>([]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });
      const world = avatarWorldFor([1], 0, 2)[1];
      const screen = worldToScreen(surface.camera(), world.x, world.y);
      surface.pointerDown(screen.x, screen.y, combatCtx([1]));
      expect(surface.pointerUp(0, 0)).toEqual({ kind: "none" });
      dispose();
    });
  });

  it("combat drag of your creature emits combat-drop", () => {
    createRoot((dispose) => {
      const card = logical(5, 50, 50, { kind: "creature", controller: 0 });
      const [cards] = createSignal<RenderCard[]>([card]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });
      const screen = worldToScreen(surface.camera(), 50 + CARD_W / 2, 50 + CARD_H / 2);
      surface.pointerDown(screen.x, screen.y, combatCtx());
      expect(surface.dragging()?.id).toBe(5);
      const drop = { x: screen.x + 80, y: screen.y + 80 };
      surface.pointerMove(drop.x, drop.y);
      expect(surface.pointerUp(drop.x, drop.y)).toEqual({
        kind: "combat-drop",
        card,
        x: drop.x,
        y: drop.y,
      });
      expect(surface.dragging()).toBeNull();
      dispose();
    });
  });

  it("combat drag of a clustered creature still emits combat-drop after long-press delay", () => {
    vi.useFakeTimers();
    createRoot((dispose) => {
      const card = logical(5, 50, 50, {
        kind: "creature",
        controller: 0,
        cluster: 4,
        clusterMembers: [5, 6, 7, 8],
      });
      const [cards] = createSignal<RenderCard[]>([card]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
      });
      const screen = worldToScreen(surface.camera(), 50 + CARD_W / 2, 50 + CARD_H / 2);
      surface.pointerDown(screen.x, screen.y, combatCtx());
      expect(surface.dragging()?.id).toBe(5);
      vi.advanceTimersByTime(CLUSTER_LONG_PRESS_MS + 50);
      const drop = { x: screen.x + 80, y: screen.y + 80 };
      surface.pointerMove(drop.x, drop.y);
      expect(surface.pointerUp(drop.x, drop.y)).toEqual({
        kind: "combat-drop",
        card,
        x: drop.x,
        y: drop.y,
      });
      dispose();
      vi.useRealTimers();
    });
  });

  it("hitCard reaches a fanned member peek when the face is not raised", () => {
    createRoot((dispose) => {
      const cluster = logical(10, 400, 100, {
        cluster: 3,
        clusterMembers: [10, 11, 12],
      });
      const [cards] = createSignal<RenderCard[]>([cluster]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
        reducedMotion: () => true,
      });
      // Open the fan (hover the collapsed face).
      const face = worldToScreen(surface.camera(), 400 + CARD_W / 2, 100 + CARD_H / 2);
      surface.pointerMove(face.x, face.y);
      // Center of the slot lands on a raised fan member immediately (not stuck on null hover).
      expect(surface.drawnCards()[surface.drawnCards().length - 1]?.id).toBeTypeOf("number");
      expect(surface.drawnCards().map((c) => c.id)).toEqual(expect.arrayContaining([10, 11, 12]));
      // Leftmost member peek: left edge of the fan, not covered by later members.
      const left = surface.drawnCards().find((c) => c.id === 10)!;
      const peek = worldToScreen(surface.camera(), left.x + CARD_W * 0.1, left.y + CARD_H / 2);
      surface.pointerMove(peek.x, peek.y);
      expect(surface.hitCard(peek.x, peek.y)?.id).toBe(10);
      dispose();
    });
  });

  it("selectedId keeps its fan member raised at the end of drawnCards", () => {
    createRoot((dispose) => {
      const [selectedId, setSelectedId] = createSignal<number | null>(null);
      const cluster = logical(10, 400, 100, {
        cluster: 3,
        clusterMembers: [10, 11, 12],
      });
      const [cards] = createSignal<RenderCard[]>([cluster]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
        reducedMotion: () => true,
        selectedId,
      });
      const face = worldToScreen(surface.camera(), 400 + CARD_W / 2, 100 + CARD_H / 2);
      surface.pointerMove(face.x, face.y);
      setSelectedId(11);
      const drawn = surface.drawnCards();
      expect(drawn[drawn.length - 1]?.id).toBe(11);
      dispose();
    });
  });

  it("selecting a cluster member opens the fan even without prior hover", () => {
    createRoot((dispose) => {
      const [selectedId, setSelectedId] = createSignal<number | null>(null);
      const cluster = logical(10, 400, 100, {
        cluster: 3,
        clusterMembers: [10, 11, 12],
      });
      const [cards] = createSignal<RenderCard[]>([cluster]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
        reducedMotion: () => true,
        selectedId,
      });
      expect(surface.drawnCards()).toHaveLength(1);
      setSelectedId(11);
      const drawn = surface.drawnCards();
      expect(drawn.map((c) => c.id)).toEqual([10, 12, 11]); // 11 raised to end
      expect(drawn.find((c) => c.id === 12)?.x).toBeGreaterThan(400);
      dispose();
    });
  });

  it("drawnCards matches logical layout under reduced motion (paint ≠ hit list)", () => {
    createRoot((dispose) => {
      const card = logical(9, 40, 40);
      const [cards] = createSignal<RenderCard[]>([card]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
        reducedMotion: () => true,
      });
      expect(surface.drawnCards()[0]?.id).toBe(9);
      expect(surface.drawnCards()[0]?.x).toBe(40);
      const screen = worldToScreen(surface.camera(), 40 + CARD_W / 2, 40 + CARD_H / 2);
      expect(surface.hitCard(screen.x, screen.y)?.id).toBe(9);
      dispose();
    });
  });

  it("tryPinInspect prefers hand aux hover over board hit", () => {
    createRoot((dispose) => {
      const card = logical(2, 50, 50, { name: "Board Bolt" });
      const [cards] = createSignal<RenderCard[]>([card]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
        reducedMotion: () => true,
      });
      const screen = worldToScreen(surface.camera(), 50 + CARD_W / 2, 50 + CARD_H / 2);
      surface.notePointer(screen.x, screen.y);
      surface.setAuxHover("hand", "Hand Shock");
      // Hand bar sits over the battlefield; Alt-inspect should take the hand card under the cursor.
      expect(surface.tryPinInspect()).toEqual({ name: "Hand Shock", prepared: false });
      surface.clearInspect();
      expect(surface.inspectPin()).toBeNull();

      surface.setAuxHover("hand", null);
      expect(surface.tryPinInspect()).toEqual(expect.objectContaining({ name: "Board Bolt", objectId: 2 }));
      dispose();
    });
  });

  it("tryPinInspect prefers stack aux hover over board hit", () => {
    createRoot((dispose) => {
      const card = logical(2, 50, 50, { name: "Board Bolt" });
      const [cards] = createSignal<RenderCard[]>([card]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
        reducedMotion: () => true,
      });
      const screen = worldToScreen(surface.camera(), 50 + CARD_W / 2, 50 + CARD_H / 2);
      surface.notePointer(screen.x, screen.y);
      surface.setAuxHover("stack", "Stack Counterspell");
      expect(surface.tryPinInspect()).toEqual({ name: "Stack Counterspell", prepared: false });
      surface.clearInspect();

      surface.setAuxHover("stack", null);
      expect(surface.tryPinInspect()).toEqual(expect.objectContaining({ name: "Board Bolt", objectId: 2 }));
      dispose();
    });
  });

  it("tryPinInspect prefers hand over stack when both aux hovers are set", () => {
    createRoot((dispose) => {
      const [cards] = createSignal<RenderCard[]>([]);
      const surface = useTableSurface({
        me: () => 0,
        playerCount: () => 2,
        cards,
        handBarH: 210,
        initialSize: { x: 800, y: 600 },
        reducedMotion: () => true,
      });
      surface.setAuxHover("hand", "Hand Shock");
      surface.setAuxHover("stack", "Stack Counterspell");
      expect(surface.tryPinInspect()).toEqual({ name: "Hand Shock", prepared: false });
      dispose();
    });
  });

  describe("seedEntrances", () => {
    const cam = fitCamera({ x: 800, y: 600 }, 2, 210);
    const baseOpts = {
      moves: new Map<number, number>(),
      fromStack: new Set<number>(),
      stackLength: 0,
      size: { x: 800, y: 600 },
      camera: cam,
      me: 0,
      playerCount: 2,
      lastDrop: null as { x: number; y: number } | null,
    };

    it("places the next viewer battlefield permanent at lastDrop", () => {
      const anim = new Map([[1, { x: 100, y: 100 }]]);
      const { lastDrop } = seedEntrances(anim, [logical(1, 100, 100), logical(2, 200, 200)], {
        ...baseOpts,
        lastDrop: { x: 12, y: 34 },
      });
      expect(anim.get(2)).toEqual({ x: 12, y: 34 });
      expect(lastDrop).toBeNull();
    });

    it("seeds a new id at the zoneMoves predecessor position", () => {
      const anim = new Map([[1, { x: 80, y: 90 }]]);
      seedEntrances(anim, [logical(2, 300, 300)], {
        ...baseOpts,
        moves: new Map([[2, 1]]),
      });
      expect(anim.get(2)).toEqual({ x: 80, y: 90 });
    });

    it("seeds fromStack ids at the stack overlay world origin", () => {
      const anim = new Map([[1, { x: 100, y: 100 }]]);
      seedEntrances(anim, [logical(1, 100, 100), logical(2, 400, 400)], {
        ...baseOpts,
        fromStack: new Set([2]),
        stackLength: 0,
      });
      const scr = stackAimOrigin(800, 600, 1, stackPeekFor(1, 600, STACK_VERTICAL_RESERVED));
      const w = screenToWorld(cam, scr.x, scr.y);
      expect(anim.get(2)?.x).toBeCloseTo(w.x - CARD_W / 2, 5);
      expect(anim.get(2)?.y).toBeCloseTo(w.y - CARD_H / 2, 5);
    });

    it("seeds fromStack with compressed peek when the pile would overflow", () => {
      const tall = { x: 800, y: 280 };
      const shortCam = fitCamera(tall, 2, 210);
      const anim = new Map([[1, { x: 100, y: 100 }]]);
      const stackLength = 12;
      seedEntrances(anim, [logical(2, 400, 400)], {
        ...baseOpts,
        size: tall,
        camera: shortCam,
        fromStack: new Set([2]),
        stackLength,
      });
      const count = stackLength + 1;
      const peek = stackPeekFor(count, tall.y, STACK_VERTICAL_RESERVED);
      expect(peek).toBeLessThan(STACK_PEEK);
      const scr = stackAimOrigin(tall.x, tall.y, count, peek);
      const defaultScr = stackAimOrigin(tall.x, tall.y, count, STACK_PEEK);
      expect(scr.y).not.toBeCloseTo(defaultScr.y, 1);
      const w = screenToWorld(shortCam, scr.x, scr.y);
      expect(anim.get(2)?.x).toBeCloseTo(w.x - CARD_W / 2, 5);
      expect(anim.get(2)?.y).toBeCloseTo(w.y - CARD_H / 2, 5);
    });
  });
});
