import { html } from "foldkit/html";
import { describe, expect, it } from "vitest";
import { appVersionBadge } from "./app-version";

const h = html<never>();

describe("appVersionBadge", () => {
  it("renders nothing until the API version is known", () => {
    expect(appVersionBadge(h, null)).toBeNull();
  });

  it("renders the fetched API tag", () => {
    const badge = appVersionBadge(h, "1.2.3");
    expect(badge).not.toBeNull();
    expect(JSON.stringify(badge)).toContain("app-version");
    expect(JSON.stringify(badge)).toContain("API 1.2.3");
  });
});
