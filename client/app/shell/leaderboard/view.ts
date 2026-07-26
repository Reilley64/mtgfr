import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import type { AppChromeMeta } from "../../domain/ui/app-version";
import { buttonClass } from "../../domain/ui/buttonClass";
import { listRowClass } from "../../domain/ui/surfaces";
import type { ClosedAccountMenu, GotAuthMessage, ToggledAccountMenu } from "../../messages";
import { HomeRoute, routePath } from "../../routes";
import { accountChrome } from "../account-chrome/view";
import { shellFrame } from "../frame/shell-frame";
import {
  type Message as LeaderboardMessage,
  RequestedLeaderboardNextPage,
  RequestedLeaderboardRefresh,
} from "./messages";
import type { LeaderboardStatus, LeaderboardSubmodel } from "./submodel";

export type ViewMessage =
  | LeaderboardMessage
  | typeof ClosedAccountMenu.Type
  | typeof GotAuthMessage.Type
  | typeof ToggledAccountMenu.Type;

export type ViewInputs = {
  username: string;
  meGravatarHash: string | null;
  chrome: AppChromeMeta;
};

const h = html<ViewMessage>();

function statusCopy(status: LeaderboardStatus): string | null {
  switch (status) {
    case "idle":
      return "Leaderboard has not loaded yet.";
    case "loading":
      return "Loading leaderboard...";
    case "ready":
      return null;
    case "error":
      return null;
    default: {
      const exhaustive: never = status;
      return exhaustive;
    }
  }
}

function row(entry: LeaderboardSubmodel["entries"][number]): Html {
  return h.div(
    [
      h.DataAttribute("testid", "leaderboard-row"),
      h.Class(listRowClass("grid grid-cols-[72px_1fr_96px] items-center gap-md rounded-control px-md py-sm")),
    ],
    [
      h.span([h.Class("text-label text-lichen")], [`#${entry.rank}`]),
      h.span([h.Class("min-w-0 truncate text-body")], [entry.username]),
      h.span([h.Class("text-right text-game text-priority-gold")], [String(entry.rating)]),
    ],
  );
}

export const view = Submodel.defineView<LeaderboardSubmodel, ViewMessage, ViewInputs>((model, viewInputs): Html => {
  const { chrome, meGravatarHash, username } = viewInputs;
  const status = statusCopy(model.status);
  const canLoadMore = model.status !== "error" && model.entries.length < model.total;

  return shellFrame(h, {
    atmosphere: "shell",
    title: "Leaderboard",
    chrome,
    leading: h.a([h.Href(routePath(HomeRoute())), h.Class(buttonClass("ghost"))], ["Play"]),
    trailing: accountChrome(h, {
      username,
      gravatarHash: meGravatarHash,
      menuOpen: model.accountMenuOpen,
      showLeaderboardLink: false,
    }),
    stage: h.div(
      [h.Class("h-full overflow-y-auto"), h.DataAttribute("testid", "leaderboard-page")],
      [
        h.section(
          [h.Class("mx-auto flex max-w-[720px] flex-col gap-sm")],
          [
            model.error == null
              ? null
              : h.div([h.Role("alert"), h.Class("text-label text-reconnect-rust")], [model.error]),
            status == null ? null : h.div([h.Class("text-label text-lichen")], [status]),
            model.status === "ready" && model.entries.length === 0
              ? h.div([h.Class("text-label text-lichen")], ["No rated games yet."])
              : null,
            ...model.entries.map(row),
            canLoadMore
              ? h.button(
                  [
                    h.Type("button"),
                    h.OnClick(RequestedLeaderboardNextPage()),
                    h.Class(buttonClass("ghost", "mt-md self-start")),
                    h.Disabled(model.status === "loading"),
                  ],
                  [model.status === "loading" ? "Loading..." : "Load more"],
                )
              : null,
            model.status === "error"
              ? h.button(
                  [
                    h.Type("button"),
                    h.OnClick(RequestedLeaderboardRefresh()),
                    h.Class(buttonClass("ghost", "mt-md self-start")),
                  ],
                  ["Try again"],
                )
              : null,
          ],
        ),
      ],
    ),
  });
});
