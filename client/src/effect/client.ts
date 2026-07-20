// Hand-written Effect client over same-origin `/api/rpc`. Deck 422s fail as Schema tagged errors.

import * as Effect from "effect/Effect";
import * as Schema from "effect/Schema";
import * as Stream from "effect/Stream";
import * as FetchHttpClient from "effect/unstable/http/FetchHttpClient";
import * as HttpClient from "effect/unstable/http/HttpClient";
import * as HttpClientError from "effect/unstable/http/HttpClientError";
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest";
import * as HttpClientResponse from "effect/unstable/http/HttpClientResponse";
import {
  Ack,
  type CatalogCard,
  CreateDeck422,
  type Credentials,
  DeckDetail,
  DeckError,
  DeckSummary,
  type IntentEnvelope,
  Me,
  type SaveDeckRequest,
  type SignupCredentials,
  type StackDwellRequest,
  type StreamFrame,
  UpdateDeck422,
  type YieldRequest,
} from "~/wire/types";

const API_ORIGIN = "/api/rpc";

/** CatalogCard Schema deferred (WireKind union); accept any JSON array from the BFF. */
const CatalogCardList = Schema.Array(Schema.Unknown) as Schema.Codec<ReadonlyArray<CatalogCard>>;

function withCredentials(fetchImpl: typeof globalThis.fetch): typeof globalThis.fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) =>
    fetchImpl(input, { ...init, credentials: "include" })) as typeof globalThis.fetch;
}

/** Build a client over a specific `fetch`. The fetch layer has no finalizers, so the `HttpClient`
 * resolves synchronously. Tests use this with a stub `fetch`; the app uses the default `client`. */
export function makeClient(fetchImpl: typeof globalThis.fetch) {
  const httpClient = Effect.runSync(
    HttpClient.HttpClient.pipe(
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

  /** Execute `request`, Schema-decoding a 2xx JSON body. */
  function json<A, I, RD>(
    schema: Schema.ConstraintCodec<A, I, RD, unknown>,
    request: HttpClientRequest.HttpClientRequest,
  ): Effect.Effect<A, HttpClientError.HttpClientError | Schema.SchemaError, RD> {
    return base.execute(request).pipe(
      Effect.flatMap(
        HttpClientResponse.matchStatus({
          "2xx": (response) => HttpClientResponse.schemaBodyJson(schema)(response),
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

  /** Deck write: 422 → tagged Schema error with decoded `DeckError`. */
  function jsonOrDeckError<A, I, RD, E extends CreateDeck422 | UpdateDeck422>(
    schema: Schema.ConstraintCodec<A, I, RD, unknown>,
    toTagged: (cause: typeof DeckError.Type) => E,
    request: HttpClientRequest.HttpClientRequest,
  ): Effect.Effect<A, HttpClientError.HttpClientError | Schema.SchemaError | E, RD> {
    return base.execute(request).pipe(
      Effect.flatMap(
        HttpClientResponse.matchStatus({
          "2xx": (response) => HttpClientResponse.schemaBodyJson(schema)(response),
          "422": (response) =>
            HttpClientResponse.schemaBodyJson(DeckError)(response).pipe(
              Effect.flatMap((cause) => Effect.fail(toTagged(cause))),
            ),
          orElse: unexpectedStatus,
        }),
      ),
    );
  }

  return {
    httpClient: base,

    signup: (payload: SignupCredentials) =>
      json(Me, HttpClientRequest.post("/auth/signup").pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    login: (payload: Credentials) =>
      json(Me, HttpClientRequest.post("/auth/login").pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    logout: () => empty(HttpClientRequest.post("/auth/logout")),
    me: () => json(Me, HttpClientRequest.get("/auth/me")),

    searchCards: (params: { q: string; limit: number; offset: number }) =>
      json(
        CatalogCardList,
        HttpClientRequest.get("/cards/search").pipe(
          HttpClientRequest.setUrlParams({ q: params.q, limit: params.limit, offset: params.offset }),
        ),
      ),
    lookupCards: (ids: ReadonlyArray<string>) =>
      json(CatalogCardList, HttpClientRequest.get("/cards/lookup").pipe(HttpClientRequest.setUrlParams({ ids }))),

    listDecks: () => json(Schema.Array(DeckSummary), HttpClientRequest.get("/decks")),
    createDeck: (payload: SaveDeckRequest) =>
      jsonOrDeckError(
        DeckDetail,
        (cause) => new CreateDeck422({ cause }),
        HttpClientRequest.post("/decks").pipe(HttpClientRequest.bodyJsonUnsafe(payload)),
      ),
    getDeck: (id: string) => json(DeckDetail, HttpClientRequest.get(`/decks/${id}`)),
    updateDeck: (id: string, payload: SaveDeckRequest) =>
      jsonOrDeckError(
        DeckDetail,
        (cause) => new UpdateDeck422({ cause }),
        HttpClientRequest.put(`/decks/${id}`).pipe(HttpClientRequest.bodyJsonUnsafe(payload)),
      ),
    deleteDeck: (id: string) => empty(HttpClientRequest.make("DELETE")(`/decks/${id}`)),

    submitIntent: (table: string, envelope: IntentEnvelope) =>
      json(Ack, HttpClientRequest.post(`/game/${table}/intent`).pipe(HttpClientRequest.bodyJsonUnsafe(envelope))),
    setYield: (table: string, payload: YieldRequest) =>
      json(Ack, HttpClientRequest.post(`/game/${table}/yield`).pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    setTurnYield: (table: string, payload: YieldRequest) =>
      json(Ack, HttpClientRequest.post(`/game/${table}/turn-yield`).pipe(HttpClientRequest.bodyJsonUnsafe(payload))),
    setStackDwell: (table: string, payload: StackDwellRequest) =>
      json(Ack, HttpClientRequest.post(`/game/${table}/stack-dwell`).pipe(HttpClientRequest.bodyJsonUnsafe(payload))),

    /** SSE delta stream. Full StreamFrame Schema is deferred; invalid JSON fails the stream. */
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

/** The wire client (over the real `fetch`). Wrap its methods in an `Atom` (client-shell-deck-builder-and-observability spec). */
export const client = makeClient(globalThis.fetch);

export type Client = typeof client;

/**
 * Fold any recoverable failure to `null`, leaving the success value untouched.
 *
 * Defects pass through as defects: a bug must not masquerade as an unreachable server.
 */
export const orNull = <A, E, R>(effect: Effect.Effect<A, E, R>): Effect.Effect<A | null, never, R> =>
  Effect.catch(effect, () => Effect.succeed(null));

/**
 * `orNull` for an endpoint that answers with no body: fold the outcome to did-it-land.
 */
export const succeeded = <E, R>(effect: Effect.Effect<unknown, E, R>): Effect.Effect<boolean, never, R> =>
  effect.pipe(
    Effect.as(true),
    Effect.catch(() => Effect.succeed(false)),
  );

/**
 * The HTTP status carried by a failed client Effect, when it has a response.
 * Tagged deck 422s and Schema decode errors have no status.
 */
export function statusOf(error: unknown): number | undefined {
  if (HttpClientError.isHttpClientError(error)) {
    return error.response?.status;
  }
  return undefined;
}
