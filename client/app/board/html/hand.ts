// Bottom action bar: command / hand / graveyard / exile as Arena peek tiles.
//
// Geometry matches Solid `hand.tsx`: each flex slot stays peek-wide × visible-tall; the face is
// `top-0 right-0` so the excess hangs *below* the viewport (screen clips). Raise is paint-only
// (`translateY`); the hit strip is bottom-anchored and grows upward on hover. Fan tilt + cast-cost
// pips live on the face column. Buried cards hit on the left peek only; the section's rightmost
// card uses the full face (`handBarHitWidth`).

import { Option } from "effect";
import { type Attribute, type Html, html } from "foldkit/html";
import { type CostPip, costPips } from "~/costPips";
import { cardArt } from "~/ui/card-art";
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import { formatMessage } from "../../domain/i18n/message";
import { HAND_BAR_PEEK, handBarHitHeight, handBarHitWidth, handBarRaiseTranslateY } from "../geometry/handBarHit";
import { ZONE } from "../geometry/layout";
import { DiscardChosen, HandActionActivated, InspectAuxHovered, type Message } from "../messages";
import { HAND_FACE_W } from "../motion/flights";
import type { HandDragState } from "../submodel";
import { barZoneAura, byObject, bySection, handTileCaption, modesForObject } from "./actions";
import { MountHandBarDrag } from "./hand-drag-mount";
import { pipChip } from "./pip-chip";

const h = html<Message>();

export const HAND_CARD_W = HAND_FACE_W;
export const HAND_CARD_PEEK = HAND_BAR_PEEK;
export const HAND_CARD_OVERLAP = HAND_CARD_W - HAND_CARD_PEEK;
export const HAND_CARD_H = Math.round(HAND_CARD_W / 0.716);
export const HAND_VISIBLE_H = 178;
/** Room above each face for cast-cost pips (reserved band outside the card). */
const HAND_PIP_ROW_H = 24;
/** Height of the bottom action bar — tuck + pip row + padding. */
export const HAND_BAR_H = HAND_VISIBLE_H + HAND_PIP_ROW_H + 16;
/**
 * From the viewport bottom: band where sticky Alt-inspect hand hover stays latched after leaving
 * the peek hit strip (raised faces extend above `HAND_BAR_H` into the board).
 */
