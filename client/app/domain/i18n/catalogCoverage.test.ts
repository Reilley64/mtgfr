import { describe, expect, it } from "vitest";
import { enCatalog } from "./catalog/en";
import rustKeys from "./rustKeys.json";

describe("catalog coverage", () => {
  it("includes every key the engine/server can emit", () => {
    const missing = rustKeys.filter((key) => !(key in enCatalog));
    expect(missing).toEqual([]);
  });
});
