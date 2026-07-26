import { Option, Schema as S } from "effect";
import { LobbyView } from "./types";

const CreatedTable = S.Struct({ table_id: S.String });
const ApiMeta = S.Struct({
  version: S.String,
  faithful_count: S.optional(S.Number),
  oracle_total: S.optional(S.Number),
});
const CoverageSetMeta = S.Struct({
  code: S.String,
  name: S.String,
  released_at: S.optional(S.NullOr(S.String)),
  faithful: S.Number,
  oracle_total: S.optional(S.NullOr(S.Number)),
});
const CoverageMetaResponse = S.Struct({
  faithful_count: S.optional(S.NullOr(S.Number)),
  oracle_total: S.optional(S.NullOr(S.Number)),
  sets: S.Array(CoverageSetMeta),
});

const decodeLobbyView = S.decodeUnknownOption(LobbyView);
const decodeCreatedTable = S.decodeUnknownOption(CreatedTable);
const decodeApiMeta = S.decodeUnknownOption(ApiMeta);
const decodeCoverageMeta = S.decodeUnknownOption(CoverageMetaResponse);

async function lobbyFetchJson(path: string, init?: RequestInit): Promise<unknown | null> {
  const res = await fetch(`/api/${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
  try {
    return await res.json();
  } catch {
    return null;
  }
}

function decodeOrNull<A>(decode: (u: unknown) => Option.Option<A>, body: unknown | null): A | null {
  if (body == null) return null;
  return Option.getOrNull(decode(body));
}

export async function createTable(): Promise<{ table_id: string } | null> {
  const body = await lobbyFetchJson("tables/v1", { method: "POST", body: "{}" });
  return decodeOrNull(decodeCreatedTable, body);
}

export async function joinTable(tableId: string, payload: { deck_id: number }): Promise<LobbyView | null> {
  const body = await lobbyFetchJson(`tables/${encodeURIComponent(tableId)}/join/v1`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
  return decodeOrNull(decodeLobbyView, body);
}

export async function readyUp(tableId: string, payload: { ready: boolean }): Promise<LobbyView | null> {
  const body = await lobbyFetchJson(`tables/${encodeURIComponent(tableId)}/ready/v1`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
  return decodeOrNull(decodeLobbyView, body);
}

export async function startGame(tableId: string): Promise<LobbyView | null> {
  const body = await lobbyFetchJson(`tables/${encodeURIComponent(tableId)}/start/v1`, {
    method: "POST",
    body: "{}",
  });
  return decodeOrNull(decodeLobbyView, body);
}

export async function lobbyState(table: string): Promise<LobbyView | null> {
  const body = await lobbyFetchJson(`tables/${encodeURIComponent(table)}/lobby/v1`);
  return decodeOrNull(decodeLobbyView, body);
}

export async function apiMeta(): Promise<{
  version: string;
  faithfulCount: number | null;
  oracleTotal: number | null;
} | null> {
  const body = await lobbyFetchJson("meta/version/v1");
  const decoded = decodeOrNull(decodeApiMeta, body);
  if (!decoded) return null;
  return {
    version: decoded.version,
    faithfulCount: decoded.faithful_count ?? null,
    oracleTotal: decoded.oracle_total ?? null,
  };
}

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

export async function coverageMeta(): Promise<CoverageMeta | null> {
  const body = await lobbyFetchJson("meta/coverage/v1");
  const decoded = decodeOrNull(decodeCoverageMeta, body);
  if (!decoded) return null;

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
