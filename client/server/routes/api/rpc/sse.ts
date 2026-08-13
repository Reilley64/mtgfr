import type { StreamFrame } from "../../../../app/domain/wire/types";
import { stringifyWireJson } from "../../../../app/domain/wire/wireJson";

export function sseChunk(frame: StreamFrame): Uint8Array {
  return new TextEncoder().encode(`data: ${stringifyWireJson(frame)}\n\n`);
}
