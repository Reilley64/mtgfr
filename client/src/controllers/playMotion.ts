// PlayMotion (client-game-board-and-interaction spec): canvas flight layer for play-committed motion — spawn on play
// commit, retarget on delta bind / layout, absorb/orphan in one ordered pass, expose hide
// sets for hand + canvas resting faces. TableSurface keeps non-play entrance glides.

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

export type PlayMotionDeps = {
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

export function usePlayMotion(deps: PlayMotionDeps) {
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
  const ownedIds = createMemo(() => new Set(flights().keys()));

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
      // Do not clear spawnedFromStack here. Resolve provenance (fromStack) stays live until the
      // next delta; clearing the guard lets absorb re-spawn a from-stack actor at the stack aim
      // after the real flight already settled (Elvish Mystic snap-back / stuck stack ghost).
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
  const cancel = (cardId: number) => {
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

  // One ordered pass: land rebind → stack-entrance rebind → from-stack absorb → orphan GC.
  // Absorb before orphan so #39's pendingResolve race stays a module invariant.
  createEffect(() => {
    const plays = deps.landPlays();
    const ents = deps.stackEntrances();
    const resolved = deps.fromStack();
    const exited = deps.fromStackExit();
    const moves = deps.zoneMoves();
    const sources = deps.stackSourceIds();
    const objects = deps.objectIds();
    const cards = deps.cards();
    const cam = deps.camera();
    const len = deps.stackLength();
    const pendingResolve = resolved.size > 0 || exited.size > 0;

    let next = flights();
    let changed = false;

    for (const [permanent, from] of plays) {
      if (next.has(permanent) || !next.has(from)) continue;
      next = rebindFlightId(next, from, permanent);
      const f = next.get(permanent);
      if (f) next.set(permanent, { ...f, kind: "battlefield" });
      changed = true;
    }

    for (const [spell, meta] of ents) {
      if (next.has(spell) || !next.has(meta.from)) continue;
      next = rebindFlightId(next, meta.from, spell);
      const f = next.get(spell);
      if (f) {
        const stackLen = Math.max(1, len);
        const peek = stackPeekFor(stackLen, deps.size().y, STACK_VERTICAL_RESERVED);
        const to = stackAimOrigin(deps.size().x, deps.size().y, stackLen, peek);
        next.set(
          spell,
          retargetFlight({ ...f, kind: "stack" }, { x: to.x, y: to.y, scale: stackFlightScale(cam.zoom) }),
        );
      }
      changed = true;
    }

    const originCount = Math.max(1, prevStackLen || len + 1);
    const peek = stackPeekFor(originCount, deps.size().y, STACK_VERTICAL_RESERVED);
    const fromAim = stackAimOrigin(deps.size().x, deps.size().y, originCount, peek);
    for (const id of new Set([...resolved, ...exited])) {
      if (spawnedFromStack.has(id)) continue;

      const card = cards.find((c) => c.id === id);
      const scr = card ? worldToScreen(cam, card.x + CARD_W / 2, card.y + CARD_H / 2) : fromAim;
      const target = { x: scr.x, y: scr.y, scale: 1 as const };

      // Absorb ladder (preserve all rungs): entrances → zoneMoves → hand-keyed singleton → print/name.
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
      if (stackFlight?.kind !== "stack") {
        const handKeyed = [...next].filter(([fid, f]) => f.kind === "stack" && f.fromCardId === fid);
        let pick = handKeyed.length === 1 ? handKeyed[0] : undefined;
        if (!pick && card) {
          pick = handKeyed.find(
            ([, f]) => (card.print !== "" && f.print === card.print) || (card.name !== "" && f.name === card.name),
          );
        }
        if (pick) {
          next = rebindFlightId(next, pick[0], id);
          stackFlight = next.get(id);
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
        x: fromAim.x,
        y: fromAim.y,
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

    const spellToPermanent = new Map<number, number>();
    for (const [permanent, spell] of moves) spellToPermanent.set(spell, permanent);

    for (const [id, f] of [...next]) {
      if (f.kind !== "stack") continue;
      if (sources.has(id)) continue;

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
            const stackLen = Math.max(1, len);
            const stackPeek = stackPeekFor(stackLen, deps.size().y, STACK_VERTICAL_RESERVED);
            const to = stackAimOrigin(deps.size().x, deps.size().y, stackLen, stackPeek);
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
        if (objects.size === 0) continue;
        if (pendingResolve) continue;
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
    ownedIds,
    handHidden,
    spawnFromHand,
    cancel,
  };
}
