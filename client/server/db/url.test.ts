import { afterEach, describe, expect, it } from "vitest";
import { DEFAULT_WEB_DATABASE_URL, webDatabaseUrl } from "./url";

describe("webDatabaseUrl", () => {
  const previous = process.env.WEB_DATABASE_URL;

  afterEach(() => {
    if (previous === undefined) delete process.env.WEB_DATABASE_URL;
    else process.env.WEB_DATABASE_URL = previous;
  });

  it("defaults to the local compose mtgfr_web URL", () => {
    delete process.env.WEB_DATABASE_URL;
    expect(webDatabaseUrl()).toBe(DEFAULT_WEB_DATABASE_URL);
    expect(webDatabaseUrl()).toContain("mtgfr_web");
  });

  it("prefers WEB_DATABASE_URL when set", () => {
    process.env.WEB_DATABASE_URL = "postgresql://example/custom";
    expect(webDatabaseUrl()).toBe("postgresql://example/custom");
  });
});
