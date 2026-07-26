import { expect, test } from "vitest";
import { coveragePercentText, visibleCoverageRows } from "./view";

test("visibleCoverageRows sorts by percent descending then name", () => {
  const rows = visibleCoverageRows({
    query: "",
    sets: [
      { code: "beta", name: "Beta", releasedAt: null, faithful: 1, oracleTotal: 2 },
      { code: "alpha", name: "Alpha", releasedAt: null, faithful: 1, oracleTotal: 2 },
      { code: "soc", name: "Secrets of Strixhaven", releasedAt: null, faithful: 10, oracleTotal: 400 },
      { code: "mystery", name: "Mystery Set", releasedAt: null, faithful: 3, oracleTotal: null },
    ],
  });

  expect(rows.map((row) => row.code)).toEqual(["alpha", "beta", "soc", "mystery"]);
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
