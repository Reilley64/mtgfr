// Same-origin BFF: sticky-route `/api/*` to a versioned Axum instance by `mtgfr-instance` cookie,
// stripping the `/api` prefix. Upstream map is runtime env so nested drain peers work without rebuild.

import type { APIEvent } from "@solidjs/start/server";
import { getRequestHeader, getRequestURL, proxyRequest } from "vinxi/http";
import { isBlockedPublicApiPath, resolveUpstreamBase } from "~/lib/apiUpstream";

function strippedPath(event: APIEvent): string {
  return event.params.path ?? "";
}

async function forward(event: APIEvent) {
  const path = strippedPath(event);
  if (isBlockedPublicApiPath(path)) {
    return new Response("Not Found", { status: 404 });
  }

  const search = getRequestURL(event.nativeEvent).search;
  const base = resolveUpstreamBase({
    upstreamsJson: process.env.API_UPSTREAMS,
    activeInstanceId: process.env.API_ACTIVE_INSTANCE_ID,
    cookieHeader: getRequestHeader(event.nativeEvent, "cookie"),
    fallbackUpstream: process.env.API_UPSTREAM,
  });
  return proxyRequest(event.nativeEvent, `${base}/${path}${search}`);
}

export const GET = forward;
export const HEAD = forward;
export const POST = forward;
export const PUT = forward;
export const PATCH = forward;
export const DELETE = forward;
export const OPTIONS = forward;
