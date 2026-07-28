import * as Menu from "@foldkit/ui/menu";
import { Option } from "effect";
import { Command } from "foldkit";
import type { Url } from "foldkit/url";
import { FetchApiVersion } from "./fetch-api-version";
import { GotAuthMessage } from "./messages";
import { nextFromUrl, normalizeAppRoute, pathWithSearch, routeFromUrl } from "./routes";
import { ACCOUNT_MENU_ID } from "./shell/account-chrome/menu";
import { initialAuthSubmodel } from "./shell/auth/submodel";
import { FetchMe } from "./shell/auth/update";
import { initialCoverageSubmodel } from "./shell/coverage/submodel";
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
    coverage: initialCoverageSubmodel(),
    lobby: initialLobbySlice(),
    game: null,
    accountMenu: Menu.init({ id: ACCOUNT_MENU_ID }),
    landscapeRotate: { active: isPortraitPhone() },
  };
  const commands = [
    Command.mapMessage(FetchMe(), (message) => GotAuthMessage({ message })),
    FetchApiVersion(),
  ] as const;

  return [model, commands] as const;
};
