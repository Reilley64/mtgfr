import { apiUpstream, parseLiveStatus } from "./api-upstream-auth";
import { ensureOracleTotalRefresh, getCachedOracleTotal } from "./scryfall-oracle-total";
import { ensureSetOracleTotalsRefresh, getCachedSetOracleTotals } from "./scryfall-set-oracle-totals";
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
  setOracleTotals: Readonly<Record<string, number>> | null,
  faithfulBySet: Readonly<Record<string, number>> | null,
): CoverageSetRow[] {
  if (sets == null || sets.length === 0) return [];

  return sets.map((set) => ({
    code: set.code,
    name: set.name,
    releasedAt: set.releasedAt,
    faithful: faithfulBySet?.[set.code] ?? 0,
    oracleTotal: setOracleTotals?.[set.code] ?? null,
  }));
}

function unavailableCoverageMeta(): CoverageMeta {
  return {
    faithfulCount: null,
    oracleTotal: getCachedOracleTotal(),
    sets: joinCoverageSetRows(getCachedScryfallSets(), getCachedSetOracleTotals(), null),
  };
}

export async function fetchCoverageMeta(): Promise<CoverageMeta> {
  ensureOracleTotalRefresh();
  ensureSetOracleTotalsRefresh();
  ensureScryfallSetsRefresh();

  try {
    const res = await fetch(`${apiUpstream()}/health/live`);
    if (!res.ok) return unavailableCoverageMeta();

    const parsed = parseLiveStatus(await res.json());
    if (!parsed) return unavailableCoverageMeta();

    return {
      faithfulCount: parsed.faithfulCount,
      oracleTotal: getCachedOracleTotal(),
      sets: joinCoverageSetRows(getCachedScryfallSets(), getCachedSetOracleTotals(), parsed.faithfulBySet),
    };
  } catch {
    return unavailableCoverageMeta();
  }
}
