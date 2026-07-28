import type { WireAttack, WireBlock } from "~/wire/types";
import { type Camera, worldToScreen } from "../geometry/camera";
import type { RenderCard } from "../geometry/layout";
import type { AvatarScreenPositions } from "./avatars";

export type CombatArrowEndpoint = {
  from: { x: number; y: number };
  to: { x: number; y: number };
  kind: "attack" | "block";
};

type Vec = { x: number; y: number };

function cardCenter(camera: Camera, card: RenderCard): Vec {
  return worldToScreen(camera, card.x + card.w / 2, card.y + card.h / 2);
}

export function allBlockersDeclared(
  attackers: ReadonlyArray<WireAttack>,
  blockersDeclared: ReadonlyArray<number>,
): boolean {
  const defenders = new Set(attackers.map((attack) => attack.defender));
  for (const defender of defenders) {
    if (!blockersDeclared.includes(defender)) return false;
  }
  return true;
}

export function combatArrowEndpoints(input: {
  camera: Camera;
  cards: ReadonlyArray<RenderCard>;
  avatars: AvatarScreenPositions;
  attackers: ReadonlyArray<WireAttack>;
  blocks: ReadonlyArray<WireBlock>;
  blockersDeclared: ReadonlyArray<number>;
  blockedAttackers: ReadonlyArray<number>;
}): CombatArrowEndpoint[] {
  const byId = new Map(input.cards.map((card) => [card.id, card]));
  const blocked = new Set(input.blockedAttackers);
  const post = allBlockersDeclared(input.attackers, input.blockersDeclared);
  const endpoints: CombatArrowEndpoint[] = [];

  const defenderPoint = (attack: WireAttack): Vec | null => {
    const planeswalker = attack.defender_planeswalker;
    if (planeswalker != null) {
      const card = byId.get(planeswalker);
      if (card != null) return cardCenter(input.camera, card);
    }
    return input.avatars[attack.defender] ?? null;
  };

  if (!post) {
    for (const attack of input.attackers) {
      const fromCard = byId.get(attack.attacker);
      const to = defenderPoint(attack);
      if (fromCard == null || to == null) continue;
      endpoints.push({ from: cardCenter(input.camera, fromCard), to, kind: "attack" });
    }
    for (const block of input.blocks) {
      const fromCard = byId.get(block.blocker);
      const toCard = byId.get(block.attacker);
      if (fromCard == null || toCard == null) continue;
      endpoints.push({
        from: cardCenter(input.camera, fromCard),
        to: cardCenter(input.camera, toCard),
        kind: "block",
      });
    }
    return endpoints;
  }

  for (const attack of input.attackers) {
    const fromCard = byId.get(attack.attacker);
    if (fromCard == null) continue;
    const from = cardCenter(input.camera, fromCard);
    if (blocked.has(attack.attacker)) {
      const livingBlockers = input.blocks
        .filter((block) => block.attacker === attack.attacker)
        .map((block) => byId.get(block.blocker))
        .filter((card): card is RenderCard => card != null);
      for (const blocker of livingBlockers) {
        endpoints.push({ from, to: cardCenter(input.camera, blocker), kind: "attack" });
      }
      continue;
    }
    const to = defenderPoint(attack);
    if (to == null) continue;
    endpoints.push({ from, to, kind: "attack" });
  }
  return endpoints;
}
