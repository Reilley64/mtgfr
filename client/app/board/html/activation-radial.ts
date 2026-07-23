// Activation radial around a selected permanent: continuous SVG donut of legal options.
//
// Port of Solid `activation-radial.tsx` (#73): single-option rings use an evenodd double-circle
// path (`wedgePath` when count ≤ 1) plus opaque SVG fills — a full-circle A command collapses
// and Tailwind fill utilities often miss `<path>`.

import { Option } from "effect";
import { type Html, html } from "foldkit/html";
import type { VisibleState } from "~/wire/types";
import { layout, ZONE } from "../geometry/layout";
import {
  activationRadialInnerRadius,
  activationRadialOuterRadius,
  type RadialOption,
  radialOptionKey,
  radialOptions,
  radialOverlayPlacement,
  radialScreenCenter,
  wedgeLabelPoint,
  wedgePath,
} from "../geometry/radial";
import {
  type Message,
  RadialOptionPicked,
  RadialWedgeArmed,
  RadialWedgeHovered,
  RadialWedgeReleased,
} from "../messages";
import type { BoardModel } from "../submodel";

const h = html<Message>();

function truncateLabel(label: string): string {
  return label.length > 18 ? `${label.slice(0, 16)}…` : label;
}

export function selectedRadialOptions(board: BoardModel, state: VisibleState): RadialOption[] {
  const id = board.selectedId;
  if (id == null) return [];
  const card = layout(state, state.viewer).find((c) => c.id === id);
  if (card == null) return [];
  return radialOptions(
    id,
    state.actions,
    card.tapsForMana,
    card.tapped,
    state.can_act,
    card.summoningSick,
    card.hasHaste,
  );
}

export function activationRadialView(board: BoardModel, state: VisibleState): Html | null {
  const id = board.selectedId;
  if (id == null) return null;
  const obj = state.objects.find((o) => o.id === id);
  if (obj == null || obj.zone !== ZONE.Battlefield) return null;

  const cards = layout(state, state.viewer);
  const card = cards.find((c) => c.id === id) ?? cards.find((c) => c.clusterMembers.includes(id));
  if (card == null) return null;

  const options = radialOptions(
    id,
    state.actions,
    card.tapsForMana,
    card.tapped,
    state.can_act,
    card.summoningSick,
    card.hasHaste,
  );
  if (options.length === 0) return null;

  const center = radialScreenCenter(board.camera, card);
  const zoom = board.camera.zoom;
  const inner = activationRadialInnerRadius(zoom);
  const outer = activationRadialOuterRadius(zoom);
  const size = outer * 2 + 8;
  const origin = size / 2;
  const placement = radialOverlayPlacement(center, size, board.viewport);
  const n = options.length;
  const armed = board.radialPress.armed;
  const hover = board.radialHover;

  const wedges = options.map((opt, i) => {
    const d = wedgePath(i, n, inner, outer);
    const label = wedgeLabelPoint(i, n, inner, outer);
    const active = !opt.disabled && (hover === i || armed === i);
    return h.g(
      [h.DataAttribute("wedge", String(i)), h.DataAttribute("testid", `radial-wedge-${radialOptionKey(opt)}`)],
      [
        h.path(
          [
            h.D(d),
            h.Tabindex(0),
            h.Role("button"),
            h.AriaLabel(opt.label),
            h.Attribute("aria-disabled", opt.disabled ? "true" : "false"),
            h.FillRule("evenodd"),
            h.Fill(opt.disabled ? "#26302a" : active ? "#276B3C" : "#15241c"),
            h.Stroke(opt.disabled ? "#7a6a3a" : "#FFD76A"),
            h.StrokeWidth(active ? "2.5" : "2"),
            h.StrokeOpacity(opt.disabled ? "0.55" : "1"),
            h.Class(opt.disabled ? "cursor-not-allowed opacity-60 outline-none" : "cursor-pointer outline-none"),
            h.OnPointerDown((_pt, _button, _sx, _sy, _ts, _cx, _cy) => Option.some(RadialWedgeArmed({ index: i }))),
            h.OnPointerUp((_sx, _sy, _pt, _ts) => Option.some(RadialWedgeReleased({ index: i }))),
            h.OnMouseEnter(RadialWedgeHovered({ index: i })),
            h.OnMouseLeave(RadialWedgeHovered({ index: null })),
            h.OnKeyDownPreventDefault((key) => {
              if (key !== "Enter" && key !== " ") return Option.none();
              return Option.some(RadialOptionPicked({ index: i }));
            }),
          ],
          [],
        ),
        h.text(
          [
            h.X(String(label.x)),
            h.Y(String(label.y)),
            h.TextAnchor("middle"),
            h.DominantBaseline("middle"),
            h.Class("pointer-events-none font-semibold"),
            h.Fill(opt.disabled ? "#9aa39d" : "#EEFFFF"),
            h.FontSize("11px"),
          ],
          [truncateLabel(opt.label)],
        ),
      ],
    );
  });

  return h.div(
    [h.Class("pointer-events-none fixed inset-0 z-30"), h.DataAttribute("testid", "activation-radial")],
    [
      h.button(
        [
          h.Type("button"),
          h.AriaLabel("Close"),
          h.Class(
            "pointer-events-auto absolute inset-0 cursor-default rounded-none border-0 bg-transparent hover:bg-transparent",
          ),
          h.OnPointerUp((_sx, _sy, _pt, _ts) => Option.some(RadialWedgeReleased({ index: null }))),
        ],
        [],
      ),
      h.svg(
        [
          h.Role("group"),
          h.AriaLabel("Activation options"),
          h.Class("pointer-events-none absolute z-[31]"),
          h.Attribute("viewBox", `0 0 ${size} ${size}`),
          h.Style(placement),
        ],
        [h.g([h.Transform(`translate(${origin}, ${origin})`), h.Class("pointer-events-auto")], wedges)],
      ),
    ],
  );
}
