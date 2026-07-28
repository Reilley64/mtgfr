import * as VirtualList from "@foldkit/ui/virtualList";
import { Subscription } from "foldkit";
import { subscriptions as gameSubscriptions } from "./game";
import type { Message } from "./messages";
import { GotDeckBuilderMessage, GotGameMessage, GotLobbyMessage, LandscapeRotateChanged } from "./messages";
import type { Model } from "./model";
import { GotPrintGridMessage } from "./shell/decks/builder/messages";
import { subscriptions as lobbySubscriptions } from "./shell/lobby/subscriptions";

const PORTRAIT_QUERY = "(orientation: portrait) and (max-width: 900px)";

export function isPortraitPhone(): boolean {
  if (typeof window === "undefined") return false;
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia(PORTRAIT_QUERY).matches;
}

const appSubscriptions = Subscription.make<Model, Message>()(() => ({
  landscapeRotate: Subscription.persistent(
    Subscription.fromEvent<Event, Message>({
      target: () => (typeof window.matchMedia === "function" ? window.matchMedia(PORTRAIT_QUERY) : window),
      type: "change",
      toMessage: () => LandscapeRotateChanged({ active: isPortraitPhone() }),
    }),
  ),
}));

export const subscriptions = Subscription.aggregate<Model, Message>()(
  appSubscriptions,
  Subscription.lift(gameSubscriptions)<Model, Message>({
    toChildModel: (model) => model,
    toParentMessage: (message) => GotGameMessage({ message }),
  }),
  Subscription.lift(lobbySubscriptions)<Model, Message>({
    toChildModel: (model) => model.lobby,
    toParentMessage: (message) => GotLobbyMessage({ message }),
  }),
  // The print picker's grid learns its height and scroll position here — VirtualList has no view
  // handlers, so without this the grid stays unmeasured and paints nothing.
  Subscription.lift(VirtualList.subscriptions)<Model, Message>({
    toChildModel: (model) => model.decks.builder.printGrid,
    toParentMessage: (message) => GotDeckBuilderMessage({ message: GotPrintGridMessage({ message }) }),
  }),
);
