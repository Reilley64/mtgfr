import { Effect } from "effect";
import * as HttpClientError from "effect/unstable/http/HttpClientError";
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest";
import { Story } from "foldkit";
import { afterEach, expect, test, vi } from "vitest";
import { client } from "../../../lib/rpc-client";
import { init, update } from "../../main-exports";
import { AuthFailed, NavigationCompleted, ReceivedMe } from "../../messages";
import { RpcClient } from "../../resources";
import { Logout } from "./update";

afterEach(() => {
  vi.unstubAllGlobals();
});

test("session folds me", () => {
  const [model] = init();
  const redirect = {
    name: "Redirect",
    args: { path: "/login?next=%2F" },
    effect: Effect.succeed(NavigationCompleted()),
  };

  Story.story(
    update,
    Story.with(model),
    Story.message(ReceivedMe({ me: null })),
    Story.Command.resolve(redirect, NavigationCompleted()),
    Story.model((m) => {
      expect(m.session.me).toBeNull();
    }),
  );
});

test("logout failure stays signed in and reports the error", async () => {
  const replaceState = vi.fn();
  const dispatchEvent = vi.fn();
  const logoutError = new HttpClientError.HttpClientError({
    reason: new HttpClientError.TransportError({
      request: HttpClientRequest.post("/auth/logout"),
      description: "logout failed",
    }),
  });
  const failingClient = {
    ...client,
    logout: () => Effect.fail(logoutError),
  };

  vi.stubGlobal("CustomEvent", class CustomEventStub {});
  vi.stubGlobal("window", {
    history: { replaceState },
    dispatchEvent,
  });

  const message = await Effect.runPromise(Logout().effect.pipe(Effect.provideService(RpcClient, failingClient)));

  expect(message).toEqual(AuthFailed({ message: "Couldn't sign out — try again." }));
  expect(replaceState).not.toHaveBeenCalled();
  expect(dispatchEvent).not.toHaveBeenCalled();
});
