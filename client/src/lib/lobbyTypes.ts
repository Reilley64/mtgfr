export type SeatView = {
  player: number;
  claimed: boolean;
  username: string | null;
  deck_name: string | null;
  /** Present when claimed — used to warm card art (prints are not zone-secret). */
  deck_id: number | null;
  ready: boolean;
  is_host: boolean;
  is_you: boolean;
};

export type LobbyView = {
  table_id: string;
  seats: SeatView[];
  you: number | null;
  started: boolean;
  start_error?: string | null;
  error?: string | null;
};
