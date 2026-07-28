// Account chrome: the avatar trigger and its dropdown.
//
// Open/close, Escape, click-away, arrow-key navigation, typeahead, focus return to the trigger, and
// Floating-UI anchoring all come from @foldkit/ui's Menu. Menu renders its own trigger button and
// item elements and takes class *strings*, so the chrome comes from `menuPanelClass` /
// `menuItemClass` rather than from wrapping the W1 `button()` component.
//
// Both rows commit as a `Selected` out-message the owner acts on — the gravatar link is not an
// `<a>`, because Menu's keyboard commit programmatically clicks the `role="menuitem"` div and a
// nested anchor would not follow.

import type * as Menu from "@foldkit/ui/menu";
import { childAttributes, type html as createHtml, type Html } from "foldkit/html";
import { button } from "../../domain/ui/button";
import { menuItemClass, menuPanelClass } from "../../domain/ui/menu";
import { seatFace } from "../../domain/ui/seat-face";
import { LeaderboardRoute, routePath } from "../../routes";
import { ACCOUNT_MENU_ID, AccountMenu, type AccountMenuItem } from "./menu";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

export type AccountChromeOptions<Msg> = {
  /** The owner's menu state. Create with `Menu.init({ id: ACCOUNT_MENU_ID })`. */
  menu: Menu.Model;
  /** Lifts a `Menu.Message` into the owner's message union. */
  toMenuMessage: (message: Menu.Message) => Msg;
  username: string;
  gravatarHash: string | null;
  showLeaderboardLink: boolean;
};

const ITEM_LABEL: Record<AccountMenuItem, string> = {
  gravatar: "Change at Gravatar",
  "sign-out": "Sign out",
};

const ITEMS: ReadonlyArray<AccountMenuItem> = ["gravatar", "sign-out"];

export function accountChrome<Msg>(h: HtmlFactory<Msg>, options: AccountChromeOptions<Msg>): Html {
  const { menu, toMenuMessage, username, gravatarHash, showLeaderboardLink } = options;

  return h.div(
    [h.Class("flex flex-wrap items-center gap-md")],
    [
      showLeaderboardLink
        ? button(
            h,
            { as: "a", href: routePath(LeaderboardRoute()), testId: "header-leaderboard-link", variant: "ghost" },
            ["Leaderboard"],
          )
        : null,
      h.submodel({
        slotId: ACCOUNT_MENU_ID,
        model: menu,
        view: AccountMenu.view,
        viewInputs: {
          items: ITEMS,
          ariaLabel: "Account",
          buttonContent: seatFace(h, { seat: 0, username, gravatarHash, className: "size-9" }),
          buttonClassName: "cursor-pointer rounded-full border-none bg-transparent p-0",
          buttonAttributes: childAttributes([h.DataAttribute("testid", "account-menu-trigger")]),
          itemsClassName: menuPanelClass("min-w-[180px]"),
          itemsAttributes: childAttributes([h.DataAttribute("testid", "account-menu")]),
          itemToConfig: (item: AccountMenuItem) => ({
            className: menuItemClass(),
            content: h.span([h.DataAttribute("testid", `account-menu-${item}`)], [ITEM_LABEL[item]]),
          }),
          // The signed-in username is a heading, not a row: Menu has no static-content slot and
          // every item is selectable.
          itemGroupKey: () => "account",
          groupToHeading: () => ({
            className: "px-md py-xs text-label text-lichen",
            content: h.span([h.DataAttribute("testid", "account-menu-username")], [username]),
          }),
          anchor: { placement: "bottom-end" as const, gap: 4 },
        },
        toParentMessage: toMenuMessage,
      }) as Html,
    ],
  );
}
