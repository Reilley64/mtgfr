import type { PlayerView } from "~/wire/types";

type MulliganPlayer = Pick<
  PlayerView,
  "can_mulligan" | "hand_kept" | "lost" | "mulligans_taken" | "player" | "username"
>;

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

function seatLabel(player: MulliganPlayer): string {
  const name = player.username?.trim();
  if (name) return name;
  return `P${player.player}`;
}

function joinNames(names: readonly string[]): string {
  if (names.length === 0) return "";
  if (names.length === 1) return names[0] ?? "";
  if (names.length === 2) return `${names[0]} and ${names[1]}`;
  const head = names.slice(0, -1).join(", ");
  return `${head}, and ${names[names.length - 1]}`;
}

const waitingStatus = (waiting: readonly MulliganPlayer[]): string => {
  if (waiting.length === 0) return "All players kept. Starting game…";
  return `Waiting for ${joinNames(waiting.map(seatLabel))} to choose.`;
};

export function mulliganChrome(input: MulliganChromeInput): MulliganChrome {
  if (!input.mulliganing) return hiddenChrome();

  const local = input.players.find((p) => p.player === input.localSeat);
  if (!local) return hiddenChrome();

  const waiting = input.players.filter((p) => !p.hand_kept && !p.lost);
  const waitingCount = waiting.length;
  const mulligansTaken = local.mulligans_taken ?? 0;
  const showControls = !local.hand_kept;

  return {
    show: true,
    showControls,
    canMulligan: showControls && (local.can_mulligan ?? false),
    waitingCount,
    mulligansTaken,
    title: "Opening hand",
    status: showControls ? "Keep this hand or take a mulligan." : waitingStatus(waiting),
    keepLabel: "Keep",
    mulliganLabel: mulligansTaken === 0 ? "Mulligan" : `Mulligan (${mulligansTaken} taken)`,
  };
}
