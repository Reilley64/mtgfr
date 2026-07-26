import { Option, Schema as S } from "effect";
import { LobbyView } from "./types";

const CreatedTable = S.Struct({ table_id: S.String });
const ApiVersion = S.Struct({ version: S.String });

const decodeLobbyView = S.decodeUnknownOption(LobbyView);
const decodeCreatedTable = S.decodeUnknownOption(CreatedTable);
const decodeApiVersion = S.decodeUnknownOption(ApiVersion);

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

export function createTable(): Promise<{ table_id: string } | null> {
  return lobbyFetchJson("tables/v1", { method: "POST", body: "{}" }).then((body) =>
    decodeOrNull(decodeCreatedTable, body),
  );
}

export function joinTable(payload: { table_id: string; deck_id: number }): Promise<LobbyView | null> {
  return lobbyFetchJson("tables/join/v1", { method: "POST", body: JSON.stringify(payload) }).then((body) =>
    decodeOrNull(decodeLobbyView, body),
  );
}

export function readyUp(payload: { table_id: string; ready: boolean }): Promise<LobbyView | null> {
  return lobbyFetchJson("tables/ready/v1", { method: "POST", body: JSON.stringify(payload) }).then((body) =>
    decodeOrNull(decodeLobbyView, body),
  );
}

export function startGame(payload: { table_id: string }): Promise<LobbyView | null> {
  return lobbyFetchJson("tables/start/v1", { method: "POST", body: JSON.stringify(payload) }).then((body) =>
    decodeOrNull(decodeLobbyView, body),
  );
}

export function lobbyState(table: string): Promise<LobbyView | null> {
  return lobbyFetchJson(`tables/${encodeURIComponent(table)}/lobby/v1`).then((body) =>
    decodeOrNull(decodeLobbyView, body),
  );
}

export function apiVersion(): Promise<{ version: string } | null> {
  return lobbyFetchJson("meta/version/v1").then((body) => decodeOrNull(decodeApiVersion, body));
}
