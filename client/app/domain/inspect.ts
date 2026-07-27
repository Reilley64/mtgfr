// Pure helpers for card/player inspect: which face to show, pin lifecycle, commander-damage rows.

import type { ObjectView, PlayerView } from "./wire/types";

export type InspectFace = "front" | "back";

/** Play-face default: back only for a prepared permanent with a back. */
export function playFace(prepared: boolean, hasBack: boolean): InspectFace {
  return prepared && hasBack ? "back" : "front";
}

/** Catalog name to show for the given face. */
export function shownName(frontName: string, backName: string | null | undefined, face: InspectFace): string {
  if (face === "back" && backName) return backName;
  return frontName;
}

export type InspectPin = {
  name: string;
  prepared: boolean;
  /** Battlefield object id when Alt-pinning a permanent; absent for hand/stack/catalog history. */
  objectId?: number;
  /** Card (oracle) id when known — drives the catalog lookup. */
  cardId?: string;
  /** Printing UUID; absent falls back to catalog `default_print`. */
  print?: string;
  /** Life-orb seat when Alt-pinning a player for commander-damage inspect. */
  playerSeat?: number;
};

export type CommanderDamageRow = {
  fromSeat: number;
  label: string;
  amount: number;
  text: string;
};

function seatLabel(seat: number, players: ReadonlyArray<PlayerView>): string {
  const match = players.find((p) => p.player === seat);
  const name = match?.username?.trim();
  return name && name.length > 0 ? name : `P${seat}`;
}

function commanderNameForSeat(seat: number, objects: ReadonlyArray<ObjectView>): string | null {
  const commander = objects.find((o) => o.is_commander && o.owner === seat && o.name.trim().length > 0);
  return commander?.name ?? null;
}

/** Per-source commander damage lines for a seat's inspect dock (`Cmd N` on the orb stays max-only). */
export function commanderDamageBreakdown(
  player: PlayerView,
  players: ReadonlyArray<PlayerView>,
  objects: ReadonlyArray<ObjectView>,
): CommanderDamageRow[] {
  const rows = player.commander_damage;
  if (rows == null || rows.length === 0) return [];
  return rows.map((row) => {
    const owner = seatLabel(row.from, players);
    const commander = commanderNameForSeat(row.from, objects);
    const label = commander != null ? `${owner} — ${commander}` : owner;
    return {
      fromSeat: row.from,
      label,
      amount: row.amount,
      text: `${label}: ${row.amount} / 21`,
    };
  });
}

/** True when a new Alt-pin should replace the current pin (different card, object, or seat). */
export function inspectPinChanged(prev: InspectPin | null, next: InspectPin): boolean {
  if (!prev) return true;
  return prev.name !== next.name || prev.objectId !== next.objectId || prev.playerSeat !== next.playerSeat;
}

/** Build an InspectPin from a clicked RenderCard hit, when alt is held. Returns null if not pinnable. */
export function pinFromCard(
  altDown: boolean,
  hit: {
    name: string;
    faceDown?: boolean;
    prepared?: boolean;
    id?: number;
    zone?: number;
    pile?: number;
    cardId?: string;
    print?: string;
  } | null,
  battlefieldZone: number,
): InspectPin | null {
  if (!altDown || !hit || hit.faceDown || !hit.name) return null;
  // Don't pin a pile card — piles expand instead.
  if ((hit.pile ?? 0) > 0) return null;
  const onBattlefield = hit.zone === battlefieldZone && hit.id != null;
  return {
    name: hit.name,
    prepared: hit.prepared ?? false,
    ...(onBattlefield ? { objectId: hit.id } : {}),
    ...(hit.cardId ? { cardId: hit.cardId } : {}),
    ...(hit.print ? { print: hit.print } : {}),
  };
}

/** Build an InspectPin from a life-orb seat hit while Alt is held. */
export function pinFromPlayer(
  altDown: boolean,
  seat: number | null,
  player: PlayerView | null | undefined,
): InspectPin | null {
  if (!altDown || seat == null || player == null) return null;
  const name = player.username?.trim();
  return {
    name: name && name.length > 0 ? name : `P${seat}`,
    prepared: false,
    playerSeat: seat,
  };
}
