import { Schema as S } from "effect";

export const CoverageSetRow = S.Struct({
  code: S.String,
  name: S.String,
  releasedAt: S.NullOr(S.String),
  faithful: S.Number,
  oracleTotal: S.NullOr(S.Number),
});
export type CoverageSetRow = typeof CoverageSetRow.Type;

export const CoverageStatus = S.Union([
  S.Literal("idle"),
  S.Literal("loading"),
  S.Literal("ready"),
  S.Literal("error"),
]);
export type CoverageStatus = typeof CoverageStatus.Type;

export const CoverageSubmodel = S.Struct({
  status: CoverageStatus,
  query: S.String,
  sets: S.Array(CoverageSetRow),
  faithfulCount: S.NullOr(S.Number),
  oracleTotal: S.NullOr(S.Number),
  error: S.NullOr(S.String),
  accountMenuOpen: S.Boolean,
});
export type CoverageSubmodel = typeof CoverageSubmodel.Type;

export function initialCoverageSubmodel(): CoverageSubmodel {
  return {
    status: "idle",
    query: "",
    sets: [],
    faithfulCount: null,
    oracleTotal: null,
    error: null,
    accountMenuOpen: false,
  };
}
