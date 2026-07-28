import * as Menu from "@foldkit/ui/menu";
import { Schema as S } from "effect";
import { m } from "foldkit/message";

/** Delegation envelope for the account menu's Menu submodel. */
export const GotAccountMenuMessage = m("GotAccountMenuMessage", { message: Menu.Message });

export const Message = S.Union([GotAccountMenuMessage]);
export type Message = typeof Message.Type;
