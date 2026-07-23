/**
 * W3C Trace Context helpers for BFF ↔ browser ↔ API parenting.
 * Spec: https://www.w3.org/TR/trace-context/
 */

export type TraceContext = {
  traceId: string;
  spanId: string;
  /** Bit field; bit 0 = sampled. */
  traceFlags: number;
};

const TRACEPARENT_RE = /^([0-9a-f]{2})-([0-9a-f]{32})-([0-9a-f]{16})-([0-9a-f]{2})$/i;

/** Parse a `traceparent` header; returns null when missing or invalid. */
export function parseTraceparent(header: string | null | undefined): TraceContext | null {
  if (header == null) return null;
  const trimmed = header.trim();
  if (trimmed.length === 0) return null;

  const match = TRACEPARENT_RE.exec(trimmed);
  if (!match) return null;

  const [, versionRaw, traceIdRaw, spanIdRaw, traceFlagsRaw] = match;
  if (!versionRaw || !traceIdRaw || !spanIdRaw || !traceFlagsRaw) return null;

  const version = versionRaw.toLowerCase();
  // Version 0xff is forbidden; unknown versions are still accepted if the shape matches.
  if (version === "ff") return null;

  const traceId = traceIdRaw.toLowerCase();
  const spanId = spanIdRaw.toLowerCase();
  if (traceId === "0".repeat(32) || spanId === "0".repeat(16)) return null;

  const traceFlags = Number.parseInt(traceFlagsRaw, 16);
  if (!Number.isFinite(traceFlags)) return null;

  return { traceId, spanId, traceFlags };
}

/** Format a W3C `traceparent` value (version 00). */
export function formatTraceparent(ctx: {
  traceId: string;
  spanId: string;
  sampled?: boolean;
  traceFlags?: number;
}): string {
  const flags = ctx.traceFlags ?? (ctx.sampled === false ? 0x00 : 0x01);
  return `00-${ctx.traceId}-${ctx.spanId}-${flags.toString(16).padStart(2, "0")}`;
}
