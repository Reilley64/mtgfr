import * as Menu from "@foldkit/ui/menu";

/** The two things the account menu can do. `Menu.create` keys the `Selected` out-message to this
 *  union, so a new row and its handler cannot drift apart. */
export type AccountMenuItem = "gravatar" | "sign-out";

export const AccountMenu = Menu.create<AccountMenuItem>();

/** Document-unique id. Menu keys its button, ARIA, focus, and anchoring commands on it. */
export const ACCOUNT_MENU_ID = "account-menu";

export const GRAVATAR_URL = "https://gravatar.com";
