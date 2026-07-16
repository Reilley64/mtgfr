/** Sticky / path helpers for the SolidStart `/api` BFF. */

export const DEV_UPSTREAM = "http://127.0.0.1:8080";

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

/** Parse `tables/{id}/…` game routes for BFF Postgres lookup. */
export function tableIdFromGamePath(path: string): string | null {
  const match = path.match(/^tables\/([^/]+)\/(stream|intent|yield|turn-yield|stack-dwell)\/v1$/);
  if (!match) return null;
  try {
    return decodeURIComponent(match[1]!);
  } catch {
    return null;
  }
}

/** Turn a `table_routes.pod_dns` value into an HTTP base URL for proxying. */
export function upstreamFromPodDns(pod: string): string {
  if (pod.startsWith("http://") || pod.startsWith("https://")) {
    return pod.replace(/\/$/, "");
  }
  return `http://${pod}:8080`;
}
