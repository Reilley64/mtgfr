import { apiUpstream, parseLiveStatus } from "./api-upstream-auth";
import { loadOracleTotal } from "./scryfall-oracle-total";
import { loadSetOracleTotals } from "./scryfall-set-oracle-totals";
import { loadScryfallSets, type ScryfallSetRow } from "./scryfall-sets";

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

export async function fetchCoverageMeta(): Promise<CoverageMeta> {
  // Coverage is useless with a cold default_cards cache (every Scryfall cell is "—").
  // Await cold fills; warm paths return immediately and refresh in the background.
  const [oracleTotal, setOracleTotals, sets] = await Promise.all([
    loadOracleTotal(),
    loadSetOracleTotals(),
    loadScryfallSets(),
  ]);

  try {
    const res = await fetch(`${apiUpstream()}/health/live`);
    if (!res.ok) {
      return {
        faithfulCount: null,
        oracleTotal,
        sets: joinCoverageSetRows(sets, setOracleTotals, null),
      };
    }

    const parsed = parseLiveStatus(await res.json());
    if (!parsed) {
      return {
        faithfulCount: null,
        oracleTotal,
        sets: joinCoverageSetRows(sets, setOracleTotals, null),
      };
    }

    return {
      faithfulCount: parsed.faithfulCount,
      oracleTotal,
      sets: joinCoverageSetRows(sets, setOracleTotals, parsed.faithfulBySet),
    };
  } catch {
    return {
      faithfulCount: null,
      oracleTotal,
      sets: joinCoverageSetRows(sets, setOracleTotals, null),
    };
  }
}
