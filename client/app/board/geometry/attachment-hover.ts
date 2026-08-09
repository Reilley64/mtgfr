import { type Camera, screenToWorld } from "./camera";
import type { RenderCard } from "./layout";

export const ATTACHMENT_HOVER_RISE = 0.2;

function clampProgress(progress: number): number {
  return Math.min(1, Math.max(0, progress));
}

function rootHost(cards: readonly RenderCard[], attachment: RenderCard): RenderCard | null {
  if (attachment.attachedTo == null) return null;

  const byId = new Map(cards.map((card) => [card.id, card]));
  const seen = new Set<number>();
  let current = attachment;

  while (current.attachedTo != null) {
    if (seen.has(current.id)) return null;
    seen.add(current.id);

    const host = byId.get(current.attachedTo);
    if (host == null) return null;
    current = host;
  }

  return current;
}

function attachmentOffset(card: RenderCard, cards: readonly RenderCard[], progress: number, viewer: number): number {
  const root = rootHost(cards, card);
  if (root == null) return 0;

  const direction = root.controller === viewer ? -1 : 1;
  return direction * card.h * ATTACHMENT_HOVER_RISE * clampProgress(progress);
}

function contains(card: RenderCard, x: number, y: number): boolean {
  return x >= card.x && x <= card.x + card.w && y >= card.y && y <= card.y + card.h;
}

export function attachmentHoverOffset(
  cards: readonly RenderCard[],
  hoveredId: number | null,
  progress: number,
  viewer: number,
): number {
  if (hoveredId == null) return 0;

  const attachment = cards.find((card) => card.id === hoveredId);
  if (attachment == null) return 0;

  return attachmentOffset(attachment, cards, progress, viewer);
}

export function attachmentHoverCards(
  cards: readonly RenderCard[],
  progressById: ReadonlyMap<number, number>,
  viewer: number,
): RenderCard[] {
  return cards.map((card) => {
    const progress = clampProgress(progressById.get(card.id) ?? 0);
    if (progress === 0) return card;

    return { ...card, y: card.y + attachmentOffset(card, cards, progress, viewer) };
  });
}

export function hitAttachmentHover(
  camera: Camera,
  screenX: number,
  screenY: number,
  cards: readonly RenderCard[],
  hoveredId: number | null,
  viewer: number,
): number | null {
  const point = screenToWorld(camera, screenX, screenY);

  for (let index = cards.length - 1; index >= 0; index--) {
    const card = cards[index];
    const root = rootHost(cards, card);
    const inRestingRect = contains(card, point.x, point.y);
    const inHoveredRect =
      card.id === hoveredId &&
      root != null &&
      contains({ ...card, y: card.y + attachmentOffset(card, cards, 1, viewer) }, point.x, point.y);

    if (!inRestingRect && !inHoveredRect) continue;
    if (root == null) return null;
    return card.id;
  }

  return null;
}
