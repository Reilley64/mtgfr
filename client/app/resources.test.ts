import * as Effect from "effect/Effect";
import { describe, expect, it } from "vitest";
import { LobbyClient, resources } from "./resources";

describe("resources", () => {
  it("provides LobbyClient from the Foldkit resources layer", async () => {
    const hasLobby = Effect.gen(function* () {
      const lobby = yield* LobbyClient;
      return typeof lobby.createTable === "function";
    }).pipe(Effect.provide(resources));

    await expect(Effect.runPromise(hasLobby)).resolves.toBe(true);
  });
});
