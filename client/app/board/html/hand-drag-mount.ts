// Window-captured hand-bar drag: pointerdown on a playable tile starts a ghost that
// follows the cursor; pointerup dispatches HandDragEnded for the submodel play threshold.

import { Effect, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import type { ActionView, WireCost } from "~/wire/types";
import type { FaceData } from "../../domain/card-render/frame";
import { formatMessage } from "../../domain/i18n/message";
import { HandActionHovered, HandDragEnded, HandDragMoved, HandDragStarted } from "../messages";
import { clientToBoardPoint } from "./camera-gesture-mount";

type HandDragMessage =
  | typeof HandDragStarted.Type
  | typeof HandDragMoved.Type
  | typeof HandDragEnded.Type
  | typeof HandActionHovered.Type;

function boardGestureHost(): HTMLElement | null {
  if (typeof document === "undefined") return null;
  return document.querySelector<HTMLElement>('[data-testid="board-camera-gesture-mount"]');
}

/**
 * Map viewport client coordinates into board logical space for canvas drag paint.
 * HTML ghosts used `position:fixed` with client coords; the flight canvas paints in
 * board viewport space (stretched via CSS), so raw clientX/Y sit off-cursor.
 */
export function clientToHandDragPoint(clientX: number, clientY: number): { x: number; y: number } {
  const host = boardGestureHost();
  if (host == null) return { x: clientX, y: clientY };
  const mapped = clientToBoardPoint(host, clientX, clientY);
  if (mapped != null) return mapped;
  const rect = host.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return { x: clientX, y: clientY };
  const boardWidth = Number(host.dataset.boardWidth);
  const boardHeight = Number(host.dataset.boardHeight);
  const width = Number.isFinite(boardWidth) && boardWidth > 0 ? boardWidth : rect.width;
  const height = Number.isFinite(boardHeight) && boardHeight > 0 ? boardHeight : rect.height;
  return {
    x: ((clientX - rect.left) / rect.width) * width,
    y: ((clientY - rect.top) / rect.height) * height,
  };
}

function readBarZone(zone: string | undefined): "hand" | "command" | "graveyard" | "exile" | null {
  if (zone === "hand" || zone === "command" || zone === "graveyard" || zone === "exile") {
    return zone;
  }
  return null;
}

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
  const zone = readBarZone(hit.dataset.barZone) ?? readBarZone(action.section) ?? "hand";
  return HandDragStarted({
    action,
    name: hit.dataset.cardName ?? formatMessage(action.label),
    print: hit.dataset.cardPrint ?? "",
    face: readFace(hit),
    manaCost: readManaCost(hit.dataset.manaCost),
    kind: hit.dataset.objectKind,
    zone,
    x,
    y,
  });
}

/** The tile paints its face through a `data-face` host; the ghost flies that same face. */
function readFace(hit: HTMLElement): FaceData | undefined {
  const raw = hit.querySelector("[data-face]")?.getAttribute("data-face");
  if (raw == null) return undefined;
  try {
    return JSON.parse(raw) as FaceData;
  } catch {
    return undefined;
  }
}

function readManaCost(raw: string | undefined): WireCost {
  if (raw == null) return { generic: 0, colored: [0, 0, 0, 0, 0] };
  try {
    return JSON.parse(raw) as WireCost;
  } catch {
    return { generic: 0, colored: [0, 0, 0, 0, 0] };
  }
}

export function setHandDragGrabbingCursor(active: boolean): void {
  if (typeof document === "undefined") return;
  document.documentElement.style.cursor = active ? "grabbing" : "";
}

export function armHandDragGrabbingCursor(): () => void {
  setHandDragGrabbingCursor(true);
  return () => setHandDragGrabbingCursor(false);
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
          let clearGrab = () => setHandDragGrabbingCursor(false);

          const teardown = () => {
            if (move) window.removeEventListener("pointermove", move);
            if (up) window.removeEventListener("pointerup", up);
            if (cancel) window.removeEventListener("pointercancel", cancel);
            move = null;
            up = null;
            cancel = null;
            clearGrab();
            clearGrab = () => setHandDragGrabbingCursor(false);
          };

          const onPointerDown = (event: Event) => {
            if (!(event instanceof PointerEvent) || event.button !== 0) return;
            const hit = handDragTargetFromEvent(event.target);
            if (hit == null) return;
            event.preventDefault();
            teardown();
            const at = clientToHandDragPoint(event.clientX, event.clientY);
            const payload = readHandDragPayload(hit, at.x, at.y);
            if (payload == null) return;
            Queue.offerUnsafe(queue, payload);
            clearGrab = armHandDragGrabbingCursor();
            move = (ev) => {
              const point = clientToHandDragPoint(ev.clientX, ev.clientY);
              Queue.offerUnsafe(queue, HandDragMoved({ x: point.x, y: point.y }));
            };
            up = (ev) => {
              teardown();
              const point = clientToHandDragPoint(ev.clientX, ev.clientY);
              Queue.offerUnsafe(queue, HandDragEnded({ x: point.x, y: point.y }));
            };
            cancel = () => {
              teardown();
              Queue.offerUnsafe(queue, HandDragEnded({ x: at.x, y: at.y }));
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
            setHandDragGrabbingCursor(false);
            element.removeEventListener("pointerdown", handle.onPointerDown);
            element.removeEventListener("pointerover", handle.onPointerOver);
            element.removeEventListener("pointerout", handle.onPointerOut);
          }),
      );

      return yield* Effect.never;
    }),
  ),
);
