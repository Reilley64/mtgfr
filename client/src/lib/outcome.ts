// Who won, who's out, and whether the game is over — folded from the seats the wire already sends
// (`PlayerView.lost`). Mirrors `engine::Game::winner`: the game ends when at most one player is
// still in it, and that player (if any) has won.
//
// Nothing else in the client knew a game could end: a winner kept holding priority over a board of
// faded avatars, and an eliminated player kept a live hand and an enabled pass button that the
// server silently rejected.

import type { CommanderDamageView, PlayerView } from "~/wire/types";

/** Commander damage from a *single* commander that eliminates a player (CR 903.10a). The engine
 * owns the rule; this is the client's copy of the number it draws on the life orb. */
export const LETHAL_COMMANDER_DAMAGE = 21;

/** The largest tally any one commander has against a player — the number that can actually kill
 * them, since the 21 must come from one source. Two commanders at 20 apiece kill nobody. */
export function worstCommanderDamage(taken: CommanderDamageView[] = []): number {
  return taken.reduce((worst, d) => Math.max(worst, d.amount), 0);
}

export type Outcome =
  /** The game is still going and the viewer is still in it (or it hasn't loaded yet). */
  | { kind: "playing" }
  /** The viewer is the last player standing. */
  | { kind: "won" }
  /** The viewer has been eliminated. `winner` is set once someone has actually won. */
  | { kind: "lost"; winner: number | null }
  /** The game ended and the viewer wasn't in it — a spectator. `winner` is null if everyone died. */
  | { kind: "over"; winner: number | null };

export function outcome(players: PlayerView[], viewer: number): Outcome {
  // A table with fewer than two seats isn't a finished game, it's one that hasn't loaded: the lobby
  // refuses to start without two players (`NeedTwoPlayers`). Without this guard the empty `players`
  // of the first frame would read as "everyone is dead" and flash a game-over overlay on connect.
  if (players.length < 2) return { kind: "playing" };

  const living = players.filter((p) => !p.lost);
  const over = living.length <= 1;
  // Only a finished game has a winner; a two-way race has none yet. Zero survivors (a mutual
  // death) leaves the game over with nobody winning, exactly as the engine reports it.
  const winner = over ? (living[0]?.player ?? null) : null;

  const me = players.find((p) => p.player === viewer);
  if (!me) return over ? { kind: "over", winner } : { kind: "playing" };
  if (me.lost) return { kind: "lost", winner };
  if (over) return { kind: "won" };
  return { kind: "playing" };
}
