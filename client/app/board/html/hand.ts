// Bottom action bar: command / hand / graveyard / exile as Arena peek tiles.
//
// Geometry matches Solid `hand.tsx`: each flex slot stays peek-wide × visible-tall; the face is
// `top-0 right-0` so the excess hangs *below* the viewport (screen clips). Raise is paint-only
// (`translateY`); the hit strip is bottom-anchored and grows upward on hover. Fan tilt + cast-cost
// pips live on the face column. Buried cards hit on the left peek only; the section's rightmost
// card uses the full face (`handBarHitWidth`).

import { Option } from "effect";
import { type Attribute, type Html, html } from "foldkit/html";
import { type CostPip, costPipPlate, costPips } from "~/costPips";
import { cardArt } from "~/ui/card-art";
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import { HAND_BAR_PEEK, handBarHitHeight, handBarHitWidth, handBarRaiseTranslateY } from "../geometry/handBarHit";
import { ZONE } from "../geometry/layout";
import { HandActionActivated, InspectAuxHovered, type Message } from "../messages";
import { HAND_FACE_W } from "../motion/flights";
import type { HandDragState } from "../submodel";
import { barZoneAura, byObject, bySection, handExtras } from "./actions";
import { MountHandBarDrag } from "./hand-drag-mount";

const h = html<Message>();

export const HAND_CARD_W = HAND_FACE_W;
export const HAND_CARD_PEEK = HAND_BAR_PEEK;
export const HAND_CARD_OVERLAP = HAND_CARD_W - HAND_CARD_PEEK;
export const HAND_CARD_H = Math.round(HAND_CARD_W / 0.716);
export const HAND_VISIBLE_H = 130;
/** Room above each face for cast-cost pips (reserved band outside the card). */
const HAND_PIP_ROW_H = 20;
/** Height of the bottom action bar — tuck + pip row + padding (Solid HAND_BAR_H). */
export const HAND_BAR_H = HAND_VISIBLE_H + HAND_PIP_ROW_H + 12;

const emptyCost = (): WireCost => ({ generic: 0, colored: [0, 0, 0, 0, 0] });

/** MTGA fan: left/right tilt out; centre rises toward the board (edges sit lower). */
function fanTransform(index: number, count: number): string {
  const off = index - (count - 1) / 2;
  const angle = Math.max(-10, Math.min(10, off * 2.5));
  const rise = Math.max(0, 14 - off * off * 1.2);
  return `rotate(${angle}deg) translateY(${-rise}px)`;
}

function actionCaption(kind: string): string | undefined {
  if (kind === "cycle") return "Cycle";
  if (kind === "suspend") return "Suspend";
  if (kind === "activate_hand_ability") return "Discard";
  return undefined;
}

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

