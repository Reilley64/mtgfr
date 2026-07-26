import { lookup } from "node:dns/promises";
import { isIP } from "node:net";
import { contentLengthTooLarge } from "../faro/collect";

export const PROXY_ART_MAX_BYTES = 5 * 1024 * 1024;
const PROXY_ART_TIMEOUT_MS = 5_000;
const ALLOWED_IMAGE_CONTENT_TYPES = new Set(["image/gif", "image/jpeg", "image/png", "image/webp"]);
const BLOCKED_HOSTNAMES = new Set(["localhost", "metadata.google.internal"]);
const BLOCKED_HOST_SUFFIXES = [".home.arpa", ".internal", ".local", ".localhost"];

type LookupAddress = { address: string; family: number };
type LookupHost = (hostname: string, options: { all: true; verbatim: true }) => Promise<ReadonlyArray<LookupAddress>>;
type FetchImpl = (input: string | URL | Request, init?: RequestInit) => Promise<Response>;

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

async function assertPublicResolvedHost(target: URL, lookupHost: LookupHost): Promise<void> {
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
}

function imageContentType(response: Response): string {
  const contentType = response.headers.get("content-type");
  const normalized = contentType?.split(";")[0]?.trim().toLowerCase();
  if (!normalized || !ALLOWED_IMAGE_CONTENT_TYPES.has(normalized)) {
    throw new UpstreamProxyError("unexpected content type");
  }
  return normalized;
}

async function readResponseBodyCapped(response: Response, maxBytes: number): Promise<Uint8Array> {
  if (contentLengthTooLarge(response.headers.get("content-length"), maxBytes)) {
    throw new UpstreamProxyError("body too large");
  }

  if (!response.body) {
    return new Uint8Array(0);
  }

  const reader = response.body.getReader();
  const chunks: Array<Uint8Array> = [];
  let total = 0;

  while (true) {
    const next = await reader.read();
    if (next.done) break;
    total += next.value.byteLength;
    if (total > maxBytes) {
      await reader.cancel().catch(() => undefined);
      throw new UpstreamProxyError("body too large");
    }
    chunks.push(next.value);
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
  const fetchImpl = deps.fetchImpl ?? fetch;
  const lookupHost = deps.lookupHost ?? lookup;
  const maxBytes = deps.maxBytes ?? PROXY_ART_MAX_BYTES;
  const timeoutMs = deps.timeoutMs ?? PROXY_ART_TIMEOUT_MS;

  try {
    const target = assertSafeProxyTarget(raw);
    await assertPublicResolvedHost(target, lookupHost);

    const response = await fetchImpl(target, {
      headers: { accept: "image/*" },
      redirect: "manual",
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (!response.ok) {
      throw new UpstreamProxyError("upstream not ok");
    }

    const contentType = imageContentType(response);
    const body = await readResponseBodyCapped(response, maxBytes);
    return { ok: true, body, contentType };
  } catch (error) {
    if (error instanceof UnsafeProxyTargetError) {
      return { ok: false, status: 400 };
    }
    return { ok: false, status: 502 };
  }
}
