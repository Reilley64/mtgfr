import { type Camera, screenToWorld } from "./camera";
import { isFlipped, type RenderCard } from "./layout";

export const ATTACHMENT_HOVER_RISE = 0.2;

function clampProgress(progress: number): number {
  return Math.min(1, Math.max(0, progress));
}

function cardIndex(cards: readonly RenderCard[]): ReadonlyMap<number, RenderCard> {
  return new Map(cards.map((card) => [card.id, card]));
}

function rootHost(byId: ReadonlyMap<number, RenderCard>, attachment: RenderCard): RenderCard | null {
  if (attachment.attachedTo == null) return null;

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

function attachmentOffset(
  card: RenderCard,
  root: RenderCard,
  progress: number,
  viewer: number,
  playerCount: number,
): number {
  const direction = isFlipped(root.controller, viewer, playerCount) ? 1 : -1;
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
  playerCount: number,
): number {
  if (hoveredId == null) return 0;

  const byId = cardIndex(cards);
  const attachment = byId.get(hoveredId);
  if (attachment == null) return 0;

  const root = rootHost(byId, attachment);
  if (root == null) return 0;

  return attachmentOffset(attachment, root, progress, viewer, playerCount);
}

export function attachmentHoverCards(
  cards: readonly RenderCard[],
  progressById: ReadonlyMap<number, number>,
  viewer: number,
  playerCount: number,
): RenderCard[] {
  const byId = cardIndex(cards);

  return cards.map((card) => {
    const progress = clampProgress(progressById.get(card.id) ?? 0);
    if (progress === 0) return card;

    const root = rootHost(byId, card);
    if (root == null) return card;

    return { ...card, y: card.y + attachmentOffset(card, root, progress, viewer, playerCount) };
  });
}

export function hitAttachmentHover(
  camera: Camera,
  screenX: number,
  screenY: number,
  cards: readonly RenderCard[],
  hoveredId: number | null,
  viewer: number,
  playerCount: number,
): number | null {
  const point = screenToWorld(camera, screenX, screenY);
  const byId = cardIndex(cards);

  for (let index = cards.length - 1; index >= 0; index--) {
    const card = cards[index];
    const root = rootHost(byId, card);
    const inRestingRect = contains(card, point.x, point.y);
    const inHoveredRect =
      card.id === hoveredId &&
      root != null &&
      contains({ ...card, y: card.y + attachmentOffset(card, root, 1, viewer, playerCount) }, point.x, point.y);

    if (!inRestingRect && !inHoveredRect) continue;
    if (root == null) return null;
    return card.id;
  }

  return null;
}
