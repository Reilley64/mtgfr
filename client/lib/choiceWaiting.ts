/** Passive waiting copy when another seat must answer `pending_choice`. */

export type PendingChoiceWaitingPlayer = {
  player: number;
  username?: string;
};

export type PendingChoiceWaitingInput = {
  pendingPlayer: number | null;
  viewer: number;
  mulliganing?: boolean;
  players: ReadonlyArray<PendingChoiceWaitingPlayer>;
};

function seatLabel(player: PendingChoiceWaitingPlayer | undefined, seat: number): string {
  const name = player?.username?.trim();
  if (name) return name;
  return `P${seat}`;
}

/** Status line for non-deciders (and spectators) while a pending choice awaits another seat. */
export function pendingChoiceWaitingText(input: PendingChoiceWaitingInput): string | null {
  if (input.mulliganing) return null;
  if (input.pendingPlayer == null) return null;
  if (input.pendingPlayer === input.viewer) return null;
  const awaited = input.players.find((p) => p.player === input.pendingPlayer);
  return `Waiting for ${seatLabel(awaited, input.pendingPlayer)}…`;
}
