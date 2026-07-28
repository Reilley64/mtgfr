// Server-only gRPC client for the BFF. Do not import from the browser bundle.
// Channels/runtimes are cached per base URL.
//
// Trace parenting: the gRPC ManagedRuntime is separate from the OTEL runtime.
// Effect parent spans live in fiber Context from the BFF OTEL runtime; gRPC effects use this
// client's ManagedRuntime context and always pass `outboundTraceparent` explicitly. Do not
// reintroduce Node ALS for this.

import { GrpcClientProtocol, type GrpcStatusCode, GrpcStatusError } from "@effect-grpc/effect-grpc";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as Stream from "effect/Stream";
import { EXCEPTION_TYPE, RPC_GRPC_STATUS_CODE, rpcAttrs } from "../otel/semconv";
import {
  AuthServiceClient,
  AuthServiceClientLayer,
  AuthServiceGrpcRegistry,
  CardsServiceClient,
  CardsServiceClientLayer,
  CardsServiceGrpcRegistry,
  DecksServiceClient,
  DecksServiceClientLayer,
  DecksServiceGrpcRegistry,
  GameServiceClient,
  GameServiceClientLayer,
  GameServiceGrpcRegistry,
  type Me as ProtoMe,
  RatingsServiceClient,
  RatingsServiceClientLayer,
  RatingsServiceGrpcRegistry,
  TablesServiceClient,
  TablesServiceClientLayer,
  TablesServiceGrpcRegistry,
} from "./generated/mtgfr/v1/mtgfr_effect_grpc";
import {
  ackFromProto,
  catalogCardsFromProto,
  createDeckToProto,
  deckDetailFromProto,
  deckSummaryListFromProto,
  intentEnvelopeToProto,
  leaderboardFromProto,
  loginRequestToProto,
  seedRequestToProto,
  seedResponseFromProto,
  signupRequestToProto,
  streamFrameFromProto,
  updateDeckToProto,
} from "./protoMap";
import type {
  Ack,
  CatalogCard,
  DeckDetail,
  IntentEnvelope,
  Leaderboard,
  Me,
  SaveDeckRequest,
  SeedRequest,
  SeedResponse,
  StreamFrame,
} from "./types";

export const SESSION_METADATA_KEY = "x-session-token";
export const TRACEPARENT_METADATA_KEY = "traceparent";

const AUTH_SERVICE = "mtgfr.v1.Auth";
const DECKS_SERVICE = "mtgfr.v1.Decks";
const RATINGS_SERVICE = "mtgfr.v1.Ratings";
const CARDS_SERVICE = "mtgfr.v1.Cards";
const GAME_SERVICE = "mtgfr.v1.Game";
const TABLES_SERVICE = "mtgfr.v1.Tables";

/**
 * Per-request bag for every BFF → API gRPC call.
 * Capture once at the HTTP edge (`currentTraceparent` under `runTracedRequest`);
 * pass the same object into lobby helpers and `dispatchRpc`.
 */
export type GrpcRequestEnv = {
  readonly sessionToken: string | null;
  readonly traceparent: string | null;
};

const AllGrpcRegistry = new Map([
  ...AuthServiceGrpcRegistry,
  ...DecksServiceGrpcRegistry,
  ...RatingsServiceGrpcRegistry,
  ...CardsServiceGrpcRegistry,
  ...GameServiceGrpcRegistry,
  ...TablesServiceGrpcRegistry,
]);

function meFromProto(me: ProtoMe): Me {
  return { id: Number(me.id), email: me.email, username: me.username };
}

/** Build gRPC call metadata: session cookie token + optional W3C traceparent. */
export function callOpts(sessionToken: string | null, traceparent: string | null) {
  const metadata: Array<readonly [string, string]> = [];
  if (sessionToken) metadata.push([SESSION_METADATA_KEY, sessionToken]);
  if (traceparent) metadata.push([TRACEPARENT_METADATA_KEY, traceparent]);
  if (metadata.length === 0) return undefined;
  return { metadata };
}

