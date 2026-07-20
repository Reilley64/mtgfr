// Discoverability: hint strip + legend panel (findings: undiscoverable interaction grammar).

import { For } from "solid-js";
import { Button, Hud } from "~/components/atoms";
import { RESPONSE_COLOR } from "~/lib/boardDraw";

/** Coaching strip — parent positions it in the left chrome column above the log. */
export function HintStrip(props: { onDismiss: () => void }) {
  return (
    <Hud
      data-testid="board-hint"
      // Lichen, not prose ink: this is metadata about the interface (DESIGN.md §6).
      class="flex max-w-[min(420px,46vw)] items-center gap-md text-chip text-lichen"
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
  // Top-left, under the '?' toggle — keeps the bottom-right free for Pass / Next / yield.
  return (
    <Hud class="fixed top-12 left-md z-21 w-[240px]">
      <div class="mb-sm flex items-center justify-between gap-sm">
        <span class="font-bold text-label text-seafoam">Board legend</span>
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
      <div class="flex flex-col gap-xs">
        <For each={LEGEND_ITEMS}>
          {(item) => (
            <div class="flex items-center gap-sm">
              {/* The swatch's colour is canvas paint, so it arrives as data (a CSS variable) and the
                  classes read it — the same colours drawCard() uses, not a second encoding of them. */}
              <span style={{ "--c": item.color }} class={legendSwatch(item.shape)} />
              <span class="text-caption text-mist">{item.label}</span>
            </div>
          )}
        </For>
      </div>
    </Hud>
  );
}
function legendSwatch(shape: "dot" | "badge" | "outline") {
  const base = "inline-block h-[14px] w-[14px] shrink-0";
  if (shape === "dot") return `${base} rounded-full border border-morph-slate bg-(--c)`;
  if (shape === "badge") return `${base} rounded-focus border border-morph-slate bg-(--c)`;
  return `${base} rounded-focus border-2 border-(--c)`;
}
