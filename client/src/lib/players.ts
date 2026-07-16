import type { PlayerView } from "~/api/generated";

/** Display name for a seat — falls back to P{n} until usernames arrive on the wire. */
export function playerLabel(players: PlayerView[], seat: number): string {
  const name = players.find((p) => p.player === seat)?.username?.trim();
  return name || `P${seat}`;
}