export class GrpcCallError extends Error {
  readonly code: GrpcStatusCode.GrpcStatusCode;
  constructor(code: GrpcStatusCode.GrpcStatusCode, message: string) {
    super(message);
    this.name = "GrpcCallError";
    this.code = code;
  }
}

/** Map transport / Effect failures to `GrpcCallError`. Idempotent — the game stream path runs
 * `Stream.mapError(toCallError)` and then `Effect.catch(… toCallError)` on the same failure; a
 * second wrap used to turn `unavailable` into `unknown` and the SSE connect into a bare 500. */
export function toCallError(err: unknown): GrpcCallError {
  if (err instanceof GrpcCallError) return err;
  if (err instanceof GrpcStatusError.GrpcStatusError) {
    return new GrpcCallError(err.code, err.message);
  }
  return new GrpcCallError("unknown", err instanceof Error ? err.message : String(err));
}

export function grpcSpanName(service: string, method: string): string {
  return `${service}/${method}`;
}

function rpcFailureAttrs(err: unknown): Record<string, string> {
  const attrs: Record<string, string> = {
    [EXCEPTION_TYPE]: err instanceof Error && err.name ? err.name : "Error",
  };
  if (err instanceof GrpcStatusError.GrpcStatusError) {
    attrs[RPC_GRPC_STATUS_CODE] = err.code;
  }
  return attrs;
}

function annotateRpcFailure(err: unknown): Effect.Effect<void, never, never> {
  return Effect.annotateCurrentSpan(rpcFailureAttrs(err));
}

export function withRpcSpan<A, E, R>(
  service: string,
  method: string,
  operation: Effect.Effect<A, E, R>,
): Effect.Effect<A, E, R>;
export function withRpcSpan<A, E, R>(
  service: string,
  method: string,
  operation: Stream.Stream<A, E, R>,
): Stream.Stream<A, E, R>;
export function withRpcSpan(
  service: string,
  method: string,
  operation: Effect.Effect<unknown, unknown, unknown> | Stream.Stream<unknown, unknown, unknown>,
): Effect.Effect<unknown, unknown, unknown> | Stream.Stream<unknown, unknown, unknown> {
  const name = grpcSpanName(service, method);
  const attributes = rpcAttrs({ service, method });

  if (Effect.isEffect(operation)) {
    return operation.pipe(Effect.tapError(annotateRpcFailure), Effect.withSpan(name, { attributes }));
  }

  return operation.pipe(Stream.tapError(annotateRpcFailure), Stream.withSpan(name, { attributes }));
}

/** Normalize `host:port` or `http(s)://…` to the `http://host:port` baseUrl effect-grpc expects. */
export function grpcBaseUrl(address: string): string {
  if (address.startsWith("http://") || address.startsWith("https://")) {
    return address.replace(/\/$/, "");
  }
  return `http://${address}`;
}

type Clients =
  | AuthServiceClient
  | DecksServiceClient
  | RatingsServiceClient
  | CardsServiceClient
  | GameServiceClient
  | TablesServiceClient;

type GrpcRuntime = ManagedRuntime.ManagedRuntime<Clients, never>;

const runtimeCache = new Map<string, GrpcRuntime>();

function runtimeFor(address: string): GrpcRuntime {
  const baseUrl = grpcBaseUrl(address);
  const cached = runtimeCache.get(baseUrl);
  if (cached) return cached;

  const protocol = GrpcClientProtocol.layer({
    baseUrl,
    registry: AllGrpcRegistry,
  });
  const clients = Layer.mergeAll(
    AuthServiceClientLayer,
    DecksServiceClientLayer,
    RatingsServiceClientLayer,
    CardsServiceClientLayer,
    GameServiceClientLayer,
    TablesServiceClientLayer,
  ).pipe(Layer.provide(protocol));

  const runtime = ManagedRuntime.make(clients) as GrpcRuntime;
  runtimeCache.set(baseUrl, runtime);
  return runtime;
}

