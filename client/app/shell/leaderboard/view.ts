import type * as Menu from "@foldkit/ui/menu";
import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import type { AppChromeMeta } from "../../domain/ui/app-version";
import { button } from "../../domain/ui/button";
import { listRowClass } from "../../domain/ui/surfaces";
import { GotAccountMenuMessage, type GotAuthMessage } from "../../messages";
import { HomeRoute, routePath } from "../../routes";
import { accountChrome } from "../account-chrome/view";
import { shellFrame } from "../frame/shell-frame";
import { shellStatusChrome } from "../frame/shell-status";
import {
  type Message as LeaderboardMessage,
  RequestedLeaderboardNextPage,
  RequestedLeaderboardRefresh,
} from "./messages";
import type { LeaderboardSubmodel } from "./submodel";

export type ViewMessage = LeaderboardMessage | typeof GotAccountMenuMessage.Type | typeof GotAuthMessage.Type;

export type ViewInputs = {
  username: string;
  meGravatarHash: string | null;
  chrome: AppChromeMeta;
  accountMenu: Menu.Model;
};

const h = html<ViewMessage>();

function row(entry: LeaderboardSubmodel["entries"][number]): Html {
  return h.div(
    [
      h.DataAttribute("testid", "leaderboard-row"),
      h.Class(listRowClass("grid grid-cols-[72px_1fr_96px] items-center gap-md rounded-control px-md py-sm")),
    ],
    [
      h.span([h.Class("font-display text-title text-lichen")], [`#${entry.rank}`]),
      h.span([h.Class("min-w-0 truncate text-body text-snow")], [entry.username]),
      h.span([h.Class("text-right text-game text-vine")], [String(entry.rating)]),
    ],
  );
}

export const view = Submodel.defineView<LeaderboardSubmodel, ViewMessage, ViewInputs>((model, viewInputs): Html => {
  const { accountMenu, chrome, meGravatarHash, username } = viewInputs;
  const canLoadMore = model.status !== "error" && model.entries.length < model.total;

  return shellFrame(h, {
    atmosphere: "shell",
    title: "Leaderboard",
    chrome,
    leading: button(h, { as: "a", href: routePath(HomeRoute()), variant: "ghost" }, ["Play"]),
    trailing: accountChrome(h, {
      username,
      gravatarHash: meGravatarHash,
      menu: accountMenu,
      toMenuMessage: (message) => GotAccountMenuMessage({ message }),
      showLeaderboardLink: false,
    }),
    stage: h.div(
      [h.Class("h-full overflow-y-auto"), h.DataAttribute("testid", "leaderboard-page")],
      [
        h.section(
          [h.Class("mx-auto flex max-w-[720px] flex-col gap-sm")],
          [
            ...shellStatusChrome(h, {
              noun: "Leaderboard",
              status: model.status,
              error: model.error,
              retry: { testId: "leaderboard-try-again", onClick: RequestedLeaderboardRefresh() },
            }),
            model.status === "ready" && model.entries.length === 0
              ? h.div(
                  [h.Class("text-label text-lichen"), h.DataAttribute("testid", "leaderboard-empty")],
                  ["No rated games yet."],
                )
              : null,
            ...model.entries.map(row),
            canLoadMore
              ? button(
                  h,
                  {
                    testId: "leaderboard-load-more",
                    onClick: RequestedLeaderboardNextPage(),
                    variant: "ghost",
                    class: "mt-md self-start",
                    disabled: model.status === "loading",
                  },
                  [model.status === "loading" ? "Loading..." : "Load more"],
                )
              : null,
          ].filter((v): v is Html => v !== null),
        ),
      ],
    ),
  });
});
