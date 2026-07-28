import type * as Menu from "@foldkit/ui/menu";
import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import { type AppChromeMeta, formatFaithfulPercent } from "../../domain/ui/app-version";
import { button } from "../../domain/ui/button";
import { input } from "../../domain/ui/input";
import { alertClass, listRowClass } from "../../domain/ui/surfaces";
import { GotAccountMenuMessage, type GotAuthMessage } from "../../messages";
import { HomeRoute, routePath } from "../../routes";
import { accountChrome } from "../account-chrome/view";
import { shellFrame } from "../frame/shell-frame";
import { ChangedCoverageQuery, type Message as CoverageMessage, RequestedCoverageRefresh } from "./messages";
import type { CoverageSetRow, CoverageStatus, CoverageSubmodel } from "./submodel";

export type ViewMessage = CoverageMessage | typeof GotAccountMenuMessage.Type | typeof GotAuthMessage.Type;

export type ViewInputs = {
  username: string;
  meGravatarHash: string | null;
  chrome: AppChromeMeta;
  accountMenu: Menu.Model;
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

function compareReleasedAtDescending(left: CoverageSetRow, right: CoverageSetRow): number {
  if (left.releasedAt == null && right.releasedAt == null) return 0;
  if (left.releasedAt == null) return 1;
  if (right.releasedAt == null) return -1;
  return right.releasedAt.localeCompare(left.releasedAt);
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
    const releasedAtOrder = compareReleasedAtDescending(left, right);
    if (releasedAtOrder !== 0) return releasedAtOrder;

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
        [h.Class("text-right font-display text-game text-vine")],
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

  return shellFrame(h, {
    atmosphere: "shell",
    title: "Coverage",
    subtitle: h.p(
      [h.Class("m-0 text-label text-lichen"), h.DataAttribute("testid", "coverage-global-percent")],
      [`${globalPercent} faithful`],
    ),
    chrome,
    lockStageScroll: true,
    leading: button(h, { as: "a", href: routePath(HomeRoute()), variant: "ghost" }, ["Play"]),
    trailing: accountChrome(h, {
      username,
      gravatarHash: meGravatarHash,
      menu: viewInputs.accountMenu,
      toMenuMessage: (message) => GotAccountMenuMessage({ message }),
      showLeaderboardLink: true,
    }),
    stage: h.div(
      [
        // Fill the contained shell stage so only the table body scrolls (not the page).
        h.Class("flex h-full min-h-0 flex-1 flex-col overflow-hidden"),
        h.DataAttribute("testid", "coverage-page"),
      ],
      [
        h.section(
          [h.Class("mx-auto flex min-h-0 w-full max-w-[960px] flex-1 flex-col gap-sm")],
          [
            model.error == null ? null : h.div([h.Role("alert"), h.Class(alertClass("shrink-0"))], [model.error]),
            status == null ? null : h.div([h.Class("shrink-0 text-label text-lichen")], [status]),
            model.status !== "loading"
              ? input(h, {
                  id: "coverage-search",
                  type: "search",
                  testId: "coverage-search",
                  ariaLabel: "Search sets",
                  placeholder: "Search sets…",
                  value: model.query,
                  onInput: (query) => ChangedCoverageQuery({ query }),
                  class: "mb-sm w-full max-w-[420px] shrink-0",
                })
              : null,
            model.status === "ready" && rows.length > 0
              ? h.div(
                  [h.Class("flex min-h-0 flex-1 flex-col gap-xs"), h.DataAttribute("testid", "coverage-table")],
                  [
                    h.div(
                      [
                        h.Class(
                          "grid shrink-0 grid-cols-[minmax(0,1.75fr)_96px_96px_80px] gap-md px-md text-label text-lichen",
                        ),
                      ],
                      [
                        h.span([], ["Set"]),
                        h.span([h.Class("text-right")], ["Faithful"]),
                        h.span([h.Class("text-right")], ["Scryfall"]),
                        h.span([h.Class("text-right")], ["%"]),
                      ],
                    ),
                    h.div(
                      [
                        h.Class("flex min-h-0 flex-1 flex-col gap-xs overflow-y-auto overscroll-contain"),
                        h.DataAttribute("testid", "coverage-table-body"),
                      ],
                      rows.map(tableRow),
                    ),
                  ],
                )
              : null,
            model.status === "ready" && rows.length === 0
              ? h.div([h.Class("text-label text-lichen"), h.DataAttribute("testid", "coverage-empty")], [emptyCopy])
              : null,
            model.status === "error"
              ? button(
                  h,
                  {
                    testId: "coverage-try-again",
                    onClick: RequestedCoverageRefresh(),
                    variant: "ghost",
                    class: "mt-md self-start",
                  },
                  ["Try again"],
                )
              : null,
          ],
        ),
      ],
    ),
  });
});
