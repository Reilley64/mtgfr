/** Sticky / path helpers for the SolidStart `/api` BFF. */

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

/** Safe upstream path, or `null` if blocked (traversal / admin / health/drain). */
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

export function resolveUpstreamBase(opts: {
  upstreamsJson?: string;
  activeInstanceId?: string;
  cookieHeader?: string;
  fallbackUpstream?: string;
}): string {
  return upstreamBasesInOrder(opts)[0] ?? DEV_UPSTREAM;
}

/** Cookie (if known), then active, then remaining peers. */
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
    if (!id || seen.has(id) || !upstreams[id]) return;
    seen.add(id);
    ordered.push(upstreams[id]);
  };

  pushId(cookieValue(opts.cookieHeader, "mtgfr-instance"));
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
