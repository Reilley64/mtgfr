import { Option } from "effect";
import type { Url } from "foldkit/url";
import { FetchApiVersion } from "./fetch-api-version";
import type { Model } from "./model";
import { nextFromUrl, normalizeAppRoute, pathWithSearch, routeFromUrl } from "./routes";
import { initialAuthSubmodel } from "./shell/auth/submodel";
import { FetchMe } from "./shell/auth/update";
import { initialDecksSubmodel } from "./shell/decks/submodel";
import { initialLobbySlice } from "./shell/lobby/submodel";
import { isPortraitPhone } from "./subscriptions";

export const init = (
  url?: Url,
): readonly [Model, ReadonlyArray<ReturnType<typeof FetchMe> | ReturnType<typeof FetchApiVersion>>] => {
  const fallbackUrl: Url = {
    protocol: "http:",
    host: "localhost",
    port: Option.none(),
    pathname: "/",
    search: Option.none(),
    hash: Option.none(),
  };
  const currentPath = pathWithSearch(url ?? fallbackUrl);
  const route = normalizeAppRoute(routeFromUrl(url ?? fallbackUrl), currentPath);
  const next = url == null ? "/" : nextFromUrl(url);

  return [
    {
      ready: true,
      route,
      currentPath,
      session: { me: null },
      sessionLoaded: false,
      apiVersion: null,
      auth: initialAuthSubmodel(next),
      decks: initialDecksSubmodel(),
      lobby: initialLobbySlice(),
      game: null,
      portraitGate: { open: isPortraitPhone() },
    },
    [FetchMe(), FetchApiVersion()],
  ];
};
