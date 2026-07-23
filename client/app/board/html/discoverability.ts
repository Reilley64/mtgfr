// Discoverability: hint strip + legend panel (Solid `board-discoverability.tsx`).

import { type Html, html } from "foldkit/html";
import { cn } from "~/cn";
import { combatCoachFromState } from "~/combatCoach";
import { isActivePlayer } from "~/spectator";
import { buttonClass } from "~/ui/buttonClass";
import type { VisibleState } from "~/wire/types";
import { EXILE_OUTLINE, GRAVEYARD_OUTLINE, PLAYABLE_BORDER } from "../chrome";
import { HintDismissed, LegendToggled, type Message } from "../messages";
import type { BoardModel } from "../submodel";
import { HAND_BAR_H } from "./hand";

const h = html<Message>();

export const HINT_DISMISSED_KEY = "mtgfr.hintDismissed";

const LEGEND_ITEMS: ReadonlyArray<{ color: string; shape: "dot" | "badge" | "outline"; label: string }> = [
  { color: "#e8b24a", shape: "badge", label: "Summoning sick" },
  { color: "#7a3b13", shape: "dot", label: "Goaded" },
  { color: "#0c1412", shape: "dot", label: "Keyword / ability (Mana font)" },
  { color: "#55cc99", shape: "badge", label: "Prepared (P)" },
  { color: "#e9b84a", shape: "outline", label: "Commander" },
  { color: PLAYABLE_BORDER, shape: "outline", label: "Playable action" },
  { color: GRAVEYARD_OUTLINE, shape: "outline", label: "Graveyard halo (with playable)" },
  { color: EXILE_OUTLINE, shape: "outline", label: "Exile halo (with playable)" },
  { color: "#2f7d46", shape: "badge", label: "+1/+1 counters" },
  { color: "#8f2f2f", shape: "badge", label: "Marked damage" },
  { color: "#f4efe2", shape: "badge", label: "Power / toughness / loyalty" },
  { color: "#FF5555", shape: "outline", label: "Attacking" },
  { color: "#66FF99", shape: "outline", label: "Blocking" },
];

export function readHintDismissed(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(HINT_DISMISSED_KEY) === "1";
}

export function persistHintDismissed(): void {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(HINT_DISMISSED_KEY, "1");
  }
}

function legendSwatch(shape: "dot" | "badge" | "outline"): string {
  const base = "inline-block h-[14px] w-[14px] shrink-0";
  if (shape === "dot") return cn(base, "rounded-full border border-morph-slate bg-(--c)");
  if (shape === "badge") return cn(base, "rounded-focus border border-morph-slate bg-(--c)");
  return cn(base, "rounded-focus border-(--c) border-2");
}

function seatedPlayer(state: VisibleState): boolean {
  return isActivePlayer(state.players, state.viewer);
}

function hintVisible(board: BoardModel): boolean {
  return !board.hintDismissed && !board.hintAutoHidden;
}

function hintStripView(): Html {
  return h.div(
    [
      h.DataAttribute("testid", "board-hint"),
      h.Class(
        "pointer-events-auto flex max-w-[min(420px,46vw)] items-center gap-md rounded-hud bg-forest-hud p-md text-chip text-lichen shadow-hud",
      ),
    ],
    [
      h.span([], ["Drag to play · Click to activate · Alt inspect · Space pass"]),
      h.button(
        [
          h.Type("button"),
          h.Attribute("aria-label", "Dismiss hint"),
          h.OnClick(HintDismissed()),
          h.Class(buttonClass("ghost", "min-w-0 border-none p-0 text-lichen")),
        ],
        ["✕"],
      ),
    ],
  );
}

function combatCoachView(text: string): Html {
  return h.div(
    [
      h.DataAttribute("testid", "board-combat-coach"),
      h.Class(
        "pointer-events-none flex max-w-[min(480px,52vw)] items-center gap-md rounded-hud border border-mountain-red/40 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [h.span([], [text])],
  );
}

function legendPanelView(): Html {
  return h.div(
    [
      h.DataAttribute("testid", "board-legend"),
      h.Class("pointer-events-auto fixed top-12 left-md z-21 w-[240px] rounded-hud bg-forest-hud p-md shadow-hud"),
    ],
    [
      h.div(
        [h.Class("mb-sm flex items-center justify-between gap-sm")],
        [
          h.span([h.Class("font-bold text-label text-seafoam")], ["Board legend"]),
          h.button(
            [
              h.Type("button"),
              h.Attribute("aria-label", "Close legend"),
              h.OnClick(LegendToggled()),
              h.Class(buttonClass("ghost", "min-w-0 border-none p-0 text-lichen")),
            ],
            ["✕"],
          ),
        ],
      ),
      h.div(
        [h.Class("flex flex-col gap-xs")],
        LEGEND_ITEMS.map((item) =>
          h.div(
            [h.Class("flex items-center gap-sm")],
            [
              h.span([h.Style({ "--c": item.color }), h.Class(legendSwatch(item.shape))], []),
              h.span([h.Class("text-caption text-mist")], [item.label]),
            ],
          ),
        ),
      ),
    ],
  );
}

function legendToggleButton(expanded: boolean): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", "board-legend-toggle"),
      h.Attribute("aria-label", "Board legend"),
      h.Attribute("aria-expanded", expanded ? "true" : "false"),
      h.OnClick(LegendToggled()),
      h.Class(buttonClass("ghost", "pointer-events-auto px-md py-xs")),
    ],
    ["?"],
  );
}

/** Hint strip, combat staging coach, legend toggle, and legend panel for seated active players. */
export function discoverabilityView(board: BoardModel, state: VisibleState): Html | null {
  if (!seatedPlayer(state)) return null;

  const showHint = hintVisible(board);
  const showLegend = board.legendOpen;
  const coach = combatCoachFromState(state, {
    attackersConfirmed: board.attackersConfirmed,
    blockersConfirmed: board.blockersConfirmed,
  });

  if (!showHint && !showLegend && coach == null) {
    return legendToggleButton(false);
  }

  const bottomStrips: Html[] = [];
  if (coach != null) bottomStrips.push(combatCoachView(coach));
  if (showHint) bottomStrips.push(hintStripView());

  return h.div(
    [],
    [
      h.div([h.Class("pointer-events-none flex items-center gap-xs")], [legendToggleButton(showLegend)]),
      showLegend ? legendPanelView() : null,
      bottomStrips.length > 0
        ? h.div(
            [
              h.Style({ "--b": `${HAND_BAR_H + 10}px` }),
              h.Class(
                "pointer-events-none fixed bottom-(--b) left-md z-20 flex max-w-[min(480px,52vw)] flex-col items-start gap-sm",
              ),
            ],
            bottomStrips,
          )
        : null,
    ].filter((v): v is Html => v !== null),
  );
}
