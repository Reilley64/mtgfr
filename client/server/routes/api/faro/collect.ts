// Same-origin Faro collect proxy → Alloy faro.receiver (ClusterIP only).
// Body size capped; no public CORS wildcards (browser posts same-origin).

import { defineHandler, type H3Event } from "nitro/h3";
import { bodyCapped, upstreamUrl } from "../../../../app/domain/faro/collect";

function arrayBufferOf(bytes: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy.buffer;
}

async function proxyCollect(event: H3Event): Promise<Response> {
  const upstream = upstreamUrl();
  if (!upstream) {
    // Local/dev without Alloy: accept and drop so Faro does not retry loudly.
    return new Response(null, { status: 204 });
  }

  const rawBody = new Uint8Array(await event.req.arrayBuffer());
  const capped = bodyCapped(event.req.headers.get("content-length") ?? null, rawBody);
  if (!capped.ok) {
    return new Response(JSON.stringify({ error: "PayloadTooLarge" }), {
      status: 413,
      headers: { "content-type": "application/json" },
    });
  }

  const headers = new Headers();
  const contentType = event.req.headers.get("content-type");
  if (contentType) headers.set("content-type", contentType);
  const faroSession = event.req.headers.get("x-faro-session-id");
  if (faroSession) headers.set("x-faro-session-id", faroSession);

  const res = await fetch(upstream, {
    method: "POST",
    headers,
    body: arrayBufferOf(capped.body),
  });
  return new Response(res.body, {
    status: res.status,
    headers: {
      "content-type": res.headers.get("content-type") ?? "application/json",
    },
  });
}

export default defineHandler(async (event) => {
  if (event.req.method === "OPTIONS") {
    /** Same-origin only — no ACAO wildcards. */
    return new Response(null, { status: 204 });
  }
  if (event.req.method !== "POST") {
    return new Response("Method Not Allowed", { status: 405 });
  }
  return proxyCollect(event);
});
