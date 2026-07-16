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

export function resolveUpstreamBase(opts: {
  upstreamsJson?: string;
  activeInstanceId?: string;
  cookieHeader?: string;
  fallbackUpstream?: string;
}): string {
  const upstreams = parseUpstreamsJson(opts.upstreamsJson);
  const activeId = opts.activeInstanceId ?? "";
  if (Object.keys(upstreams).length === 0) {
    return (opts.fallbackUpstream ?? DEV_UPSTREAM).replace(/\/$/, "");
  }

  const instance = cookieValue(opts.cookieHeader, "mtgfr-instance");
  if (instance && upstreams[instance]) return upstreams[instance];
  if (activeId && upstreams[activeId]) return upstreams[activeId];
  const first = Object.values(upstreams)[0];
  return first ?? DEV_UPSTREAM;
}

export function isBlockedPublicApiPath(path: string): boolean {
  return path === "health/drain" || path.startsWith("admin/") || path === "admin";
}
