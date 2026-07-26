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
  const tableId = match[1];
  if (!tableId) return null;
  try {
    return decodeURIComponent(tableId);
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

/** The tonic gRPC address (`host:port` — grpc-js address format, no scheme) for a pod DNS name
 * handed back by `Tables.Seed`, or an already-absolute `http(s)://host:port` upstream.
 * The pod's HTTP port (8080, `upstreamFromPodDns`) and gRPC port (50051) differ; this is the
 * gRPC-port analogue used once a table is routed to a specific pod. */
export function grpcUpstreamFromPodDns(pod: string): string {
  if (pod.startsWith("http://") || pod.startsWith("https://")) {
    return `${new URL(pod).hostname}:50051`;
  }
  return `${pod}:50051`;
}
