import { describe, expect, it } from "vitest";
import { gravatarHash, gravatarUrl, monogramLetter } from "./gravatar";

describe("gravatarHash", () => {
  it("SHA-256 hex of trimmed lowercase email", async () => {
    // echo -n 'foo@example.com' | sha256sum
    expect(await gravatarHash("  Foo@Example.com ")).toBe(
      "321ba197033e81286fedb719d60d4ed5cecaed170733cb4a92013811afc0e3b6",
    );
  });
});

describe("gravatarUrl", () => {
  it("builds d=404 URL or null for empty hash", () => {
    expect(gravatarUrl("abc", 128)).toBe("https://www.gravatar.com/avatar/abc?s=128&d=404");
    expect(gravatarUrl("")).toBeNull();
    expect(gravatarUrl("   ")).toBeNull();
  });
});

describe("monogramLetter", () => {
  it("uses username initial or seat digit", () => {
    expect(monogramLetter("alice", 0)).toBe("A");
    expect(monogramLetter("  bob", 1)).toBe("B");
    expect(monogramLetter("", 2)).toBe("2");
    expect(monogramLetter(null, 3)).toBe("3");
  });
});
