import { type JSX, splitProps } from "solid-js";
import { cva, type VariantProps } from "~/components/atoms/cva";
import { cn } from "~/lib/cn";

/** DESIGN.md §5 button vocabulary — variants live here, not in CSS `@apply`. */
const button = cva({
  base: "cursor-pointer",
  variants: {
    variant: {
      primary: "rounded-control border-none bg-llanowar px-lg py-sm text-button text-snow-mint disabled:opacity-50",
      ghost: "rounded-control border border-vine bg-transparent px-lg py-sm text-button text-mist disabled:opacity-50",
      // Ghost chrome + destructive ink (two-step confirm).
      danger:
        "rounded-control border border-burn-red bg-transparent px-lg py-sm text-button text-burn-red disabled:opacity-50",
      // Inline-sentence action that is not navigation.
      link: "border-none bg-transparent p-0 font-[inherit] text-vine underline",
      game: [
        "min-w-[132px] rounded-game border-none bg-llanowar-deep px-[26px] py-[11px]",
        "text-game text-snow-mint shadow-press",
        "transition-[background_0.15s_ease,transform_0.06s_ease,box-shadow_0.15s_ease]",
        "hover:enabled:bg-llanowar",
        "active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active",
        "disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
      ],
      // HUD-scale tactility on the game chrome.
      "game-quiet": [
        "min-w-0 rounded-game border-none bg-tapped-out px-lg py-[7px]",
        "text-label text-mist shadow-press",
        "transition-[background_0.15s_ease,transform_0.06s_ease,box-shadow_0.15s_ease]",
        "hover:enabled:bg-quiet-hover",
        "active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active",
        "disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
      ],
      // Yielded pass — amber earth, not priority gold (The Gold Means Act Rule).
      "game-yielded": [
        "min-w-0 rounded-game border-none bg-yielded px-lg py-[7px]",
        "text-label text-yielded-ink shadow-press",
        "transition-[background_0.15s_ease,transform_0.06s_ease,box-shadow_0.15s_ease]",
        "hover:enabled:bg-yielded-hover",
        "active:enabled:translate-y-px active:enabled:scale-[0.99] active:enabled:shadow-press-active",
        "disabled:bg-tapped-out disabled:text-tapped-ink disabled:shadow-none",
      ],
    },
    hitQuiet: {
      true: "hit-quiet",
      false: null,
    },
  },
  defaultVariants: {
    variant: "primary",
    hitQuiet: false,
  },
});

export type ButtonVariant = NonNullable<VariantProps<typeof button>["variant"]>;

type Props = JSX.ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof button> & {
    /** Skip the coarse-pointer 44×44 floor (DESIGN.md: quiet HUD dismiss). */
    hitQuiet?: boolean;
  };

/** Screen / game / link buttons from DESIGN.md §5. */
export function Button(props: Props) {
  const [local, rest] = splitProps(props, ["variant", "hitQuiet", "class", "children"]);
  return (
    <button
      {...rest}
      data-ui={local.variant === "link" ? "link" : undefined}
      class={button({
        variant: local.variant,
        hitQuiet: local.hitQuiet,
        class: local.class,
      })}
    >
      {local.children}
    </button>
  );
}

/** Exposed for unit tests — prefer `<Button>` at call sites. */
export function buttonClass(variant?: ButtonVariant, ...extra: Array<string | false | null | undefined>): string {
  return button({ variant, class: cn(...extra) });
}
