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
  ChangedBuilderQuery,
  ClearedBuilderHover,
  ClosedBuilderMenu,
  ClosedBuilderPrintPicker,
  ConfirmedBuilderDiscard,
  DeckBuilderLoadFailed,
  DeckSaved,
  DeckSaveFailed,
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
  RemovedBuilderCard,
  RequestedBuilderCancel,
  RequestedNextBuilderPage,
  SetBuilderCommander,
  SubmittedDeckSave,
} from "./builder/messages";
export {
  AskedDeckDelete,
  CancelledDeckDelete,
  ClearedDeckListHover,
  DeckDeleted,
  DeckDeleteFailed,
  DecksLoadFailed,
  MovedDeckListHover,
  ReceivedDeckListCommanders,
  ReceivedDecks,
  RequestedDeckDelete,
  RequestedDecksRefresh,
} from "./list/messages";
