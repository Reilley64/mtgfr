import { describe, expect, it } from "vitest";
import { appVersion, gitCommit } from "~/lib/buildMeta";

describe("buildMeta", () => {
  it("prefers APP_VERSION / GIT_COMMIT env over Vite defaults", () => {
    expect(appVersion({ APP_VERSION: " 2.4.0 " }, "vite-fallback")).toBe("2.4.0");
    expect(gitCommit({ GIT_COMMIT: " abc123 " }, "vite-sha")).toBe("abc123");
  });

  it("falls back to Vite then to placeholders", () => {
    expect(appVersion({}, "1.2.3")).toBe("1.2.3");
    expect(gitCommit({}, "deadbeef")).toBe("deadbeef");
    expect(appVersion({}, undefined)).toBe("dev");
    expect(gitCommit({}, undefined)).toBe("unknown");
  });
});
