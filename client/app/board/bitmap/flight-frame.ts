import type { ActionView, PlayerView, WireAttack, WireBlock } from "~/wire/types";
import type { RenderCard } from "../geometry/layout";
import type { CardFlight } from "../motion/flights";
import type { BitmapFrame } from "./mount";

export type RestingPaintSnapshot = string;

function sortedSetValues(set: ReadonlySet<number>): number[] {
  return [...set].sort((a, b) => a - b);
}

function cardPaintKey(card: RenderCard): Record<string, unknown> {
  return {
    id: card.id,
    x: card.x,
    y: card.y,
    w: card.w,
    h: card.h,
    tapped: card.tapped ?? false,
    tapFrac: card.tapFrac ?? null,
    print: card.print ?? "",
    faceDown: card.faceDown ?? false,
    summoningSick: card.summoningSick ?? false,
    hasHaste: card.hasHaste ?? false,
    isCommander: card.isCommander ?? false,
    prepared: card.prepared ?? false,
    goaded: card.goaded ?? false,
    pile: card.pile ?? 0,
    cluster: card.cluster ?? 0,
    pt: card.pt ?? "",
    counters: card.counters ?? 0,
    markedDamage: card.markedDamage ?? 0,
    keywords: [...(card.keywords ?? [])].sort(),
    zone: card.zone ?? 0,
    owner: card.owner ?? 0,
    controller: card.controller ?? 0,
    name: card.name ?? "",
    fanAngle: card.fanAngle ?? 0,
  };
}

function playerPaintKey(player: PlayerView): Record<string, unknown> {
  const commanderDamage = [...(player.commander_damage ?? [])].map((row) => `${row.from}:${row.amount}`).sort();
  return {
    player: player.player,
    life: player.life,
    lost: player.lost,
    username: player.username ?? "",
    hand_count: player.hand_count,
    commander_damage: commanderDamage,
  };
}

function attackKey(attack: WireAttack): string {
  return `${attack.attacker}:${attack.defender}`;
}

function blockKey(block: WireBlock): string {
  return `${block.blocker}:${block.attacker}`;
}

function actionPaintKey(action: ActionView): Record<string, unknown> {
  return {
    id: action.id,
    section: action.section,
    object: action.object ?? null,
    taps_self: action.taps_self ?? false,
  };
}

export function restingPaintSnapshot(frame: Omit<BitmapFrame, "flights">): RestingPaintSnapshot {
  const cursorActive = frame.aimFrom != null || (frame.combatDragFrom != null && frame.combatDragStroke != null);

  const payload = {
    width: frame.width,
    height: frame.height,
    camera: frame.camera,
    viewer: frame.viewer,
    priority: frame.priority,
    hideCardIds: sortedSetValues(frame.hideCardIds),
    targetObjects: sortedSetValues(frame.targetObjects),
    pickedObjects: sortedSetValues(frame.pickedObjects),
    targetPlayers: sortedSetValues(frame.targetPlayers),
    paymentPreviewIds: sortedSetValues(frame.paymentPreviewIds),
    cards: [...frame.cards].sort((a, b) => a.id - b.id).map(cardPaintKey),
    players: [...frame.players].sort((a, b) => a.player - b.player).map(playerPaintKey),
    combat: {
      attackers: [...frame.combat.attackers].map(attackKey).sort(),
      blocks: [...frame.combat.blocks].map(blockKey).sort(),
      attackers_declared: frame.combat.attackers_declared,
      blockers_declared: [...frame.combat.blockers_declared].sort((a, b) => a - b),
    },
    stagedAttackers: [...frame.stagedAttackers].map(attackKey).sort(),
    stagedBlocks: [...frame.stagedBlocks].map(blockKey).sort(),
    aimFrom: frame.aimFrom,
    cursor: cursorActive ? frame.cursor : null,
    combatDragFrom: frame.combatDragFrom,
    combatDragStroke: frame.combatDragStroke,
    actions: frame.actions == null ? null : [...frame.actions].sort((a, b) => a.id - b.id).map(actionPaintKey),
  };

  return JSON.stringify(payload);
}

export function restingPaintChanged(prev: RestingPaintSnapshot | null, next: RestingPaintSnapshot): boolean {
  if (prev == null) return true;
  return prev !== next;
}

export function mergeFlightPoses(live: readonly CardFlight[], incoming: readonly CardFlight[]): CardFlight[] {
  const liveById = new Map(live.map((f) => [f.id, f]));
  return incoming.map((inc) => {
    const prev = liveById.get(inc.id);
    if (prev == null) return inc;
    return {
      ...inc,
      x: prev.x,
      y: prev.y,
      scale: prev.scale,
      phase: prev.phase,
    };
  });
}
