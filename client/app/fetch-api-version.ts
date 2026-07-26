import { Effect } from "effect";
import { Command } from "foldkit";
import { apiMeta } from "../lib/lobby/client";
import { ReceivedApiVersion } from "./messages";

export const FetchApiVersion = Command.define(
  "FetchApiVersion",
  ReceivedApiVersion,
)(
  Effect.tryPromise(() => apiMeta()).pipe(
    Effect.map((response) => {
      const tag = response?.version?.trim();
      return ReceivedApiVersion({ version: tag ? tag : null });
    }),
    Effect.catch(() => Effect.succeed(ReceivedApiVersion({ version: null }))),
  ),
);
