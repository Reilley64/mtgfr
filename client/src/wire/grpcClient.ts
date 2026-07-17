// Server-only gRPC client (ADR 0032): the BFF's on-ramp to tonic via `@effect-grpc/effect-grpc`
// (Connect native-gRPC transport + Effect Rpc clients generated from `proto/mtgfr/v1/mtgfr.proto`).
// Never import this from anything that reaches the browser bundle — only `~/routes/api/**` and the
// server-only helpers they call (`~/lib/apiUpstreamAuth`, `~/wire/rpcServer`) may import it.
//
// Channels/runtimes are cached per base URL so repeated calls against the same pod reuse one
// transport instead of paying a new-connection cost.

import { GrpcClientProtocol, type GrpcStatusCode, GrpcStatusError } from "@effect-grpc/effect-grpc";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as Stream from "effect/Stream";
import type {
  SaveDeckRequest as ProtoSaveDeckRequest,
  SeedRequest as ProtoSeedRequest,
} from "~/wire/generated/mtgfr/v1/catalog_effect_grpc";
import type { IntentEnvelope as ProtoIntentEnvelope } from "~/wire/generated/mtgfr/v1/intent_effect_grpc";
import {
  AuthClient,
  AuthClientLayer,
  AuthGrpcRegistry,
  CardsClient,
  CardsClientLayer,
  CardsGrpcRegistry,
  DecksClient,
  DecksClientLayer,
  DecksGrpcRegistry,
  GameClient,
  GameClientLayer,
  GameGrpcRegistry,
  type Me as ProtoMe,
  TablesClient,
  TablesClientLayer,
  TablesGrpcRegistry,
} from "~/wire/generated/mtgfr/v1/mtgfr_effect_grpc";
import {
  catalogCardsFromProto,
  deckDetailFromProto,
  deckSummaryListFromProto,
  intentEnvelopeToProto,
  saveDeckToProto,
  seedRequestToProto,
  seedResponseFromProto,
  streamFrameFromProto,
} from "~/wire/protoMap";
import type {
  Ack,
  CatalogCard,
  DeckDetail,
  IntentEnvelope,
  Me,
  SaveDeckRequest,
  SeedRequest,
  SeedResponse,
  StreamFrame,
} from "~/wire/types";

/** The metadata key the session token travels under (matches `crates/server::grpc::auth_ctx`). */
export const SESSION_METADATA_KEY = "x-session-token";

const AllGrpcRegistry = new Map([
  ...AuthGrpcRegistry,
  ...DecksGrpcRegistry,
  ...CardsGrpcRegistry,
  ...GameGrpcRegistry,
  ...TablesGrpcRegistry,
]);

function meFromProto(me: ProtoMe): Me {
  return { id: Number(me.id), email: me.email, username: me.username };
}

function sessionOpts(sessionToken: string | null) {
  if (!sessionToken) return undefined;
  return { metadata: [[SESSION_METADATA_KEY, sessionToken] as const] };
}

/** A failed unary/streaming call, carrying the gRPC status code + message so callers can map it
 * back to an HTTP status without re-parsing a connect/tonic error. */
export class GrpcCallError extends Error {
  readonly code: GrpcStatusCode.GrpcStatusCode;
  constructor(code: GrpcStatusCode.GrpcStatusCode, message: string) {
    super(message);
    this.name = "GrpcCallError";
    this.code = code;
  }
}

function toCallError(err: unknown): GrpcCallError {
  if (err instanceof GrpcStatusError.GrpcStatusError) {
    return new GrpcCallError(err.code, err.message);
  }
  if (
    err !== null &&
    typeof err === "object" &&
    "_tag" in err &&
    (err as { _tag: string })._tag === "GrpcStatusError"
  ) {
    const g = err as GrpcStatusError.GrpcStatusError;
    return new GrpcCallError(g.code, g.message);
  }
  return new GrpcCallError("unknown", err instanceof Error ? err.message : String(err));
}

/** Normalize `host:port` or `http(s)://…` to the `http://host:port` baseUrl effect-grpc expects. */
export function grpcBaseUrl(address: string): string {
  if (address.startsWith("http://") || address.startsWith("https://")) {
    return address.replace(/\/$/, "");
  }
  return `http://${address}`;
}

