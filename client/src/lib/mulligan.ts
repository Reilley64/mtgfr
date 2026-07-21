import type { PlayerView } from "~/wire/types";

type MulliganPlayer = Pick<PlayerView, "can_mulligan" | "hand_kept" | "mulligans_taken" | "player">;

export type MulliganChromeInput = {
  mulliganing?: boolean;
  localSeat: number;
  players: readonly MulliganPlayer[];
};

export type MulliganChrome = {
  show: boolean;
  showControls: boolean;
  canMulligan: boolean;
  waitingCount: number;
  mulligansTaken: number;
  title: string;
  status: string;
  keepLabel: string;
  mulliganLabel: string;
};

const hiddenChrome = (): MulliganChrome => ({
  show: false,
  showControls: false,
  canMulligan: false,
  waitingCount: 0,
  mulligansTaken: 0,
  title: "Opening hand",
  status: "",
  keepLabel: "Keep",
  mulliganLabel: "Mulligan",
});

const waitingStatus = (count: number): string => {
  if (count === 0) return "All players kept. Starting game…";
  if (count === 1) return "Waiting for 1 player to choose.";
  return `Waiting for ${count} players to choose.`;
};

export function mulliganChrome(input: MulliganChromeInput): MulliganChrome {
  if (!input.mulliganing) return hiddenChrome();

  const local = input.players.find((p) => p.player === input.localSeat);
  if (!local) return hiddenChrome();

  const waitingCount = input.players.filter((p) => !p.hand_kept).length;
  const mulligansTaken = local.mulligans_taken ?? 0;
  const showControls = !local.hand_kept;

  return {
    show: true,
    showControls,
    canMulligan: showControls && (local.can_mulligan ?? false),
    waitingCount,
    mulligansTaken,
    title: "Opening hand",
    status: showControls ? "Keep this hand or take a mulligan." : waitingStatus(waitingCount),
    keepLabel: "Keep",
    mulliganLabel: mulligansTaken === 0 ? "Mulligan" : `Mulligan (${mulligansTaken} taken)`,
  };
}
