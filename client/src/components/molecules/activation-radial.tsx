// Activation radial around a selected permanent: continuous SVG donut of legal options.

import { createSignal, For } from "solid-js";
import { Button } from "~/components/atoms";
import {
  activationRadialInnerRadius,
  activationRadialOuterRadius,
  type RadialOption,
  type RadialPress,
  radialOptionKey,
  radialPressDown,
  radialPressUp,
  radialWedgeAtPoint,
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
      {/* biome-ignore lint/a11y/useSemanticElements: SVG donut must stay svg; group keeps wedge buttons in the a11y tree (img would not) */}
      <svg
        role="group"
        aria-label="Activation options"
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
                  {/* biome-ignore lint/a11y/useSemanticElements: SVG wedge hit targets must be paths, not HTML buttons */}
                  <path
                    d={d()}
                    tabindex={0}
                    role="button"
                    aria-label={opt.label}
                    // Explicit SVG paints — Tailwind fill/stroke utilities often miss <path>.
                    fill-rule="evenodd"
                    style={{
                      // Opaque band that reads on forest felt (8-digit hex fills wash out).
                      fill: active() ? "#276B3C" : "#15241c",
                      stroke: "#FFD76A",
                      strokeWidth: active() ? 2.5 : 2,
                      strokeOpacity: 1,
                    }}
                    class="cursor-pointer outline-none focus-visible:stroke-[3px]"
                    onPointerDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
                      setPress(radialPressDown(press(), i()));
                    }}
                    onPointerUp={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      applyUp(radialWedgeAtPoint(e.clientX, e.clientY, document.elementFromPoint.bind(document)));
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
                    class="pointer-events-none fill-snow font-semibold text-[11px]"
                    style={{ fill: "#EEFFFF" }}
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
