/** JSON transport helpers for browser-facing wire values. */

import * as Effect from "effect/Effect";
import * as Schema from "effect/Schema";
import type { StreamFrame } from "./types";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** JSON has no bigint primitive. Preserve every bigint losslessly as its decimal spelling. */
export function stringifyWireJson(value: unknown): string {
  return JSON.stringify(value, (_key, child: unknown) => (typeof child === "bigint" ? child.toString() : child));
}

const UINT64_MAX_DECIMAL = "18446744073709551615";
const UINT64_DECIMAL = /^(0|[1-9]\d*)$/;

function isCanonicalUint64(value: string): boolean {
  if (value.length > UINT64_MAX_DECIMAL.length || !UINT64_DECIMAL.test(value)) return false;
  return value.length < UINT64_MAX_DECIMAL.length || value <= UINT64_MAX_DECIMAL;
}

/** Restore the one browser wire field whose domain type intentionally remains bigint.
 * Other decimal-looking strings are application data and stay strings. */
function reviveStreamStackEntryIds(value: unknown): unknown {
  if (!isRecord(value) || (value.frame !== "snapshot" && value.frame !== "delta")) return value;
  if (!isRecord(value.state) || !Array.isArray(value.state.stack)) return value;

  const stack = value.state.stack.map((entry) => {
    if (!isRecord(entry) || typeof entry.entry_id !== "string" || !isCanonicalUint64(entry.entry_id)) return entry;
    return { ...entry, entry_id: BigInt(entry.entry_id) };
  });
  return { ...value, state: { ...value.state, stack } };
}

/** Focused validation for the trusted BFF's StreamFrame envelope. The generated domain model is
 * intentionally type-only; validate its discriminant and required transport containers here. */
function isStreamFrame(value: unknown): value is StreamFrame {
  if (!isRecord(value)) return false;
  if (value.frame === "heartbeat") return true;
  if (value.frame !== "snapshot" && value.frame !== "delta") return false;
  if (typeof value.seq !== "number" || !isRecord(value.state) || !Array.isArray(value.state.stack)) return false;
  if (!value.state.stack.every((entry) => isRecord(entry) && typeof entry.entry_id === "bigint")) return false;
  return value.frame === "snapshot" || Array.isArray(value.events);
}

const StreamFrameSchema = Schema.declare<StreamFrame>(isStreamFrame, { identifier: "StreamFrame" });

export class StreamFrameParseError extends Schema.TaggedErrorClass<StreamFrameParseError>()("StreamFrameParseError", {
  message: Schema.String,
}) {}

const invalidStreamFrame = () =>
  StreamFrameParseError.make({
    message: "Invalid SSE stream frame",
  });

/** Parse, normalize, and validate one SSE frame without allowing parser exceptions to defect. */
export function parseStreamFrameJson(json: string): Effect.Effect<StreamFrame, StreamFrameParseError> {
  return Effect.try({
    try: () => reviveStreamStackEntryIds(JSON.parse(json)),
    catch: invalidStreamFrame,
  }).pipe(
    Effect.flatMap((value) =>
      Schema.decodeUnknownEffect(StreamFrameSchema)(value).pipe(Effect.mapError(invalidStreamFrame)),
    ),
  );
}
