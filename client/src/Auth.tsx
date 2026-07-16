// Login / signup. On success the server sets the session cookie and we go to the deck list.

import { useAtomSet } from "@effect/atom-solid";
import { useNavigate, useSearchParams } from "@solidjs/router";
import * as Effect from "effect/Effect";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createSignal, Show } from "solid-js";
import { client, statusOf } from "~/effect/client";
import { Button, Felt, Field, Panel } from "~/ui";

type Creds = { mode: "login" | "signup"; email: string; password: string; username: string };

// Only a same-site absolute path is a safe post-auth redirect: reject a missing/relative `next`,
// a protocol-relative "//evil.com" or backslash-variant "/\evil.com" (browsers treat backslashes
// as slashes), and anything carrying a URL scheme — all open-redirect vectors.
function safeNext(next: string | string[] | undefined): string {
  const path = Array.isArray(next) ? next[0] : next;
  if (!path?.startsWith("/") || path.startsWith("//") || path.startsWith("/\\")) return "/";
  if (/^[a-z][a-z\d+.-]*:/i.test(path)) return "/";
  return path;
}

// Every branch resolves to a message-or-"ok" so the awaited promise never rejects. A
// bad-credentials 401 / duplicate-email 409 arrive as an `HttpClientError` (the spec no longer
// declares them), read off the status; anything else is a generic failure.
const authenticateFn = Atom.fn((creds: Creds) =>
  (creds.mode === "login"
    ? client.login({ payload: { email: creds.email, password: creds.password } })
    : client.signup({ payload: { email: creds.email, password: creds.password, username: creds.username } })
  ).pipe(
    Effect.as("ok" as const),
    Effect.catch((err) =>
      Effect.succeed(
        statusOf(err) === 401
          ? "Wrong email or password."
          : statusOf(err) === 409
            ? "That email is already registered."
            : "Something went wrong.",
      ),
    ),
  ),
);

export default function Auth() {
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const [mode, setMode] = createSignal<"login" | "signup">("login");
  const [email, setEmail] = createSignal("");
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const authenticate = useAtomSet(() => authenticateFn, { mode: "promise" });

  const submit = async (e: Event) => {
    e.preventDefault();
    setError(null);
    const outcome = await authenticate({
      mode: mode(),
      email: email(),
      username: username(),
      password: password(),
    });
    if (outcome === "ok") {
      navigate(safeNext(params.next), { replace: true });
      return;
    }
    setError(outcome);
  };

  const isLogin = () => mode() === "login";

  return (
    <Felt class="fixed inset-0 overflow-y-auto">
      <div class="flex min-h-full items-center justify-center p-xxl">
        <Panel as="main">
          <form onSubmit={submit} class="contents">
            <h1 class="m-0 text-title">mtgfr — {isLogin() ? "sign in" : "create account"}</h1>
            <label for="email" class="text-label text-lichen">
              Email
            </label>
            <Field
              id="email"
              type="email"
              autocomplete="email"
              value={email()}
              onInput={(e) => setEmail(e.currentTarget.value)}
            />
            <Show when={!isLogin()}>
              <label for="username" class="text-label text-lichen">
                Username
              </label>
              <Field
                id="username"
                type="text"
                autocomplete="username"
                value={username()}
                onInput={(e) => setUsername(e.currentTarget.value)}
              />
            </Show>
            <label for="password" class="text-label text-lichen">
              Password
            </label>
            <Field
              id="password"
              type="password"
              autocomplete={isLogin() ? "current-password" : "new-password"}
              value={password()}
              onInput={(e) => setPassword(e.currentTarget.value)}
            />
            <Button type="submit">{isLogin() ? "Sign in" : "Sign up"}</Button>
            <Show when={error()}>
              <div role="alert" class="text-burn-red text-caption">
                {error()}
              </div>
            </Show>
            <div class="text-label text-lichen">
              {isLogin() ? "No account? " : "Have an account? "}
              <Button
                type="button"
                variant="link"
                onClick={() => {
                  setError(null);
                  setMode(isLogin() ? "signup" : "login");
                }}
              >
                {isLogin() ? "Create one" : "Sign in"}
              </Button>
            </div>
          </form>
        </Panel>
      </div>
    </Felt>
  );
}
