import { createRoot, createSignal } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useCardFlights } from "~/controllers/cardFlights";
import type { RenderCard } from "~/layout";
import { fitCamera } from "~/lib/interaction";

afterEach(() => {
  vi.unstubAllGlobals();
});

function mountFlights(opts?: {
  landPlays?: Map<number, number>;
  stackEntrances?: Map<number, { from: number; controller: number }>;
  fromStack?: Set<number>;
  zoneMoves?: Map<number, number>;
  cards?: RenderCard[];
  stackSourceIds?: Set<number>;
}) {
  vi.stubGlobal(
    "requestAnimationFrame",
    vi.fn(() => 1),
  );
  vi.stubGlobal("cancelAnimationFrame", vi.fn());
  let dispose!: () => void;
  let api!: ReturnType<typeof useCardFlights>;
  const [landPlays, setLandPlays] = createSignal(opts?.landPlays ?? new Map<number, number>());
  const [stackEntrances, setStackEntrances] = createSignal(
    opts?.stackEntrances ?? new Map<number, { from: number; controller: number }>(),
  );
  const [fromStack, setFromStack] = createSignal(opts?.fromStack ?? new Set<number>());
  const [fromStackExit] = createSignal(new Set<number>());
  const [zoneMoves, setZoneMoves] = createSignal(opts?.zoneMoves ?? new Map<number, number>());
  const [cards, setCards] = createSignal<RenderCard[]>(opts?.cards ?? []);
  const [stackLength, setStackLength] = createSignal(0);
  const [stackSourceIds, setStackSourceIds] = createSignal(opts?.stackSourceIds ?? new Set<number>());
  createRoot((d) => {
    dispose = d;
    const size = () => ({ x: 800, y: 600 });
    api = useCardFlights({
      camera: () => fitCamera(size(), 2, 128),
      size,
      cards,
      stackLength,
      stackSourceIds,
      landPlays,
      fromStack,
      fromStackExit,
      stackEntrances,
      zoneMoves,
      reducedMotion: () => true, // snap so settle is deterministic when we step
      onTick: () => {},
    });
  });
  return {
    api,
    dispose,
    setLandPlays,
    setStackEntrances,
    setFromStack,
    setZoneMoves,
    setCards,
    setStackLength,
    setStackSourceIds,
  };
}

