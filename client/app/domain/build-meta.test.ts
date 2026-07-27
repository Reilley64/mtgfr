import { describe, expect, it } from "vitest";
import { appVersion, gitCommit } from "./build-meta";

describe("build-meta (browser-safe)", () => {
  it("falls back to vite / defaults without reading a missing process global", () => {
    expect(appVersion({}, undefined)).toBe("dev");
    expect(appVersion({}, "1.2.3")).toBe("1.2.3");
    expect(appVersion({ APP_VERSION: " 9.9.9 " }, "1.2.3")).toBe("9.9.9");
    expect(gitCommit({}, undefined)).toBe("unknown");
    expect(gitCommit({}, "abc")).toBe("abc");
  });
});
