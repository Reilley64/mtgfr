// Shared atoms per ADR 0019: screens consume these via `@effect/atom-solid` hooks.
// Screen-local atoms live in their own component files, not here.

import * as Effect from "effect/Effect";
import * as Atom from "effect/unstable/reactivity/Atom";
import type { DeckSummary, Me } from "~/api/generated";
import { client } from "~/effect/client";

/** The signed-in user, or `null` when not signed in. Any failure (401, decode, transport) is
 * folded to "not signed in" — mirrors guard.ts's `useAuthGuard` semantics. */
export const meAtom = Atom.make(client.me({}).pipe(Effect.catch(() => Effect.succeed(null as Me | null))));

/** Skip `listDecks` until we know there's a session — anonymous first visit must not race a 401
 * into the error boundary while the auth guard is still redirecting to /login. */
export function decksEffectForMe<E, R>(
  me: Me | null,
  listDecks: Effect.Effect<ReadonlyArray<DeckSummary>, E, R>,
): Effect.Effect<ReadonlyArray<DeckSummary>, E, R> {
  if (me == null) return Effect.succeed([]);
  return listDecks;
}

/** The saved-deck list. Waits on `meAtom`; returns `[]` when not signed in. Callers still gate the
 * UI on a signed-in user (Decks / Play) so an anonymous visit never mounts this atom needlessly. */
export const decksAtom = Atom.make((get) =>
  get.result(meAtom).pipe(Effect.flatMap((me) => decksEffectForMe(me, client.listDecks({})))),
);