export const HAND_INSPECT_STICKY_BAND = HAND_BAR_H - HAND_VISIBLE_H + HAND_CARD_H;
/** How far into the hand bar a release may still count as play (px). */
export const HAND_PLAY_SLACK_PX = 96;

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
  return pipChip(h, { ms, code, sizePx });
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
  slotInert: boolean;
  /** Action id of the active hand drag — fades the source tile while the canvas ghost follows. */
  draggingActionId?: number | null;
  caption?: string;
  index: number;
  count: number;
  discardSelectable?: boolean;
  discardSelected?: boolean;
}): Html {
  const {
    name,
    print,
    cardId,
    zone,
    objectId,
    objectKind,
    manaCost,
    action,
    slotInert,
    draggingActionId,
    caption,
    index,
    count,
    discardSelectable = false,
    discardSelected = false,
  } = args;
  const playable = (action != null || discardSelectable) && !slotInert;
  const testId = objectId != null ? `hand-card-${objectId}` : undefined;
  const hitW = handBarHitWidth(index, count, HAND_CARD_PEEK, HAND_CARD_W);
  const restHitH = handBarHitHeight(false, HAND_VISIBLE_H, HAND_CARD_H);
  const raisedHitH = handBarHitHeight(true, HAND_VISIBLE_H, HAND_CARD_H);
  const raiseY = handBarRaiseTranslateY(true, HAND_VISIBLE_H, HAND_CARD_H);
  const pips = costPips(manaCost, { showZero: objectKind != null && objectKind !== "land" });
  // Raise on hover or when the group carries data-selected=true (discard / hand-put picks).
  const faceClass =
    "pointer-events-none absolute top-0 right-0 transition-transform duration-[120ms] ease-state group-hover/hand-tile:[transform:translateY(var(--raise-y))] group-data-[selected=true]/hand-tile:[transform:translateY(var(--raise-y))]";

  // The drag source fades so the canvas DragGhost carries the face; inert slots stay non-interactive.
  const dragSource = playable && action != null && draggingActionId != null && action.id === draggingActionId;
  const artClass = [
    "pointer-events-none block touch-none rounded-game object-cover shadow-hand transition-[filter,opacity] duration-[80ms] ease-state",
    dragSource ? "opacity-25" : "",
    playable && !dragSource ? "group-hover/hand-tile:brightness-110" : "",
  ]
    .filter((v) => v !== "")
    .join(" ");
  const pickChrome = discardSelectable || discardSelected;
  const faceChromeClass = [
    "relative origin-bottom rounded-game",
    pickChrome
      ? [
          "ring-2",
          "group-data-[selected=true]/hand-tile:ring-llanowar group-data-[selected=true]/hand-tile:shadow-[0_0_12px_rgba(47,125,70,0.55)]",
          "group-data-[selected=false]/hand-tile:group-data-[selectable=true]/hand-tile:ring-island-blue group-data-[selected=false]/hand-tile:group-data-[selectable=true]/hand-tile:shadow-[0_0_12px_rgba(74,158,255,0.45)]",
        ].join(" ")
      : dragSource
        ? ""
        : barZoneAura(zone, playable),
  ]
    .filter((v) => v !== "")
    .join(" ");

  const hitClass = [
    "pointer-events-auto absolute bottom-0",
    "group-hover/hand-tile:[height:var(--hit-raised-h)] group-data-[selected=true]/hand-tile:[height:var(--hit-raised-h)]",
    playable ? "cursor-grab" : "cursor-not-allowed",
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
  } else if (discardSelectable && objectId != null && !slotInert) {
    hitAttrs.push(h.Role("button"));
    hitAttrs.push(h.Tabindex(0));
    hitAttrs.push(h.DataAttribute("discard-cost-id", String(objectId)));
    hitAttrs.push(h.OnClick(DiscardChosen({ ids: [objectId] })));
    hitAttrs.push(
      h.OnKeyDownPreventDefault((key) => {
        if (key !== "Enter" && key !== " ") return Option.none();
        return Option.some(DiscardChosen({ ids: [objectId] }));
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
  const cardFaceAttrs: Attribute<Message>[] = [h.Class(faceChromeClass), h.Style(cardBoxStyle)];
  if (objectId != null) {
    cardFaceAttrs.push(h.DataAttribute("testid", `hand-card-face-${objectId}`));
  }

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
            [
              "flex items-center justify-center rounded-game bg-forest-shadow p-1 text-center text-caption text-snow shadow-hand transition-[filter,opacity] duration-[80ms] ease-state",
              dragSource ? "opacity-25" : "",
              playable && !dragSource ? "group-hover/hand-tile:brightness-110" : "",
            ]
              .filter((v) => v !== "")
              .join(" "),
          ),
          h.Style(cardBoxStyle),
        ],
        [h.div([h.Class("overflow-hidden text-ellipsis whitespace-nowrap font-semibold")], [name])],
      );

  const tileAttrs: Attribute<Message>[] = [
    h.Class(
      "group/hand-tile pointer-events-none relative shrink-0 origin-bottom overflow-visible [z-index:var(--hand-z)] hover:[z-index:50]",
    ),
    h.Style({
      width: `${HAND_CARD_PEEK}px`,
      height: `${HAND_VISIBLE_H}px`,
      transform: fanTransform(index, count),
      "--raise-y": `${raiseY}px`,
      "--hand-z": String(index + 1),
    }),
    h.DataAttribute("hand-index", String(index)),
  ];
  if (objectId != null) {
    tileAttrs.push(h.DataAttribute("testid", `hand-tile-${objectId}`));
  }
  if (pickChrome) {
    tileAttrs.push(h.DataAttribute("selected", discardSelected ? "true" : "false"));
    tileAttrs.push(h.DataAttribute("selectable", discardSelectable || discardSelected ? "true" : "false"));
  }

  return h.div(tileAttrs, [
    h.div(
      [h.Class(faceClass), h.Style({ width: `${HAND_CARD_W}px` })],
      [
        pipRow,
        h.div(
          cardFaceAttrs,
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
  ]);
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
  /** Object ids legal for the live local discard cost; null when not discarding. */
  discardCostIds?: ReadonlySet<number> | null;
  /** Object ids currently selected for discard cost / pending discard pick. */
  discardSelectedIds?: ReadonlySet<number> | null;
};

export function handView(inputs: HandViewInputs): Html {
  const { state, hiddenId, flyingIds, hiddenIds, handDrag, discardCostIds = null, discardSelectedIds = null } = inputs;
  const viewer = state.viewer;
  const grouped = bySection(state.actions);
  const commandActionByObject = byObject(grouped.command);
  // Coerce wire numbers — proto/json sometimes delivers numeric fields as strings after folds.
  const commandCards: ObjectView[] = state.objects.filter(
    (o) => Number(o.zone) === ZONE.Command && Number(o.owner) === Number(viewer),
  );
  const handCards: ObjectView[] = state.objects.filter(
    (o) => Number(o.zone) === ZONE.Hand && Number(o.owner) === Number(viewer),
  );
  const commanderTax = state.players.find((p) => p.player === viewer)?.commander_tax ?? 0;
  const objectsById = new Map(state.objects.map((o) => [o.id, o]));

  const slotInert = (id: number) => id === hiddenId || flyingIds.has(id);

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
  const draggingActionId = handDrag?.action.id ?? null;

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
      slotInert: slotInert(c.id),
      draggingActionId,
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
    slotInert: boolean;
    caption?: string;
    discardSelectable?: boolean;
    discardSelected?: boolean;
  };
  const handSlots: HandSlot[] = [];
  for (const c of handCards) {
    if (hiddenIds.has(c.id)) continue;
    const modes = modesForObject(grouped.hand, c.id);
    const action = modes[0] ?? null;
    handSlots.push({
      name: c.name,
      print: c.print ?? "",
      cardId: c.card_id,
      objectId: c.id,
      objectKind: c.kind.kind,
      manaCost: c.mana_cost,
      action,
      slotInert: slotInert(c.id),
      caption: handTileCaption(modes),
      discardSelectable: discardCostIds?.has(c.id) ?? false,
      discardSelected: discardSelectedIds?.has(c.id) ?? false,
    });
  }
  const handTiles = handSlots.map((slot, index) =>
    tile({
      ...slot,
      zone: "hand",
      draggingActionId,
      index,
      count: handSlots.length,
    }),
  );

  // GY/exile peeks are playable-only (unlike command, which always shows the commander).
  // Zone purple/green still layers with mint via barZoneAura when those tiles appear.
  const zoneTiles = (zone: "graveyard" | "exile", actions: ActionView[]) =>
    actions.map((a, index, arr) => {
      const meta = metaFor(a.object);
      const id = a.object ?? undefined;
      return tile({
        name: formatMessage(a.label),
        print: meta.print,
        cardId: meta.cardId,
        zone,
        objectId: id,
        objectKind: meta.kind,
        manaCost: meta.manaCost,
        action: a,
        slotInert: id != null ? slotInert(id) : false,
        draggingActionId,
        caption: actionCaption(a.kind),
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
    ].filter((child): child is Html => child !== null),
  );
}
