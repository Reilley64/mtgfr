// Solid controller for the canvas flight layer (ADR 0035): spawn on play commit, retarget on
// delta bind / layout, step each frame, expose hide sets for hand + canvas resting faces.

import { type Accessor, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import { CARD_H, CARD_W, type RenderCard } from "~/layout";
import { STACK_VERTICAL_RESERVED, stackAimOrigin, stackPeekFor } from "~/lib/boardDraw";
import { type Camera, worldToScreen } from "~/lib/camera";
import {
  type CardFlight,
  flightSettled,
  handFlightScale,
  rebindFlightId,
  retargetFlight,
  spawnFlight,
  stackFlightScale,
  stepFlights,
} from "~/lib/cardFlight";

export type CardFlightsDeps = {
  camera: Accessor<Camera>;
  size: Accessor<{ x: number; y: number }>;
  cards: Accessor<RenderCard[]>;
  stackLength: Accessor<number>;
  landPlays: Accessor<Map<number, number>>;
  fromStack: Accessor<Set<number>>;
  fromStackExit: Accessor<Set<number>>;
  stackEntrances: Accessor<Map<number, { from: number; controller: number }>>;
  reducedMotion: Accessor<boolean>;
  onTick: () => void;
};

export function useCardFlights(deps: CardFlightsDeps) {
  const [flights, setFlights] = createSignal(new Map<number, CardFlight>());
  let rafId = 0;
  let lastFrame = 0;
  let prevStackLen = 0;
  /** Resolve/exit ids we've already spawned so the effect doesn't loop. */
  const spawnedFromStack = new Set<number>();
  const [handHidden, setHandHidden] = createSignal(new Set<number>());

  const flightList = createMemo(() => [...flights().values()]);
  const hideCardIds = createMemo(() => {
    const hide = new Set<number>();
    for (const f of flights().values()) {
      if (f.phase === "flying") hide.add(f.id);
    }
    return hide;
  });
  const flightOwnedIds = createMemo(() => new Set(flights().keys()));

  const kickRaf = () => {
    if (rafId) return;
    lastFrame = performance.now();
    rafId = requestAnimationFrame(frame);
  };

  const frame = (now: number) => {
    const dt = now - lastFrame;
    lastFrame = now;
    const cam = deps.camera();
    const cards = deps.cards();
    const byId = new Map(cards.map((c) => [c.id, c]));
    let next = new Map(flights());

    for (const [id, f] of next) {
      if (f.phase === "settled") continue;
      if (f.kind === "stack" && !byId.has(id)) {
        const len = deps.stackLength();
        const peek = stackPeekFor(Math.max(1, len || 1), deps.size().y, STACK_VERTICAL_RESERVED);
        const to = stackAimOrigin(deps.size().x, deps.size().y, Math.max(1, len || 1), peek);
        next.set(id, retargetFlight(f, { x: to.x, y: to.y, scale: stackFlightScale(cam.zoom) }));
        continue;
      }
      const card = byId.get(id);
      if (card) {
        const scr = worldToScreen(cam, card.x + card.w / 2, card.y + card.h / 2);
        next.set(id, retargetFlight(f, { x: scr.x, y: scr.y, scale: 1 }));
      }
    }

    const stepped = stepFlights(next, dt, deps.reducedMotion());
    next = stepped.flights;
    for (const [id, f] of [...next]) {
      if (!flightSettled(f)) continue;
      next.delete(id);
      spawnedFromStack.delete(id);
      const fromId = f.fromCardId;
      if (fromId != null) {
        setHandHidden((h) => {
          const n = new Set(h);
          n.delete(fromId);
          return n;
        });
      }
    }
    setFlights(next);
    deps.onTick();
    rafId = next.size === 0 ? 0 : requestAnimationFrame(frame);
  };

  const spawnFromHand = (opts: {
    cardId: number;
    print: string;
    name: string;
    screen: { x: number; y: number };
    kind: "battlefield" | "stack";
  }) => {
    const cam = deps.camera();
    const startScale = handFlightScale(cam.zoom);
    let targetX = opts.screen.x;
    let targetY = opts.screen.y;
    let targetScale = 1;
    if (opts.kind === "stack") {
      const len = deps.stackLength();
      const peek = stackPeekFor(len + 1, deps.size().y, STACK_VERTICAL_RESERVED);
      const to = stackAimOrigin(deps.size().x, deps.size().y, len + 1, peek);
      targetX = to.x;
      targetY = to.y;
      targetScale = stackFlightScale(cam.zoom);
    }
    const f = spawnFlight({
      id: opts.cardId,
      print: opts.print,
      name: opts.name,
      x: opts.screen.x,
      y: opts.screen.y,
      scale: startScale,
      targetX,
      targetY,
      targetScale,
      kind: opts.kind,
      fromCardId: opts.cardId,
    });
    setFlights((prev) => new Map(prev).set(opts.cardId, f));
    setHandHidden((h) => new Set(h).add(opts.cardId));
    kickRaf();
  };

  createEffect(() => {
    const plays = deps.landPlays();
    let next = flights();
    let changed = false;
    for (const [permanent, from] of plays) {
      if (next.has(permanent) || !next.has(from)) continue;
      next = rebindFlightId(next, from, permanent);
      const f = next.get(permanent);
      if (f) next.set(permanent, { ...f, kind: "battlefield" });
      changed = true;
      setHandHidden((h) => {
        const n = new Set(h);
        n.delete(from);
        return n;
      });
    }
    if (changed) {
      setFlights(next);
      kickRaf();
    }
  });

  createEffect(() => {
    const ents = deps.stackEntrances();
    const cam = deps.camera();
    let next = flights();
    let changed = false;
    for (const [spell, meta] of ents) {
      if (next.has(spell) || !next.has(meta.from)) continue;
      next = rebindFlightId(next, meta.from, spell);
      const f = next.get(spell);
      if (f) {
        const len = Math.max(1, deps.stackLength());
        const peek = stackPeekFor(len, deps.size().y, STACK_VERTICAL_RESERVED);
        const to = stackAimOrigin(deps.size().x, deps.size().y, len, peek);
        next.set(
          spell,
          retargetFlight({ ...f, kind: "stack" }, { x: to.x, y: to.y, scale: stackFlightScale(cam.zoom) }),
        );
      }
      changed = true;
      setHandHidden((h) => {
        const n = new Set(h);
        n.delete(meta.from);
        return n;
      });
    }
    if (changed) {
      setFlights(next);
      kickRaf();
    }
  });

  createEffect(() => {
    const resolved = deps.fromStack();
    const exited = deps.fromStackExit();
    const cam = deps.camera();
    const cards = deps.cards();
    const len = deps.stackLength();
    const originCount = Math.max(1, prevStackLen || len + 1);
    const peek = stackPeekFor(originCount, deps.size().y, STACK_VERTICAL_RESERVED);
    const from = stackAimOrigin(deps.size().x, deps.size().y, originCount, peek);
    let next = flights();
    let changed = false;
    for (const id of new Set([...resolved, ...exited])) {
      if (next.has(id) || spawnedFromStack.has(id)) continue;
      const card = cards.find((c) => c.id === id);
      const scr = card ? worldToScreen(cam, card.x + CARD_W / 2, card.y + CARD_H / 2) : from;
      const f = spawnFlight({
        id,
        print: card?.print ?? "",
        name: card?.name ?? "",
        x: from.x,
        y: from.y,
        scale: stackFlightScale(cam.zoom),
        targetX: scr.x,
        targetY: scr.y,
        targetScale: 1,
        kind: "from-stack",
      });
      next = new Map(next);
      next.set(id, f);
      spawnedFromStack.add(id);
      changed = true;
    }
    prevStackLen = len;
    if (changed) {
      setFlights(next);
      kickRaf();
    }
  });

  onCleanup(() => {
    if (rafId) cancelAnimationFrame(rafId);
  });

  return {
    flights: flightList,
    hideCardIds,
    flightOwnedIds,
    handHidden,
    spawnFromHand,
  };
}
