// The camera transform: the single source of truth for pan/zoom, shared by the
// canvas render loop and the DOM hover overlay so both stay perfectly aligned.
//
// Model: a world point maps to a screen point by `screen = world * zoom + pan`.
// `pan` is in screen pixels; `zoom` is a scale factor. Everything here is pure —
// no DOM, no mutation — so it is trivially unit-testable and renderer-agnostic.

export interface Camera {
  panX: number;
  panY: number;
  zoom: number;
}

export interface Vec2 {
  x: number;
  y: number;
}

export const MIN_ZOOM = 0.2;
export const MAX_ZOOM = 5;

export function worldToScreen(cam: Camera, wx: number, wy: number): Vec2 {
  return { x: wx * cam.zoom + cam.panX, y: wy * cam.zoom + cam.panY };
}

export function screenToWorld(cam: Camera, sx: number, sy: number): Vec2 {
  return { x: (sx - cam.panX) / cam.zoom, y: (sy - cam.panY) / cam.zoom };
}

// Drag-pan by a screen-pixel delta.
export function panBy(cam: Camera, dxScreen: number, dyScreen: number): Camera {
  return { ...cam, panX: cam.panX + dxScreen, panY: cam.panY + dyScreen };
}

// Zoom by `factor` while keeping the world point currently under (sx, sy) fixed
// under the cursor — the behavior that makes wheel-zoom feel right.
export function zoomAt(cam: Camera, sx: number, sy: number, factor: number): Camera {
  const zoom = clamp(cam.zoom * factor, MIN_ZOOM, MAX_ZOOM);
  const world = screenToWorld(cam, sx, sy);
  // Solve pan so that `world` maps back to (sx, sy) at the new zoom.
  return { zoom, panX: sx - world.x * zoom, panY: sy - world.y * zoom };
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}
