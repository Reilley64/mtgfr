// Same-origin Faro collect proxy → Alloy faro.receiver (ClusterIP only).
// Body size capped; no public CORS wildcards (browser posts same-origin).

import type { APIEvent } from "@solidjs/start/server";

/** Soft cap for Faro payloads (browser RUM batches). Oversize → 413. */
export const FARO_MAX_BODY_BYTES = 512 * 1024;

export function upstreamUrl(env: NodeJS.ProcessEnv = process.env): string | null {
  const raw = env.FARO_COLLECT_UPSTREAM?.trim();
  return raw && raw.length > 0 ? raw : null;
}

/** Reject when Content-Length is present and over the cap. */
export function contentLengthTooLarge(
  contentLengthHeader: string | null,
  maxBytes: number = FARO_MAX_BODY_BYTES,
): boolean {
  if (contentLengthHeader === null) return false;
  const n = Number(contentLengthHeader);
  return Number.isFinite(n) && n > maxBytes;
}

export async function readBodyCapped(
  request: Request,
  maxBytes: number = FARO_MAX_BODY_BYTES,
): Promise<{ ok: true; body: ArrayBuffer } | { ok: false; status: 413 }> {
  if (contentLengthTooLarge(request.headers.get("content-length"), maxBytes)) {
    return { ok: false, status: 413 };
  }
  const body = await request.arrayBuffer();
  if (body.byteLength > maxBytes) {
    return { ok: false, status: 413 };
  }
  return { ok: true, body };
}

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
