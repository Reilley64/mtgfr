import type { LobbyView } from "./types";

async function lobbyFetch<T>(path: string, init?: RequestInit): Promise<T | null> {
  const res = await fetch(`/api/${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
  if (!res.ok) return null;
  return (await res.json()) as T;
}

export function createTable(): Promise<{ table_id: string } | null> {
  return lobbyFetch("tables/v1", { method: "POST", body: "{}" });
}

export function joinTable(payload: { table_id: string; deck_id: number }): Promise<LobbyView | null> {
  return lobbyFetch("tables/join/v1", { method: "POST", body: JSON.stringify(payload) });
}

export function readyUp(payload: { table_id: string; ready: boolean }): Promise<LobbyView | null> {
  return lobbyFetch("tables/ready/v1", { method: "POST", body: JSON.stringify(payload) });
}

export function startGame(payload: { table_id: string }): Promise<LobbyView | null> {
  return lobbyFetch("tables/start/v1", { method: "POST", body: JSON.stringify(payload) });
}

export function lobbyState(table: string): Promise<LobbyView | null> {
  return lobbyFetch(`tables/${encodeURIComponent(table)}/lobby/v1`);
}

export function apiVersion(): Promise<{ version: string } | null> {
  return lobbyFetch("meta/version/v1");
}
