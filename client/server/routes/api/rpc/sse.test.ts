import { describe, expect, it } from "vitest";
import { sseChunk } from "./sse";

describe("sseChunk", () => {
  it("writes bigint stack entry ids as exact decimal JSON strings", () => {
    const chunk = sseChunk({
      frame: "snapshot",
      seq: 1,
      state: { stack: [{ entry_id: 9_007_199_254_740_993n }] },
    } as never);

    expect(new TextDecoder().decode(chunk)).toContain('"entry_id":"9007199254740993"');
  });
});
