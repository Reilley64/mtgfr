import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { LobbyView } from "../../../lib/lobby/types";

export const ChangedLobbyDeck = m("ChangedLobbyDeck", { deckId: S.Number });
export const ChangedLobbyCode = m("ChangedLobbyCode", { code: S.String });
export const RequestedLobbyHost = m("RequestedLobbyHost");
export const LobbyTableCreated = m("LobbyTableCreated", { tableId: S.String });
export const RequestedLobbyJoin = m("RequestedLobbyJoin");
export const RequestedLobbyReady = m("RequestedLobbyReady", { ready: S.Boolean });
export const RequestedLobbyStart = m("RequestedLobbyStart");
export const RequestedLobbyCopy = m("RequestedLobbyCopy");
export const LobbyCopyCompleted = m("LobbyCopyCompleted", { ok: S.Boolean });
export const ReceivedLobbyView = m("ReceivedLobbyView", { view: LobbyView });
export const LobbyRequestFailed = m("LobbyRequestFailed", { message: S.String });

export const Message = S.Union([
  ChangedLobbyDeck,
  ChangedLobbyCode,
  RequestedLobbyHost,
  LobbyTableCreated,
  RequestedLobbyJoin,
  RequestedLobbyReady,
  RequestedLobbyStart,
  RequestedLobbyCopy,
  LobbyCopyCompleted,
  ReceivedLobbyView,
  LobbyRequestFailed,
]);
export type Message = typeof Message.Type;
