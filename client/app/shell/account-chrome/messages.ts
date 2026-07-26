import { Schema as S } from "effect";
import { m } from "foldkit/message";

export const ToggledAccountMenu = m("ToggledAccountMenu");
export const ClosedAccountMenu = m("ClosedAccountMenu");

export const Message = S.Union([ToggledAccountMenu, ClosedAccountMenu]);
export type Message = typeof Message.Type;
