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

export async function createTable(): Promise<{ table_id: string } | null> {
  const body = await lobbyFetchJson("tables/v1", { method: "POST", body: "{}" });
  return decodeOrNull(decodeCreatedTable, body);
}

export async function joinTable(payload: { table_id: string; deck_id: number }): Promise<LobbyView | null> {
  const body = await lobbyFetchJson("tables/join/v1", { method: "POST", body: JSON.stringify(payload) });
  return decodeOrNull(decodeLobbyView, body);
}

export async function readyUp(payload: { table_id: string; ready: boolean }): Promise<LobbyView | null> {
  const body = await lobbyFetchJson("tables/ready/v1", { method: "POST", body: JSON.stringify(payload) });
  return decodeOrNull(decodeLobbyView, body);
}

export async function setTableOptions(payload: {
  table_id: string;
  commander_damage_enabled: boolean;
}): Promise<LobbyView | null> {
  const body = await lobbyFetchJson("tables/options/v1", { method: "POST", body: JSON.stringify(payload) });
  return decodeOrNull(decodeLobbyView, body);
}

export async function startGame(payload: { table_id: string }): Promise<LobbyView | null> {
  const body = await lobbyFetchJson("tables/start/v1", { method: "POST", body: JSON.stringify(payload) });
  return decodeOrNull(decodeLobbyView, body);
}

export async function lobbyState(table: string): Promise<LobbyView | null> {
  const body = await lobbyFetchJson(`tables/${encodeURIComponent(table)}/lobby/v1`);
  return decodeOrNull(decodeLobbyView, body);
}

export async function apiVersion(): Promise<{ version: string } | null> {
  const body = await lobbyFetchJson("meta/version/v1");
  return decodeOrNull(decodeApiVersion, body);
}
