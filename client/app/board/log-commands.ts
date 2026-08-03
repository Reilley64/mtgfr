import { Effect, Schema as S } from "effect";
import { Command } from "foldkit";
import { LogCopyCompleted } from "./messages";

export const CopyBoardLog = Command.define("CopyBoardLog", {
  args: { text: S.String },
  messages: [LogCopyCompleted],
  execute: ({ text }) =>
    Effect.tryPromise(() => navigator.clipboard.writeText(text)).pipe(
      Effect.as(LogCopyCompleted({ ok: true })),
      Effect.catch(() => Effect.succeed(LogCopyCompleted({ ok: false }))),
    ),
});
