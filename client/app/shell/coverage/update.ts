import { Effect, Match as M } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import { coverageMeta } from "../../../lib/lobby/client";
import type { RpcClient } from "../../resources";
import { CoverageLoadFailed, type Message, ReceivedCoverageMeta } from "./messages";
import type { CoverageSubmodel } from "./submodel";

const COVERAGE_LOAD_ERROR = "Could not load coverage.";

export const FetchCoverage = Command.define(
  "FetchCoverage",
  ReceivedCoverageMeta,
  CoverageLoadFailed,
)(
  Effect.tryPromise(() => coverageMeta()).pipe(
    Effect.map((response) =>
      response == null
        ? CoverageLoadFailed({ message: COVERAGE_LOAD_ERROR })
        : ReceivedCoverageMeta({
            faithfulCount: response.faithfulCount,
            oracleTotal: response.oracleTotal,
            sets: response.sets,
          }),
    ),
    Effect.catch(() => Effect.succeed(CoverageLoadFailed({ message: COVERAGE_LOAD_ERROR }))),
  ),
);

export function loadCoverage(
  model: CoverageSubmodel,
): readonly [CoverageSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  return [
    {
      ...model,
      status: "loading",
      sets: [],
      faithfulCount: null,
      oracleTotal: null,
      error: null,
      accountMenuOpen: false,
    },
    [FetchCoverage()],
  ];
}

export const update = (
  model: CoverageSubmodel,
  message: Message,
): readonly [CoverageSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [CoverageSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
    M.tagsExhaustive({
      RequestedCoverageRefresh: () => loadCoverage(model),
      ChangedCoverageQuery: ({ query }) => [{ ...model, query }, []],
      ReceivedCoverageMeta: ({ faithfulCount, oracleTotal, sets }) => [
        {
          ...model,
          status: "ready",
          faithfulCount,
          oracleTotal,
          sets: [...sets],
          error: null,
        },
        [],
      ],
      CoverageLoadFailed: ({ message }) => [{ ...model, status: "error", error: message }, []],
    }),
  );
