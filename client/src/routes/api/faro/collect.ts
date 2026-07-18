// Same-origin Faro collect proxy → Alloy faro.receiver (ClusterIP only).
// Body size capped; no public CORS wildcards (browser posts same-origin).

import type { APIEvent } from "@solidjs/start/server";
import { readBodyCapped, upstreamUrl } from "~/lib/faroCollect";

async function proxyCollect(event: APIEvent): Promise<Response> {
  const upstream = upstreamUrl();
  if (!upstream) {
    // Local/dev without Alloy: accept and drop so Faro does not retry loudly.
    return new Response(null, { status: 204 });
  }

  const capped = await readBodyCapped(event.request);
  if (!capped.ok) {
    return new Response(JSON.stringify({ error: "PayloadTooLarge" }), {
      status: 413,
      headers: { "content-type": "application/json" },
    });
  }

  const headers = new Headers();
  const contentType = event.request.headers.get("content-type");
  if (contentType) headers.set("content-type", contentType);
  const faroSession = event.request.headers.get("x-faro-session-id");
  if (faroSession) headers.set("x-faro-session-id", faroSession);

  const res = await fetch(upstream, {
    method: "POST",
    headers,
    body: capped.body,
  });
  return new Response(res.body, {
    status: res.status,
    headers: {
      "content-type": res.headers.get("content-type") ?? "application/json",
    },
  });
}

export async function POST(event: APIEvent): Promise<Response> {
  return proxyCollect(event);
}

/** Same-origin only — no ACAO wildcards. */
export async function OPTIONS(): Promise<Response> {
  return new Response(null, { status: 204 });
}
