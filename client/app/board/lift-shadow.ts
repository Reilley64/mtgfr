/**
 * Hand-drag / in-flight lift shadow.
 * Source of truth: design.tokens.json → --drop-shadow-drag.
 */
export const DROP_SHADOW_DRAG_CSS = "0 16px 36px rgb(0 0 0 / 0.72)";

/** Canvas mapping of --drop-shadow-drag (`0 <offsetY>px <blur>px …`). */
export const LIFT_SHADOW_OFFSET_Y = 16;
export const LIFT_SHADOW_BLUR = 36;
export const LIFT_SHADOW_COLOR = "rgba(0,0,0,0.72)";