function tile(args: {
  name: string;
  print: string;
  cardId?: string;
  zone: "hand" | "command" | "graveyard" | "exile";
  objectId?: number;
  objectKind?: string;
  manaCost: WireCost;
  action: ActionView | null;
  dimmed: boolean;
  caption?: string;
  index: number;
  count: number;
}): Html {
  const { name, print, cardId, zone, objectId, objectKind, manaCost, action, dimmed, caption, index, count } = args;
  const playable = action != null && !dimmed;
  const testId = objectId != null ? `hand-card-${objectId}` : undefined;
  const hitW = handBarHitWidth(index, count, HAND_CARD_PEEK, HAND_CARD_W);
  const restHitH = handBarHitHeight(false, HAND_VISIBLE_H, HAND_CARD_H);
  const raisedHitH = handBarHitHeight(true, HAND_VISIBLE_H, HAND_CARD_H);
  const raiseY = handBarRaiseTranslateY(true, HAND_VISIBLE_H, HAND_CARD_H);
  const pips = costPips(manaCost, { showZero: objectKind != null && objectKind !== "land" });
  const faceClass = [
    "pointer-events-none absolute top-0 right-0 transition-transform duration-[120ms] ease-state",
    "group-hover/hand-tile:z-30 group-hover/hand-tile:[transform:translateY(var(--raise-y))]",
  ].join(" ");

  const artClass = [
    "pointer-events-none block touch-none rounded-game object-cover shadow-hand transition-[filter] duration-[80ms] ease-state",
    barZoneAura(zone),
    dimmed ? "brightness-[0.55]" : "",
    playable ? "group-hover/hand-tile:brightness-110" : "",
  ]
    .filter((v) => v !== "")
    .join(" ");

  const hitClass = [
    "pointer-events-auto absolute bottom-0 group-hover/hand-tile:[height:var(--hit-raised-h)]",
    playable ? "cursor-grab" : "cursor-default",
  ].join(" ");

  const hitAttrs: Attribute<Message>[] = [
    h.Class(hitClass),
    h.Style({
      width: `${hitW}px`,
      height: `${restHitH}px`,
      right: `${HAND_CARD_W - hitW}px`,
      "--hit-raised-h": `${raisedHitH}px`,
    }),
  ];
  if (playable) {
    const ariaBase = caption ? `${name}: ${caption}` : name;
    hitAttrs.push(h.Attribute("aria-label", zone === "hand" ? ariaBase : `${ariaBase} (${zone})`));
  }
  if (testId) hitAttrs.push(h.DataAttribute("testid", testId));
  hitAttrs.push(h.DataAttribute("bar-zone", zone));
  if (objectKind) hitAttrs.push(h.DataAttribute("object-kind", objectKind));
  // Alt-inspect aux hover (Solid `onHoverCard`) — every face-up bar tile, playable or not.
  hitAttrs.push(
    h.OnMouseEnter(
      InspectAuxHovered({
        source: "hand",
        card: {
          name,
          ...(cardId ? { cardId } : {}),
          ...(print ? { print } : {}),
        },
      }),
    ),
  );
  hitAttrs.push(h.OnMouseLeave(InspectAuxHovered({ source: "hand", card: null })));
  if (playable && action != null) {
    hitAttrs.push(h.DataAttribute("action-id", String(action.id)));
    hitAttrs.push(h.DataAttribute("action-payload", JSON.stringify(action)));
    hitAttrs.push(h.DataAttribute("card-name", name));
    hitAttrs.push(h.DataAttribute("card-print", print));
    hitAttrs.push(h.DataAttribute("mana-cost", JSON.stringify(manaCost)));
    hitAttrs.push(h.DataAttribute("action-kind", action.kind));
    hitAttrs.push(h.DataAttribute("needs-target", action.needs_target ? "1" : "0"));
  }
  if (playable && action != null) {
    hitAttrs.push(h.Role("button"));
    hitAttrs.push(h.Tabindex(0));
    hitAttrs.push(
      h.OnKeyDownPreventDefault((key) => {
        if (key !== "Enter" && key !== " ") return Option.none();
        return Option.some(HandActionActivated({ action }));
      }),
    );
  }

  const pipRow =
    pips.length > 0
      ? h.div(
          [
            h.DataAttribute("testid", "hand-cost-pips"),
            h.Class("absolute right-0 left-0 z-20 flex items-end justify-end gap-px pb-0.5"),
            h.Style({ top: `-${HAND_PIP_ROW_H}px`, height: `${HAND_PIP_ROW_H}px` }),
            h.Attribute("aria-hidden", "true"),
          ],
          pips.map((pip: CostPip) => costPipView(pip.ms, pip.code, 14)),
        )
      : null;

  const cardBoxStyle = {
    width: `${HAND_CARD_W}px`,
    height: `${HAND_CARD_H}px`,
  };

  const art: Html = print
    ? cardArt(h, {
        print,
        alt: name,
        className: artClass,
        style: cardBoxStyle,
      })
    : h.div(
        [
          h.Class(
            `flex items-center justify-center rounded-game bg-forest-shadow p-1 text-center text-caption text-snow shadow-hand ${barZoneAura(zone)} ${dimmed ? "brightness-[0.55]" : ""}`,
          ),
          h.Style(cardBoxStyle),
        ],
        [h.div([h.Class("overflow-hidden text-ellipsis whitespace-nowrap font-semibold")], [name])],
      );

  return h.div(
    [
      h.Class("group/hand-tile pointer-events-none relative shrink-0 origin-bottom overflow-visible"),
      h.Style({
        width: `${HAND_CARD_PEEK}px`,
        height: `${HAND_VISIBLE_H}px`,
        transform: fanTransform(index, count),
        "--raise-y": `${raiseY}px`,
        "z-index": String(index + 1),
      }),
      h.DataAttribute("hand-index", String(index)),
    ],
    [
      h.div(
        [h.Class(faceClass), h.Style({ width: `${HAND_CARD_W}px` })],
        [
          pipRow,
          h.div(
            [h.Class("relative origin-bottom rounded-game"), h.Style(cardBoxStyle)],
            [
              art,
              caption
                ? h.div(
                    [
                      h.Class(
                        "pointer-events-none absolute right-0 bottom-2 left-0 mx-1.5 overflow-hidden text-ellipsis whitespace-nowrap rounded-control bg-forest-hud px-1 py-0.5 text-center font-semibold text-micro text-snow",
                      ),
                    ],
                    [caption],
                  )
                : null,
            ].filter((v): v is Html => v !== null),
          ),
        ].filter((v): v is Html => v !== null),
      ),
      h.div(hitAttrs, []),
    ],
  );
}

