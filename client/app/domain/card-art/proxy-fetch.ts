import type { LookupAddress } from "node:dns";
import { lookup } from "node:dns/promises";
import type { IncomingMessage } from "node:http";
import { request as httpsRequest } from "node:https";
import type { LookupFunction } from "node:net";
import { isIP } from "node:net";
import { contentLengthTooLarge } from "../faro/collect";

export const PROXY_ART_MAX_BYTES = 5 * 1024 * 1024;
const PROXY_ART_TIMEOUT_MS = 5_000;
const ALLOWED_IMAGE_CONTENT_TYPES = new Set(["image/gif", "image/jpeg", "image/png", "image/webp"]);
const BLOCKED_HOSTNAMES = new Set(["localhost", "metadata.google.internal"]);
const BLOCKED_HOST_SUFFIXES = [".home.arpa", ".internal", ".local", ".localhost"];

type LookupHost = (hostname: string, options: { all: true; verbatim: true }) => Promise<ReadonlyArray<LookupAddress>>;
type ProxyFetchInit = {
  headers: Record<string, string>;
  lookup: LookupFunction;
  redirect: "manual";
  signal: AbortSignal;
};
type FetchImpl = (target: URL, init: ProxyFetchInit) => Promise<Response>;

type ProxySuccess = {
  ok: true;
  body: Uint8Array;
  contentType: string;
};

type ProxyFailure = {
  ok: false;
  status: 400 | 502;
};

export type FetchProxyCardArtDeps = {
  fetchImpl?: FetchImpl;
  lookupHost?: LookupHost;
  maxBytes?: number;
  timeoutMs?: number;
};

class UnsafeProxyTargetError extends Error {}
class UpstreamProxyError extends Error {}

function abortError(): DOMException {
  return new DOMException("The operation was aborted.", "AbortError");
}

function normalizeHostname(hostname: string): string {
  return hostname
    .replaceAll(/^\[|\]$/g, "")
    .replaceAll(/\.+$/g, "")
    .toLowerCase();
}

function parseIpv4(address: string): Uint8Array | null {
  const parts = address.split(".");
  if (parts.length !== 4) return null;

  const octets = new Uint8Array(4);
  for (const [index, part] of parts.entries()) {
    if (!/^\d+$/.test(part)) return null;
    const n = Number(part);
    if (!Number.isInteger(n) || n < 0 || n > 255) return null;
    octets[index] = n;
  }

  return octets;
}

function isBlockedIpv4(address: string): boolean {
  const octets = parseIpv4(address);
  if (!octets) return false;

  const [a, b] = octets;
  if (a === 0 || a === 10 || a === 127) return true;
  if (a === 100 && b >= 64 && b <= 127) return true;
  if (a === 169 && b === 254) return true;
  if (a === 172 && b >= 16 && b <= 31) return true;
  if (a === 192 && (b === 0 || b === 168)) return true;
  if (a === 198 && (b === 18 || b === 19)) return true;
  if (a >= 224) return true;
  return false;
}

function parseIpv6(address: string): Uint16Array | null {
  const withoutZone = address.split("%")[0]?.toLowerCase();
  if (!withoutZone) return null;

  const mappedIpv4 = withoutZone.match(/^(.*::ffff:)(\d+\.\d+\.\d+\.\d+)$/);
  if (mappedIpv4) {
    const mapped = parseIpv4(mappedIpv4[2]);
    if (!mapped) return null;
    return Uint16Array.from([0, 0, 0, 0, 0, 0xffff, (mapped[0] << 8) | mapped[1], (mapped[2] << 8) | mapped[3]]);
  }

  const halves = withoutZone.split("::");
  if (halves.length > 2) return null;

  const left = halves[0] ? halves[0].split(":").filter((part) => part.length > 0) : [];
  const right = halves[1] ? halves[1].split(":").filter((part) => part.length > 0) : [];
  const missingGroups = 8 - (left.length + right.length);

  if ((halves.length === 1 && missingGroups !== 0) || missingGroups < 0) return null;

  const groups = [...left, ...Array.from({ length: halves.length === 2 ? missingGroups : 0 }, () => "0"), ...right];

  if (groups.length !== 8) return null;

  const parsed = new Uint16Array(8);
  for (const [index, group] of groups.entries()) {
    if (!/^[0-9a-f]{1,4}$/.test(group)) return null;
    parsed[index] = Number.parseInt(group, 16);
  }

  return parsed;
}

