import type { html as createHtml, Html } from "foldkit/html";
import { cn } from "../cn";
import { appVersionClass } from "./surfaces";

export type AppChromeMeta = {
  version: string | null;
  faithfulCount: number | null;
  oracleTotal: number | null;
  coverageHref: string | null;
};

export function formatFaithfulPercent(faithfulCount: number, oracleTotal: number): string | null {
  if (!(oracleTotal > 0) || !Number.isFinite(faithfulCount) || !Number.isFinite(oracleTotal)) {
    return null;
  }
  const pct = (100 * faithfulCount) / oracleTotal;
  if (pct < 10) return `${pct.toFixed(1)}%`;
  return `${Math.round(pct)}%`;
}

/** Fixed bottom-left API badge — hidden until `version` is known (Solid AppVersion parity). */
export function appVersionBadge<M>(
  h: ReturnType<typeof createHtml<M>>,
  meta: AppChromeMeta,
): Html | null {
  if (meta.version == null) return null;
  const pct =
    meta.faithfulCount != null && meta.oracleTotal != null
      ? formatFaithfulPercent(meta.faithfulCount, meta.oracleTotal)
      : null;
  const coverage =
    pct == null
      ? null
      : meta.coverageHref == null
        ? h.div([h.DataAttribute("testid", "pool-coverage")], [`${pct} faithful`])
        : h.a(
            [
              h.Href(meta.coverageHref),
              h.DataAttribute("testid", "pool-coverage"),
              h.Class("pointer-events-auto underline-offset-2 hover:underline"),
            ],
            [`${pct} faithful`],
          );
  return h.div(
    [h.Class(cn(appVersionClass(), "flex flex-col gap-0"))],
    [
      coverage,
      h.div([h.DataAttribute("testid", "app-version")], [`API ${meta.version}`]),
    ],
  );
}
