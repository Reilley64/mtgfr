import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { Me } from "../../../lib/wire/types";
import { AuthMode } from "./submodel";

export const ChangedAuthMode = m("ChangedAuthMode", { mode: AuthMode });
export const ChangedAuthEmail = m("ChangedAuthEmail", { email: S.String });
export const ChangedAuthUsername = m("ChangedAuthUsername", { username: S.String });
export const ChangedAuthPassword = m("ChangedAuthPassword", { password: S.String });
export const SubmittedAuth = m("SubmittedAuth");
export const RequestedLogout = m("RequestedLogout");
export const ReceivedMe = m("ReceivedMe", { me: S.NullOr(Me) });
export const AuthFailed = m("AuthFailed", { message: S.String });

export const Message = S.Union([
  ChangedAuthMode,
  ChangedAuthEmail,
  ChangedAuthUsername,
  ChangedAuthPassword,
  SubmittedAuth,
  RequestedLogout,
  ReceivedMe,
  AuthFailed,
]);
export type Message = typeof Message.Type;
