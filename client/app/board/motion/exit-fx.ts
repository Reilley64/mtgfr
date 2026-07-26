export type ExitFxKind = "destroy" | "exile";

export type ExitFx = {
  id: number;
  print: string;
  name: string;
  kind: ExitFxKind;
  x: number;
  y: number;
  scale: number;
  /** 0 → 1 */
  progress: number;
  seed: number;
};

export type ExitParticle = {
  x: number;
  y: number;
  r: number;
  color: string;
  alpha: number;
};

export const EXIT_FX_DURATION_MS = 550;
export const EXIT_FX_MAX_PARTICLES = 80;
export const EXIT_FX_BASE_PARTICLES = 8;

const DESTROY_COLORS = ["#ffb040", "#ff6020"] as const;
const EXILE_COLORS = ["#3DDC97", "#7ee8d0"] as const;

export function spawnExitFx(input: {
  id: number;
  print: string;
  name: string;
  kind: ExitFxKind;
  x: number;
  y: number;
  scale: number;
  seed?: number;
}): ExitFx {
  return {
    id: input.id,
    print: input.print,
    name: input.name,
    kind: input.kind,
    x: input.x,
    y: input.y,
    scale: input.scale,
    progress: 0,
    seed: input.seed ?? (input.id * 2654435761) >>> 0,
  };
}

export function stepExitFx(
  prev: ReadonlyMap<number, ExitFx>,
  dtMs: number,
  reducedMotion: boolean,
): { exitFx: Map<number, ExitFx>; active: boolean; completedIds: number[] } {
  const exitFx = new Map<number, ExitFx>();
  const completedIds: number[] = [];

  if (reducedMotion) {
    for (const id of prev.keys()) completedIds.push(id);
    return { exitFx, active: false, completedIds };
  }

  for (const [id, cur] of prev) {
    const progress = Math.min(1, cur.progress + dtMs / EXIT_FX_DURATION_MS);
    if (progress >= 1) {
      completedIds.push(id);
      continue;
    }
    exitFx.set(id, { ...cur, progress });
  }

  return { exitFx, active: exitFx.size > 0, completedIds };
}

export function particleAllowancePerFx(activeCount: number, maxTotal = EXIT_FX_MAX_PARTICLES): number {
  if (activeCount <= 0) return 0;
  // Floor-at-1 keeps huge wipes visible, even if that can slightly exceed the nominal global cap.
  return Math.max(1, Math.floor(maxTotal / activeCount));
}

function seededUnit(seed: number, index: number): number {
  let h = (seed + index * 2654435761) >>> 0;
  h = Math.imul(h ^ (h >>> 16), 2246822507) >>> 0;
  h = Math.imul(h ^ (h >>> 13), 3266489909) >>> 0;
  return (h ^ (h >>> 16)) / 0xffffffff;
}

export function exitFxParticles(fx: ExitFx, particleAllowance: number): ExitParticle[] {
  if (particleAllowance <= 0) return [];

  const count = Math.min(particleAllowance, EXIT_FX_BASE_PARTICLES);
  const particles: ExitParticle[] = [];
  const { x, y, scale, progress, seed, kind } = fx;
  const colors = kind === "destroy" ? DESTROY_COLORS : EXILE_COLORS;
  const fade = 1 - progress;

  for (let i = 0; i < count; i++) {
    const jitterX = (seededUnit(seed, i * 2) - 0.5) * scale;
    const jitterY = (seededUnit(seed, i * 2 + 1) - 0.5) * scale;

    let px: number;
    let py: number;

    if (kind === "destroy") {
      px = x + jitterX * 40;
      py = y + jitterY * 20 - progress * scale * 60;
    } else {
      const offsetX = jitterX * 80;
      const offsetY = jitterY * 80;
      px = x + offsetX * (1 - progress);
      py = y + offsetY * (1 - progress);
    }

    particles.push({
      x: px,
      y: py,
      r: scale * (2 + seededUnit(seed, i * 3) * 3),
      color: colors[i % colors.length],
      alpha: fade * (0.5 + seededUnit(seed, i * 4) * 0.5),
    });
  }

  return particles;
}
