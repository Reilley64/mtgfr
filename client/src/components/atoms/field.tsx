import { type JSX, splitProps } from "solid-js";
import { cn } from "~/lib/cn";

type Props = JSX.InputHTMLAttributes<HTMLInputElement>;

/** Glass + vine input from DESIGN.md §5. */
export function Field(props: Props) {
  const [local, rest] = splitProps(props, ["class"]);
  return (
    <input
      {...rest}
      class={cn("rounded-control border border-vine bg-glass px-md py-sm text-body text-snow", local.class)}
    />
  );
}
