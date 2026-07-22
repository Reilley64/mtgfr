import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { UrlRequest } from "foldkit/navigation";
import { Url } from "foldkit/url";
import { ModalOpened } from "../lib/ui/confirmDialog";
import { Message as BoardMessage } from "./board/messages";
import { Message as GameMessage } from "./game/messages";
import { Message as AuthMessage } from "./shell/auth/messages";
import { Message as DecksMessage } from "./shell/decks/messages";
import { Message as LobbyMessage } from "./shell/lobby/messages";

export const Booted = m("Booted");
export const ReceivedApiVersion = m("ReceivedApiVersion", { version: S.NullOr(S.String) });
export const UrlChanged = m("UrlChanged", { url: Url });
export const UrlRequested = m("UrlRequested", { request: UrlRequest });
export const NavigationCompleted = m("NavigationCompleted");
export const PortraitGateChanged = m("PortraitGateChanged", { open: S.Boolean });
export const PortraitGateCancelled = m("PortraitGateCancelled");
export const CompletedPortraitGateModal = m("CompletedPortraitGateModal");
export { ModalOpened };

export const Message = S.Union([
  Booted,
  ReceivedApiVersion,
  UrlChanged,
  UrlRequested,
  NavigationCompleted,
  PortraitGateChanged,
  PortraitGateCancelled,
  CompletedPortraitGateModal,
  ModalOpened,
  BoardMessage,
  AuthMessage,
  DecksMessage,
  LobbyMessage,
  GameMessage,
]);
export type Message = typeof Message.Type;

export {
  ArtLoaded,
  BoardPointerDown,
  BoardPointerMove,
  BoardPointerUp,
  TickedFrame,
} from "./board/messages";
export {
  IntentAcked,
  IntentRejected,
  ReceivedDelta,
  ReceivedSnapshot,
  StreamStatus,
  StreamTerminalError,
} from "./game/messages";
export {
  AuthFailed,
  ChangedAuthEmail,
  ChangedAuthMode,
  ChangedAuthPassword,
  ChangedAuthUsername,
  ReceivedMe,
  RequestedLogout,
  SubmittedAuth,
} from "./shell/auth/messages";
export {
  ActivatedBuilderTarget,
  AddedBuilderCard,
  AskedDeckDelete,
  BuilderPrintSearchFailed,
  BuilderSearchFailed,
  CancelledBuilderDiscard,
  CancelledDeckDelete,
  ChangedBuilderName,
  ChangedBuilderQuery,
  ClearedBuilderHover,
  ClosedBuilderMenu,
  ClosedBuilderPrintPicker,
  ConfirmedBuilderDiscard,
  DeckBuilderLoadFailed,
  DeckDeleted,
  DeckDeleteFailed,
  DeckSaved,
  DeckSaveFailed,
  DecksLoadFailed,
  HydratedBuilderCards,
  MovedBuilderHover,
  NavigatedAwayFromBuilder,
  OpenedBuilderMenu,
  OpenedBuilderPrintPicker,
  PickedBuilderPrint,
  RanBuilderMenuAction,
  ReceivedBuilderPrints,
  ReceivedBuilderSearchPage,
  ReceivedDeckForBuilder,
  ReceivedDeckListCommanders,
  ReceivedDecks,
  RemovedBuilderCard,
  RequestedBuilderCancel,
  RequestedDeckDelete,
  RequestedDecksRefresh,
  RequestedNextBuilderPage,
  SetBuilderCommander,
  SubmittedDeckSave,
} from "./shell/decks/messages";
export {
  ChangedLobbyCode,
  ChangedLobbyDeck,
  LobbyCopyCompleted,
  LobbyRequestFailed,
  LobbyTableCreated,
  ReceivedLobbyView,
  RequestedLobbyCopy,
  RequestedLobbyHost,
  RequestedLobbyJoin,
  RequestedLobbyReady,
  RequestedLobbyStart,
} from "./shell/lobby/messages";