type Clients = AuthClient | DecksClient | CardsClient | GameClient | TablesClient;

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
    AuthClientLayer,
    DecksClientLayer,
    CardsClientLayer,
    GameClientLayer,
    TablesClientLayer,
  ).pipe(Layer.provide(protocol));

  const runtime = ManagedRuntime.make(clients) as GrpcRuntime;
  runtimeCache.set(baseUrl, runtime);
  return runtime;
}

function run<A>(address: string, effect: Effect.Effect<A, unknown, Clients>): Promise<A> {
  return runtimeFor(address)
    .runPromise(effect)
    .catch((err: unknown) => {
      throw toCallError(err);
    });
}

export interface GrpcClient {
  auth: {
    signup(
      req: { email: string; password: string; username: string },
      sessionToken: string | null,
    ): Promise<{ me: Me; sessionToken: string }>;
    login(
      req: { email: string; password: string },
      sessionToken: string | null,
    ): Promise<{ me: Me; sessionToken: string }>;
    logout(sessionToken: string | null): Promise<void>;
    getMe(sessionToken: string | null): Promise<Me>;
  };
  decks: {
    create(req: SaveDeckRequest, sessionToken: string | null): Promise<DeckDetail>;
    list(
      sessionToken: string | null,
    ): Promise<Array<{ commander: string; commander_print?: string; id: number; name: string }>>;
    get(id: number, sessionToken: string | null): Promise<DeckDetail>;
    update(id: number, req: SaveDeckRequest, sessionToken: string | null): Promise<DeckDetail>;
    delete(id: number, sessionToken: string | null): Promise<void>;
  };
  cards: {
    catalog(): Promise<Array<CatalogCard>>;
    search(q: string, limit: number, offset: number): Promise<Array<CatalogCard>>;
    lookup(ids: Array<string>): Promise<Array<CatalogCard>>;
  };
  game: {
    submitIntent(tableId: string, envelope: IntentEnvelope, sessionToken: string | null): Promise<Ack>;
    setYield(tableId: string, enabled: boolean, sessionToken: string | null): Promise<Ack>;
    setTurnYield(tableId: string, enabled: boolean, sessionToken: string | null): Promise<Ack>;
    setStackDwell(tableId: string, dwelling: boolean, sessionToken: string | null): Promise<Ack>;
    stream(tableId: string, sessionToken: string | null): AsyncIterable<StreamFrame>;
  };
  tables: {
    seed(req: SeedRequest, sessionToken: string | null): Promise<SeedResponse>;
  };
}

const clientCache = new Map<string, GrpcClient>();

