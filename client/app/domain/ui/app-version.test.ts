import { describe, expect, it } from "vitest";
import { testHtml } from "~/test-html";
import { appVersionBadge, formatFaithfulPercent } from "./app-version";

const h = testHtml<never>();

describe("formatFaithfulPercent", () => {
  it("returns null when oracleTotal is not positive", () => {
    expect(formatFaithfulPercent(10, 0)).toBeNull();
    expect(formatFaithfulPercent(10, -1)).toBeNull();
  });

  it("uses one decimal below 10%", () => {
    expect(formatFaithfulPercent(662, 28412)).toBe("2.3%");
  });

  it("uses whole percent at 10% and above", () => {
    expect(formatFaithfulPercent(1000, 10000)).toBe("10%");
    expect(formatFaithfulPercent(2500, 10000)).toBe("25%");
  });

  it("clamps to 100% when faithful exceeds oracle total", () => {
    expect(formatFaithfulPercent(500, 400)).toBe("100%");
  });
});

describe("appVersionBadge", () => {
  it("renders nothing until the API version is known", () => {
    expect(
      appVersionBadge(h, {
        version: null,
        faithfulCount: 1,
        oracleTotal: 100,
        coverageHref: null,
      }),
    ).toBeNull();
  });

  it("renders version only when coverage is incomplete", () => {
    const badge = appVersionBadge(h, {
      version: "1.2.3",
      faithfulCount: null,
      oracleTotal: null,
      coverageHref: null,
    });
    const s = JSON.stringify(badge);
    expect(s).toContain("app-version");
    expect(s).toContain("API 1.2.3");
    expect(s).not.toContain("pool-coverage");
  });

  it("stacks percent faithful above API version", () => {
    const badge = appVersionBadge(h, {
      version: "1.2.3",
      faithfulCount: 662,
      oracleTotal: 28412,
      coverageHref: null,
    });
    const s = JSON.stringify(badge);
    expect(s).toContain("pool-coverage");
    expect(s).toContain("2.3% faithful");
    expect(s).toContain("API 1.2.3");
    expect(s.indexOf("pool-coverage")).toBeLessThan(s.indexOf("app-version"));
  });

  it("renders pool coverage as a link when coverageHref is set", () => {
    const badge = appVersionBadge(h, {
      version: "1.2.3",
      faithfulCount: 662,
      oracleTotal: 28412,
      coverageHref: "/coverage",
    });
    const s = JSON.stringify(badge);
    expect(s).toContain('"sel":"a"');
    expect(s).toContain('"/coverage"');
    expect(s).toContain("pool-coverage");
    expect(s).toContain("pointer-events-auto");
  });
});
