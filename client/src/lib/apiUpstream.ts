/** Pure sticky / path helpers for the SolidStart `/api` BFF (testable without Vinxi). */

export const DEV_UPSTREAM = "http://127.0.0.1:8080";

export function parseUpstreamsJson(raw: string | undefined): Record<string, string> {
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    const out: Record<string, string> = {};
    for (const [id, url] of Object.entries(parsed as Record<string, unknown>)) {
      if (typeof url === "string" && url.length > 0) out[id] = url.replace(/\/$/, "");
    }
    return out;
  } catch {
    return {};
  }
}

export function cookieValue(header: string | undefined, name: string): string | undefined {
  if (!header) return undefined;
  for (const part of header.split(";")) {
    const trimmed = part.trim();
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    if (trimmed.slice(0, eq) !== name) continue;
    return trimmed.slice(eq + 1);
  }
  return undefined;
}

/**
 * Collapse a catch-all `/api/*` path to a safe upstream path, or `null` if it must not be
 * forwarded (traversal, admin, health/drain). Trailing slashes are stripped so `health/drain/`
 * cannot slip past an exact-match block.
 */
export function normalizePublicApiPath(path: string): string | null {
  let decoded: string;
  try {
    decoded = decodeURIComponent(path);
  } catch {
    return null;
  }
  const trimmed = decoded.replace(/^\/+/, "").replace(/\/+$/, "");
  if (!trimmed) return "";
  const segments = trimmed.split("/");
  if (segments.some((s) => s === "" || s === "." || s === "..")) return null;
  if (segments[0] === "admin") return null;
  if (segments[0] === "health" && segments[1] === "drain") return null;
  return segments.join("/");
}

export function isBlockedPublicApiPath(path: string): boolean {
  return normalizePublicApiPath(path) === null;
}

export function resolveUpstreamBase(opts: {
  upstreamsJson?: string;
  activeInstanceId?: string;
  cookieHeader?: string;
  fallbackUpstream?: string;
}): string {
  const bases = upstreamBasesInOrder(opts);
  return bases[0] ?? DEV_UPSTREAM;
}

/** Prefer sticky cookie (when known), then active, then remaining drain peers — for join fan-out. */
export function upstreamBasesInOrder(opts: {
  upstreamsJson?: string;
  activeInstanceId?: string;
  cookieHeader?: string;
  fallbackUpstream?: string;
}): string[] {
  const upstreams = parseUpstreamsJson(opts.upstreamsJson);
  if (Object.keys(upstreams).length === 0) {
    return [(opts.fallbackUpstream ?? DEV_UPSTREAM).replace(/\/$/, "")];
  }

  const ordered: string[] = [];
  const seen = new Set<string>();
  const pushId = (id: string | undefined) => {
    if (!id) return;
    const url = upstreams[id];
    if (!url || seen.has(id)) return;
    seen.add(id);
    ordered.push(url);
  };

  const instance = cookieValue(opts.cookieHeader, "mtgfr-instance");
  pushId(instance);
  pushId(opts.activeInstanceId);
  for (const id of Object.keys(upstreams)) pushId(id);
  return ordered;
}

export function isUnknownTableLobbyBody(body: string): boolean {
  try {
    const parsed = JSON.parse(body) as { error?: unknown };
    return parsed.error === "UnknownTable";
  } catch {
    return false;
  }
}

/** Guests joining by code have no sticky cookie; fan out across versioned peers. */
export function shouldFanOutJoin(path: string, method: string): boolean {
  return method === "POST" && path === "tables/join/v1";
}
