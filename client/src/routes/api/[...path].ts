// Same-origin BFF: sticky-route `/api/*` to a versioned Axum instance by `mtgfr-instance` cookie,
// stripping the `/api` prefix. Upstream map is runtime env so nested drain peers work without rebuild.
// Guests joining by table code have no sticky cookie — fan out `POST tables/join/v1` across peers.

import type { APIEvent } from "@solidjs/start/server";
import { getRequestHeader, getRequestURL, proxyRequest } from "vinxi/http";
import {
  isUnknownTableLobbyBody,
  normalizePublicApiPath,
  shouldFanOutJoin,
  upstreamBasesInOrder,
} from "~/lib/apiUpstream";

function strippedPath(event: APIEvent): string {
  return event.params.path ?? "";
}

function upstreamRequestHeaders(req: Request): Headers {
  const out = new Headers();
  for (const name of ["cookie", "content-type", "accept", "authorization"] as const) {
    const value = req.headers.get(name);
    if (value) out.set(name, value);
  }
  return out;
}

function responseFromUpstream(res: Response, body: string): Response {
  const headers = new Headers();
  res.headers.forEach((value, key) => {
    if (key.toLowerCase() === "set-cookie") return;
    headers.set(key, value);
  });
  const out = new Response(body, { status: res.status, statusText: res.statusText, headers });
  const setCookies =
    typeof res.headers.getSetCookie === "function" ? res.headers.getSetCookie() : [];
  for (const cookie of setCookies) {
    out.headers.append("set-cookie", cookie);
  }
  if (setCookies.length === 0) {
    const single = res.headers.get("set-cookie");
    if (single) out.headers.append("set-cookie", single);
  }
  return out;
}

async function fanOutJoin(
  event: APIEvent,
  path: string,
  search: string,
  bases: string[],
): Promise<Response> {
  const body = await event.request.arrayBuffer();
  const headers = upstreamRequestHeaders(event.request);
  let last: Response | null = null;

  for (const base of bases) {
    const res = await fetch(`${base}/${path}${search}`, {
      method: "POST",
      headers,
      body: body.byteLength > 0 ? body.slice(0) : undefined,
    });
    const text = await res.text();
    last = responseFromUpstream(res, text);
    if (!isUnknownTableLobbyBody(text)) return last;
  }

  return last ?? new Response("Not Found", { status: 404 });
}

async function forward(event: APIEvent) {
  const path = normalizePublicApiPath(strippedPath(event));
  if (path === null) {
    return new Response("Not Found", { status: 404 });
  }

  const search = getRequestURL(event.nativeEvent).search;
  const bases = upstreamBasesInOrder({
    upstreamsJson: process.env.API_UPSTREAMS,
    activeInstanceId: process.env.API_ACTIVE_INSTANCE_ID,
    cookieHeader: getRequestHeader(event.nativeEvent, "cookie"),
    fallbackUpstream: process.env.API_UPSTREAM,
  });

  if (shouldFanOutJoin(path, event.request.method) && bases.length > 1) {
    return fanOutJoin(event, path, search, bases);
  }

  return proxyRequest(event.nativeEvent, `${bases[0]}/${path}${search}`);
}

export const GET = forward;
export const HEAD = forward;
export const POST = forward;
export const PUT = forward;
export const PATCH = forward;
export const DELETE = forward;
export const OPTIONS = forward;
