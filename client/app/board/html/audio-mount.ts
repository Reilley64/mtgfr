// Board audio + coaching timers: table-feel cues, attention pings, hint auto-hide, priority watch.

import { Queue, Stream } from "effect";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Mount from "foldkit/mount";
import { SPECTATOR_VIEWER } from "~/spectator";
import {
  playAttentionPriority,
  playAttentionYourTurn,
  playTableFeelDamage,
  playTableFeelLand,
  playTableFeelResolve,
  playTableFeelStack,
} from "~/tableAudio";
import { watchElapsed } from "~/watch";
import { ArtLoaded, HintAutoHidden, PriorityElapsed } from "../messages";

function readData(el: Element, name: string): string | null {
  return el.getAttribute(`data-${name}`);
}

function readBool(el: Element, name: string): boolean {
  return readData(el, name) === "1";
}

function readNumber(el: Element, name: string, fallback: number): number {
  const raw = readData(el, name);
  if (raw == null) return fallback;
  const n = Number(raw);
  return Number.isFinite(n) ? n : fallback;
}

/** Play table-feel + attention audio from `data-*` on the board root. */
export const MountBoardAudio = Mount.define(
  "MountBoardAudio",
  ArtLoaded,
)((element) =>
  Effect.gen(function* () {
    yield* Effect.acquireRelease(
      Effect.sync(() => {
        if (!(element instanceof HTMLElement)) return null;

        let prevSeq: number | undefined;
        let prevAttention: { turn: boolean; priority: boolean } | undefined;

        const tick = (): void => {
          const seq = readNumber(element, "game-seq", 0);
          const viewer = readNumber(element, "viewer", SPECTATOR_VIEWER);
          const active = readNumber(element, "active-player", 0);
          const priority = readNumber(element, "priority", 0);
          const canHear = readBool(element, "can-hear-attention");

          if (prevSeq !== undefined && seq !== prevSeq) {
            if (readBool(element, "feel-land")) playTableFeelLand();
            if (readBool(element, "feel-stack")) playTableFeelStack();
            if (readBool(element, "feel-resolve")) playTableFeelResolve();
            if (readBool(element, "feel-damage")) playTableFeelDamage();
          }
          prevSeq = seq;

          const turn = active === viewer;
          const yoursPriority = priority === viewer;
          if (canHear && prevAttention !== undefined) {
            const gainedTurn = turn && !prevAttention.turn;
            const gainedPriority = yoursPriority && !prevAttention.priority;
            if (gainedTurn) playAttentionYourTurn();
            else if (gainedPriority) playAttentionPriority();
          }
          prevAttention = { turn, priority: yoursPriority };
        };

        tick();
        const observer = new MutationObserver(tick);
        observer.observe(element, {
          attributes: true,
          attributeFilter: [
            "data-game-seq",
            "data-viewer",
            "data-active-player",
            "data-priority",
            "data-can-hear-attention",
            "data-feel-land",
            "data-feel-stack",
            "data-feel-resolve",
            "data-feel-damage",
          ],
        });
        return observer;
      }),
      (observer) =>
        Effect.sync(() => {
          observer?.disconnect();
        }),
    );

    return ArtLoaded();
  }),
);

/** Hide the coaching hint after 12s while it remains visible. */
export const MountHintAutoHide = Mount.defineStream(
  "MountHintAutoHide",
  HintAutoHidden,
)((el) =>
  Stream.callback<typeof HintAutoHidden.Type>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          if (!(el instanceof HTMLElement)) return null;
          if (readData(el, "hint-visible") !== "1") return null;
          const timer = window.setTimeout(() => Queue.offerUnsafe(queue, HintAutoHidden()), 12_000);
          return timer;
        }),
        (timer) =>
          Effect.sync(() => {
            if (timer != null) window.clearTimeout(timer);
          }),
      );
      return yield* Effect.never;
    }),
  ),
);

/** Drive the priority-watch shame clock; restarts when `data-priority` changes. */
export const MountPriorityWatch = Mount.defineStream(
  "MountPriorityWatch",
  PriorityElapsed,
)((el) =>
  Stream.callback<typeof PriorityElapsed.Type>((queue) =>
    Effect.gen(function* () {
      let fiber: Fiber.Fiber<void, never> | null = null;

      const restart = (): void => {
        if (fiber != null) Effect.runFork(Fiber.interrupt(fiber));
        fiber = Effect.runFork(
          watchElapsed((seconds) => {
            Queue.offerUnsafe(queue, PriorityElapsed({ seconds }));
          }),
        );
      };

      yield* Effect.acquireRelease(
        Effect.sync(() => {
          if (!(el instanceof HTMLElement)) return null;
          restart();
          const observer = new MutationObserver(() => restart());
          observer.observe(el, { attributes: true, attributeFilter: ["data-priority"] });
          return observer;
        }),
        (observer) =>
          Effect.sync(() => {
            observer?.disconnect();
            if (fiber != null) Effect.runFork(Fiber.interrupt(fiber));
          }),
      );

      return yield* Effect.never;
    }),
  ),
);
