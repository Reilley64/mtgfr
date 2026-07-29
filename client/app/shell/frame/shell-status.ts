// Shared shell status chrome: the idle/loading copy line, the error alert, and the Try again
// ghost, used by every data-backed list surface (coverage, leaderboard). Bake layout-neutral
// shrink-0 in — block columns ignore it and contained flex columns (coverage) require it.

import { Schema as S } from "effect";
import type { html as createHtml, Html } from "foldkit/html";
import { button } from "../../domain/ui/button";
import { alertClass } from "../../domain/ui/surfaces";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

export const ShellStatus = S.Union([S.Literal("idle"), S.Literal("loading"), S.Literal("ready"), S.Literal("error")]);
export type ShellStatus = typeof ShellStatus.Type;

export function shellStatusCopy(noun: string, status: ShellStatus): string | null {
  switch (status) {
    case "idle":
      return `${noun} has not loaded yet.`;
    case "loading":
      return `Loading ${noun.toLowerCase()}...`;
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

/** [error alert, status line, try-again button] — each null when not applicable. */
export function shellStatusChrome<Msg>(
  h: HtmlFactory<Msg>,
  opts: {
    noun: string;
    status: ShellStatus;
    error: string | null;
    retry: { testId: string; onClick: Msg };
  },
): ReadonlyArray<Html | null> {
  const copy = shellStatusCopy(opts.noun, opts.status);
  return [
    opts.error == null ? null : h.div([h.Role("alert"), h.Class(alertClass("shrink-0"))], [opts.error]),
    copy == null ? null : h.div([h.Class("shrink-0 text-label text-lichen")], [copy]),
    opts.status === "error"
      ? button(
          h,
          { testId: opts.retry.testId, onClick: opts.retry.onClick, variant: "ghost", class: "mt-md self-start" },
          ["Try again"],
        )
      : null,
  ];
}
