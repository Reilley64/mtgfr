// Felt backdrop for the battlefield canvas.

export function drawFelt(ctx: CanvasRenderingContext2D, w: number, h: number) {
  ctx.save();
  ctx.fillStyle = "#0B1310";
  ctx.fillRect(0, 0, w, h);
  // Deterministic speckles so the felt doesn't shimmer every frame.
  ctx.globalAlpha = 0.04;
  ctx.fillStyle = "#1a2a22";
  for (let i = 0; i < 120; i++) {
    const x = ((i * 73) % 97) / 97;
    const y = ((i * 41) % 89) / 89;
    ctx.fillRect(x * w, y * h, 2, 2);
  }
  ctx.globalAlpha = 1;
  const g = ctx.createRadialGradient(w / 2, h / 2, Math.min(w, h) * 0.2, w / 2, h / 2, Math.max(w, h) * 0.72);
  g.addColorStop(0, "rgba(0,0,0,0)");
  g.addColorStop(1, "rgba(0,0,0,0.45)");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  ctx.restore();
}
