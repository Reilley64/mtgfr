// Faro collect helpers — kept out of the SolidStart route module so Nitro's
// production bundle does not drop named exports the handler still calls
// (`ReferenceError: upstreamUrl is not defined` in `.output/server`).

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

export function bodyCapped(
  contentLengthHeader: string | null,
  body: Uint8Array,
  maxBytes: number = FARO_MAX_BODY_BYTES,
): { ok: true; body: Uint8Array } | { ok: false; status: 413 } {
  if (contentLengthTooLarge(contentLengthHeader, maxBytes)) {
    return { ok: false, status: 413 };
  }
  if (body.byteLength > maxBytes) {
    return { ok: false, status: 413 };
  }
  return { ok: true, body };
}
