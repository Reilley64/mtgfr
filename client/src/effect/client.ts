// The wire API as the generated Effect client, consumed directly by callers.
//
// `make` (from `../api/generated`, regenerated from the server's OpenAPI at build time) turns a
// concrete `HttpClient` into a typed method-per-endpoint client whose methods are self-contained
// Effects (`Effect<A, HttpClientError | SchemaError | MtgfrError<…>>`, no service requirement). We
// build the fetch-backed `HttpClient` once here and share the client; callers wrap its methods in
// an `Atom` (ADR 0019) and fold failures inside the pipeline — branching on `statusOf` (HTTP
// errors) or the generated `MtgfrError` tags (declared error bodies, e.g. a deck's 422). There is
// no hand-written service wrapper, and nothing runs a client Effect to a promise by hand.

import * as Effect from "effect/Effect";
import * as FetchHttpClient from "effect/unstable/http/FetchHttpClient";
import * as HttpClient from "effect/unstable/http/HttpClient";
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest";
import { make } from "~/api/generated";

/** Same-origin BFF prefix — SolidStart proxies `/api/*` to `API_UPSTREAM` (strip `/api`). */
const API_ORIGIN = "/api";

/** Include cookies on every request (session + sticky affinity). */
function withCredentials(fetchImpl: typeof globalThis.fetch): typeof globalThis.fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) =>
    fetchImpl(input, { ...init, credentials: "include" })) as typeof globalThis.fetch;
}

/** Build a client over a specific `fetch`. The fetch layer has no finalizers, so the `HttpClient`
 * resolves synchronously. Tests use this with a stub `fetch`; the app uses the default `client`. */
export function makeClient(fetchImpl: typeof globalThis.fetch) {
  const httpClient = Effect.runSync(
    Effect.gen(function* () {
      return yield* HttpClient.HttpClient;
    }).pipe(
      Effect.provide(FetchHttpClient.layer),
      Effect.provideService(FetchHttpClient.Fetch, withCredentials(fetchImpl)),
    ),
  );
  return make(HttpClient.mapRequest(httpClient, HttpClientRequest.prependUrl(API_ORIGIN)));
}

/** The generated wire client (over the real `fetch`). Wrap its methods in an `Atom` (ADR 0019). */
export const client = makeClient(globalThis.fetch);

export type Client = typeof client;

/**
 * Fold any recoverable failure to `null`, leaving the success value untouched.
 *
 * For endpoints that answer every *logical* failure with a 200 body (the lobby's actions return a
 * `LobbyView` whose `error` field names the reason), a failed Effect means only one thing: the
 * request never landed. `null` is that, as a value the caller can branch on — so the fold lives
 * here in the pipeline rather than as a `try`/`catch` around an awaited promise in a component.
 *
 * Defects pass through as defects: a bug must not masquerade as an unreachable server.
 */
export const orNull = <A, E, R>(effect: Effect.Effect<A, E, R>): Effect.Effect<A | null, never, R> =>
  Effect.catch(effect, () => Effect.succeed(null));

/**
 * `orNull` for an endpoint that answers with no body: fold the outcome to did-it-land. Same seam and
 * same defect semantics, but `void | null` would make "succeeded" and "failed" both falsy.
 */
export const succeeded = <E, R>(effect: Effect.Effect<unknown, E, R>): Effect.Effect<boolean, never, R> =>
  effect.pipe(
    Effect.as(true),
    Effect.catch(() => Effect.succeed(false)),
  );

/**
 * The HTTP status carried by a failed client Effect, when it has a response — an `HttpClientError`
 * (e.g. auth 401/409, a session-expired 401 on `/intent`) or a generated `MtgfrError` both expose
 * `response.status`. `undefined` for a transport/decoding failure with no response.
 */
export function statusOf(error: unknown): number | undefined {
  return (error as { response?: { status?: number } } | null)?.response?.status;
}
