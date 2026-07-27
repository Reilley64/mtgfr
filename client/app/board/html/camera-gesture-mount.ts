import { Effect, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import { BoardCameraZoomed } from "../messages";

export type BoardPoint = { x: number; y: number };

type CameraGestureMessage = typeof BoardCameraZoomed.Type;

const WHEEL_ZOOM_SPEED = 0.0015;

export function clientToBoardPoint(element: HTMLElement, clientX: number, clientY: number): BoardPoint | null {
  const rect = element.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return null;
  if (clientX < rect.left || clientX > rect.left + rect.width) return null;
  if (clientY < rect.top || clientY > rect.top + rect.height) return null;

  const boardWidth = readPositiveNumber(element.dataset.boardWidth, rect.width);
  const boardHeight = readPositiveNumber(element.dataset.boardHeight, rect.height);
  return {
    x: ((clientX - rect.left) / rect.width) * boardWidth,
    y: ((clientY - rect.top) / rect.height) * boardHeight,
  };
}

export function wheelZoomFactor(deltaY: number): number {
  if (!Number.isFinite(deltaY)) return 1;
  return Math.exp(-deltaY * WHEEL_ZOOM_SPEED);
}

export function pinchDistance(a: BoardPoint, b: BoardPoint): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

export function pinchCenter(a: BoardPoint, b: BoardPoint): BoardPoint {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

export function pinchZoomFactor(previousDistance: number, nextDistance: number): number {
  if (!Number.isFinite(previousDistance) || previousDistance <= 0) return 1;
  if (!Number.isFinite(nextDistance) || nextDistance <= 0) return 1;
  return nextDistance / previousDistance;
}

function readPositiveNumber(raw: string | undefined, fallback: number): number {
  if (raw == null) return fallback;
  const value = Number(raw);
  if (!Number.isFinite(value) || value <= 0) return fallback;
  return value;
}

type PinchSnapshot = { center: BoardPoint; distance: number };

function touchPoint(element: HTMLElement, touch: Touch): BoardPoint | null {
  return clientToBoardPoint(element, touch.clientX, touch.clientY);
}

function pinchSnapshot(element: HTMLElement, event: TouchEvent): PinchSnapshot | null {
  if (event.touches.length !== 2) return null;
  const first = event.touches.item(0);
  const second = event.touches.item(1);
  if (first == null || second == null) return null;

  const a = touchPoint(element, first);
  const b = touchPoint(element, second);
  if (a == null || b == null) return null;

  return { center: pinchCenter(a, b), distance: pinchDistance(a, b) };
}

function isWheelEvent(event: Event): event is WheelEvent {
  return typeof WheelEvent !== "undefined" && event instanceof WheelEvent;
}

function isTouchEvent(event: Event): event is TouchEvent {
  return typeof TouchEvent !== "undefined" && event instanceof TouchEvent;
}

export const MountBoardCameraGesture = Mount.defineStream(
  "MountBoardCameraGesture",
  BoardCameraZoomed,
)((element) =>
  Stream.callback<CameraGestureMessage>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          if (!(element instanceof HTMLElement)) return null;

          let previousPinch: PinchSnapshot | null = null;

          const offerZoom = (point: BoardPoint, factor: number): void => {
            if (!Number.isFinite(factor) || factor <= 0) return;
            Queue.offerUnsafe(queue, BoardCameraZoomed({ x: point.x, y: point.y, factor }));
          };

          const onWheel = (event: Event): void => {
            if (!isWheelEvent(event)) return;
            const point = clientToBoardPoint(element, event.clientX, event.clientY);
            if (point == null) return;

            event.preventDefault();
            offerZoom(point, wheelZoomFactor(event.deltaY));
          };

          const onTouchStart = (event: Event): void => {
            if (!isTouchEvent(event)) return;
            previousPinch = pinchSnapshot(element, event);
            if (previousPinch == null) return;
            event.preventDefault();
          };

          const onTouchMove = (event: Event): void => {
            if (!isTouchEvent(event)) return;
            if (previousPinch == null) return;

            const nextPinch = pinchSnapshot(element, event);
            if (nextPinch == null) {
              previousPinch = null;
              return;
            }

            event.preventDefault();
            offerZoom(nextPinch.center, pinchZoomFactor(previousPinch.distance, nextPinch.distance));
            previousPinch = nextPinch;
          };

          const onTouchEnd = (event: Event): void => {
            if (!isTouchEvent(event)) return;
            previousPinch = pinchSnapshot(element, event);
          };

          window.addEventListener("wheel", onWheel, { passive: false });
          window.addEventListener("touchstart", onTouchStart, { passive: false });
          window.addEventListener("touchmove", onTouchMove, { passive: false });
          window.addEventListener("touchend", onTouchEnd, { passive: false });
          window.addEventListener("touchcancel", onTouchEnd, { passive: false });

          return { onWheel, onTouchStart, onTouchMove, onTouchEnd };
        }),
        (handle) =>
          Effect.sync(() => {
            if (handle == null) return;
            window.removeEventListener("wheel", handle.onWheel);
            window.removeEventListener("touchstart", handle.onTouchStart);
            window.removeEventListener("touchmove", handle.onTouchMove);
            window.removeEventListener("touchend", handle.onTouchEnd);
            window.removeEventListener("touchcancel", handle.onTouchEnd);
          }),
      );

      return yield* Effect.never;
    }),
  ),
);
