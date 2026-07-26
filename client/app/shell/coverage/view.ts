import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import { type AppChromeMeta, appVersionBadge, formatFaithfulPercent } from "../../domain/ui/app-version";
import { buttonClass } from "../../domain/ui/buttonClass";
import { feltClass, fieldClass, listRowClass } from "../../domain/ui/surfaces";
import type { ClosedAccountMenu, GotAuthMessage, ToggledAccountMenu } from "../../messages";
import { HomeRoute, routePath } from "../../routes";
import { accountChrome } from "../account-chrome/view";
import {
  ChangedCoverageQuery,
  type Message as CoverageMessage,
  RequestedCoverageRefresh,
} from "./messages";
import type { CoverageSetRow, CoverageStatus, CoverageSubmodel } from "./submodel";

export type ViewMessage =
  | CoverageMessage
  | typeof ClosedAccountMenu.Type
  | typeof GotAuthMessage.Type
  | typeof ToggledAccountMenu.Type;

export type ViewInputs = {
  username: string;
  meGravatarHash: string | null;
  chrome: AppChromeMeta;
};

const h = html<ViewMessage>();

function statusCopy(status: CoverageStatus): string | null {
  switch (status) {
    case "idle":
      return "Coverage has not loaded yet.";
    case "loading":
      return "Loading coverage...";
    case "ready":
      return null;
    case "error":
      return null;
    default: {
      const exhaustive: never = status;
      return exhaustive;
    }
  }
}

function percentValue(row: CoverageSetRow): number {
  if (row.oracleTotal == null || !(row.oracleTotal > 0)) return Number.NEGATIVE_INFINITY;
  return row.faithful / row.oracleTotal;
}

export function coveragePercentText(faithfulCount: number | null, oracleTotal: number | null): string {
  if (faithfulCount == null || oracleTotal == null) return "—";
  return formatFaithfulPercent(faithfulCount, oracleTotal) ?? "—";
}

export function visibleCoverageRows(model: Pick<CoverageSubmodel, "query" | "sets">): CoverageSetRow[] {
  const query = model.query.trim().toLowerCase();
  const filtered =
    query === ""
      ? model.sets
      : model.sets.filter((row) => {
          return row.code.toLowerCase().includes(query) || row.name.toLowerCase().includes(query);
        });

  return [...filtered].sort((left, right) => {
    const percentDelta = percentValue(right) - percentValue(left);
    if (percentDelta !== 0) return percentDelta;

    const nameOrder = left.name.localeCompare(right.name);
    if (nameOrder !== 0) return nameOrder;

    return left.code.localeCompare(right.code);
  });
}

function tableRow(row: CoverageSetRow): Html {
  return h.div(
    [
      h.Class(listRowClass("grid grid-cols-[minmax(0,1.75fr)_96px_96px_80px] items-center gap-md px-md py-sm")),
      h.DataAttribute("testid", `coverage-row-${row.code}`),
    ],
    [
      h.div(
        [h.Class("flex min-w-0 flex-col gap-2xs")],
        [
          h.span([h.Class("text-label text-lichen uppercase")], [row.code]),
          h.span([h.Class("truncate text-body text-snow")], [row.name]),
        ],
      ),
      h.span([h.Class("text-right text-body text-snow")], [String(row.faithful)]),
      h.span([h.Class("text-right text-body text-snow")], [row.oracleTotal == null ? "—" : String(row.oracleTotal)]),
      h.span(
        [h.Class("text-right text-game text-priority-gold")],
        [coveragePercentText(row.faithful, row.oracleTotal)],
      ),
    ],
  );
}

export const view = Submodel.defineView<CoverageSubmodel, ViewMessage, ViewInputs>((model, viewInputs): Html => {
  const { chrome, meGravatarHash, username } = viewInputs;
  const status = statusCopy(model.status);
  const rows = visibleCoverageRows(model);
  const globalPercent = coveragePercentText(model.faithfulCount, model.oracleTotal);
  const emptyCopy = model.query.trim() === "" ? "No set coverage available." : "No sets match.";

  return h.main(
    [
      h.Class(
        feltClass(
          "h-full overflow-y-auto p-xxl pt-[max(1.5rem,env(safe-area-inset-top))] pr-[max(1.5rem,env(safe-area-inset-right))] pb-[max(1.5rem,env(safe-area-inset-bottom))] pl-[max(1.5rem,env(safe-area-inset-left))]",
        ),
      ),
      h.DataAttribute("testid", "coverage-page"),
    ],
    [
      h.div(
        [h.Class("mx-auto mb-5 flex max-w-[960px] flex-wrap items-center justify-between gap-md")],
        [
          h.div(
            [h.Class("flex min-w-0 flex-col gap-xs")],
            [
              h.h1([h.Class("m-0 text-title")], ["Coverage"]),
              h.div(
                [h.Class("text-label text-lichen"), h.DataAttribute("testid", "coverage-global-percent")],
                [`${globalPercent} faithful`],
              ),
            ],
          ),
          h.div(
            [h.Class("flex flex-wrap items-center gap-md")],
            [
              h.a([h.Href(routePath(HomeRoute())), h.Class(buttonClass("ghost"))], ["Play"]),
              accountChrome(h, {
                username,
                gravatarHash: meGravatarHash,
                menuOpen: model.accountMenuOpen,
                showLeaderboardLink: true,
              }),
            ],
          ),
        ],
      ),
      h.section(
        [h.Class("mx-auto flex max-w-[960px] flex-col gap-sm")],
        [
          model.error == null
            ? null
            : h.div([h.Role("alert"), h.Class("text-label text-reconnect-rust")], [model.error]),
          status == null ? null : h.div([h.Class("text-label text-lichen")], [status]),
          model.status !== "loading"
            ? h.input([
                h.Type("search"),
                h.DataAttribute("testid", "coverage-search"),
                h.AriaLabel("Search sets"),
                h.Placeholder("Search sets…"),
                h.Value(model.query),
                h.OnInput((query) => ChangedCoverageQuery({ query })),
                h.Class(fieldClass("mb-sm w-full max-w-[420px]")),
              ])
            : null,
          model.status === "ready" && rows.length > 0
            ? h.div(
                [h.Class("flex flex-col gap-xs"), h.DataAttribute("testid", "coverage-table")],
                [
                  h.div(
                    [h.Class("grid grid-cols-[minmax(0,1.75fr)_96px_96px_80px] gap-md px-md text-label text-lichen")],
                    [
                      h.span([], ["Set"]),
                      h.span([h.Class("text-right")], ["Faithful"]),
                      h.span([h.Class("text-right")], ["Scryfall"]),
                      h.span([h.Class("text-right")], ["%"]),
                    ],
                  ),
                  ...rows.map(tableRow),
                ],
              )
            : null,
          model.status === "ready" && rows.length === 0
            ? h.div([h.Class("text-label text-lichen")], [emptyCopy])
            : null,
          model.status === "error"
            ? h.button(
                [
                  h.Type("button"),
                  h.OnClick(RequestedCoverageRefresh()),
                  h.Class(buttonClass("ghost", "mt-md self-start")),
                ],
                ["Try again"],
              )
            : null,
        ],
      ),
      appVersionBadge(h, chrome),
    ],
  );
});
