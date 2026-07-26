import * as Effect from "effect/Effect";
import { describe, expect, it } from "vitest";
import { client as lobbyHttpClient } from "./domain/lobby/client";
import { LobbyHttpError } from "./domain/lobby/errors";
import { FetchApiVersion } from "./fetch-api-version";
import { ReceivedApiVersion } from "./messages";
import { LobbyClient } from "./resources";

describe("FetchApiVersion", () => {
  it("loads API metadata through LobbyClient", async () => {
    const metaClient = {
      ...lobbyHttpClient,
      apiMeta: () => Effect.succeed({ version: " 1.2.3 ", faithfulCount: 662, oracleTotal: 28412 }),
    };

    const message = await Effect.runPromise(
      FetchApiVersion().effect.pipe(Effect.provideService(LobbyClient, metaClient)),
    );

    expect(message).toEqual(ReceivedApiVersion({ version: "1.2.3", faithfulCount: 662, oracleTotal: 28412 }));
  });

  it("falls back to null metadata when the request fails", async () => {
    const failingClient = {
      ...lobbyHttpClient,
      apiMeta: () => Effect.fail(new LobbyHttpError({ status: 500, description: "Server Error" })),
    };

    const message = await Effect.runPromise(
      FetchApiVersion().effect.pipe(Effect.provideService(LobbyClient, failingClient)),
    );

    expect(message).toEqual(ReceivedApiVersion({ version: null, faithfulCount: null, oracleTotal: null }));
  });
});
