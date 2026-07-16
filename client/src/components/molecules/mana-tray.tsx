// Per-seat mana tray: world-anchored DOM chips (mana-font) outside the seat band.

import { For, type JSX, Show } from "solid-js";
import { cn } from "~/lib/cn";
import type { ManaTrayChip } from "~/lib/manaPips";
import type { ManaTraySeat } from "~/lib/manaTrayProject";

export type { ManaTraySeat };

/** True when the mana-font class is already a number pip (`ms-2`) — count is the glyph. */
function isNumericPip(ms: string): boolean {
  return /^\d+$/.test(ms) || ms === "100" || ms === "1000000" || ms === "1-2";
}

function CountInside(props: { amount: number /** Light number on dark/multicolor faces. */; light?: boolean }) {
  return (
    <span class={cn("ms-tray-count-num", props.light && "ms-tray-count-light")} aria-hidden="true">
      {props.amount}
    </span>
  );
}

function Chip(props: { chip: ManaTrayChip; zoom: number }) {
  const fontPx = () => Math.max(1, Math.round(14 * props.zoom));
  const amount = () => props.chip.amount;
  const label = (base: string) => (amount() > 1 ? `${amount()}×${base}` : base);

  const wrap = (inner: JSX.Element, baseLabel: string) => (
    <span
      class="inline-flex items-center"
      style={{ "font-size": `${fontPx()}px` }}
      role="img"
      aria-label={label(baseLabel)}
    >
      {inner}
    </span>
  );

  switch (props.chip.kind) {
    case "glyph": {
      const numbered = isNumericPip(props.chip.ms);
      const countIn = amount() > 1 && !numbered;
      return wrap(
        <i
          class={cn("relative", "ms", "ms-cost", `ms-${props.chip.ms}`, countIn && "ms-tray-count")}
          aria-hidden="true"
        >
          <Show when={countIn}>
            <CountInside amount={amount()} />
          </Show>
        </i>,
        `{${props.chip.code}}`,
      );
    }
    case "any":
      return wrap(
        <i class={cn("relative", "ms", "ms-duo", "ms-duo-color", "ms-multicolor", "ms-grad")} aria-hidden="true">
          <Show when={amount() > 1}>
            <CountInside amount={amount()} light />
          </Show>
        </i>,
        "any color",
      );
    case "ci":
      return wrap(
        <i
          class={cn("relative", "ms", "ms-ci", `ms-ci-${props.chip.n}`, `ms-ci-${props.chip.suffix}`)}
          aria-hidden="true"
        >
          <Show when={amount() > 1}>
            <CountInside amount={amount()} light />
          </Show>
        </i>,
        props.chip.code,
      );
    case "text":
      return (
        <span
          class="inline-flex items-center gap-px font-semibold text-[#7fd4a8]"
          style={{ "font-size": `${fontPx()}px` }}
        >
          <span class="leading-none">{props.chip.text}</span>
          <Show when={amount() > 1}>
            <span class="leading-none">{amount()}</span>
          </Show>
        </span>
      );
    default: {
      const _exhaustive: never = props.chip;
      return _exhaustive;
    }
  }
}

export default function ManaTray(props: { trays: ManaTraySeat[] }) {
  return (
    <Show when={props.trays.length > 0}>
      <div class="pointer-events-none fixed inset-0 z-[18]">
        <For each={props.trays}>
          {(t) => (
            <div
              style={{ "--x": `${t.x}px`, "--y": `${t.y}px` }}
              class="absolute top-(--y) left-(--x) flex -translate-x-1/2 -translate-y-1/2 items-center gap-1"
            >
              <For each={t.chips}>{(chip) => <Chip chip={chip} zoom={t.zoom} />}</For>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}
