// Same-origin `/api` BFF: sticky cookie → versioned API; join fans out across peers.

import type { APIEvent } from "@solidjs/start/server";
import { getRequestHeader, getRequestURL, proxyRequest } from "vinxi/http";
import {
  isUnknownTableLobbyBody,
  normalizePublicApiPath,
  upstreamBasesInOrder,
} from "~/lib/apiUpstream";

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
  for (const cookie of res.headers.getSetCookie()) {
    out.headers.append("set-cookie", cookie);
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
  let last!: Response;

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

  return last;
}

async function forward(event: APIEvent) {
  const path = normalizePublicApiPath(event.params.path ?? "");
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

  if (event.request.method === "POST" && path === "tables/join/v1" && bases.length > 1) {
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
