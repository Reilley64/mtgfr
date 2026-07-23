import { Runtime } from "foldkit";
import "../styles/global.css";
import { initFaro } from "./faro";
import { init } from "./init";
import { Message, UrlChanged, UrlRequested } from "./messages";
import { Model } from "./model";
import { resources } from "./resources";
import { subscriptions } from "./subscriptions";
import { update } from "./update";
import { view } from "./view";

initFaro();

const app = Runtime.makeApplication({
  Model,
  init,
  update,
  view,
  resources,
  subscriptions,
  routing: {
    onUrlChange: (url) => UrlChanged({ url }),
    onUrlRequest: (request) => UrlRequested({ request }),
  },
  container: document.getElementById("app"),
  devTools: {
    Message,
  },
});

Runtime.run(app);
