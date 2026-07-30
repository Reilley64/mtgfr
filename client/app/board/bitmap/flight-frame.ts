import type { ActionView, PlayerView, StackObjectView, WireAttack, WireBlock } from "~/wire/types";
import type { RenderCard } from "../geometry/layout";
import type { StackPresentation } from "../geometry/stackLayout";
import type { ExitFx } from "../motion/exit-fx";
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
  };
}

function playerPaintKey(player: PlayerView): Record<string, unknown> {
  const commanderDamage = [...(player.commander_damage ?? [])].map((row) => `${row.from}:${row.amount}`).sort();
  return {
    player: player.player,
    life: player.life,
    lost: player.lost,
    username: player.username ?? "",
    gravatar_hash: player.gravatar_hash ?? "",
    hand_count: player.hand_count,
    commander_damage: commanderDamage,
    poison: player.poison ?? 0,
    rad: player.rad ?? 0,
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

function stackEntryPaintKey(entry: StackObjectView): string {
  const targets = (entry.targets ?? (entry.target != null ? [entry.target] : []))
    .map((t) => (t.kind === "player" ? `p${t.player}` : `o${t.id}`))
    .join(",");
  return `${entry.source}:${entry.kind}:${targets}`;
}

export function restingPaintSnapshot(
  frame: Omit<BitmapFrame, "flights" | "exitFx" | "dragGhost">,
): RestingPaintSnapshot {
  const cursorActive = frame.aimFrom != null || (frame.combatDragFrom != null && frame.combatDragStroke != null);

  const payload = {
    width: frame.width,
    height: frame.height,
    // A DPR change resizes the backing store, which clears it — that has to force a repaint.
    dpr: frame.dpr,
    camera: frame.camera,
    viewer: frame.viewer,
    priority: frame.priority,
    hideCardIds: sortedSetValues(frame.hideCardIds),
    targetObjects: sortedSetValues(frame.targetObjects),
    pickedObjects: sortedSetValues(frame.pickedObjects),
    assignAmounts: [...frame.assignAmounts.entries()]
      .filter(([, amount]) => amount > 0)
      .sort((a, b) => a[0] - b[0])
      .map(([id, amount]) => `${id}:${amount}`),
    targetPlayers: sortedSetValues(frame.targetPlayers),
    pickedPlayers: sortedSetValues(frame.pickedPlayers),
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
    stack: (frame.stack ?? []).map(stackEntryPaintKey),
    stackPresentation: (frame.stackPresentation ?? "pile") as StackPresentation,
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
    // Same id, or land/stack rebind where the permanent/spell id replaces the hand seed id.
    // Without the fromCardId match, publish resets to the stale model spawn pose and the card
    // restarts its glide — the every-time land double animation.
    const prev =
      liveById.get(inc.id) ??
      (inc.fromCardId != null ? liveById.get(inc.fromCardId) : undefined) ??
      live.find(
        (flight) =>
          (inc.fromCardId != null && flight.fromCardId === inc.fromCardId) ||
          (flight.fromCardId != null && flight.fromCardId === inc.id),
      );
    if (prev == null) return inc;
    return {
      ...inc,
      x: prev.x,
      y: prev.y,
      scale: prev.scale,
      // Authority retargets release hold and set flying — don't trap that as a live settled park.
      phase: prev.phase === "flying" || inc.phase === "flying" ? "flying" : "settled",
    };
  });
}

export function mergeExitFxPoses(live: readonly ExitFx[], incoming: readonly ExitFx[]): ExitFx[] {
  const liveById = new Map(live.map((fx) => [fx.id, fx]));
  return incoming.map((incomingFx) => {
    const prior = liveById.get(incomingFx.id);
    if (prior == null) return incomingFx;
    return {
      ...incomingFx,
      progress: prior.progress,
    };
  });
}
