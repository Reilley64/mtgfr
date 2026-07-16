import { type JSX, type ParentProps, splitProps } from "solid-js";
import { Dynamic } from "solid-js/web";
import { cn } from "~/lib/cn";

type DivProps = ParentProps & JSX.HTMLAttributes<HTMLDivElement>;

type ShellProps = ParentProps &
  JSX.HTMLAttributes<HTMLElement> & {
    as?: "div" | "main";
  };

/** Auth/lobby card surface — use `as="main"` when it is the page landmark. */
export function Panel(props: ShellProps) {
  const [local, rest] = splitProps(props, ["as", "class", "children"]);
  return (
    <Dynamic
      component={local.as ?? "div"}
      data-ui="panel"
      {...rest}
      class={cn(
        "flex w-full min-w-0 max-w-[min(100%-2rem,420px)] flex-col gap-lg rounded-panel border border-vine",
        "bg-forest-surface p-xxl text-snow shadow-table",
        local.class,
      )}
    >
      {local.children}
    </Dynamic>
  );
}

/** Modal surface from DESIGN.md §5. */
export function Modal(props: DivProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <div
      {...rest}
      class={cn(
        "rounded-modal border border-vine bg-forest-surface p-xl text-body text-snow shadow-table",
        local.class,
      )}
    >
      {local.children}
    </div>
  );
}

/** Turn track, log, hint strip. */
export function Hud(props: DivProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <div {...rest} class={cn("rounded-hud bg-forest-hud p-md text-label text-seafoam leading-normal", local.class)}>
      {local.children}
    </div>
  );
}

/** Deck list / catalog row fill. */
export function ListRow(props: DivProps) {
  const [local, rest] = splitProps(props, ["class", "children"]);
  return (
    <div {...rest} class={cn("border border-vine-dim bg-glass-dim text-snow hover:bg-white/8", local.class)}>
      {local.children}
    </div>
  );
}

/** Page shell — The One Felt Rule. Forest Floor only. */
export function Felt(props: ShellProps) {
  const [local, rest] = splitProps(props, ["as", "class", "children"]);
  return (
    <Dynamic
      component={local.as ?? "div"}
      {...rest}
      class={cn("bg-forest-floor font-sans text-body text-snow", local.class)}
    >
      {local.children}
    </Dynamic>
  );
}
