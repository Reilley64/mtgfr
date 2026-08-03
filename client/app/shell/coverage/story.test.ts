import { Story } from "foldkit";
import { expect, test } from "vitest";
import { update as appUpdate, init } from "../../main-exports";
import { GotCoverageMessage } from "../../messages";
import { CoverageLoadFailed, RequestedCoverageRefresh } from "./messages";
import { FetchCoverage } from "./update";

test("GotCoverageMessage updates coverage through the parent update", () => {
  const [model] = init();
  const load = FetchCoverage();

  Story.story(
    appUpdate,
    Story.given({
      ...model,
      coverage: {
        ...model.coverage,
        error: "Could not load coverage.",
        query: "soc",
        sets: [
          {
            code: "soc",
            name: "Secrets of Strixhaven",
            releasedAt: "2026-04-01",
            faithful: 10,
            oracleTotal: 400,
          },
        ],
        status: "error",
        faithfulCount: 662,
        oracleTotal: 28412,
      },
    }),
    Story.message(GotCoverageMessage({ message: RequestedCoverageRefresh() })),
    Story.Command.expectExact(load),
    Story.model((next) => {
      expect(next.coverage.sets).toEqual([]);
      expect(next.coverage.error).toBeNull();
      expect(next.coverage.status).toBe("loading");
      expect(next.coverage.query).toBe("soc");
    }),
    Story.Command.resolve(load, CoverageLoadFailed({ message: "Could not load coverage." })),
    Story.model((next) => {
      expect(next.coverage.status).toBe("error");
      expect(next.coverage.error).toBe("Could not load coverage.");
    }),
  );
});
