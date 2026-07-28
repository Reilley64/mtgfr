import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { UrlRequest } from "foldkit/navigation";
import { Url } from "foldkit/url";
import { Message as BoardMessage } from "./board/messages";
import { DeckCardFlipTick } from "./deck-card-nav";
import { CardArtTick } from "./domain/ui/card-art";
import { ModalOpened } from "./domain/ui/native-dialog";
import { Message as GameMessage } from "./game/messages";
import { Message as AccountChromeMessage } from "./shell/account-chrome/messages";
import { Message as AuthMessage } from "./shell/auth/messages";
import { Message as CoverageMessage } from "./shell/coverage/messages";
import { Message as DeckBuilderMessage } from "./shell/decks/builder/messages";
import { Message as DeckListMessage } from "./shell/decks/list/messages";
import { Message as LeaderboardMessage } from "./shell/leaderboard/messages";
import { Message as LobbyMessage } from "./shell/lobby/messages";

export const Booted = m("Booted");
export const ReceivedApiVersion = m("ReceivedApiVersion", {
  version: S.NullOr(S.String),
  faithfulCount: S.NullOr(S.Number),
  oracleTotal: S.NullOr(S.Number),
});
export const UrlChanged = m("UrlChanged", { url: Url });
export const UrlRequested = m("UrlRequested", { request: UrlRequest });
export const NavigationCompleted = m("NavigationCompleted");
export const LandscapeRotateChanged = m("LandscapeRotateChanged", { active: S.Boolean });
export const ReceivedMeGravatarHash = m("ReceivedMeGravatarHash", { email: S.String, hash: S.String });
export const GotAuthMessage = m("GotAuthMessage", { message: AuthMessage });
export const GotDeckListMessage = m("GotDeckListMessage", { message: DeckListMessage });
export const GotDeckBuilderMessage = m("GotDeckBuilderMessage", { message: DeckBuilderMessage });
export const GotCoverageMessage = m("GotCoverageMessage", { message: CoverageMessage });
export const GotLeaderboardMessage = m("GotLeaderboardMessage", { message: LeaderboardMessage });
export const GotLobbyMessage = m("GotLobbyMessage", { message: LobbyMessage });
export const GotBoardMessage = m("GotBoardMessage", { message: BoardMessage });
export const GotGameMessage = m("GotGameMessage", { message: GameMessage });
export { CardArtTick, DeckCardFlipTick, ModalOpened };

export const Message = S.Union([
  Booted,
  ReceivedApiVersion,
  UrlChanged,
  UrlRequested,
  NavigationCompleted,
  LandscapeRotateChanged,
  ReceivedMeGravatarHash,
  GotAuthMessage,
  GotDeckListMessage,
  GotDeckBuilderMessage,
  GotCoverageMessage,
  GotLeaderboardMessage,
  GotLobbyMessage,
  GotBoardMessage,
  GotGameMessage,
  ModalOpened,
  CardArtTick,
  DeckCardFlipTick,
  AccountChromeMessage,
]);
export type Message = typeof Message.Type;

export {
  ArtLoaded,
  BoardPointerDown,
  BoardPointerMove,
  BoardPointerUp,
  FlightsSynced,
} from "./board/messages";
export {
  IntentAcked,
  IntentRejected,
  ReceivedDelta,
  ReceivedSnapshot,
  StreamStatus,
  StreamTerminalError,
} from "./game/messages";
export { GotAccountMenuMessage } from "./shell/account-chrome/messages";
export {
  LeaderboardLoadFailed,
  ReceivedLeaderboardPage,
  RequestedLeaderboardNextPage,
  RequestedLeaderboardRefresh,
} from "./shell/leaderboard/messages";
