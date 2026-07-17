// Auth guard for protected screens: consume the shared `meAtom`, redirecting to /login if absent.

import { useAtomRefresh, useAtomResource } from "@effect/atom-solid";
import { useNavigate } from "@solidjs/router";
import { type Accessor, createEffect, type JSX, type Resource, Show } from "solid-js";
import type { Me } from "~/api/generated";
import { meAtom } from "~/atoms";

/** The signed-in user, or `null` when not signed in. Redirects to /login once loaded with no
 * user. `meAtom` is shared, cross-screen state, so a prior screen may have already cached a
 * `null` (fetched before sign-in); refresh it on mount so a guard mounted right after a
 * successful `/login` navigation sees the current session instead of the stale `null`. */
export function useAuthGuard(): Resource<Me | null> {
  const navigate = useNavigate();
  const [user] = useAtomResource(() => meAtom);
  const refreshMe = useAtomRefresh(() => meAtom);
  refreshMe();
  createEffect(() => {
    if (user.state === "ready" && user() === null) {
      const next = encodeURIComponent(location.pathname + location.search);
      navigate(`/login?next=${next}`, { replace: true });
    }
  });
  return user;
}

/** Mount children only once a session is known, with a non-null `Me`. Owns redirect + gate so
 * protected screens don't each hand-roll `useAuthGuard` + `<Show>`. */
export function RequireAuth(props: { children: (user: Accessor<Me>) => JSX.Element }) {
  const user = useAuthGuard();
  return <Show when={user()}>{(u) => props.children(u)}</Show>;
}
