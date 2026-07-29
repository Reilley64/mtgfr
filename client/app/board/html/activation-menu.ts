import { Option } from "effect";
import { type Html, html } from "foldkit/html";
import { costPipPlate, costPips } from "~/costPips";
import { manaFontClass } from "~/oracleText";
import type { VisibleState } from "~/wire/types";
import { layout, ZONE } from "../geometry/layout";
import {
  ACTIVATION_MENU_WIDTH_PX,
  activationCostChip,
  activationMenuEstimatedHeight,
  activationMenuPlacement,
  type RadialOption,
  radialOptionKey,
  radialOptions,
  radialScreenCenter,
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

function costPipView(ms: string, code: string, sizePx: number): Html {
  return h.span(
    [
      h.Class("inline-flex shrink-0 items-center justify-center rounded-full shadow-[0_1px_2px_rgb(0_0_0/0.9)]"),
      h.Style({
        width: `${sizePx}px`,
        height: `${sizePx}px`,
        "background-color": costPipPlate(code),
        color: "#111",
        "font-size": `${Math.round(sizePx * 0.82)}px`,
      }),
    ],
    [h.i([h.Class(`ms ms-${ms}`)], [])],
  );
}

function costChipView(opt: RadialOption): Html | null {
  const chip = activationCostChip(opt);
  if (chip == null) return null;

  const tap = (() => {
    const ms = manaFontClass("T");
    if (ms == null) return null;
    return h.i([h.Class(`ms ms-cost ms-${ms}`)], []);
  })();

  const mana = "cost" in chip ? costPips(chip.cost) : [];
  return h.span(
    [
      h.DataAttribute("testid", "activation-menu-cost"),
      h.Attribute("aria-hidden", "true"),
      h.Class("ml-auto inline-flex shrink-0 items-center gap-1 text-[14px] text-snow"),
    ],
    [
      chip.kind === "tap" || chip.kind === "tap_and_mana" ? tap : null,
      ...mana.map((pip) => costPipView(pip.ms, pip.code, 14)),
    ].filter((child): child is Html => child !== null),
  );
}

// Row chrome is attribute-driven: JS sets data-active (hover/armed) and aria-disabled,
// Tailwind variants own the look — no class ternaries.
function rowClass(): string {
  return [
    "pointer-events-auto flex w-full items-center gap-sm rounded-hud border border-vine/40 bg-glass/40 px-sm py-xs text-left outline-none transition-colors duration-100 ease-state cursor-pointer",
    "aria-disabled:cursor-not-allowed aria-disabled:opacity-60",
    "data-[active=true]:border-priority-gold data-[active=true]:bg-llanowar-deep",
  ].join(" ");
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

export function activationMenuView(board: BoardModel, state: VisibleState): Html | null {
  const id = board.selectedId;
  if (id == null) return null;
  const obj = state.objects.find((object) => object.id === id);
  if (obj == null || obj.zone !== ZONE.Battlefield) return null;

  const options = selectedRadialOptions(board, state);
  if (options.length === 0) return null;

  const cards = layout(state, state.viewer);
  const card =
    cards.find((renderCard) => renderCard.id === id) ??
    cards.find((renderCard) => renderCard.clusterMembers.includes(id));
  if (card == null) return null;

  const center = radialScreenCenter(board.camera, card);
  const zoom = board.camera.zoom;
  const placement = activationMenuPlacement(
    center,
    { w: card.w * zoom, h: card.h * zoom },
    { width: ACTIVATION_MENU_WIDTH_PX, height: activationMenuEstimatedHeight(options.length) },
    board.viewport,
  );
  const armed = board.radialPress.armed;
  const hover = board.radialHover;

  return h.div(
    [h.Class("pointer-events-none fixed inset-0 z-30"), h.DataAttribute("testid", "activation-menu")],
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
      h.div(
        [
          h.DataAttribute("testid", "activation-menu-panel"),
          h.Role("group"),
          h.AriaLabel("Activation options"),
          h.Class(
            "pointer-events-auto absolute z-[31] flex max-h-full flex-col overflow-y-auto rounded-hud border border-vine/50 bg-forest-hud p-sm text-chip text-snow shadow-hud",
          ),
          h.Style(placement),
        ],
        options.map((opt, index) => {
          const active = !opt.disabled && (hover === index || armed === index);
          return h.button(
            [
              h.Type("button"),
              h.AriaLabel(opt.label),
              h.DataAttribute("testid", `activation-menu-row-${radialOptionKey(opt)}`),
              h.DataAttribute("wedge", String(index)),
              h.Role("button"),
              h.Tabindex(0),
              h.Attribute("aria-disabled", opt.disabled ? "true" : "false"),
              h.DataAttribute("active", String(active)),
              h.Class(rowClass()),
              h.OnPointerDown((_pt, _button, _sx, _sy, _ts, _cx, _cy) => Option.some(RadialWedgeArmed({ index }))),
              h.OnPointerUp((_sx, _sy, _pt, _ts) => Option.some(RadialWedgeReleased({ index }))),
              h.OnMouseEnter(RadialWedgeHovered({ index })),
              h.OnMouseLeave(RadialWedgeHovered({ index: null })),
              h.OnKeyDownPreventDefault((key) => {
                if (key !== "Enter" && key !== " ") return Option.none();
                return Option.some(RadialOptionPicked({ index }));
              }),
            ],
            [h.span([h.Class("min-w-0 flex-1 line-clamp-2 text-left")], [opt.label]), costChipView(opt)].filter(
              (child): child is Html => child !== null,
            ),
          );
        }),
      ),
    ],
  );
}