describe("useCardFlights", () => {
  it("cancelFlight removes the in-flight card and undims the hand slot", () => {
    const { api, dispose } = mountFlights();
    api.spawnFromHand({
      cardId: 9,
      print: "p",
      name: "Shock",
      screen: { x: 100, y: 500 },
      kind: "stack",
    });
    expect(api.flights().some((f) => f.id === 9)).toBe(true);
    expect(api.handHidden().has(9)).toBe(true);

    api.cancelFlight(9);

    expect(api.flights().some((f) => f.id === 9)).toBe(false);
    expect(api.handHidden().has(9)).toBe(false);
    dispose();
  });

  it("keeps the source hand slot dimmed after stack entrance rebind until the flight settles", () => {
    const { api, dispose, setStackEntrances, setStackLength, setStackSourceIds } = mountFlights();
    api.spawnFromHand({
      cardId: 9,
      print: "p",
      name: "Bear",
      screen: { x: 100, y: 500 },
      kind: "stack",
    });
    expect(api.handHidden().has(9)).toBe(true);

    setStackLength(1);
    setStackSourceIds(new Set([42]));
    setStackEntrances(new Map([[42, { from: 9, controller: 0 }]]));

    // Rebind moves ownership to spell id 42, but the command/hand source must stay dimmed.
    expect(api.flights().some((f) => f.id === 42)).toBe(true);
    expect(api.handHidden().has(9)).toBe(true);
    dispose();
  });

  it("converts an unfinished stack flight into from-stack instead of spawning a second actor", () => {
    const { api, dispose, setStackEntrances, setFromStack, setStackLength, setStackSourceIds, setCards } =
      mountFlights();
    api.spawnFromHand({
      cardId: 9,
      print: "p",
      name: "Bear",
      screen: { x: 100, y: 500 },
      kind: "stack",
    });
    setStackLength(1);
    setStackSourceIds(new Set([42]));
    setStackEntrances(new Map([[42, { from: 9, controller: 0 }]]));
    expect(api.flights().filter((f) => f.kind === "stack" || f.kind === "from-stack")).toHaveLength(1);

    setCards([
      {
        id: 42,
        name: "Bear",
        print: "p",
        x: 200,
        y: 200,
        w: 96,
        h: 134,
        zone: 2,
        owner: 0,
        controller: 0,
        kind: "creature",
        tapped: false,
        prepared: false,
        faceDown: false,
      } as RenderCard,
    ]);
    setStackLength(0);
    setStackSourceIds(new Set<number>());
    setFromStack(new Set([42]));

    const actors = api.flights().filter((f) => f.id === 42 || f.fromCardId === 9);
    expect(actors).toHaveLength(1);
    expect(actors[0]?.kind).toBe("from-stack");
    dispose();
  });

  it("absorbs a stack-bound spell flight when permanent_entered mints a new permanent id", () => {
    // Real cast path: hand id → spell id (stackEntrances) → permanent id (permanent_entered.from
    // is the spell). Absorb must rebind spell→permanent; otherwise a ghost kind:"stack" actor
    // stays aimed at the stack while a second from-stack flies to the battlefield (snap-back).
    const bear = {
      id: 60,
      name: "Bear",
      print: "p",
      x: 200,
      y: 200,
      w: 96,
      h: 134,
      zone: 2,
      owner: 0,
      controller: 0,
      kind: "creature",
      tapped: false,
      prepared: false,
      faceDown: false,
    } as RenderCard;
    const { api, dispose, setStackEntrances, setFromStack, setStackLength, setStackSourceIds, setCards, setZoneMoves } =
      mountFlights();
    api.spawnFromHand({
      cardId: 9,
      print: "p",
      name: "Bear",
      screen: { x: 100, y: 500 },
      kind: "stack",
    });
    setStackLength(1);
    setStackSourceIds(new Set([42]));
    setStackEntrances(new Map([[42, { from: 9, controller: 0 }]]));
    expect(api.flights().some((f) => f.id === 42 && f.kind === "stack")).toBe(true);

    setCards([bear]);
    setStackLength(0);
    setStackSourceIds(new Set<number>());
    setZoneMoves(new Map([[60, 42]])); // permanent ← spell (store zoneMoves)
    setFromStack(new Set([60]));

    const actors = api.flights().filter((f) => f.kind === "stack" || f.kind === "from-stack");
    expect(actors).toHaveLength(1);
    expect(actors[0]?.id).toBe(60);
    expect(actors[0]?.kind).toBe("from-stack");
    expect(api.flights().some((f) => f.kind === "stack")).toBe(false);
    dispose();
  });

  it("drops orphan stack ghosts after resolve when fromStack provenance was already cleared", () => {
    // Refresh fixes stuck stack cards because it wipes the flight map. If absorb misses the one
    // delta that carries fromStack/zoneMoves, the frame loop keeps retargeting kind:"stack" at
    // the stack aim forever — slide/snap-back when the next spell is cast.
    const bear = {
      id: 60,
      name: "Bear",
      print: "p",
      x: 200,
      y: 200,
      w: 96,
      h: 134,
      zone: 2,
      owner: 0,
      controller: 0,
      kind: "creature",
      tapped: false,
      prepared: false,
      faceDown: false,
    } as RenderCard;
    const { api, dispose, setStackEntrances, setStackLength, setStackSourceIds, setCards, setFromStack, setZoneMoves } =
      mountFlights();
    api.spawnFromHand({
      cardId: 9,
      print: "p",
      name: "Bear",
      screen: { x: 100, y: 500 },
      kind: "stack",
    });
    setStackLength(1);
    setStackSourceIds(new Set([42]));
    setStackEntrances(new Map([[42, { from: 9, controller: 0 }]]));
    expect(api.flights().some((f) => f.id === 42 && f.kind === "stack")).toBe(true);

    // Missed absorb: next delta clears provenance while the ghost spell flight remains.
    setStackEntrances(new Map());
    setZoneMoves(new Map());
    setFromStack(new Set<number>());
    setCards([bear]);
    setStackLength(0);
    setStackSourceIds(new Set<number>());

    expect(api.flights().some((f) => f.kind === "stack")).toBe(false);
    dispose();
  });

  it("keeps a pre-rebind hand→stack flight while the cast is still in flight", () => {
    const { api, dispose, setStackSourceIds } = mountFlights();
    api.spawnFromHand({
      cardId: 9,
      print: "p",
      name: "Bear",
      screen: { x: 100, y: 500 },
      kind: "stack",
    });
    setStackSourceIds(new Set<number>()); // spell_cast not applied yet
    expect(api.flights().some((f) => f.id === 9 && f.kind === "stack")).toBe(true);
    dispose();
  });
});
