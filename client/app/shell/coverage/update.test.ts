import * as Effect from "effect/Effect";
import { describe, expect, it } from "vitest";
import { client as lobbyHttpClient } from "../../domain/lobby/client";
import { LobbyHttpError } from "../../domain/lobby/errors";
import { LobbyClient } from "../../resources";
import { CoverageLoadFailed, ReceivedCoverageMeta } from "./messages";
import { FetchCoverage } from "./update";

describe("FetchCoverage", () => {
  it("loads coverage metadata through LobbyClient", async () => {
    const coverageClient = {
      ...lobbyHttpClient,
      coverageMeta: () =>
        Effect.succeed({
          faithfulCount: 662,
          oracleTotal: 28412,
          sets: [
            {
              code: "soc",
              name: "Secrets of Strixhaven",
              releasedAt: "2026-04-01",
              faithful: 10,
              oracleTotal: 400,
            },
          ],
        }),
    };

    const message = await Effect.runPromise(
      FetchCoverage().effect.pipe(Effect.provideService(LobbyClient, coverageClient)),
    );

    expect(message).toEqual(
      ReceivedCoverageMeta({
        faithfulCount: 662,
        oracleTotal: 28412,
        sets: [
          {
            code: "soc",
            name: "Secrets of Strixhaven",
            releasedAt: "2026-04-01",
            faithful: 10,
            oracleTotal: 400,
          },
        ],
      }),
    );
  });

  it("keeps the existing load failure message", async () => {
    const failingClient = {
      ...lobbyHttpClient,
      coverageMeta: () => Effect.fail(new LobbyHttpError({ status: 500, description: "Server Error" })),
    };

    const message = await Effect.runPromise(
      FetchCoverage().effect.pipe(Effect.provideService(LobbyClient, failingClient)),
    );

    expect(message).toEqual(CoverageLoadFailed({ message: "Could not load coverage." }));
  });
});
