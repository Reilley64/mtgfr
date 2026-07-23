// Per-seat mana tray: world-anchored DOM chips (mana-font) outside the seat band.

import { type Html, html } from "foldkit/html";
import type { ManaTrayChip } from "~/manaPips";
import type { VisibleState } from "~/wire/types";
import { type ManaTraySeat, projectManaTrays } from "../geometry/manaTrayProject";
import type { Message } from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();

/** True when the mana-font class is already a number pip (`ms-2`) — count is the glyph. */
function isNumericPip(ms: string): boolean {
  return /^\d+$/.test(ms) || ms === "100" || ms === "1000000" || ms === "1-2";
}

function countInside(amount: number, light = false): Html {
  return h.span(
    [h.Class(`ms-tray-count-num${light ? " ms-tray-count-light" : ""}`), h.Attribute("aria-hidden", "true")],
    [String(amount)],
  );
}

function chipLabel(chip: ManaTrayChip): string {
  const base =
    chip.kind === "glyph"
      ? `{${chip.code}}`
      : chip.kind === "any"
        ? "any color"
        : chip.kind === "ci"
          ? chip.code
          : chip.text;
  return chip.amount > 1 ? `${chip.amount}×${base}` : base;
}

function chipView(chip: ManaTrayChip, zoom: number): Html {
  const fontPx = Math.max(1, Math.round(14 * zoom));
  const label = chipLabel(chip);
  const wrap = (inner: Html): Html =>
    h.span(
      [
        h.Class("inline-flex items-center"),
        h.Style({ "font-size": `${fontPx}px` }),
        h.Role("img"),
        h.Attribute("aria-label", label),
      ],
      [inner],
    );

  switch (chip.kind) {
    case "glyph": {
      const numbered = isNumericPip(chip.ms);
      const countIn = chip.amount > 1 && !numbered;
      return wrap(
        h.i(
          [
            h.Class(`relative ms ms-cost ms-${chip.ms}${countIn ? " ms-tray-count" : ""}`),
            h.Attribute("aria-hidden", "true"),
          ],
          countIn ? [countInside(chip.amount)] : [],
        ),
      );
    }
    case "any":
      return wrap(
        h.i(
          [h.Class("relative ms ms-duo ms-duo-color ms-multicolor ms-grad"), h.Attribute("aria-hidden", "true")],
          chip.amount > 1 ? [countInside(chip.amount, true)] : [],
        ),
      );
    case "ci":
      return wrap(
        h.i(
          [h.Class(`relative ms ms-ci ms-ci-${chip.n} ms-ci-${chip.suffix}`), h.Attribute("aria-hidden", "true")],
          chip.amount > 1 ? [countInside(chip.amount, true)] : [],
        ),
      );
    case "text":
      return h.span(
        [
          h.Class("inline-flex items-center gap-px font-semibold text-seat-forest"),
          h.Style({ "font-size": `${fontPx}px` }),
        ],
        [
          h.span([h.Class("leading-none")], [chip.text]),
          chip.amount > 1 ? h.span([h.Class("leading-none")], [String(chip.amount)]) : null,
        ].filter((v): v is Html => v !== null),
      );
    default: {
      const _exhaustive: never = chip;
      return _exhaustive;
    }
  }
}

function seatView(tray: ManaTraySeat): Html {
  return h.div(
    [
      h.Class("absolute top-(--y) left-(--x) flex -translate-x-1/2 -translate-y-1/2 items-center gap-1"),
      h.Style({ "--x": `${tray.x}px`, "--y": `${tray.y}px` }),
      h.DataAttribute("mana-tray-seat", String(tray.seat)),
    ],
    tray.chips.map((c) => chipView(c, tray.zoom)),
  );
}

export function manaTrayView(board: BoardModel, state: VisibleState): Html | null {
  const trays = projectManaTrays(state.players, state.viewer, state.players.length, board.camera);
  if (trays.length === 0) return null;

  return h.div(
    // Layer 2: composed in view.ts between vector canvas and bitmap (under permanents).
    [h.DataAttribute("testid", "mana-tray"), h.Class("pointer-events-none fixed inset-0")],
    trays.map(seatView),
  );
}
