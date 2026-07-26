/** True when `you` is a claimed host seat. Seat 0 is valid and must not be treated as absent. */
export function lobbyIsHost(
  you: number | null | undefined,
  seats: ReadonlyArray<{ is_host?: boolean } | undefined> | null | undefined,
): boolean {
  if (you == null) return false;
  if (!seats) return false;
  return seats[you]?.is_host ?? false;
}