function run<A>(
  address: string,
  service: string,
  method: string,
  effect: Effect.Effect<A, unknown, Clients>,
): Effect.Effect<A, GrpcCallError> {
  return runtimeFor(address).contextEffect.pipe(
    Effect.flatMap((context) => withRpcSpan(service, method, Effect.provideContext(effect, context))),
    Effect.mapError(toCallError),
  );
}

function runStream<A>(
  address: string,
  service: string,
  method: string,
  stream: Stream.Stream<A, unknown, Clients>,
): Stream.Stream<A, GrpcCallError> {
  return Stream.unwrap(
    runtimeFor(address).contextEffect.pipe(
      Effect.map((context) =>
        withRpcSpan(service, method, stream.pipe(Stream.provideContext(context))).pipe(Stream.mapError(toCallError)),
      ),
    ),
  );
}

export interface GrpcClient {
  auth: {
    signup(
      req: { email: string; password: string; username: string },
      sessionToken: string | null,
    ): Effect.Effect<{ me: Me; sessionToken: string }, GrpcCallError>;
    login(
      req: { email: string; password: string },
      sessionToken: string | null,
    ): Effect.Effect<{ me: Me; sessionToken: string }, GrpcCallError>;
    logout(sessionToken: string | null): Effect.Effect<void, GrpcCallError>;
    getMe(sessionToken: string | null): Effect.Effect<Me, GrpcCallError>;
  };
  decks: {
    create(req: SaveDeckRequest, sessionToken: string | null): Effect.Effect<DeckDetail, GrpcCallError>;
    list(
      sessionToken: string | null,
    ): Effect.Effect<Array<{ commander: string; commander_print?: string; id: number; name: string }>, GrpcCallError>;
    get(id: number, sessionToken: string | null): Effect.Effect<DeckDetail, GrpcCallError>;
    update(id: number, req: SaveDeckRequest, sessionToken: string | null): Effect.Effect<DeckDetail, GrpcCallError>;
    delete(id: number, sessionToken: string | null): Effect.Effect<void, GrpcCallError>;
  };
  ratings: {
    getLeaderboard(
      req: { limit: number; offset: number },
      sessionToken: string | null,
    ): Effect.Effect<Leaderboard, GrpcCallError>;
  };
  cards: {
    catalog(): Effect.Effect<Array<CatalogCard>, GrpcCallError>;
    search(q: string, limit: number, offset: number): Effect.Effect<Array<CatalogCard>, GrpcCallError>;
    lookup(ids: Array<string>): Effect.Effect<Array<CatalogCard>, GrpcCallError>;
  };
  game: {
    submitIntent(
      tableId: string,
      envelope: IntentEnvelope,
      sessionToken: string | null,
    ): Effect.Effect<Ack, GrpcCallError>;
    setYield(tableId: string, enabled: boolean, sessionToken: string | null): Effect.Effect<Ack, GrpcCallError>;
    setTurnYield(tableId: string, enabled: boolean, sessionToken: string | null): Effect.Effect<Ack, GrpcCallError>;
    setStackDwell(tableId: string, dwelling: boolean, sessionToken: string | null): Effect.Effect<Ack, GrpcCallError>;
    stream(tableId: string, sessionToken: string | null): Stream.Stream<StreamFrame, GrpcCallError>;
  };
  tables: {
    seed(req: SeedRequest, sessionToken: string | null): Effect.Effect<SeedResponse, GrpcCallError>;
  };
}

const clientCache = new Map<string, GrpcClient>();

/**
 * gRPC client for one request. Parenting comes from `env.traceparent` (BFF span),
 * never from ambient context — the gRPC ManagedRuntime is separate from OTEL (production-topology-and-operations spec).
 */
export function grpcClientFor(address: string, env: GrpcRequestEnv): GrpcClient {
  return grpcClient(address, env.traceparent);
}

