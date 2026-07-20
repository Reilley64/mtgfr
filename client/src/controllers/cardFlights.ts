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
  /** Live stack object source ids — used to GC orphan `kind:"stack"` flights. */
  stackSourceIds: Accessor<ReadonlySet<number>>;
  /** All visible object ids — hand-keyed flights stay only while the hand card still exists. */
  objectIds: Accessor<ReadonlySet<number>>;
  landPlays: Accessor<Map<number, number>>;
  fromStack: Accessor<Set<number>>;
  fromStackExit: Accessor<Set<number>>;
  stackEntrances: Accessor<Map<number, { from: number; controller: number }>>;
  /** New-object-id → predecessor id (permanent → spell for `permanent_entered`). */
  zoneMoves: Accessor<Map<number, number>>;
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

  /** Drop an in-flight card (e.g. staged cast cancel) so it cannot race the return animation. */
  const cancelFlight = (cardId: number) => {
    setFlights((prev) => {
      const next = new Map(prev);
      for (const [id, f] of prev) {
        if (id === cardId || f.fromCardId === cardId) next.delete(id);
      }
      return next;
    });
    setHandHidden((h) => {
      const n = new Set(h);
      n.delete(cardId);
      return n;
    });
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
      // Keep `from` in handHidden until settle — commander/bar faces stay dim under the flight.
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
      // Keep meta.from dimmed until settle (same as land rebind).
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
    const ents = deps.stackEntrances();
    const moves = deps.zoneMoves();
    const len = deps.stackLength();
    const originCount = Math.max(1, prevStackLen || len + 1);
    const peek = stackPeekFor(originCount, deps.size().y, STACK_VERTICAL_RESERVED);
    const from = stackAimOrigin(deps.size().x, deps.size().y, originCount, peek);
    let next = flights();
    let changed = false;
    for (const id of new Set([...resolved, ...exited])) {
      if (spawnedFromStack.has(id)) continue;

      const card = cards.find((c) => c.id === id);
      const scr = card ? worldToScreen(cam, card.x + CARD_W / 2, card.y + CARD_H / 2) : from;
      const target = { x: scr.x, y: scr.y, scale: 1 as const };

      // Absorb an unfinished stack flight for this resolve instead of drawing a second actor.
      // Hand→spell rebind uses stackEntrances; spell→permanent uses zoneMoves (permanent_entered.from).
      let stackFlight = next.get(id);
      if (stackFlight?.kind !== "stack") {
        const fromHand = ents.get(id)?.from;
        if (fromHand != null && next.get(fromHand)?.kind === "stack") {
          next = rebindFlightId(next, fromHand, id);
          stackFlight = next.get(id);
        }
      }
      if (stackFlight?.kind !== "stack") {
        const fromSpell = moves.get(id);
        if (fromSpell != null && next.get(fromSpell)?.kind === "stack") {
          next = rebindFlightId(next, fromSpell, id);
          stackFlight = next.get(id);
        }
      }
      // stackEntrances never rebound: flight still keyed by the consumed hand id.
      if (stackFlight?.kind !== "stack") {
        for (const [fid, f] of next) {
          if (f.kind !== "stack" || f.fromCardId !== fid) continue;
          next = rebindFlightId(next, fid, id);
          stackFlight = next.get(id);
          break;
        }
      }
      if (stackFlight?.kind === "stack") {
        next = new Map(next);
        next.set(
          id,
          retargetFlight(
            {
              ...stackFlight,
              id,
              kind: "from-stack",
              print: card?.print || stackFlight.print,
              name: card?.name || stackFlight.name,
            },
            target,
          ),
        );
        spawnedFromStack.add(id);
        changed = true;
        continue;
      }

      if (next.has(id)) continue;

      const f = spawnFlight({
        id,
        print: card?.print ?? "",
        name: card?.name ?? "",
        x: from.x,
        y: from.y,
        scale: stackFlightScale(cam.zoom),
        targetX: target.x,
        targetY: target.y,
        targetScale: target.scale,
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

  // If absorb misses the one delta that carries fromStack/zoneMoves, kind:"stack" flights are
  // retargeted at the stack aim forever (refresh clears them). Drop or promote orphans whose
  // ids are no longer on the live stack. Hand-keyed flights (fromCardId === id) stay only while
  // the hand object still exists or stackEntrances can still rebind them.
  createEffect(() => {
    const sources = deps.stackSourceIds();
    const objects = deps.objectIds();
    const ents = deps.stackEntrances();
    const moves = deps.zoneMoves();
    const cards = deps.cards();
    const cam = deps.camera();
    let next = flights();
    let changed = false;

    const spellToPermanent = new Map<number, number>();
    for (const [permanent, spell] of moves) spellToPermanent.set(spell, permanent);

    for (const [id, f] of [...next]) {
      if (f.kind !== "stack") continue;
      if (sources.has(id)) continue;

      // Still keyed by hand id — try late hand→spell rebind, else keep only while hand exists.
      if (f.fromCardId === id) {
        let spellId: number | undefined;
        for (const [spell, meta] of ents) {
          if (meta.from === id) {
            spellId = spell;
            break;
          }
        }
        if (spellId != null) {
          next = rebindFlightId(next, id, spellId);
          const rebound = next.get(spellId);
          if (rebound) {
            const len = Math.max(1, deps.stackLength());
            const peek = stackPeekFor(len, deps.size().y, STACK_VERTICAL_RESERVED);
            const to = stackAimOrigin(deps.size().x, deps.size().y, len, peek);
            next = new Map(next);
            next.set(
              spellId,
              retargetFlight({ ...rebound, kind: "stack" }, { x: to.x, y: to.y, scale: stackFlightScale(cam.zoom) }),
            );
            changed = true;
          }
          continue;
        }
        if (objects.has(id)) continue;
        // No object snapshot yet (boot / tests) — don't invent a disappearance.
        if (objects.size === 0) continue;
        // Hand object consumed and never rebound — drop the ghost (same end as refresh).
        next = new Map(next);
        next.delete(id);
        spawnedFromStack.delete(id);
        setHandHidden((h) => {
          const n = new Set(h);
          n.delete(id);
          return n;
        });
        changed = true;
        continue;
      }

      const permanentId = spellToPermanent.get(id);
      const card = permanentId != null ? cards.find((c) => c.id === permanentId) : undefined;
      if (permanentId != null && card) {
        next = rebindFlightId(next, id, permanentId);
        const scr = worldToScreen(cam, card.x + CARD_W / 2, card.y + CARD_H / 2);
        const rebound = next.get(permanentId);
        if (rebound) {
          next = new Map(next);
          next.set(
            permanentId,
            retargetFlight(
              {
                ...rebound,
                id: permanentId,
                kind: "from-stack",
                print: card.print || rebound.print,
                name: card.name || rebound.name,
              },
              { x: scr.x, y: scr.y, scale: 1 },
            ),
          );
          spawnedFromStack.add(permanentId);
          changed = true;
        }
        continue;
      }

      next = new Map(next);
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
      changed = true;
    }

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
    cancelFlight,
  };
}
