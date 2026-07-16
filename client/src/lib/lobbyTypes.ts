/** Lobby wire shapes (BFF-owned; no longer on Axum OpenAPI paths). */
export type SeatView = {
  player: number;
  claimed: boolean;
  username: string | null;
  deck_name: string | null;
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
