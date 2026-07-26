import { Effect } from "effect";
import { Command } from "foldkit";
import { ReceivedApiVersion } from "./messages";
import { LobbyClient } from "./resources";

export const FetchApiVersion = Command.define(
  "FetchApiVersion",
  ReceivedApiVersion,
)(
  Effect.gen(function* () {
    const lobby = yield* LobbyClient;
    const response = yield* lobby.apiMeta();
    const tag = response.version.trim();
    return ReceivedApiVersion({
      version: tag ? tag : null,
      faithfulCount: response.faithfulCount,
      oracleTotal: response.oracleTotal,
    });
  }).pipe(
    Effect.catch(() => Effect.succeed(ReceivedApiVersion({ version: null, faithfulCount: null, oracleTotal: null }))),
  ),
);
