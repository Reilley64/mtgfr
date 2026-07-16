// Same-origin BFF: forward `/api/*` to the Axum API, stripping the `/api` prefix.
// `API_UPSTREAM` is read per request so Docker/k8s can set it at runtime (dev defaults to :8080).

import type { APIEvent } from "@solidjs/start/server";
import { getRequestURL, proxyRequest } from "vinxi/http";

function upstreamUrl(event: APIEvent): string {
  const base = (process.env.API_UPSTREAM ?? "http://127.0.0.1:8080").replace(/\/$/, "");
  const path = event.params.path ?? "";
  const search = getRequestURL(event.nativeEvent).search;
  return `${base}/${path}${search}`;
}

async function forward(event: APIEvent) {
  return proxyRequest(event.nativeEvent, upstreamUrl(event));
}

export const GET = forward;
export const HEAD = forward;
export const POST = forward;
export const PUT = forward;
export const PATCH = forward;
export const DELETE = forward;
export const OPTIONS = forward;
