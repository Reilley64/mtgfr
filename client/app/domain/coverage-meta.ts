import { apiUpstream, parseLiveStatus } from "./api-upstream-auth";
import { ensureOracleTotalRefresh, getCachedOracleTotal, getCachedOracleTotalBySet } from "./scryfall-oracle-total";
import { ensureScryfallSetsRefresh, getCachedScryfallSets, type ScryfallSetRow } from "./scryfall-sets";

export type CoverageSetRow = {
  code: string;
  name: string;
  releasedAt: string | null;
  faithful: number;
  oracleTotal: number | null;
};

export type CoverageMeta = {
  faithfulCount: number | null;
  oracleTotal: number | null;
  sets: CoverageSetRow[];
};

export function joinCoverageSetRows(
  sets: ReadonlyArray<ScryfallSetRow> | null,
  oracleTotalBySet: Readonly<Record<string, number>> | null,
  faithfulBySet: Readonly<Record<string, number>> | null,
): CoverageSetRow[] {
  if (sets == null || sets.length === 0) return [];

  return sets.map((set) => ({
    code: set.code,
    name: set.name,
    releasedAt: set.releasedAt,
    faithful: faithfulBySet?.[set.code] ?? 0,
    oracleTotal: oracleTotalBySet?.[set.code] ?? null,
  }));
}

function unavailableCoverageMeta(): CoverageMeta {
  return {
    faithfulCount: null,
    oracleTotal: getCachedOracleTotal(),
    sets: joinCoverageSetRows(getCachedScryfallSets(), getCachedOracleTotalBySet(), null),
  };
}

export async function fetchCoverageMeta(): Promise<CoverageMeta> {
  ensureOracleTotalRefresh();
  ensureScryfallSetsRefresh();

  try {
    const res = await fetch(`${apiUpstream()}/health/live`);
    if (!res.ok) return unavailableCoverageMeta();

    const parsed = parseLiveStatus(await res.json());
    if (!parsed) return unavailableCoverageMeta();

    return {
      faithfulCount: parsed.faithfulCount,
      oracleTotal: getCachedOracleTotal(),
      sets: joinCoverageSetRows(getCachedScryfallSets(), getCachedOracleTotalBySet(), parsed.faithfulBySet),
    };
  } catch {
    return unavailableCoverageMeta();
  }
}
