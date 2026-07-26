import { expect, test } from "vitest";
import { coveragePercentText, visibleCoverageRows } from "./view";

test("visibleCoverageRows sorts by releasedAt descending then name", () => {
  const rows = visibleCoverageRows({
    query: "",
    sets: [
      { code: "alpha", name: "Alpha", releasedAt: "2010-01-01", faithful: 1, oracleTotal: 10 },
      { code: "soc", name: "Secrets of Strixhaven", releasedAt: "2026-04-01", faithful: 1, oracleTotal: 10 },
      { code: "beta", name: "Beta", releasedAt: "2010-01-01", faithful: 1, oracleTotal: 10 },
      { code: "mystery", name: "Mystery", releasedAt: null, faithful: 1, oracleTotal: 10 },
    ],
  });

  expect(rows.map((row) => row.code)).toEqual(["soc", "alpha", "beta", "mystery"]);
});

test("visibleCoverageRows filters by lowercase code or name query", () => {
  const rows = visibleCoverageRows({
    query: "strix",
    sets: [
      { code: "soc", name: "Secrets of Strixhaven", releasedAt: null, faithful: 10, oracleTotal: 400 },
      { code: "c16", name: "Commander 2016", releasedAt: null, faithful: 5, oracleTotal: 100 },
    ],
  });

  expect(rows.map((row) => row.code)).toEqual(["soc"]);
});

test("coveragePercentText shows em dash when oracle total is missing", () => {
  expect(coveragePercentText(10, null)).toBe("—");
});

test("coveragePercentText shows em dash when faithful count is missing", () => {
  expect(coveragePercentText(null, 400)).toBe("—");
});

test("coveragePercentText shows 0.0% when faithful is zero", () => {
  expect(coveragePercentText(0, 400)).toBe("0.0%");
});
