import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command, Navigation } from "foldkit";
import { statusOf } from "../../../lib/rpc-client";
import { RpcClient } from "../../resources";
import { safeNext } from "../../routes";
import { AuthFailed, type Message, ReceivedMe } from "./messages";
import type { AuthSubmodel } from "./submodel";

function authErrorMessage(error: unknown): string {
  if (statusOf(error) === 401) return "Wrong email or password.";
  if (statusOf(error) === 409) return "That email is already registered.";
  return "Something went wrong.";
}

export const FetchMe = Command.define(
  "FetchMe",
  ReceivedMe,
)(
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.me().pipe(
      Effect.map((me) => ReceivedMe({ me })),
      Effect.catch(() => Effect.succeed(ReceivedMe({ me: null }))),
    );
  }),
);

export const Login = Command.define(
  "Login",
  { email: S.String, password: S.String, next: S.String },
  ReceivedMe,
  AuthFailed,
)(({ email, password, next }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.login({ email, password }).pipe(
      Effect.tap(() => Navigation.replaceUrl(safeNext(next))),
      Effect.map((me) => ReceivedMe({ me })),
      Effect.catch((error) => Effect.succeed(AuthFailed({ message: authErrorMessage(error) }))),
    );
  }),
);

export const Signup = Command.define(
  "Signup",
  { email: S.String, password: S.String, username: S.String, next: S.String },
  ReceivedMe,
  AuthFailed,
)(({ email, password, username, next }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.signup({ email, password, username }).pipe(
      Effect.tap(() => Navigation.replaceUrl(safeNext(next))),
      Effect.map((me) => ReceivedMe({ me })),
      Effect.catch((error) => Effect.succeed(AuthFailed({ message: authErrorMessage(error) }))),
    );
  }),
);

export const Logout = Command.define(
  "Logout",
  ReceivedMe,
  AuthFailed,
)(
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.logout().pipe(
      Effect.tap(() => Navigation.replaceUrl("/login")),
      Effect.as(ReceivedMe({ me: null })),
      Effect.catch(() => Effect.succeed(AuthFailed({ message: "Couldn't sign out — try again." }))),
    );
  }),
);

export const update = (
  model: AuthSubmodel,
  message: Message,
): readonly [AuthSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [AuthSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
    M.tagsExhaustive({
      ChangedAuthMode: ({ mode }) => [{ ...model, mode, error: null }, []],
      ChangedAuthEmail: ({ email }) => [{ ...model, email }, []],
      ChangedAuthUsername: ({ username }) => [{ ...model, username }, []],
      ChangedAuthPassword: ({ password }) => [{ ...model, password }, []],
      SubmittedAuth: () => {
        const nextModel = { ...model, error: null, submitting: true };
        if (model.mode === "login") {
          return [nextModel, [Login({ email: model.email, password: model.password, next: model.next })]];
        }
        return [
          nextModel,
          [Signup({ email: model.email, password: model.password, username: model.username, next: model.next })],
        ];
      },
      RequestedLogout: () => [model, [Logout()]],
      ReceivedMe: () => [{ ...model, password: "", error: null, submitting: false }, []],
      AuthFailed: ({ message }) => [{ ...model, error: message, submitting: false }, []],
    }),
  );