function section(name: string, tiles: ReadonlyArray<Html>): Html | null {
  if (tiles.length === 0) return null;
  return h.fieldset(
    [
      h.Class("m-0 flex min-w-0 items-end overflow-visible border-none p-0"),
      h.Style({ paddingLeft: `${HAND_CARD_OVERLAP}px` }),
      h.Attribute("aria-label", name),
    ],
    tiles,
  );
}

export type HandViewInputs = {
  state: VisibleState;
  hiddenId: number | null;
  flyingIds: ReadonlySet<number>;
  /** Ids to drop from the bar entirely (mid-flight to the battlefield / stack). Union of
   * `board.handHidden` and any external hide set. */
  hiddenIds: ReadonlySet<number>;
  handDrag: HandDragState | null;
};

function handDragGhost(drag: HandDragState): Html {
  const pips = costPips(drag.manaCost, { showZero: drag.kind != null && drag.kind !== "land" });
  const artClass = `pointer-events-none block touch-none rounded-game object-cover drop-shadow-drag shadow-hand ${barZoneAura("hand")}`;

  return h.div(
    [
      h.DataAttribute("testid", "hand-drag-ghost"),
      h.Class("pointer-events-none fixed z-[21] -translate-x-1/2 -translate-y-1/2"),
      h.Style({
        left: `${drag.x}px`,
        top: `${drag.y}px`,
        width: `${HAND_CARD_W}px`,
      }),
    ],
    [
      pips.length > 0
        ? h.div(
            [
              h.Class("pointer-events-none absolute right-0 left-0 flex items-end justify-end gap-px pb-0.5"),
              h.Style({ top: `-${HAND_PIP_ROW_H}px`, height: `${HAND_PIP_ROW_H}px` }),
              h.Attribute("aria-hidden", "true"),
            ],
            pips.map((pip: CostPip) => costPipView(pip.ms, pip.code, 17)),
          )
        : null,
      drag.print
        ? cardArt(h, {
            print: drag.print,
            alt: drag.name,
            className: artClass,
            style: { width: `${HAND_CARD_W}px`, height: `${HAND_CARD_H}px` },
          })
        : h.div(
            [
              h.Class(
                `flex items-center justify-center rounded-game bg-forest-shadow p-1 text-center text-caption text-snow shadow-hand ${barZoneAura("hand")}`,
              ),
              h.Style({ width: `${HAND_CARD_W}px`, height: `${HAND_CARD_H}px` }),
            ],
            [h.div([h.Class("overflow-hidden text-ellipsis whitespace-nowrap font-semibold")], [drag.name])],
          ),
    ].filter((v): v is Html => v !== null),
  );
}

