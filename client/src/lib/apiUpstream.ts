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
  if (segments[0] === "tables" && segments[1] === "seed") return null;
  return segments.join("/");
}

export function tableIdFromGamePath(path: string): string | null {
  const match = path.match(/^tables\/([^/]+)\/(stream|intent|yield|turn-yield|stack-dwell)\/v1$/);
  if (!match) return null;
  try {
    return decodeURIComponent(match[1]!);
  } catch {
    return null;
  }
}

export function upstreamFromPodDns(pod: string): string {
  if (pod.startsWith("http://") || pod.startsWith("https://")) {
    return pod.replace(/\/$/, "");
  }
  return `http://${pod}:8080`;
}
