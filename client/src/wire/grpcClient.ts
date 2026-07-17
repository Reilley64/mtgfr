// Server-only gRPC client for the BFF. Do not import from the browser bundle.
// Channels/runtimes are cached per base URL.

import { GrpcClientProtocol, type GrpcStatusCode, GrpcStatusError } from "@effect-grpc/effect-grpc";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Layer from "effect/Layer";
import * as ManagedRuntime from "effect/ManagedRuntime";
import * as Stream from "effect/Stream";
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
            const deck = yield* decks.create(saveDeckToProto(req), sessionOpts(sessionToken));
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
              { id: BigInt(id), request: saveDeckToProto(req) },
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
              { tableId, envelope: intentEnvelopeToProto(envelope) },
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
        // Pump the Effect stream into a buffer on a forked fiber so `return()` (SSE cancel /
        // reconnect) can `Fiber.interrupt` and tear down the upstream tonic subscription —
        // `Stream.toAsyncIterableEffect` alone does not interrupt when the consumer stops.
        const runtime = runtimeFor(key);
        let fiber: Fiber.Fiber<void, never> | undefined;
        const buffer: StreamFrame[] = [];
        const waiters: Array<{
          resolve: (result: IteratorResult<StreamFrame>) => void;
          reject: (err: unknown) => void;
        }> = [];
        let ended = false;
        let failure: unknown;

        const deliver = (result: IteratorResult<StreamFrame>) => {
          const waiter = waiters.shift();
          if (waiter) {
            waiter.resolve(result);
            return;
          }
          if (!result.done && result.value !== undefined) buffer.push(result.value);
        };

        const fail = (err: unknown) => {
          failure = err;
          ended = true;
          while (waiters.length > 0) {
            waiters.shift()?.reject(err);
          }
        };

        const finish = () => {
          ended = true;
          while (waiters.length > 0) {
            waiters.shift()?.resolve({ done: true, value: undefined });
          }
        };

        const startFiber = () => {
          fiber = runtime.runFork(
            Effect.gen(function* () {
              const game = yield* GameClient;
              yield* game.stream({ tableId }, sessionOpts(sessionToken)).pipe(
                Stream.map((msg) => streamFrameFromProto(msg)),
                Stream.mapError(toCallError),
                Stream.runForEach((frame) => Effect.sync(() => deliver({ done: false, value: frame }))),
              );
            }).pipe(
              Effect.catch((err) => Effect.sync(() => fail(toCallError(err)))),
              Effect.ensuring(Effect.sync(finish)),
            ) as Effect.Effect<void, never, Clients>,
          );
        };

        return {
          [Symbol.asyncIterator]() {
            return {
              async next(): Promise<IteratorResult<StreamFrame>> {
                if (!fiber) startFiber();
                if (buffer.length > 0) return { done: false, value: buffer.shift()! };
                if (failure) throw failure;
                if (ended) return { done: true, value: undefined };
                return new Promise<IteratorResult<StreamFrame>>((resolve, reject) => {
                  waiters.push({ resolve, reject });
                });
              },
              async return(): Promise<IteratorResult<StreamFrame>> {
                if (fiber) {
                  await runtime.runPromise(Fiber.interrupt(fiber));
                  fiber = undefined;
                }
                ended = true;
                return { done: true, value: undefined };
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
            const response = yield* tables.seed(seedRequestToProto(req), sessionOpts(sessionToken));
            return seedResponseFromProto(response);
          }),
        ),
    },
  };

  clientCache.set(key, client);
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
