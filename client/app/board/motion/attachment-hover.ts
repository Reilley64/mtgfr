export const ATTACHMENT_HOVER_DURATION_MS = 120;

export type AttachmentHoverProgress = ReadonlyMap<number, number>;

function clampedProgress(progress: number): number {
  return Math.min(1, Math.max(0, progress));
}

export function reconcileAttachmentHover(
  progress: AttachmentHoverProgress,
  hoveredId: number | null,
): Map<number, number> {
  const next = new Map(progress);
  if (hoveredId != null && !next.has(hoveredId)) next.set(hoveredId, 0);
  return next;
}

export function stepAttachmentHover(
  progress: AttachmentHoverProgress,
  hoveredId: number | null,
  dtMs: number,
  reducedMotion: boolean,
): Map<number, number> {
  if (reducedMotion) return hoveredId == null ? new Map() : new Map([[hoveredId, 1]]);

  const reconciled = reconcileAttachmentHover(progress, hoveredId);
  const step = Math.max(0, dtMs) / ATTACHMENT_HOVER_DURATION_MS;
  const next = new Map<number, number>();

  for (const [id, rawProgress] of reconciled) {
    const current = clampedProgress(rawProgress);
    if (id === hoveredId) {
      next.set(id, Math.min(1, current + step));
      continue;
    }

    const departed = Math.max(0, current - step);
    if (departed > 0) next.set(id, departed);
  }

  return next;
}

export function attachmentHoverNeedsFrame(progress: AttachmentHoverProgress, hoveredId: number | null): boolean {
  if (hoveredId != null && (progress.get(hoveredId) ?? 0) < 1) return true;

  for (const [id, value] of progress) {
    if (id !== hoveredId && value > 0) return true;
  }

  return false;
}

export function easedAttachmentHoverProgress(progress: number): number {
  const clamped = clampedProgress(progress);
  return clamped * clamped * (3 - 2 * clamped);
}