function isBlockedIpv6(address: string): boolean {
  const groups = parseIpv6(address);
  if (!groups) return false;

  const [first, second, third, fourth, fifth, sixth, seventh, eighth] = groups;
  if (first === 0 && second === 0 && third === 0 && fourth === 0 && fifth === 0 && sixth === 0 && seventh === 0) {
    return eighth === 0 || eighth === 1;
  }
  if ((first & 0xfe00) === 0xfc00) return true;
  if ((first & 0xffc0) === 0xfe80) return true;
  if ((first & 0xff00) === 0xff00) return true;
  if (first === 0 && second === 0 && third === 0 && fourth === 0 && fifth === 0 && sixth === 0xffff) {
    return isBlockedIpv4(`${seventh >> 8}.${seventh & 0xff}.${eighth >> 8}.${eighth & 0xff}`);
  }
  return false;
}

function isBlockedIpLiteral(hostname: string): boolean {
  const normalized = normalizeHostname(hostname);
  const ipVersion = isIP(normalized);
  if (ipVersion === 4) return isBlockedIpv4(normalized);
  if (ipVersion === 6) return isBlockedIpv6(normalized);
  return false;
}

function isBlockedHostname(hostname: string): boolean {
  const normalized = normalizeHostname(hostname);
  if (normalized.length === 0) return true;
  if (BLOCKED_HOSTNAMES.has(normalized)) return true;
  if (BLOCKED_HOST_SUFFIXES.some((suffix) => normalized.endsWith(suffix))) return true;
  return isBlockedIpLiteral(normalized);
}

async function assertPublicResolvedHost(target: URL, lookupHost: LookupHost): Promise<ReadonlyArray<LookupAddress>> {
  let results: ReadonlyArray<LookupAddress>;
  try {
    results = await lookupHost(target.hostname, { all: true, verbatim: true });
  } catch {
    throw new UpstreamProxyError("lookup failed");
  }

  if (results.length === 0) {
    throw new UpstreamProxyError("lookup returned no addresses");
  }

  for (const result of results) {
    if (isBlockedIpLiteral(result.address)) {
      throw new UnsafeProxyTargetError("resolved to blocked address");
    }
  }

  return results;
}

function lookupFamily(options: Parameters<LookupFunction>[1]): number {
  if (options.family === "IPv4") return 4;
  if (options.family === "IPv6") return 6;
  if (typeof options.family === "number") return options.family;
  return 0;
}

function pinnedLookupError(): NodeJS.ErrnoException {
  return Object.assign(new Error("pinned lookup returned no matching addresses"), { code: "ENOTFOUND" });
}

function pinnedLookup(addresses: ReadonlyArray<LookupAddress>): LookupFunction {
  return (_hostname, options, callback) => {
    const requestedFamily = lookupFamily(options);
    const matches = requestedFamily === 0 ? addresses : addresses.filter((address) => address.family === requestedFamily);
    if (matches.length === 0) {
      callback(pinnedLookupError(), []);
      return;
    }

    if (options.all) {
      callback(null, [...matches]);
      return;
    }

    const first = matches[0];
    callback(null, first.address, first.family);
  };
}

function responseHeaders(headersRecord: IncomingMessage["headers"]): Headers {
  const headers = new Headers();
  for (const [name, value] of Object.entries(headersRecord)) {
    if (value === undefined) continue;
    if (typeof value === "string") {
      headers.set(name, value);
      continue;
    }
    for (const item of value) {
      headers.append(name, item);
    }
  }
  return headers;
}

function responseBodyStream(response: IncomingMessage): ReadableStream<Uint8Array> {
  return new ReadableStream<Uint8Array>({
    start(controller) {
      response.on("data", (chunk: string | Uint8Array) => {
        controller.enqueue(typeof chunk === "string" ? new TextEncoder().encode(chunk) : chunk);
      });
      response.on("end", () => {
        controller.close();
      });
      response.on("error", (error) => {
        controller.error(error);
      });
    },
    cancel() {
      response.destroy();
    },
  });
}

