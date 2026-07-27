import { Schema as S } from "effect";
import { Message as BuilderMessage } from "./builder/messages";
import { Message as ListMessage } from "./list/messages";

export const Message = S.Union([BuilderMessage, ListMessage]);
export type Message = typeof Message.Type;

export {
  ActivatedBuilderTarget,
  AddedBuilderCard,
  BuilderPrintSearchFailed,
  BuilderSearchFailed,
  CancelledBuilderDiscard,
  ChangedBuilderName,
  ChangedBuilderProxyArtUrl,
  ChangedBuilderQuery,
  ClearedBuilderHover,
  ClearedBuilderProxyArt,
  ClosedBuilderMenu,
  ClosedBuilderPrintPicker,
  ClosedBuilderProxyArtPicker,
  ConfirmedBuilderDiscard,
  DeckBuilderLoadFailed,
  DeckSaved,
  DeckSaveFailed,
  HydratedBuilderCards,
  MovedBuilderHover,
  NavigatedAwayFromBuilder,
  OpenedBuilderMenu,
  OpenedBuilderPrintPicker,
  OpenedBuilderProxyArtPicker,
  PickedBuilderPrint,
  RanBuilderMenuAction,
  ReceivedBuilderPrints,
  ReceivedBuilderSearchPage,
  ReceivedDeckForBuilder,
  RemovedBuilderCard,
  RequestedBuilderCancel,
  RequestedNextBuilderPage,
  SetBuilderCommander,
  SubmittedBuilderProxyArt,
  SubmittedDeckSave,
} from "./builder/messages";
export {
  AskedDeckDelete,
  CancelledDeckDelete,
  ChangedDeckListSearch,
  ClosedDeckListMenu,
  DeckDeleted,
  DeckDeleteFailed,
  DecksLoadFailed,
  OpenedDeckListMenu,
  ReceivedDeckListCommanders,
  ReceivedDecks,
  RequestedDeckDelete,
  RequestedDecksRefresh,
} from "./list/messages";
