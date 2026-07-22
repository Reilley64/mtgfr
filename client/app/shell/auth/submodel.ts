import { Schema as S } from "effect";

export const AuthMode = S.Union([S.Literal("login"), S.Literal("signup")]);
export type AuthMode = typeof AuthMode.Type;

export const AuthSubmodel = S.Struct({
  mode: AuthMode,
  email: S.String,
  username: S.String,
  password: S.String,
  error: S.NullOr(S.String),
  next: S.String,
  submitting: S.Boolean,
});
export type AuthSubmodel = typeof AuthSubmodel.Type;

export function initialAuthSubmodel(next = "/"): AuthSubmodel {
  return {
    mode: "login",
    email: "",
    username: "",
    password: "",
    error: null,
    next,
    submitting: false,
  };
}
