// Hand-written Effect client over same-origin `/api/rpc`. Deck 422s fail as `{_tag, cause: DeckError}`.

import * as Effect from "effect/Effect";
import * as Stream from "effect/Stream";
import * as FetchHttpClient from "effect/unstable/http/FetchHttpClient";
import * as HttpClient from "effect/unstable/http/HttpClient";
import * as HttpClientError from "effect/unstable/http/HttpClientError";
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest";
import * as HttpClientResponse from "effect/unstable/http/HttpClientResponse";
import type {
  Ack,
  CatalogCard,
  Credentials,
  DeckDetail,
  DeckError,
  DeckSummary,
  IntentEnvelope,
  Me,
  SaveDeckRequest,
  SignupCredentials,
  StackDwellRequest,
  StreamFrame,
  YieldRequest,
} from "~/wire/types";

const API_ORIGIN = "/api/rpc";

export interface MtgfrError<Tag extends string, E> {
  _tag: Tag;
  cause: E;
}

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
  const base = HttpClient.mapRequest(httpClient, HttpClientRequest.prependUrl(API_ORIGIN));

  const unexpectedStatus = (response: HttpClientResponse.HttpClientResponse) =>
    Effect.flatMap(
      Effect.orElseSucceed(response.json, () => "Unexpected status code"),
      (description) =>
        Effect.fail(
          new HttpClientError.HttpClientError({
            reason: new HttpClientError.StatusCodeError({
              request: response.request,
              response,
              description: typeof description === "string" ? description : JSON.stringify(description),
            }),
          }),
        ),
    );

  /** Execute `request`, decoding a 2xx JSON body as `A`; anything else fails as `HttpClientError`. */
  function json<A>(request: HttpClientRequest.HttpClientRequest): Effect.Effect<A, HttpClientError.HttpClientError> {
    return base.execute(request).pipe(
      Effect.flatMap(
        HttpClientResponse.matchStatus({
          "2xx": (response) => response.json as Effect.Effect<A, HttpClientError.HttpClientError>,
          orElse: unexpectedStatus,
        }),
      ),
    );
  }

  /** Execute `request`, expecting a bodiless 2xx (logout, delete). */
  function empty(request: HttpClientRequest.HttpClientRequest): Effect.Effect<void, HttpClientError.HttpClientError> {
    return base.execute(request).pipe(
      Effect.flatMap(
        HttpClientResponse.matchStatus({
          "2xx": () => Effect.void,
          orElse: unexpectedStatus,
        }),
      ),
    );
  }

  /** Execute a deck write request, tagging a 422 as `MtgfrError<Tag, DeckError>` instead of an
   * opaque `HttpClientError` — the shape `deck-builder.tsx` branches on to show every problem. */
  function jsonOrDeckError<Tag extends string, A>(
    tag: Tag,
    request: HttpClientRequest.HttpClientRequest,
  ): Effect.Effect<A, HttpClientError.HttpClientError | MtgfrError<Tag, DeckError>> {
    return base.execute(request).pipe(
      Effect.flatMap(
        HttpClientResponse.matchStatus({
          "2xx": (response) => response.json as Effect.Effect<A, HttpClientError.HttpClientError>,
          "422": (response) =>
            Effect.flatMap(response.json as Effect.Effect<DeckError, HttpClientError.HttpClientError>, (cause) =>
              Effect.fail<MtgfrError<Tag, DeckError>>({ _tag: tag, cause }),
            ),
          orElse: unexpectedStatus,
        }),
      ),
    );
  }

  return {
    httpClient: base,

    signup: (payload: SignupCredentials) =>
      json<Me>(HttpClientRequest.post("/auth/signup").pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    login: (payload: Credentials) =>
      json<Me>(HttpClientRequest.post("/auth/login").pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    logout: () => empty(HttpClientRequest.post("/auth/logout")),
    me: () => json<Me>(HttpClientRequest.get("/auth/me")),

    searchCards: (params: { q: string; limit: number; offset: number }) =>
      json<ReadonlyArray<CatalogCard>>(
        HttpClientRequest.get("/cards/search").pipe(
          HttpClientRequest.setUrlParams({ q: params.q, limit: params.limit, offset: params.offset }),
        ),
      ),
    lookupCards: (ids: ReadonlyArray<string>) =>
      json<ReadonlyArray<CatalogCard>>(
        HttpClientRequest.get("/cards/lookup").pipe(HttpClientRequest.setUrlParams({ ids })),
      ),

    listDecks: () => json<ReadonlyArray<DeckSummary>>(HttpClientRequest.get("/decks")),
    createDeck: (payload: SaveDeckRequest) =>
      jsonOrDeckError<"CreateDeck422", DeckDetail>(
        "CreateDeck422",
        HttpClientRequest.post("/decks").pipe(HttpClientRequest.bodyJsonUnsafe(payload)),
      ),
    getDeck: (id: string) => json<DeckDetail>(HttpClientRequest.get(`/decks/${id}`)),
    updateDeck: (id: string, payload: SaveDeckRequest) =>
      jsonOrDeckError<"UpdateDeck422", DeckDetail>(
        "UpdateDeck422",
        HttpClientRequest.put(`/decks/${id}`).pipe(HttpClientRequest.bodyJsonUnsafe(payload)),
      ),
    deleteDeck: (id: string) => empty(HttpClientRequest.make("DELETE")(`/decks/${id}`)),

    submitIntent: (table: string, envelope: IntentEnvelope) =>
      json<Ack>(HttpClientRequest.post(`/game/${table}/intent`).pipe(HttpClientRequest.bodyJsonUnsafe(envelope))),
    setYield: (table: string, payload: YieldRequest) =>
      json<Ack>(HttpClientRequest.post(`/game/${table}/yield`).pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    setTurnYield: (table: string, payload: YieldRequest) =>
      json<Ack>(HttpClientRequest.post(`/game/${table}/turn-yield`).pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    setStackDwell: (table: string, payload: StackDwellRequest) =>
      json<Ack>(HttpClientRequest.post(`/game/${table}/stack-dwell`).pipe(HttpClientRequest.bodyJsonUnsafe(payload))),

    /** The per-viewer delta stream as SSE (`text/event-stream`), decoded the same way the old
     * generated `streamSse` was: filter to `data: ` lines, `JSON.parse` each. */
    streamSse: (table: string): Stream.Stream<StreamFrame, HttpClientError.HttpClientError> =>
      HttpClient.filterStatusOk(base)
        .execute(HttpClientRequest.get(`/game/${table}/stream`))
        .pipe(
          Effect.map((response) => response.stream),
          Stream.unwrap,
          Stream.decodeText(),
          Stream.splitLines,
          Stream.filter((line) => line.startsWith("data: ")),
          Stream.map((line) => JSON.parse(line.slice(6)) as StreamFrame),
        ),
  };
}

/** The wire client (over the real `fetch`). Wrap its methods in an `Atom` (ADR 0019). */
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
 * (e.g. auth 401/409, a session-expired 401 on `/intent`) exposes `response.status`. `undefined`
 * for a transport/decoding failure with no response, or a tagged deck `MtgfrError` (never a status).
 */
export function statusOf(error: unknown): number | undefined {
  return (error as { response?: { status?: number } } | null)?.response?.status;
}
