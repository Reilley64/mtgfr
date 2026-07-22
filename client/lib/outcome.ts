// Game outcome: who won, who's out, whether the game is over.
// Ported from Solid client/src/lib/outcome.ts.

import type { PlayerView } from "~/wire/types";

export type Outcome =
  /** The game is still going and the viewer is still in it (or it hasn't loaded yet). */
  | { kind: "playing" }
  /** The viewer is the last player standing. */
  | { kind: "won" }
  /** The viewer has been eliminated. `winner` is set once someone has actually won. */
  | { kind: "lost"; winner: number | null }
  /** The game ended and the viewer wasn't in it — a spectator. `winner` is null if everyone died. */
  | { kind: "over"; winner: number | null };

export function outcome(players: ReadonlyArray<PlayerView>, viewer: number): Outcome {
  // A table with fewer than two seats hasn't loaded yet — avoid a false game-over flash.
  if (players.length < 2) return { kind: "playing" };

  const living = players.filter((p) => !p.lost);
  const over = living.length <= 1;
  const winner = over ? (living[0]?.player ?? null) : null;

  const me = players.find((p) => p.player === viewer);
  if (!me) return over ? { kind: "over", winner } : { kind: "playing" };
  if (me.lost) return { kind: "lost", winner };
  if (over) return { kind: "won" };
  return { kind: "playing" };
}
