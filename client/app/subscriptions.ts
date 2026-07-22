import { Subscription } from "foldkit";
import { subscriptions as gameSubscriptions } from "./game/stream-subscription";
import type { Message } from "./messages";
import { PortraitGateChanged } from "./messages";
import type { Model } from "./model";
import { subscriptions as lobbySubscriptions } from "./shell/lobby/subscriptions";

const PORTRAIT_QUERY = "(orientation: portrait) and (max-width: 900px)";

export function isPortraitPhone(): boolean {
  if (typeof window === "undefined") return false;
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia(PORTRAIT_QUERY).matches;
}

const appSubscriptions = Subscription.make<Model, Message>()(() => ({
  portraitGate: Subscription.persistent(
    Subscription.fromEvent<Event, Message>({
      target: () => (typeof window.matchMedia === "function" ? window.matchMedia(PORTRAIT_QUERY) : window),
      type: "change",
      toMessage: () => PortraitGateChanged({ open: isPortraitPhone() }),
    }),
  ),
}));

export const subscriptions = Subscription.aggregate<Model, Message>()(
  appSubscriptions,
  gameSubscriptions,
  Subscription.lift(lobbySubscriptions)<Model, Message>({
    toChildModel: (model) => model.lobby,
    toParentMessage: (message) => message,
  }),
);
