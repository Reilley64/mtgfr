import { Option } from "effect";
import { Command } from "foldkit";
import type { Url } from "foldkit/url";
import { FetchApiVersion } from "./fetch-api-version";
import { GotAuthMessage } from "./messages";
import { nextFromUrl, normalizeAppRoute, pathWithSearch, routeFromUrl } from "./routes";
import { initialAuthSubmodel } from "./shell/auth/submodel";
import { FetchMe } from "./shell/auth/update";
import { initialDecksSubmodel } from "./shell/decks/submodel";
import { initialLeaderboardSubmodel } from "./shell/leaderboard/submodel";
import { initialLobbySlice } from "./shell/lobby/submodel";
import { isPortraitPhone } from "./subscriptions";

export const init = (url?: Url) => {
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

  const model = {
    ready: true,
    route,
    currentPath,
    session: { me: null, meGravatarHash: null },
    sessionLoaded: false,
    apiVersion: null,
    faithfulCount: null,
    oracleTotal: null,
    auth: initialAuthSubmodel(next),
    decks: initialDecksSubmodel(),
    leaderboard: initialLeaderboardSubmodel(),
    lobby: initialLobbySlice(),
    game: null,
    portraitGate: { open: isPortraitPhone() },
  };
  const commands = [
    Command.mapMessage(FetchMe(), (message) => GotAuthMessage({ message })),
    FetchApiVersion(),
  ] as const;

  return [model, commands] as const;
};
