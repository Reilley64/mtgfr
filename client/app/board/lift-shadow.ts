import { shadowDrag } from "~/design-tokens.generated";

/**
 * Hand-drag / in-flight lift shadow.
 * Source of truth: design.tokens.json → --drop-shadow-drag.
 */
export const DROP_SHADOW_DRAG_CSS = shadowDrag.css;

/** Canvas mapping of --drop-shadow-drag (`0 <offsetY>px <blur>px …`). */
export const LIFT_SHADOW_OFFSET_Y = shadowDrag.offsetY;
export const LIFT_SHADOW_BLUR = shadowDrag.blur;
export const LIFT_SHADOW_COLOR = shadowDrag.color;
