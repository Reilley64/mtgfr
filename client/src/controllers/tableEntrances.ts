// Pure entrance seeding for battlefield / zone-pile paint tweens.
// Called by TableSurface's retarget effect — keep this free of Solid.

import { avatarPos, CARD_H, CARD_W, type RenderCard, ZONE, zonePilePos } from "~/layout";
import { STACK_VERTICAL_RESERVED, stackAimOrigin, stackPeekFor } from "~/lib/boardDraw";
import { type Camera, screenToWorld } from "~/lib/camera";
import type { Positions } from "~/lib/tween";

export type Vec = { x: number; y: number };

export type ZonePileKind = "library" | "graveyard" | "exile";

export type EntranceSeedOpts = {
  moves: Map<number, number>;
  fromStack: Set<number>;
  /** Cards that left the stack to GY/exile — seed at the stack overlay like fromStack. */
  fromStackExit: Set<number>;
  /** Token id → creator object id (anim / stack / avatar fallback). */
  tokenCreators: Map<number, number>;
  /** Own play: new permanent id → world origin (matched via land_played.from). */
  playEntrances: Map<number, Vec>;
  /** Non-play BF entries from a zone pile when the predecessor is not on canvas. */
  zonePileEntrances: Map<number, { zone: ZonePileKind; seat: number }>;
  /** Creator ids that were on the stack when the token was minted (use stackAimOrigin). */
  stackObjectIds: Set<number>;
  stackLength: number;
  size: Vec;
  camera: Camera;
  me: number;
  playerCount: number;
  /** Object ids owned by the canvas flight layer — do not seed a competing entrance. */
  skipIds?: ReadonlySet<number>;
};

function zoneToConst(zone: ZonePileKind): typeof ZONE.Library | typeof ZONE.Graveyard | typeof ZONE.Exile {
  if (zone === "library") return ZONE.Library;
  if (zone === "graveyard") return ZONE.Graveyard;
  return ZONE.Exile;
}

/**
 * Pre-seed `anim` for ids that just appeared so the paint path glides from a meaningful origin
 * (zone predecessor, stack overlay, play origin, creator, zone pile, or opponent avatar). Pure —
 * the Solid retarget effect is a thin caller. Does not touch ids already in `anim`.
 */
export function seedEntrances(anim: Positions, targets: readonly RenderCard[], opts: EntranceSeedOpts): void {
  for (const c of targets) {
    if (anim.has(c.id)) continue;
    // Own play / stack resolve: PlayMotion (ADR 0035) owns the motion — park the tween at the
    // layout slot so settle doesn't ENTER_RISE or compete with a second glide.
    if (opts.playEntrances.has(c.id) || opts.fromStack.has(c.id) || opts.fromStackExit.has(c.id)) {
      anim.set(c.id, { x: c.x, y: c.y });
      continue;
    }
    if (opts.skipIds?.has(c.id)) {
      anim.set(c.id, { x: c.x, y: c.y });
      continue;
    }
    const origin = anim.get(opts.moves.get(c.id) ?? -1);
    if (origin) {
      anim.set(c.id, { ...origin });
      continue;
    }
    const creator = opts.tokenCreators.get(c.id);
    if (creator !== undefined) {
      const creatorPos = anim.get(creator);
      if (creatorPos) {
        anim.set(c.id, { ...creatorPos });
        continue;
      }
      if (opts.stackObjectIds.has(creator)) {
        const count = opts.stackLength + 1;
        const peek = stackPeekFor(count, opts.size.y, STACK_VERTICAL_RESERVED);
        const scr = stackAimOrigin(opts.size.x, opts.size.y, count, peek);
        const w = screenToWorld(opts.camera, scr.x, scr.y);
        anim.set(c.id, { x: w.x - CARD_W / 2, y: w.y - CARD_H / 2 });
        continue;
      }
      const a = avatarPos(c.controller, opts.me, opts.playerCount);
      anim.set(c.id, { x: a.x, y: a.y });
      continue;
    }
    const pile = opts.zonePileEntrances.get(c.id);
    if (pile) {
      const p = zonePilePos(zoneToConst(pile.zone), pile.seat, opts.me, opts.playerCount);
      anim.set(c.id, { x: p.x, y: p.y });
      continue;
    }
    if (c.controller !== opts.me) {
      const a = avatarPos(c.controller, opts.me, opts.playerCount);
      anim.set(c.id, { x: a.x, y: a.y });
    }
  }
}
