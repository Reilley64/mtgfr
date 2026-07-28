import type { html as createHtml, Html } from "foldkit/html";
import { cn } from "../../domain/cn";
import { button } from "../../domain/ui/button";
import { seatFace } from "../../domain/ui/seat-face";
import { GotAuthMessage } from "../../messages";
import { LeaderboardRoute, routePath } from "../../routes";
import * as Auth from "../auth";
import { BindAccountMenuEscape } from "./escape";
import { ClosedAccountMenu, ToggledAccountMenu } from "./messages";

const MENU_ITEM =
  "cursor-pointer rounded-control border-none bg-transparent px-md py-xs text-left text-label text-snow hover:bg-white/8 focus-visible:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

export type AccountChromeOptions = {
  username: string;
  gravatarHash: string | null;
  menuOpen: boolean;
  showLeaderboardLink: boolean;
};

export function accountChrome<Msg>(h: HtmlFactory<Msg>, options: AccountChromeOptions): Html {
  return h.div(
    [h.Class("flex flex-wrap items-center gap-md")],
    [
      options.showLeaderboardLink
        ? button(
            h,
            { as: "a", href: routePath(LeaderboardRoute()), testId: "header-leaderboard-link", variant: "ghost" },
            ["Leaderboard"],
          )
        : null,
      h.div(
        [
          h.Class("relative"),
          ...(options.menuOpen
            ? [h.DataAttribute("testid", "account-menu-root"), h.OnMount(BindAccountMenuEscape() as never)]
            : []),
        ],
        [
          button(
            h,
            {
              testId: "account-menu-trigger",
              ariaLabel: "Account",
              onClick: ToggledAccountMenu() as never,
              variant: "ghost",
              class: "rounded-full p-0",
              attrs: [h.Attribute("aria-expanded", String(options.menuOpen)), h.Attribute("aria-haspopup", "menu")],
            },
            [
              seatFace(h, {
                seat: 0,
                username: options.username,
                gravatarHash: options.gravatarHash,
                className: "size-9",
              }),
            ],
          ),
          options.menuOpen
            ? h.div(
                [],
                [
                  h.div(
                    [
                      h.Class("fixed inset-0 z-40"),
                      h.DataAttribute("testid", "account-menu-catcher"),
                      h.OnClick(ClosedAccountMenu() as never),
                      h.OnContextMenu(ClosedAccountMenu() as never),
                    ],
                    [],
                  ),
                  h.div(
                    [
                      h.DataAttribute("testid", "account-menu"),
                      h.Class(
                        "absolute top-full right-0 z-41 mt-xs flex min-w-[180px] flex-col rounded-hud border border-vine bg-forest-surface p-xs shadow-table",
                      ),
                    ],
                    [
                      h.span(
                        [
                          h.DataAttribute("testid", "account-menu-username"),
                          h.Class("px-md py-xs text-label text-lichen"),
                        ],
                        [options.username],
                      ),
                      h.a(
                        [
                          h.Href("https://gravatar.com"),
                          h.DataAttribute("testid", "account-gravatar-link"),
                          h.Attribute("target", "_blank"),
                          h.Attribute("rel", "noopener noreferrer"),
                          h.Class(cn(MENU_ITEM, "no-underline")),
                        ],
                        ["Change at Gravatar"],
                      ),
                      h.button(
                        [
                          h.Type("button"),
                          h.DataAttribute("testid", "account-menu-sign-out"),
                          h.OnClick(GotAuthMessage({ message: Auth.Message.RequestedLogout() }) as never),
                          h.Class(MENU_ITEM),
                        ],
                        ["Sign out"],
                      ),
                    ],
                  ),
                ],
              )
            : null,
        ],
      ),
    ],
  );
}
