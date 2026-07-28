import * as VirtualList from "@foldkit/ui/virtualList";
import { Subscription } from "foldkit";
import { subscriptions as gameSubscriptions } from "./game";
import type { Message } from "./messages";
import { GotDeckBuilderMessage, GotGameMessage, GotLobbyMessage, LandscapeRotateChanged } from "./messages";
import type { Model } from "./model";
import { GotPoolGridMessage, GotPrintGridMessage } from "./shell/decks/builder/messages";
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

/** A windowed grid learns its height and scroll position only here — VirtualList has no view
 *  handlers, so an unsubscribed grid stays unmeasured and paints nothing.
 *
 *  Every lift names its entry `containerEvents`, and `aggregate` rejects a duplicate key, so each
 *  grid's entries are prefixed with the grid's own name. */
function gridSubscriptions(
  name: string,
  toChildModel: (model: Model) => VirtualList.Model,
  toParentMessage: (message: VirtualList.Message) => Message,
): Record<string, Subscription.Subscriptions<Model, Message>[string]> {
  const lifted = Subscription.lift(VirtualList.subscriptions)<Model, Message>({ toChildModel, toParentMessage });
  return Object.fromEntries(Object.entries(lifted).map(([key, entry]) => [`${name}-${key}`, entry]));
}

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
  gridSubscriptions(
    "print-grid",
    (model) => model.decks.builder.printGrid,
    (message) => GotDeckBuilderMessage({ message: GotPrintGridMessage({ message }) }),
  ),
  // The pool's grid additionally pages the catalog off its scroll position.
  gridSubscriptions(
    "pool-grid",
    (model) => model.decks.builder.poolGrid,
    (message) => GotDeckBuilderMessage({ message: GotPoolGridMessage({ message }) }),
  ),
);