async function fetchPinnedHttps(target: URL, init: ProxyFetchInit): Promise<Response> {
  return new Promise<Response>((resolve, reject) => {
    let settled = false;
    let response: IncomingMessage | null = null;

    const cleanup = () => {
      init.signal.removeEventListener("abort", handleAbort);
      response?.removeListener("close", cleanup);
    };

    const settle = (next: () => void) => {
      if (settled) return;
      settled = true;
      next();
    };

    const request = httpsRequest(
      {
        headers: init.headers,
        hostname: target.hostname,
        lookup: init.lookup,
        method: "GET",
        path: `${target.pathname}${target.search}`,
        port: target.port.length > 0 ? Number(target.port) : undefined,
        servername: target.hostname,
      },
      (incomingResponse) => {
        response = incomingResponse;
        incomingResponse.once("close", cleanup);
        settle(() =>
          resolve(
            new Response(responseBodyStream(incomingResponse), {
              headers: responseHeaders(incomingResponse.headers),
              status: incomingResponse.statusCode ?? 502,
            }),
          ),
        );
      },
    );

    const handleAbort = () => {
      const error = abortError();
      response?.destroy(error);
      request.destroy(error);
    };

    request.once("error", (error) => {
      settle(() => {
        cleanup();
        reject(error);
      });
    });

    if (init.signal.aborted) {
      handleAbort();
      return;
    }

    init.signal.addEventListener("abort", handleAbort, { once: true });
    request.end();
  });
}

function imageContentType(response: Response): string {
  const contentType = response.headers.get("content-type");
  const normalized = contentType?.split(";")[0]?.trim().toLowerCase();
  if (!normalized || !ALLOWED_IMAGE_CONTENT_TYPES.has(normalized)) {
    throw new UpstreamProxyError("unexpected content type");
  }
  return normalized;
}

async function readResponseBodyCapped(response: Response, maxBytes: number, signal: AbortSignal): Promise<Uint8Array> {
  if (contentLengthTooLarge(response.headers.get("content-length"), maxBytes)) {
    throw new UpstreamProxyError("body too large");
  }

  if (!response.body) {
    return new Uint8Array(0);
  }

  const reader = response.body.getReader();
  const chunks: Array<Uint8Array> = [];
  let total = 0;
  let rejectAborted!: (error: unknown) => void;
  const aborted = new Promise<never>((_, reject) => {
    rejectAborted = reject;
  });
  const handleAbort = () => {
    void reader.cancel(abortError()).catch(() => undefined);
    rejectAborted(abortError());
  };

  if (signal.aborted) {
    handleAbort();
  } else {
    signal.addEventListener("abort", handleAbort, { once: true });
  }

  try {
    while (true) {
      const next = await Promise.race([reader.read(), aborted]);
      if (next.done) break;
      total += next.value.byteLength;
      if (total > maxBytes) {
        await reader.cancel().catch(() => undefined);
        throw new UpstreamProxyError("body too large");
      }
      chunks.push(next.value);
    }
  } finally {
    signal.removeEventListener("abort", handleAbort);
  }

  const body = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    body.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return body;
}

export function assertSafeProxyTarget(raw: string): URL {
  let target: URL;
  try {
    target = new URL(raw);
  } catch {
    throw new UnsafeProxyTargetError("invalid url");
  }

  if (target.protocol !== "https:") {
    throw new UnsafeProxyTargetError("https required");
  }
  if (target.username.length > 0 || target.password.length > 0) {
    throw new UnsafeProxyTargetError("credentials forbidden");
  }
  if (isBlockedHostname(target.hostname)) {
    throw new UnsafeProxyTargetError("blocked host");
  }

  return target;
}

export async function fetchProxyCardArt(
  raw: string,
  deps: FetchProxyCardArtDeps = {},
): Promise<ProxySuccess | ProxyFailure> {
  const fetchImpl = deps.fetchImpl ?? fetchPinnedHttps;
  const lookupHost = deps.lookupHost ?? lookup;
  const maxBytes = deps.maxBytes ?? PROXY_ART_MAX_BYTES;
  const timeoutMs = deps.timeoutMs ?? PROXY_ART_TIMEOUT_MS;
  const signal = AbortSignal.timeout(timeoutMs);

  try {
    const target = assertSafeProxyTarget(raw);
    const resolvedAddresses = await assertPublicResolvedHost(target, lookupHost);

    const response = await fetchImpl(target, {
      headers: { accept: "image/*" },
      lookup: pinnedLookup(resolvedAddresses),
      redirect: "manual",
      signal,
    });
    if (!response.ok) {
      throw new UpstreamProxyError("upstream not ok");
    }

    const contentType = imageContentType(response);
    const body = await readResponseBodyCapped(response, maxBytes, signal);
    return { ok: true, body, contentType };
  } catch (error) {
    if (error instanceof UnsafeProxyTargetError) {
      return { ok: false, status: 400 };
    }
    return { ok: false, status: 502 };
  }
}