/**
 * @param outboundTraceparent BFF span `traceparent` for every call on this client.
 *   Prefer `grpcClientFor(address, env)` at call sites.
 */
export function grpcClient(address: string, outboundTraceparent: string | null = null): GrpcClient {
  const key = grpcBaseUrl(address);
  // Per-request traceparent must not be shared across concurrent callers via the address cache.
  if (outboundTraceparent === null) {
    const cached = clientCache.get(key);
    if (cached) return cached;
  }

  const opts = (sessionToken: string | null) => callOpts(sessionToken, outboundTraceparent);

  const client: GrpcClient = {
    auth: {
      signup: (req, sessionToken) =>
        run(
          key,
          AUTH_SERVICE,
          "Signup",
          Effect.gen(function* () {
            const auth = yield* AuthServiceClient;
            const res = yield* auth.signup(signupRequestToProto(req), opts(sessionToken));
            if (!res.me) return yield* Effect.fail(new Error("SignupResponse missing me"));
            return { me: meFromProto(res.me), sessionToken: res.sessionToken };
          }),
        ),
      login: (req, sessionToken) =>
        run(
          key,
          AUTH_SERVICE,
          "Login",
          Effect.gen(function* () {
            const auth = yield* AuthServiceClient;
            const res = yield* auth.login(loginRequestToProto(req), opts(sessionToken));
            if (!res.me) return yield* Effect.fail(new Error("LoginResponse missing me"));
            return { me: meFromProto(res.me), sessionToken: res.sessionToken };
          }),
        ),
      logout: (sessionToken) =>
        run(
          key,
          AUTH_SERVICE,
          "Logout",
          Effect.gen(function* () {
            const auth = yield* AuthServiceClient;
            yield* auth.logout({}, opts(sessionToken));
          }),
        ),
      getMe: (sessionToken) =>
        run(
          key,
          AUTH_SERVICE,
          "GetMe",
          Effect.gen(function* () {
            const auth = yield* AuthServiceClient;
            return meFromProto(yield* auth.getMe({}, opts(sessionToken)));
          }),
        ),
    },
    decks: {
      create: (req, sessionToken) =>
        run(
          key,
          DECKS_SERVICE,
          "Create",
          Effect.gen(function* () {
            const decks = yield* DecksServiceClient;
            const deck = yield* decks.create(createDeckToProto(req), opts(sessionToken));
            return deckDetailFromProto(deck);
          }),
        ),
      list: (sessionToken) =>
        run(
          key,
          DECKS_SERVICE,
          "List",
          Effect.gen(function* () {
            const decks = yield* DecksServiceClient;
            const res = yield* decks.list({}, opts(sessionToken));
            return deckSummaryListFromProto(res.decks);
          }),
        ),
      get: (id, sessionToken) =>
        run(
          key,
          DECKS_SERVICE,
          "Get",
          Effect.gen(function* () {
            const decks = yield* DecksServiceClient;
            const deck = yield* decks.get({ id: BigInt(id) }, opts(sessionToken));
            return deckDetailFromProto(deck);
          }),
        ),
      update: (id, req, sessionToken) =>
        run(
          key,
          DECKS_SERVICE,
          "Update",
          Effect.gen(function* () {
            const decks = yield* DecksServiceClient;
            const deck = yield* decks.update(updateDeckToProto(id, req), opts(sessionToken));
            return deckDetailFromProto(deck);
          }),
        ),
      delete: (id, sessionToken) =>
        run(
          key,
          DECKS_SERVICE,
          "Delete",
          Effect.gen(function* () {
            const decks = yield* DecksServiceClient;
            yield* decks.delete({ id: BigInt(id) }, opts(sessionToken));
          }),
        ),
    },
    ratings: {
      getLeaderboard: (req, sessionToken) =>
        run(
          key,
          RATINGS_SERVICE,
          "GetLeaderboard",
          Effect.gen(function* () {
            const ratings = yield* RatingsServiceClient;
            const leaderboard = yield* ratings.getLeaderboard(req, opts(sessionToken));
            return leaderboardFromProto(leaderboard);
          }),
        ),
    },
    cards: {
      catalog: () =>
        run(
          key,
          CARDS_SERVICE,
          "Catalog",
          Effect.gen(function* () {
            const cards = yield* CardsServiceClient;
            const res = yield* cards.catalog({}, opts(null));
            return catalogCardsFromProto(res.cards);
          }),
        ),
      search: (q, limit, offset) =>
        run(
          key,
          CARDS_SERVICE,
          "Search",
          Effect.gen(function* () {
            const cards = yield* CardsServiceClient;
            const res = yield* cards.search({ q, limit, offset }, opts(null));
            return catalogCardsFromProto(res.cards);
          }),
        ),
      lookup: (ids) =>
        run(
          key,
          CARDS_SERVICE,
          "Lookup",
          Effect.gen(function* () {
            const cards = yield* CardsServiceClient;
            const res = yield* cards.lookup({ ids }, opts(null));
            return catalogCardsFromProto(res.cards);
          }),
        ),
    },
    game: {
      submitIntent: (tableId, envelope, sessionToken) =>
        run(
          key,
          GAME_SERVICE,
          "SubmitIntent",
          Effect.gen(function* () {
            const game = yield* GameServiceClient;
            const ack = yield* game.submitIntent(
              { tableId, envelope: intentEnvelopeToProto(envelope) },
              opts(sessionToken),
            );
            return ackFromProto(ack);
          }),
        ),
      setYield: (tableId, enabled, sessionToken) =>
        run(
          key,
          GAME_SERVICE,
          "SetYield",
          Effect.gen(function* () {
            const game = yield* GameServiceClient;
            const ack = yield* game.setYield({ tableId, enabled }, opts(sessionToken));
            return ackFromProto(ack);
          }),
        ),
      setTurnYield: (tableId, enabled, sessionToken) =>
        run(
          key,
          GAME_SERVICE,
          "SetTurnYield",
          Effect.gen(function* () {
            const game = yield* GameServiceClient;
            const ack = yield* game.setTurnYield({ tableId, enabled }, opts(sessionToken));
            return ackFromProto(ack);
          }),
        ),
      setStackDwell: (tableId, dwelling, sessionToken) =>
        run(
          key,
          GAME_SERVICE,
          "SetStackDwell",
          Effect.gen(function* () {
            const game = yield* GameServiceClient;
            const ack = yield* game.setStackDwell({ tableId, dwelling }, opts(sessionToken));
            return ackFromProto(ack);
          }),
        ),
      stream(tableId, sessionToken) {
        const capturedOpts = opts(sessionToken);
        return runStream(
          key,
          GAME_SERVICE,
          "Stream",
          Stream.unwrap(
            Effect.gen(function* () {
              const game = yield* GameServiceClient;
              return game.stream({ tableId }, capturedOpts).pipe(Stream.map((msg) => streamFrameFromProto(msg)));
            }),
          ),
        );
      },
    },
    tables: {
      seed: (req, sessionToken) =>
        run(
          key,
          TABLES_SERVICE,
          "Seed",
          Effect.gen(function* () {
            const tables = yield* TablesServiceClient;
            const response = yield* tables.seed(seedRequestToProto(req), opts(sessionToken));
            return seedResponseFromProto(response);
          }),
        ),
    },
  };

  if (outboundTraceparent === null) clientCache.set(key, client);
  return client;
}

export function httpStatusOf(code: GrpcStatusCode.GrpcStatusCode): number {
  switch (code) {
    case "ok":
      return 200;
    case "invalid_argument":
      return 422;
    case "unauthenticated":
      return 401;
    case "permission_denied":
      return 403;
    case "not_found":
      return 404;
    case "already_exists":
      return 409;
    case "unavailable":
      return 503;
    default:
      return 500;
  }
}
