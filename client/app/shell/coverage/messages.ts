import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { CoverageSetRow } from "./submodel";

export const ChangedCoverageRoute = m("ChangedCoverageRoute");
export const RequestedCoverageRefresh = m("RequestedCoverageRefresh");
export const ChangedCoverageQuery = m("ChangedCoverageQuery", { query: S.String });
export const ReceivedCoverageMeta = m("ReceivedCoverageMeta", {
  faithfulCount: S.NullOr(S.Number),
  oracleTotal: S.NullOr(S.Number),
  sets: S.Array(CoverageSetRow),
});
export const CoverageLoadFailed = m("CoverageLoadFailed", { message: S.String });

export const Message = S.Union([
  ChangedCoverageRoute,
  RequestedCoverageRefresh,
  ChangedCoverageQuery,
  ReceivedCoverageMeta,
  CoverageLoadFailed,
]);
export type Message = typeof Message.Type;
