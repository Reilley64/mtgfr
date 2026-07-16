// Pure lobby seat helpers — kept free of Solid so seat-0 falsiness can't hide behind JSX.

/** True when `you` is a claimed host seat. Seat 0 is a valid host — never treat it as "no seat". */
export function lobbyIsHost(
  you: number | null | undefined,
  seats: ReadonlyArray<{ is_host?: boolean } | undefined> | null | undefined,
): boolean {
  if (you == null || !seats) return false;
  return seats[you]?.is_host ?? false;
}
