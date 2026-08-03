import { Effect } from "effect";
import * as HttpClientError from "effect/unstable/http/HttpClientError";
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest";
import { Story } from "foldkit";
import { afterEach, expect, test, vi } from "vitest";
import { client } from "../../domain/rpc-client";
import { init, update } from "../../main-exports";
import { GotAuthMessage, NavigationCompleted, ReceivedMeGravatarHash } from "../../messages";
import { RpcClient } from "../../resources";
import { NotFoundRoute } from "../../routes";
import { HashMeGravatar } from "../../update";
import * as Auth from ".";
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
    Story.given(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me: null }) })),
    Story.Command.resolve(redirect, NavigationCompleted()),
    Story.model((m) => {
      expect(m.session.me).toBeNull();
    }),
  );
});

test("session stores me Gravatar hash from the ReceivedMe command", () => {
  const [model] = init();
  const email = "alice@example.com";
  const hash = "ff8d9819fc0e12bf0d24892e45987e249a28dce836a85cad60e28eaaa8c6d976";

  Story.story(
    update,
    Story.given({ ...model, route: NotFoundRoute({ path: "/done" }) }),
    Story.message(
      GotAuthMessage({
        message: Auth.Message.ReceivedMe({ me: { id: 1, email, username: "alice" } }),
      }),
    ),
    Story.Command.resolve(HashMeGravatar, ReceivedMeGravatarHash({ email, hash })),
    Story.model((m) => {
      expect(m.session.meGravatarHash).toBe(hash);
    }),
  );
});

test("session ignores stale me Gravatar hash results", () => {
  const [model] = init();

  Story.story(
    update,
    Story.given({
      ...model,
      sessionLoaded: true,
      session: { me: { id: 1, email: "alice@example.com", username: "alice" }, meGravatarHash: null },
    }),
    Story.message(ReceivedMeGravatarHash({ email: "bob@example.com", hash: "stale" })),
    Story.model((m) => {
      expect(m.session.meGravatarHash).toBeNull();
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

  expect(message).toEqual(Auth.Message.AuthFailed({ message: "Couldn't sign out — try again." }));
  expect(replaceState).not.toHaveBeenCalled();
  expect(dispatchEvent).not.toHaveBeenCalled();
});
