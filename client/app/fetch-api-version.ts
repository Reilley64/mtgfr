import { Effect } from "effect";
import { Command } from "foldkit";
import { apiVersion } from "../lib/lobby/client";
import { ReceivedApiVersion } from "./messages";

export const FetchApiVersion = Command.define(
  "FetchApiVersion",
  ReceivedApiVersion,
)(
  Effect.tryPromise(() => apiVersion()).pipe(
    Effect.map((response) => {
      const tag = response?.version?.trim();
      return ReceivedApiVersion({ version: tag ? tag : null });
    }),
    Effect.catch(() => Effect.succeed(ReceivedApiVersion({ version: null }))),
  ),
);
