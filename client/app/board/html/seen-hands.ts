// Chips for the opponent hands this viewer has looked at (Glasses of Urza — CR 701.20). The
// server itemizes those cards to the looker alone, and every other read of an opponent's hand on
// this board is `hand_count`, so without this strip a look leaves nothing behind but a log line.

import type { Html, HtmlBuilder } from "foldkit/html";
import { playerLabel } from "~/players";
import { button } from "~/ui/button";
import type { VisibleState } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import { type Message, PileExpanded } from "../messages";

/** Seats other than the viewer whose hand cards this snapshot itemized, and how many arrived. */
export function seenHands(state: VisibleState): Array<{ owner: number; count: number }> {
  const counts = new Map<number, number>();
  for (const object of state.objects) {
    if (Number(object.zone) !== ZONE.Hand || Number(object.owner) === Number(state.viewer)) continue;
    const owner = Number(object.owner);
    counts.set(owner, (counts.get(owner) ?? 0) + 1);
  }
  return [...counts.entries()].sort(([left], [right]) => left - right).map(([owner, count]) => ({ owner, count }));
}

/** One chip per looked-at hand, opening it in the pile overlay. Absent when nothing was seen. */
export function seenHandsView(state: VisibleState, h: HtmlBuilder<Message>): Html | null {
  const seen = seenHands(state);
  if (seen.length === 0) return null;
  return h.div(
    [h.DataAttribute("testid", "seen-hands"), h.Class("pointer-events-auto flex items-center gap-xs")],
    seen.map(({ owner, count }) =>
      button(
        h,
        {
          testId: `seen-hand-${owner}`,
          onClick: PileExpanded({ zone: ZONE.Hand, owner }),
          variant: "game-quiet",
        },
        [`${playerLabel(state.players, owner)}'s hand (${count})`],
      ),
    ),
  );
}
