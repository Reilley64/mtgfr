import { describe, expect, it } from "vitest";
import { ensureFaroSessionSampled, FARO_SESSION_STORAGE_KEY } from "~/lib/faroSession";

function memoryStorage(initial?: Record<string, string>): Storage {
  const map = new Map<string, string>(Object.entries(initial ?? {}));
  return {
    get length() {
      return map.size;
    },
    clear: () => map.clear(),
    getItem: (k) => map.get(k) ?? null,
    setItem: (k, v) => {
      map.set(k, String(v));
    },
    removeItem: (k) => {
      map.delete(k);
    },
    key: (i) => [...map.keys()][i] ?? null,
  };
}

describe("ensureFaroSessionSampled", () => {
  it("is a no-op when nothing is stored", () => {
    const storage = memoryStorage();
    ensureFaroSessionSampled(storage);
    expect(storage.getItem(FARO_SESSION_STORAGE_KEY)).toBeNull();
  });

  it("forces isSampled on legacy sessions that omit the flag", () => {
    const storage = memoryStorage({
      [FARO_SESSION_STORAGE_KEY]: JSON.stringify({
        sessionId: "abc",
        lastActivity: 1,
        started: 1,
      }),
    });
    ensureFaroSessionSampled(storage);
    const parsed = JSON.parse(storage.getItem(FARO_SESSION_STORAGE_KEY)!);
    expect(parsed.isSampled).toBe(true);
    expect(parsed.sessionMeta.attributes.isSampled).toBe("true");
  });

  it("repairs sessions explicitly stored as unsampled", () => {
    const storage = memoryStorage({
      [FARO_SESSION_STORAGE_KEY]: JSON.stringify({
        sessionId: "abc",
        isSampled: false,
        sessionMeta: { id: "abc", attributes: { isSampled: "false" } },
      }),
    });
    ensureFaroSessionSampled(storage);
    const parsed = JSON.parse(storage.getItem(FARO_SESSION_STORAGE_KEY)!);
    expect(parsed.isSampled).toBe(true);
    expect(parsed.sessionMeta.attributes.isSampled).toBe("true");
  });

  it("leaves already-sampled sessions untouched", () => {
    const raw = JSON.stringify({
      sessionId: "abc",
      isSampled: true,
      sessionMeta: { id: "abc", attributes: { isSampled: "true", keep: "me" } },
    });
    const storage = memoryStorage({ [FARO_SESSION_STORAGE_KEY]: raw });
    ensureFaroSessionSampled(storage);
    expect(storage.getItem(FARO_SESSION_STORAGE_KEY)).toBe(raw);
  });
});
