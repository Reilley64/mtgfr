// Canvas board drawing: seat bands, cards, avatars, combat/target arrows.
// Orchestration is {@link buildBoardScene} + {@link paintBoardScene}; this module
// keeps `DrawCtx` / `draw()` for the Board paint loop and re-exports.

import { type RenderCard } from "~/layout";
import { type ArrowAnimState, emptyArrowAnimState } from "~/lib/boardArrows";
import type { Vec } from "~/lib/boardPaintPrims";
import { buildBoardScene, paintBoardScene } from "~/lib/boardScene";
import type { Camera } from "~/lib/camera";
import type { CardFlight } from "~/lib/cardFlight";
import type { ImageCache } from "~/lib/imageCache";
import type { VisibleState, WireAttack, WireBlock } from "~/wire/types";

export type { StackPresentation } from "~/lib/stackLayout";
export {
  STACK_ANCHOR_FROM_RIGHT,
  STACK_CARD_W,
  STACK_EXPAND_COUNT,
  STACK_HORIZONTAL_MARGIN,
  STACK_OVERLAY_RIGHT,
  STACK_PEEK,
  STACK_STRIP_MIN_PEEK,
  STACK_VERTICAL_RESERVED,
  shouldAutoCollapseStackExpand,
  stackAimOrigin,
  stackCardH,
  stackExpandAvailable,
  stackFullPerRow,
  stackPeekFor,
  stackPresentation,
  stackStripFits,
  stackStripPeek,
  stagingAimFrom,
} from "~/lib/stackLayout";

export { ARROW_DRAW_MS, arrowDrawProgress, emptyArrowAnimState, type ArrowAnimState } from "~/lib/boardArrows";
export { RESPONSE_COLOR, TARGET_COLOR } from "~/lib/boardPaintPrims";
export { buildBoardScene, paintBoardScene, type BoardScene, type BuildBoardSceneInput } from "~/lib/boardScene";

export interface DrawCtx {
  cam: Camera;
  cards: RenderCard[];
  cache: ImageCache;
  me: number;
  active: number;
  priority: number;
  viewer: number;
  count: number;
  players: VisibleState["players"];
  combat: VisibleState["combat"];
  attackers: WireAttack[];
  blocks: WireBlock[];
  aiming: boolean;
  targetObjects: ReadonlySet<number>;
  targetPlayers: ReadonlySet<number>;
  canvasDrag: RenderCard | null;
  cursor: Vec;
  avatarScreenPositions: Record<number, Vec>;
  stepIdx: number;
  /** Selected permanent for the activation radial. */
  selectedId: number | null;
  /** Permanents that would auto-tap to pay the hovered action's mana. */
  paymentObjects: ReadonlySet<number>;
  /** True while this viewer holds instant-speed priority (dim non-usable permanents). */
  stackResponseFocus: boolean;
  /** Battlefield activates + mana sources to keep bright during instant priority. */
  responseObjects: ReadonlySet<number>;
  /** Screen-space origin for the targeting arrow while aiming (stack-card center). */
  aimFrom: Vec | null;
  /** In-flight cards (ADR 0035) — drawn in screen space above board cards. */
  flights?: readonly CardFlight[];
  /** Canvas card ids still owned by a flight — skip drawing the resting face. */
  hideCardIds?: ReadonlySet<number>;
}

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Paint one frame. When `arrowState` is omitted, a module singleton is used
 * (tests / simple callers). Board passes its own state so births survive across effects.
 */
export function draw(ctx: CanvasRenderingContext2D, d: DrawCtx, arrowState?: ArrowAnimState): boolean {
  const state = arrowState ?? fallbackArrowState;
  const dpr = window.devicePixelRatio || 1;
  const w = ctx.canvas.width / dpr;
  const h = ctx.canvas.height / dpr;
  const scene = buildBoardScene({
    ...d,
    viewportW: w,
    viewportH: h,
    nowMs: performance.now(),
    reducedMotion: prefersReducedMotion(),
  });
  const result = paintBoardScene(ctx, scene, d.cache, state);
  if (!arrowState) {
    fallbackArrowState = result.arrowState;
  } else {
    arrowState.born = result.arrowState.born;
  }
  return result.stillAnimating;
}

let fallbackArrowState: ArrowAnimState = emptyArrowAnimState();
