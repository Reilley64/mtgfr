// Activation radial around a selected permanent: continuous SVG donut of legal options.

import { For, createSignal } from "solid-js";
import { Button } from "~/components/atoms";
import { cn } from "~/lib/cn";
import {
  activationRadialInnerRadius,
  activationRadialOuterRadius,
  radialOptionKey,
  radialPressDown,
  radialPressUp,
  type RadialOption,
  type RadialPress,
  wedgeLabelPoint,
  wedgePath,
} from "~/lib/radial";
import type { ActionView } from "~/wire/types";

export default function ActivationRadial(props: {
  x: number;
  y: number;
  zoom: number;
  options: RadialOption[];
  onPick: (opt: RadialOption) => void;
  onDismiss: () => void;
  onHoverAction?: (action: ActionView | null) => void;
}) {
  const [press, setPress] = createSignal<RadialPress>({ armed: null });
  const [hover, setHover] = createSignal<number | null>(null);

  const n = () => props.options.length;
  const inner = () => activationRadialInnerRadius(props.zoom);
  const outer = () => activationRadialOuterRadius(props.zoom);
  const size = () => outer() * 2 + 8;
  const origin = () => size() / 2;

  /** Resolve wedge index from an element (`data-wedge` on the path's `<g>`). */
  const wedgeAttr = (el: EventTarget | null): number | null => {
    if (!(el instanceof Element)) return null;
    const node = el.closest("[data-wedge]");
    if (!node) return null;
    const v = node.getAttribute("data-wedge");
    if (v == null) return null;
    const i = Number(v);
    return Number.isFinite(i) ? i : null;
  };

  /** Wedge under the pointer at release — not `e.target`, which follows capture. */
  const wedgeAtPoint = (clientX: number, clientY: number): number | null =>
    wedgeAttr(document.elementFromPoint(clientX, clientY));

  const applyUp = (wedge: number | null) => {
    const result = radialPressUp(press(), wedge);
    setPress(result.state);
    if (result.dismiss) {
      props.onHoverAction?.(null);
      props.onDismiss();
      return;
    }
    if (result.commit != null) {
      const opt = props.options[result.commit];
      if (!opt) return;
      props.onHoverAction?.(null);
      props.onPick(opt);
    }
  };

  return (
    <div class="pointer-events-none fixed inset-0 z-30">
      <Button
        type="button"
        aria-label="Close"
        variant="ghost"
        hitQuiet
        class="pointer-events-auto absolute inset-0 cursor-default rounded-none border-0 bg-transparent hover:bg-transparent"
        onPointerUp={(e) => {
          e.preventDefault();
          applyUp(null);
        }}
      />
      <svg
        class="pointer-events-none absolute z-[31]"
        width={size()}
        height={size()}
        style={{
          left: `${props.x}px`,
          top: `${props.y}px`,
          transform: "translate(-50%, -50%)",
        }}
      >
        <g transform={`translate(${origin()}, ${origin()})`} class="pointer-events-auto">
          <For each={props.options}>
            {(opt, i) => {
              const d = () => wedgePath(i(), n(), inner(), outer());
              const label = () => wedgeLabelPoint(i(), n(), inner(), outer());
              const active = () => hover() === i() || press().armed === i();
              return (
                <g data-wedge={i()} data-testid={`radial-wedge-${radialOptionKey(opt)}`}>
                  <path
                    d={d()}
                    tabindex={0}
                    role="button"
                    aria-label={opt.label}
                    class={cn(
                      "cursor-pointer outline-none",
                      active()
                        ? "fill-llanowar-deep stroke-priority-gold stroke-2"
                        : "fill-forest-hud stroke-priority-gold/70 stroke-1",
                      "focus-visible:stroke-priority-gold focus-visible:stroke-2",
                    )}
                    onPointerDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
                      setPress(radialPressDown(press(), i()));
                    }}
                    onPointerUp={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      applyUp(wedgeAtPoint(e.clientX, e.clientY));
                    }}
                    onPointerEnter={() => {
                      setHover(i());
                      props.onHoverAction?.(opt.kind === "action" ? opt.action : null);
                    }}
                    onPointerLeave={() => {
                      setHover((h) => (h === i() ? null : h));
                      props.onHoverAction?.(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key !== "Enter" && e.key !== " ") return;
                      e.preventDefault();
                      props.onHoverAction?.(null);
                      props.onPick(opt);
                    }}
                  />
                  <text
                    x={label().x}
                    y={label().y}
                    text-anchor="middle"
                    dominant-baseline="middle"
                    class="pointer-events-none fill-snow text-[11px] font-semibold"
                  >
                    {opt.label.length > 18 ? `${opt.label.slice(0, 16)}…` : opt.label}
                  </text>
                </g>
              );
            }}
          </For>
        </g>
      </svg>
    </div>
  );
}
