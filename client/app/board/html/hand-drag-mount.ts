// Window-captured hand-bar drag: pointerdown on a playable tile starts a ghost that
// follows the cursor; pointerup dispatches HandDragEnded for the submodel play threshold.

import { Effect, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import type { ActionView, WireCost } from "~/wire/types";
import { HandActionHovered, HandDragEnded, HandDragMoved, HandDragStarted } from "../messages";

type HandDragMessage =
  | typeof HandDragStarted.Type
  | typeof HandDragMoved.Type
  | typeof HandDragEnded.Type
  | typeof HandActionHovered.Type;

export function handDragTargetFromEvent(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof Element)) return null;
  const hit = target.closest("[data-action-id]");
  return hit instanceof HTMLElement ? hit : null;
}

export function readHandDragPayload(hit: HTMLElement, x: number, y: number): typeof HandDragStarted.Type | null {
  const actionId = hit.dataset.actionId;
  if (actionId == null) return null;
  const actionJson = hit.dataset.actionPayload;
  if (actionJson == null) return null;
  let action: ActionView;
  try {
    action = JSON.parse(actionJson) as ActionView;
  } catch {
    return null;
  }
  return HandDragStarted({
    action,
    name: hit.dataset.cardName ?? action.label,
    print: hit.dataset.cardPrint ?? "",
    manaCost: readManaCost(hit.dataset.manaCost),
    kind: hit.dataset.objectKind,
    x,
    y,
  });
}

function readManaCost(raw: string | undefined): WireCost {
  if (raw == null) return { generic: 0, colored: [0, 0, 0, 0, 0] };
  try {
    return JSON.parse(raw) as WireCost;
  } catch {
    return { generic: 0, colored: [0, 0, 0, 0, 0] };
  }
}

export const MountHandBarDrag = Mount.defineStream(
  "MountHandBarDrag",
  HandDragStarted,
  HandDragMoved,
  HandDragEnded,
  HandActionHovered,
)((element) =>
  Stream.callback<HandDragMessage>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          if (!(element instanceof HTMLElement)) return null;

          let move: ((event: PointerEvent) => void) | null = null;
          let up: ((event: PointerEvent) => void) | null = null;
          let cancel: ((event: PointerEvent) => void) | null = null;

          const teardown = () => {
            if (move) window.removeEventListener("pointermove", move);
            if (up) window.removeEventListener("pointerup", up);
            if (cancel) window.removeEventListener("pointercancel", cancel);
            move = null;
            up = null;
            cancel = null;
          };

          const onPointerDown = (event: Event) => {
            if (!(event instanceof PointerEvent) || event.button !== 0) return;
            const hit = handDragTargetFromEvent(event.target);
            if (hit == null) return;
            event.preventDefault();
            teardown();
            const payload = readHandDragPayload(hit, event.clientX, event.clientY);
            if (payload == null) return;
            Queue.offerUnsafe(queue, payload);
            move = (ev) => Queue.offerUnsafe(queue, HandDragMoved({ x: ev.clientX, y: ev.clientY }));
            up = (ev) => {
              teardown();
              Queue.offerUnsafe(queue, HandDragEnded({ x: ev.clientX, y: ev.clientY }));
            };
            cancel = () => {
              teardown();
              Queue.offerUnsafe(queue, HandDragEnded({ x: event.clientX, y: event.clientY }));
            };
            window.addEventListener("pointermove", move);
            window.addEventListener("pointerup", up);
            window.addEventListener("pointercancel", cancel);
          };

          const onPointerOver = (event: Event) => {
            const hit = handDragTargetFromEvent(event.target);
            if (hit == null) return;
            const actionId = hit.dataset.actionId;
            if (actionId == null) return;
            Queue.offerUnsafe(queue, HandActionHovered({ actionId: Number(actionId) }));
          };

          const onPointerOut = (event: Event) => {
            const hit = handDragTargetFromEvent(event.target);
            if (hit == null) return;
            const related = event instanceof PointerEvent ? event.relatedTarget : null;
            if (related instanceof Element && hit.contains(related)) return;
            Queue.offerUnsafe(queue, HandActionHovered({ actionId: null }));
          };

          element.addEventListener("pointerdown", onPointerDown);
          element.addEventListener("pointerover", onPointerOver);
          element.addEventListener("pointerout", onPointerOut);

          return { onPointerDown, onPointerOver, onPointerOut, teardown };
        }),
        (handle) =>
          Effect.sync(() => {
            if (handle == null) return;
            handle.teardown();
            element.removeEventListener("pointerdown", handle.onPointerDown);
            element.removeEventListener("pointerover", handle.onPointerOver);
            element.removeEventListener("pointerout", handle.onPointerOut);
          }),
      );

      return yield* Effect.never;
    }),
  ),
);
