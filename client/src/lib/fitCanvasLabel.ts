/** Fit a canvas label to a max width with an ellipsis (seat names under life orbs). */

export function fitCanvasLabel(ctx: CanvasRenderingContext2D, text: string, maxWidth: number, ellipsis = "…"): string {
  if (maxWidth <= 0 || text.length === 0) return "";
  if (ctx.measureText(text).width <= maxWidth) return text;

  const tip = ellipsis;
  if (ctx.measureText(tip).width > maxWidth) return "";

  let lo = 0;
  let hi = text.length;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    const candidate = `${text.slice(0, mid)}${tip}`;
    if (ctx.measureText(candidate).width <= maxWidth) lo = mid;
    else hi = mid - 1;
  }
  return lo === 0 ? tip : `${text.slice(0, lo)}${tip}`;
}
