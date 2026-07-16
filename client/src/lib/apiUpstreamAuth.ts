export function apiUpstream(): string {
  return (process.env.API_UPSTREAM ?? "http://127.0.0.1:8080").replace(/\/$/, "");
}

export type Me = { id: number; email: string; username: string };

export async function fetchMe(cookieHeader: string | null): Promise<Me | null> {
  const res = await fetch(`${apiUpstream()}/auth/me/v1`, {
    headers: cookieHeader ? { cookie: cookieHeader } : {},
  });
  if (!res.ok) return null;
  return (await res.json()) as Me;
}

export async function fetchDeckName(cookieHeader: string | null, deckId: number): Promise<string | null> {
  const res = await fetch(`${apiUpstream()}/decks/${deckId}/v1`, {
    headers: cookieHeader ? { cookie: cookieHeader } : {},
  });
  if (!res.ok) return null;
  const body = (await res.json()) as { name?: string };
  return body.name ?? null;
}

export async function fetchApiVersion(): Promise<string | null> {
  const res = await fetch(`${apiUpstream()}/health/live`);
  if (!res.ok) return null;
  const body = (await res.json()) as { version?: string };
  return body.version ?? null;
}

export type SeedResponse = { table_id: string; pod_dns: string; version: string };

export async function seedGame(
  cookieHeader: string | null,
  body: {
    table_id: string;
    host_user_id: number;
    seats: Array<{ user_id: number; username: string; deck_id: number }>;
  },
): Promise<{ ok: true; data: SeedResponse } | { ok: false; status: number }> {
  const res = await fetch(`${apiUpstream()}/tables/seed/v1`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(cookieHeader ? { cookie: cookieHeader } : {}),
    },
    body: JSON.stringify(body),
  });
  if (!res.ok) return { ok: false, status: res.status };
  return { ok: true, data: (await res.json()) as SeedResponse };
}