export function handView(inputs: HandViewInputs): Html {
  const { state, hiddenId, flyingIds, hiddenIds, handDrag } = inputs;
  const viewer = state.viewer;
  const grouped = bySection(state.actions);
  const commandActionByObject = byObject(grouped.command);
  const handActionByObject = byObject(grouped.hand);
  // Coerce wire numbers — proto/json sometimes delivers numeric fields as strings after folds.
  const commandCards: ObjectView[] = state.objects.filter(
    (o) => Number(o.zone) === ZONE.Command && Number(o.owner) === Number(viewer),
  );
  const handCards: ObjectView[] = state.objects.filter(
    (o) => Number(o.zone) === ZONE.Hand && Number(o.owner) === Number(viewer),
  );
  const commanderTax = state.players.find((p) => p.player === viewer)?.commander_tax ?? 0;
  const objectsById = new Map(state.objects.map((o) => [o.id, o]));

  const slotDimmed = (id: number) => id === hiddenId || flyingIds.has(id);

  const metaFor = (id: number | undefined | null) => {
    const obj = id != null ? objectsById.get(id) : undefined;
    return {
      print: obj?.print ?? "",
      cardId: obj?.card_id,
      kind: obj?.kind?.kind,
      manaCost: obj?.mana_cost ?? emptyCost(),
    };
  };

  const commandVisible = commandCards.filter((c) => !hiddenIds.has(c.id));
  const commandTiles = commandVisible.map((c, index) =>
    tile({
      name: c.name,
      print: c.print ?? "",
      cardId: c.card_id,
      zone: "command",
      objectId: c.id,
      objectKind: c.kind.kind,
      manaCost: c.mana_cost,
      action: commandActionByObject.get(c.id) ?? null,
      dimmed: !commandActionByObject.get(c.id) || slotDimmed(c.id),
      caption: c.is_commander && commanderTax > 0 ? `Tax +{${commanderTax}}` : undefined,
      index,
      count: commandVisible.length,
    }),
  );

  type HandSlot = {
    name: string;
    print: string;
    cardId?: string;
    objectId?: number;
    objectKind?: string;
    manaCost: WireCost;
    action: ActionView | null;
    dimmed: boolean;
    caption?: string;
  };
  const handSlots: HandSlot[] = [];
  for (const c of handCards) {
    if (hiddenIds.has(c.id)) continue;
    const action = handActionByObject.get(c.id) ?? null;
    handSlots.push({
      name: c.name,
      print: c.print ?? "",
      cardId: c.card_id,
      objectId: c.id,
      objectKind: c.kind.kind,
      manaCost: c.mana_cost,
      action,
      dimmed: !action || slotDimmed(c.id),
      caption: actionCaption(action?.kind ?? ""),
    });
  }
  for (const extra of handExtras(grouped.hand)) {
    const meta = metaFor(extra.object);
    handSlots.push({
      name: extra.label.replace(/^[^:]+:\s*/, ""),
      print: meta.print,
      cardId: meta.cardId,
      objectId: extra.object ?? undefined,
      objectKind: meta.kind,
      manaCost: meta.manaCost,
      action: extra,
      dimmed: false,
      caption: actionCaption(extra.kind),
    });
  }
  const handTiles = handSlots.map((slot, index) =>
    tile({
      ...slot,
      zone: "hand",
      index,
      count: handSlots.length,
    }),
  );

  const zoneTiles = (zone: "graveyard" | "exile", actions: ActionView[]) =>
    actions.map((a, index, arr) => {
      const meta = metaFor(a.object);
      return tile({
        name: a.label,
        print: meta.print,
        cardId: meta.cardId,
        zone,
        objectId: a.object ?? undefined,
        objectKind: meta.kind,
        manaCost: meta.manaCost,
        action: a,
        dimmed: false,
        index,
        count: arr.length,
      });
    });

  return h.div(
    [],
    [
      h.div(
        [
          h.DataAttribute("testid", "hand-bar"),
          h.OnMount(MountHandBarDrag()),
          h.Class(
            "pointer-events-none fixed right-0 bottom-0 left-0 z-20 flex items-end justify-center gap-xl overflow-visible px-md",
          ),
          h.Style({ height: `${HAND_BAR_H}px` }),
        ],
        [
          section("Command", commandTiles),
          section("Hand", handTiles),
          section("Graveyard", zoneTiles("graveyard", grouped.graveyard)),
          section("Exile", zoneTiles("exile", grouped.exile)),
        ].filter((child): child is Html => child !== null),
      ),
      handDrag != null ? handDragGhost(handDrag) : null,
    ].filter((child): child is Html => child !== null),
  );
}
