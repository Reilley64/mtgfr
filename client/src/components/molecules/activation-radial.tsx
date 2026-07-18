// Activation radial around a selected permanent: pie of legal options.

import { For } from "solid-js";
import { Button } from "~/components/atoms";
import { cn } from "~/lib/cn";
import { activationRadialRadius, type RadialOption } from "~/lib/radial";
import type { ActionView } from "~/wire/types";

export default function ActivationRadial(props: {
  x: number;
  y: number;
  /** Camera zoom — ring radius tracks the on-screen card size. */
  zoom: number;
  options: RadialOption[];
  onPick: (opt: RadialOption) => void;
  onDismiss: () => void;
  /** Hovered action option — Board paints its `auto_tap` preview (not synthetic tap-for-mana). */
  onHoverAction?: (action: ActionView | null) => void;
}) {
  const n = () => props.options.length;
  const r = () => activationRadialRadius(props.zoom);
  const pos = (i: number) => {
    const count = n();
    const ang = -Math.PI / 2 + (i * 2 * Math.PI) / count;
    const radius = r();
    return { left: props.x + Math.cos(ang) * radius, top: props.y + Math.sin(ang) * radius };
  };
  return (
    <div class="pointer-events-none fixed inset-0 z-30">
      <Button
        type="button"
        aria-label="Close"
        variant="ghost"
        class="pointer-events-auto absolute inset-0 cursor-default rounded-none border-0 bg-transparent hover:bg-transparent"
        onClick={() => props.onDismiss()}
      />
      <For each={props.options}>
        {(opt, i) => {
          const p = () => pos(i());
          return (
            <Button
              type="button"
              variant="game-quiet"
              style={{ "--x": `${p().left}px`, "--y": `${p().top}px` }}
              class={cn(
                "pointer-events-auto absolute top-(--y) left-(--x) z-[31] max-w-[140px] -translate-x-1/2 -translate-y-1/2",
                "min-h-11 border border-priority-gold/70 bg-forest-hud px-sm py-sm font-semibold text-caption text-snow shadow-hud",
                "hover:border-priority-gold hover:bg-llanowar-deep",
              )}
              onMouseEnter={() => props.onHoverAction?.(opt.kind === "action" ? opt.action : null)}
              onMouseLeave={() => props.onHoverAction?.(null)}
              onClick={(e) => {
                e.stopPropagation();
                props.onHoverAction?.(null);
                props.onPick(opt);
              }}
            >
              {opt.label}
            </Button>
          );
        }}
      </For>
    </div>
  );
}
