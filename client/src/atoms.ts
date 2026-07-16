// Shared atoms per ADR 0019: screens consume these via `@effect/atom-solid` hooks.
// Screen-local atoms live in their own component files, not here.

import * as Effect from "effect/Effect";
import * as Atom from "effect/unstable/reactivity/Atom";
import type { Me } from "~/api/generated";
import { client } from "~/effect/client";

/** The signed-in user, or `null` when not signed in. Any failure (401, decode, transport) is
 * folded to "not signed in" — mirrors guard.ts's `useAuthGuard` semantics. */
export const meAtom = Atom.make(client.me({}).pipe(Effect.catch(() => Effect.succeed(null as Me | null))));

/** The saved-deck list. No error folding — callers let it reject (Decks.tsx, Lobby.tsx). */
export const decksAtom = Atom.make(client.listDecks({}));
