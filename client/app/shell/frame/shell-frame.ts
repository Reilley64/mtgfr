import type { Html, HtmlBuilder } from "foldkit/html";
import { cn } from "../../domain/cn";
import { type AppChromeMeta, appVersionBadge } from "../../domain/ui/app-version";

export type ShellAtmosphere = "auth" | "shell";

export type ShellFrameOptions = {
  atmosphere: ShellAtmosphere;
  title?: string;
  /** Plain caption or rich Html (e.g. coverage global % with a test id). */
  subtitle?: string | Html | null;
  leading?: Html | null;
  trailing?: Html | null;
  stage: Html | ReadonlyArray<Html | null | undefined | false>;
  chrome: AppChromeMeta;
  /**
   * When true, the stage does not scroll — children (builder catalog, coverage table)
   * own overflow. Default false so list/lobby pages can scroll in the stage.
   */
  lockStageScroll?: boolean;
  /** Root test id; default `shell-frame`. */
  testId?: string;
};

export function shellFrame<Msg>(h: HtmlBuilder<Msg>, options: ShellFrameOptions): Html {
  const atmosphereClass = options.atmosphere === "auth" ? "shell-atmosphere-auth" : "shell-atmosphere-shell";
  const stageChildren = Array.isArray(options.stage) ? options.stage : [options.stage];
  const title = options.title?.trim();
  const hasTitle = title != null && title !== "";

  return h.main(
    [
      h.DataAttribute("testid", options.testId ?? "shell-frame"),
      // Contained flex column: stage owns page scroll (or inner hosts like the builder catalog).
      // overflow-y-auto on the root made h-dvh builder/coverage pages taller than the viewport.
      h.Class(cn("fixed inset-0 flex flex-col overflow-hidden font-shell text-body text-snow", atmosphereClass)),
    ],
    [
      h.header(
        [
          h.DataAttribute("testid", "shell-header"),
          h.Class(
            cn(
              "mx-auto flex w-full max-w-[var(--size-shell-stage-max)] shrink-0 items-center gap-md",
              "px-[var(--spacing-shell-gutter)] py-[var(--spacing-shell-header-y)]",
            ),
          ),
        ],
        [
          h.div(
            [h.DataAttribute("testid", "shell-header-leading"), h.Class("flex min-w-0 flex-1 items-center gap-sm")],
            [options.leading ?? null],
          ),
          h.div(
            [
              h.DataAttribute("testid", "shell-header-title"),
              h.Class("flex min-w-0 flex-col items-center text-center"),
            ],
            [
              hasTitle ? h.h1([h.Class("m-0 font-display text-display tracking-display")], [title]) : null,
              typeof options.subtitle === "string"
                ? h.p([h.Class("m-0 text-label text-lichen")], [options.subtitle])
                : (options.subtitle ?? null),
            ],
          ),
          h.div(
            [
              h.DataAttribute("testid", "shell-header-trailing"),
              h.Class("flex min-w-0 flex-1 items-center justify-end gap-sm"),
            ],
            [options.trailing ?? null],
          ),
        ],
      ),
      h.div(
        [
          h.DataAttribute("testid", "shell-stage"),
          h.Class(
            cn(
              "shell-stage-enter mx-auto flex min-h-0 w-full max-w-[var(--size-shell-stage-max)] flex-1 flex-col",
              "px-[var(--spacing-shell-gutter)] pb-[var(--spacing-shell-gutter)]",
              options.lockStageScroll ? "overflow-hidden" : "overflow-y-auto",
            ),
          ),
        ],
        stageChildren,
      ),
      appVersionBadge(h, options.chrome),
    ],
  );
}
