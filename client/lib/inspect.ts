// Pure helpers for card inspect: which face to show, and pin lifecycle rules.

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
};

/** True when a new Alt-pin should replace the current pin (different card or object). */
export function inspectPinChanged(prev: InspectPin | null, next: InspectPin): boolean {
  if (!prev) return true;
  return prev.name !== next.name || prev.objectId !== next.objectId;
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
