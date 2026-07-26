import * as Effect from "effect/Effect";
import * as Result from "effect/Result";
import * as Schema from "effect/Schema";
import * as FetchHttpClient from "effect/unstable/http/FetchHttpClient";
import * as HttpClient from "effect/unstable/http/HttpClient";
import type * as HttpClientError from "effect/unstable/http/HttpClientError";
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest";
import {
  LobbyBadRequest,
  type LobbyClientError,
  LobbyDecodeError,
  LobbyHttpError,
  LobbyNotFound,
  LobbyUnauthorized,
} from "./errors";
import { LobbyView } from "./types";

const API_ORIGIN = "/api";

const CreatedTable = Schema.Struct({ table_id: Schema.String });
const ApiMeta = Schema.Struct({
  version: Schema.String,
  faithful_count: Schema.optional(Schema.NullOr(Schema.Number)),
  oracle_total: Schema.optional(Schema.NullOr(Schema.Number)),
});
const CoverageSetMetaResponse = Schema.Struct({
  code: Schema.String,
  name: Schema.String,
  released_at: Schema.optional(Schema.NullOr(Schema.String)),
  faithful: Schema.Number,
  oracle_total: Schema.optional(Schema.NullOr(Schema.Number)),
});
const CoverageMetaResponse = Schema.Struct({
  faithful_count: Schema.optional(Schema.NullOr(Schema.Number)),
  oracle_total: Schema.optional(Schema.NullOr(Schema.Number)),
  sets: Schema.Array(CoverageSetMetaResponse),
});

type ApiMetaResponse = typeof ApiMeta.Type;
type CoverageMetaResponse = typeof CoverageMetaResponse.Type;

export type CoverageSetMeta = {
  code: string;
  name: string;
  releasedAt: string | null;
  faithful: number;
  oracleTotal: number | null;
};

export type CoverageMeta = {
  faithfulCount: number | null;
  oracleTotal: number | null;
  sets: CoverageSetMeta[];
};

export type ApiMeta = {
  version: string;
  faithfulCount: number | null;
  oracleTotal: number | null;
};

function withCredentials(fetchImpl: typeof globalThis.fetch): typeof globalThis.fetch {
  return ((input: RequestInfo | URL, init?: RequestInit) =>
    fetchImpl(input, { ...init, credentials: "include" })) as typeof globalThis.fetch;
}

function tablePath(tableId: string, suffix: string): string {
  return `/tables/${encodeURIComponent(tableId)}/${suffix}/v1`;
}

function isOk(status: number): boolean {
  return status >= 200 && status < 300;
}

function descriptionOf(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }
  if (value == null) {
    return "Unexpected status code";
  }
  return JSON.stringify(value) ?? "Unexpected status code";
}

function failStatus(status: number, description: string): Effect.Effect<never, LobbyClientError> {
  if (status === 401) return Effect.fail(new LobbyUnauthorized());
  if (status === 404) return Effect.fail(new LobbyNotFound());
  if (status === 400) return Effect.fail(new LobbyBadRequest({ message: description }));
  return Effect.fail(new LobbyHttpError({ status, description }));
}

function failHttp(error: HttpClientError.HttpClientError): LobbyHttpError {
  return new LobbyHttpError({ status: error.response?.status ?? null, description: error.message });
}

function mapApiMeta(decoded: ApiMetaResponse): ApiMeta {
  return {
    version: decoded.version,
    faithfulCount: decoded.faithful_count ?? null,
    oracleTotal: decoded.oracle_total ?? null,
  };
}

function mapCoverageMeta(decoded: CoverageMetaResponse): CoverageMeta {
  return {
    faithfulCount: decoded.faithful_count ?? null,
    oracleTotal: decoded.oracle_total ?? null,
    sets: decoded.sets.map((set) => ({
      code: set.code,
      name: set.name,
      releasedAt: set.released_at ?? null,
      faithful: set.faithful,
      oracleTotal: set.oracle_total ?? null,
    })),
  };
}

export function makeClient(fetchImpl: typeof globalThis.fetch) {
  const httpClient = Effect.runSync(
    HttpClient.HttpClient.pipe(
      Effect.provide(FetchHttpClient.layer),
      Effect.provideService(FetchHttpClient.Fetch, withCredentials(fetchImpl)),
    ),
  );
  const base = HttpClient.mapRequest(httpClient, HttpClientRequest.prependUrl(API_ORIGIN));

  function lobbyJson<A, I, RD>(
    schema: Schema.ConstraintCodec<A, I, RD, unknown>,
    request: HttpClientRequest.HttpClientRequest,
  ): Effect.Effect<A, LobbyClientError, RD> {
    return base.execute(request).pipe(
      Effect.catch((error) => Effect.fail(failHttp(error))),
      Effect.flatMap((response) =>
        Effect.gen(function* () {
          const body = yield* Effect.result(response.json);
          if (Result.isFailure(body)) {
            if (isOk(response.status)) {
              return yield* Effect.fail(new LobbyDecodeError({ message: body.failure.message }));
            }
            return yield* failStatus(response.status, body.failure.message);
          }

          const decoded = yield* Schema.decodeUnknownEffect(schema)(body.success).pipe(Effect.result);
          if (Result.isSuccess(decoded)) {
            return decoded.success;
          }
          if (isOk(response.status)) {
            return yield* Effect.fail(new LobbyDecodeError({ message: decoded.failure.message }));
          }
          return yield* failStatus(response.status, descriptionOf(body.success));
        }),
      ),
    );
  }

  return {
    httpClient: base,

    createTable: () =>
      lobbyJson(CreatedTable, HttpClientRequest.post("/tables/v1").pipe(HttpClientRequest.bodyJsonUnsafe({}))),
    joinTable: (tableId: string, payload: { deck_id: number }) =>
      lobbyJson(
        LobbyView,
        HttpClientRequest.post(tablePath(tableId, "join")).pipe(HttpClientRequest.bodyJsonUnsafe(payload)),
      ),
    readyUp: (tableId: string, payload: { ready: boolean }) =>
      lobbyJson(
        LobbyView,
        HttpClientRequest.post(tablePath(tableId, "ready")).pipe(HttpClientRequest.bodyJsonUnsafe(payload)),
      ),
    startGame: (tableId: string) =>
      lobbyJson(
        LobbyView,
        HttpClientRequest.post(tablePath(tableId, "start")).pipe(HttpClientRequest.bodyJsonUnsafe({})),
      ),
    lobbyState: (tableId: string) => lobbyJson(LobbyView, HttpClientRequest.get(tablePath(tableId, "lobby"))),
    apiMeta: () => lobbyJson(ApiMeta, HttpClientRequest.get("/meta/version/v1")).pipe(Effect.map(mapApiMeta)),
    coverageMeta: () =>
      lobbyJson(CoverageMetaResponse, HttpClientRequest.get("/meta/coverage/v1")).pipe(Effect.map(mapCoverageMeta)),
  };
}

export const client = makeClient(globalThis.fetch);

export type Client = typeof client;
