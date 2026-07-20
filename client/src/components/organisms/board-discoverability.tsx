// Discoverability: hint strip + legend panel (findings: undiscoverable interaction grammar).

import { For } from "solid-js";
import { Button, Hud } from "~/components/atoms";
import { HAND_BAR_H } from "~/components/molecules/hand";
import { RESPONSE_COLOR } from "~/lib/boardDraw";

export function HintStrip(props: { onDismiss: () => void }) {
  return (
    <Hud
      style={{ "--b": `${HAND_BAR_H + 16}px` }}
      // Left of center so it clears the viewer's life orb (centered under their band).
      // Lichen, not prose ink: this is metadata about the interface (DESIGN.md §6).
      class="fixed bottom-(--b) left-3 z-20 flex max-w-[min(420px,46vw)] items-center gap-md text-lichen"
    >
      <span>Drag to play · Click to activate · Alt inspect · Space pass</span>
      <Button
        type="button"
        aria-label="Dismiss hint"
        onClick={props.onDismiss}
        variant="ghost"
        hitQuiet
        class="border-none p-0 text-lichen"
      >
        ✕
      </Button>
    </Hud>
  );
}

// One canonical entry per badge/dot/outline the canvas draws, in the same colors drawCard/dot/badge
// use (ATTACK_STROKE/BLOCK_STROKE below, and the dot()/badge() calls in drawCard) — kept as data so
// this list is easy to audit against the canvas, though it's a literal copy, not a shared reference
// (those consts live further down the file, after this component).
const LEGEND_ITEMS: { color: string; shape: "dot" | "badge" | "outline"; label: string }[] = [
  { color: "#e8b24a", shape: "badge", label: "Summoning sick" },
  { color: "#7a3b13", shape: "dot", label: "Goaded" },
  { color: "#0c1412", shape: "dot", label: "Keyword / ability (Mana font)" },
  { color: "#55cc99", shape: "badge", label: "Prepared (P)" },
  { color: "#e9b84a", shape: "dot", label: "Commander" },
  { color: "#2f7d46", shape: "badge", label: "+1/+1 counters" },
  { color: "#8f2f2f", shape: "badge", label: "Marked damage" },
  { color: "#f4efe2", shape: "badge", label: "Power / toughness / loyalty" },
  { color: "#FF5555", shape: "outline", label: "Attacking" },
  { color: "#66FF99", shape: "outline", label: "Blocking" },
  { color: "rgba(0,0,0,0.45)", shape: "badge", label: "Dimmed — not usable at instant speed" },
  { color: RESPONSE_COLOR, shape: "badge", label: "Bright — usable at instant speed" },
];

export function LegendPanel(props: { onClose: () => void }) {
  return (
    <Hud style={{ "--b": `${HAND_BAR_H + 92}px` }} class="fixed right-[10px] bottom-(--b) z-21 w-[220px]">
      <div class="mb-1.5 flex items-center justify-between">
        <span class="font-bold">Board legend</span>
        <Button
          type="button"
          aria-label="Close legend"
          onClick={props.onClose}
          variant="ghost"
          hitQuiet
          class="border-none p-0 text-lichen"
        >
          ✕
        </Button>
      </div>
      <For each={LEGEND_ITEMS}>
        {(item) => (
          <div class="my-1 flex items-center gap-sm">
            {/* The swatch's colour is canvas paint, so it arrives as data (a CSS variable) and the
                classes read it — the same colours drawCard() uses, not a second encoding of them. */}
            <span style={{ "--c": item.color }} class={legendSwatch(item.shape)} />
            <span>{item.label}</span>
          </div>
        )}
      </For>
    </Hud>
  );
}
function legendSwatch(shape: "dot" | "badge" | "outline") {
  const base = "inline-block h-[14px] w-[14px] shrink-0";
  if (shape === "dot") return `${base} rounded-full border border-morph-slate bg-(--c)`;
  if (shape === "badge") return `${base} rounded-focus border border-morph-slate bg-(--c)`;
  return `${base} rounded-focus border-2 border-(--c)`;
}
