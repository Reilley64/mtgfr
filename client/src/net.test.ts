// `buildIntentEnvelope` (table/`client_seq`) is plain data-building logic; test it directly
// instead of stubbing `fetch` for something that never makes a network call. The `/intent` POST it
// feeds is covered by `effect/api-endpoints.test.ts`.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { buildIntentEnvelope, parseTableCode } from "~/net";

const intent = { kind: "pass_priority", player: 0 } as const;

describe("parseTableCode", () => {
  beforeEach(() => {
    vi.stubGlobal("location", { origin: "https://edh.example.com", pathname: "/play" });
  });

  it("accepts a bare code and uppercases it", () => {
    expect(parseTableCode("abc123")).toBe("ABC123");
    expect(parseTableCode("  xyz789  ")).toBe("XYZ789");
  });

  it("extracts the code from a path-based share link", () => {
    expect(parseTableCode("https://edh.example.com/play/abc123")).toBe("ABC123");
    expect(parseTableCode("/play/XYZ789")).toBe("XYZ789");
  });

  it("extracts the code from a legacy query-based share link", () => {
    expect(parseTableCode("https://edh.example.com/play?table=abc123")).toBe("ABC123");
    expect(parseTableCode("https://edh.example.com/?table=XYZ789&deck=3")).toBe("XYZ789");
    expect(parseTableCode("/play?table=abc123")).toBe("ABC123");
    expect(parseTableCode("?table=xyz789")).toBe("XYZ789");
  });

  it("returns null for empty input", () => {
    expect(parseTableCode("")).toBeNull();
    expect(parseTableCode("   ")).toBeNull();
  });

  it("returns null for a share link with no table code", () => {
    expect(parseTableCode("https://edh.example.com/play")).toBeNull();
    expect(parseTableCode("/play")).toBeNull();
  });

  it("still accepts bare codes that are not link-shaped", () => {
    expect(parseTableCode("PLAY")).toBe("PLAY");
  });
});

describe("buildIntentEnvelope", () => {
  beforeEach(() => {
    vi.stubGlobal("location", { pathname: "/play/abc" });
  });

  it("carries the current table and the given intent", () => {
    const envelope = buildIntentEnvelope(intent);
    expect(envelope.table_id).toBe("abc");
    expect(envelope.intent).toEqual(intent);
  });

  it("assigns a fresh, strictly increasing client_seq each call", () => {
    const a = buildIntentEnvelope(intent);
    const b = buildIntentEnvelope(intent);
    expect(b.client_seq).toBe(a.client_seq + 1);
  });
});
