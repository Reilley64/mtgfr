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

/** True when a new Alt-pin should replace the inspect history root (not merely refresh the same pin). */
export function inspectRootChanged(prevRoot: InspectPin | undefined, next: InspectPin): boolean {
  if (!prevRoot) return true;
  return prevRoot.name !== next.name || prevRoot.objectId !== next.objectId;
}

/** Push a catalog-only source card onto the inspect history stack. */
export function pushInspectSource(
  history: InspectPin[],
  source: { name: string; cardId?: string; print?: string },
): InspectPin[] {
  return [
    ...history,
    {
      name: source.name,
      prepared: false,
      ...(source.cardId ? { cardId: source.cardId } : {}),
      ...(source.print ? { print: source.print } : {}),
    },
  ];
}

/** Pop one inspect history entry; no-op at the root. */
export function popInspectHistory(history: InspectPin[]): InspectPin[] {
  return history.length > 1 ? history.slice(0, -1) : history;
}

/** Pin on Alt-down over a face-up named card; otherwise null. */
export function pinFromHit(
  altDown: boolean,
  hit: {
    name: string;
    faceDown?: boolean;
    prepared?: boolean;
    id?: number;
    zone?: number;
    cardId?: string;
    print?: string;
  } | null,
  battlefieldZone: number,
): InspectPin | null {
  if (!altDown || !hit || hit.faceDown || !hit.name) return null;
  const onBattlefield = hit.zone === battlefieldZone && hit.id != null;
  return {
    name: hit.name,
    prepared: hit.prepared ?? false,
    ...(onBattlefield ? { objectId: hit.id } : {}),
    ...(hit.cardId ? { cardId: hit.cardId } : {}),
    ...(hit.print ? { print: hit.print } : {}),
  };
}