/** The gRPC client bundle for `address` (`host:port` or `http://host:port`). Cached per address. */
export function grpcClient(address: string): GrpcClient {
  const key = grpcBaseUrl(address);
  const cached = clientCache.get(key);
  if (cached) return cached;

  const client: GrpcClient = {
    auth: {
      signup: (req, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const auth = yield* AuthClient;
            const res = yield* auth.signup(req, sessionOpts(sessionToken));
            if (!res.me) throw new Error("AuthSession missing me");
            return { me: meFromProto(res.me), sessionToken: res.sessionToken };
          }),
        ),
      login: (req, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const auth = yield* AuthClient;
            const res = yield* auth.login(req, sessionOpts(sessionToken));
            if (!res.me) throw new Error("AuthSession missing me");
            return { me: meFromProto(res.me), sessionToken: res.sessionToken };
          }),
        ),
      logout: (sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const auth = yield* AuthClient;
            yield* auth.logout({}, sessionOpts(sessionToken));
          }),
        ),
      getMe: (sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const auth = yield* AuthClient;
            return meFromProto(yield* auth.getMe({}, sessionOpts(sessionToken)));
          }),
        ),
    },
    decks: {
      create: (req, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const decks = yield* DecksClient;
            const deck = yield* decks.create(saveDeckToProto(req) as ProtoSaveDeckRequest, sessionOpts(sessionToken));
            return deckDetailFromProto(deck);
          }),
        ),
      list: (sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const decks = yield* DecksClient;
            const res = yield* decks.list({}, sessionOpts(sessionToken));
            return deckSummaryListFromProto(res.decks);
          }),
        ),
      get: (id, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const decks = yield* DecksClient;
            const deck = yield* decks.get({ id: BigInt(id) }, sessionOpts(sessionToken));
            return deckDetailFromProto(deck);
          }),
        ),
      update: (id, req, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const decks = yield* DecksClient;
            const deck = yield* decks.update(
              { id: BigInt(id), request: saveDeckToProto(req) as ProtoSaveDeckRequest },
              sessionOpts(sessionToken),
            );
            return deckDetailFromProto(deck);
          }),
        ),
      delete: (id, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const decks = yield* DecksClient;
            yield* decks.delete({ id: BigInt(id) }, sessionOpts(sessionToken));
          }),
        ),
    },
    cards: {
      catalog: () =>
        run(
          key,
          Effect.gen(function* () {
            const cards = yield* CardsClient;
            const res = yield* cards.catalog({});
            return catalogCardsFromProto(res.cards);
          }),
        ),
      search: (q, limit, offset) =>
        run(
          key,
          Effect.gen(function* () {
            const cards = yield* CardsClient;
            const res = yield* cards.search({ q, limit, offset });
            return catalogCardsFromProto(res.cards);
          }),
        ),
      lookup: (ids) =>
        run(
          key,
          Effect.gen(function* () {
            const cards = yield* CardsClient;
            const res = yield* cards.lookup({ ids });
            return catalogCardsFromProto(res.cards);
          }),
        ),
    },
    game: {
      submitIntent: (tableId, envelope, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const game = yield* GameClient;
            return yield* game.submitIntent(
              { tableId, envelope: intentEnvelopeToProto(envelope) as ProtoIntentEnvelope },
              sessionOpts(sessionToken),
            );
          }),
        ),
      setYield: (tableId, enabled, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const game = yield* GameClient;
            return yield* game.setYield({ tableId, enabled }, sessionOpts(sessionToken));
          }),
        ),
      setTurnYield: (tableId, enabled, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const game = yield* GameClient;
            return yield* game.setTurnYield({ tableId, enabled }, sessionOpts(sessionToken));
          }),
        ),
      setStackDwell: (tableId, dwelling, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const game = yield* GameClient;
            return yield* game.setStackDwell({ tableId, dwelling }, sessionOpts(sessionToken));
          }),
        ),
      stream(tableId, sessionToken) {
        const streamEffect = Effect.gen(function* () {
          const game = yield* GameClient;
          const frames = game.stream({ tableId }, sessionOpts(sessionToken)).pipe(
            Stream.map((msg) => streamFrameFromProto(msg)),
            Stream.mapError(toCallError),
          );
          return yield* Stream.toAsyncIterableEffect(frames);
        });
        // Materialize the iterable synchronously enough for the route: runPromise then wrap.
        // Callers `for await` immediately; we return a lazy iterable that starts the Effect on
        // first iteration so cancel still tears down the Effect stream.
        return {
          [Symbol.asyncIterator]() {
            let inner: AsyncIterator<StreamFrame> | undefined;
            return {
              async next() {
                if (!inner) {
                  try {
                    const iterable = await run(key, streamEffect);
                    inner = iterable[Symbol.asyncIterator]();
                  } catch (err) {
                    throw toCallError(err);
                  }
                }
                return inner.next();
              },
              async return(value?: unknown) {
                return inner?.return?.(value) ?? { done: true, value: undefined };
              },
              async throw(err?: unknown) {
                return inner?.throw?.(err) ?? Promise.reject(err);
              },
            };
          },
        };
      },
    },
    tables: {
      seed: (req, sessionToken) =>
        run(
          key,
          Effect.gen(function* () {
            const tables = yield* TablesClient;
            const response = yield* tables.seed(seedRequestToProto(req) as ProtoSeedRequest, sessionOpts(sessionToken));
            return seedResponseFromProto(response);
          }),
        ),
    },
  };

  clientCache.set(key, client);
  return client;
}

/** Map a `GrpcCallError`'s status code to the HTTP status the browser should see. */
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
