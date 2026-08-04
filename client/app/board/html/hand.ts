// Bottom action bar: command / hand / graveyard / exile as Arena peek tiles.
//
// Geometry matches Solid `hand.tsx`: each flex slot stays peek-wide × visible-tall; the face is
// `top-0 right-0` so the excess hangs *below* the viewport (screen clips). Raise is paint-only
// (`translateY`); the hit strip is bottom-anchored and grows upward on hover. Fan tilt + cast-cost
// pips live on the face column. Buried cards hit on the left peek only; the section's rightmost
// card uses the full face (`handBarHitWidth`).

import { Option } from "effect";
import type { Attribute, Html, HtmlBuilder } from "foldkit/html";
import type { CardText } from "~/card-render/card-text";
import { type FaceData, faceDataFrom } from "~/card-render/frame";
import { type CostPip, costPips } from "~/costPips";
import { cardFace } from "~/ui/card-face";
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

export const HAND_CARD_PEEK = HAND_BAR_PEEK;
export const HAND_VISIBLE_H = 178;
/** Room above each face for cast-cost pips (reserved band outside the card). */
const HAND_PIP_ROW_H = 24;
/** Window the bar constants above were drawn against. */
export const HAND_DESIGN_VIEWPORT = { width: 1440, height: 900 } as const;

/**
 * The bar is a constant fraction of the window, not a fixed pixel size. A 208px face that reads
 * well on a 1440x900 laptop is a thumbnail on a 27" 2560x1440 desktop viewed from arm's length,
 * and it swallows a small laptop. Clamped so neither extreme distorts the layout.
 */
export function handUiScale(viewport: { width: number; height: number }): number {
  const raw = Math.min(viewport.width / HAND_DESIGN_VIEWPORT.width, viewport.height / HAND_DESIGN_VIEWPORT.height);
  if (!(raw > 0)) return 1;
  return Math.max(0.75, Math.min(1.5, raw));
}

export type HandMetrics = {
  scale: number;
  cardW: number;
  cardH: number;
  peek: number;
  overlap: number;
  visibleH: number;
  pipRowH: number;
  pipSize: number;
  /** Height of the bottom action bar — tuck + pip row + padding. */
  barH: number;
  /**
   * From the viewport bottom: band where sticky Alt-inspect hand hover stays latched after leaving
   * the peek hit strip (raised faces extend above `barH` into the board).
   */
  stickyBand: number;
  /** How far into the hand bar a release may still count as play (px). */
  playSlack: number;
};

/** Every hand-bar length in CSS px for this window. Rounded so inline styles stay on whole pixels. */
export function handMetrics(viewport: { width: number; height: number }): HandMetrics {
  const scale = handUiScale(viewport);
  const cardW = Math.round(HAND_FACE_W * scale);
  const cardH = Math.round(cardW / 0.716);
  const peek = Math.round(HAND_CARD_PEEK * scale);
  const visibleH = Math.round(HAND_VISIBLE_H * scale);
  const pipRowH = Math.round(HAND_PIP_ROW_H * scale);
  const barH = visibleH + pipRowH + Math.round(16 * scale);
  return {
    scale,
    cardW,
    cardH,
    peek,
    overlap: cardW - peek,
    visibleH,
    pipRowH,
    pipSize: Math.round(14 * scale),
    barH,
    stickyBand: barH - visibleH + cardH,
    playSlack: Math.round(96 * scale),
  };
}

/** The bar at its design size — for callers with no window to measure (tests, SSR). */
export const HAND_BASE_METRICS = handMetrics(HAND_DESIGN_VIEWPORT);
/** Bar height at the design window. Live boards must use `handMetrics(viewport).barH`. */
export const HAND_BAR_H = HAND_BASE_METRICS.barH;

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

function costPipView(ms: string, code: string, sizePx: number, h: HtmlBuilder<Message>): Html {
  return pipChip(h, { ms, code, sizePx });
}

function tile(
  args: {
    metrics: HandMetrics;
    name: string;
    print: string;
    /** The rendered face this tile paints. Null only when the bar has an action but no object to
     *  draw (a stale gy/exile action) — then the tile falls back to a name plate. */
    face: FaceData | null;
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
  },
  h: HtmlBuilder<Message>,
): Html {
  const {
    metrics,
    name,
    print,
    face,
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
  const hitW = handBarHitWidth(index, count, metrics.peek, metrics.cardW);
  const restHitH = handBarHitHeight(false, metrics.visibleH, metrics.cardH);
  const raisedHitH = handBarHitHeight(true, metrics.visibleH, metrics.cardH);
  const raiseY = handBarRaiseTranslateY(true, metrics.visibleH, metrics.cardH);
  const pips = costPips(manaCost, { showZero: objectKind != null && objectKind !== "land" });
  // Raise on hover or when the group carries data-selected=true (discard / hand-put picks).
  const faceClass =
    "pointer-events-none absolute top-0 right-0 transition-transform duration-[120ms] ease-state group-hover/hand-tile:[transform:translateY(var(--raise-y))] group-data-[selected=true]/hand-tile:[transform:translateY(var(--raise-y))]";

  // The drag source fades so the canvas DragGhost carries the face; inert slots stay non-interactive.
  // Art chrome is attribute-driven off the tile root (data-drag-source / data-playable).
  const dragSource = playable && action != null && draggingActionId != null && action.id === draggingActionId;
  const artClass =
    "pointer-events-none block touch-none rounded-game object-cover shadow-hand transition-[filter,opacity] duration-[80ms] ease-state group-data-[drag-source=true]/hand-tile:opacity-25 group-hover/hand-tile:group-data-[playable=true]/hand-tile:brightness-110";
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
      right: `${metrics.cardW - hitW}px`,
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
    hitAttrs.push(h.Attribute("aria-label", `${name} (discard)`));
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
            h.Style({ top: `-${metrics.pipRowH}px`, height: `${metrics.pipRowH}px` }),
            h.Attribute("aria-hidden", "true"),
          ],
          pips.map((pip: CostPip) => costPipView(pip.ms, pip.code, metrics.pipSize, h)),
        )
      : null;

  const cardBoxStyle = {
    width: `${metrics.cardW}px`,
    height: `${metrics.cardH}px`,
  };
  const cardFaceAttrs: Attribute<Message>[] = [h.Class(faceChromeClass), h.Style(cardBoxStyle)];
  if (objectId != null) {
    cardFaceAttrs.push(h.DataAttribute("testid", `hand-card-face-${objectId}`));
  }

  // ponytail: no printing → the plain name plate, as before. The rendered face doesn't need art to
  // draw (frame + name would do), but a printless object is a fixture, not a card someone holds.
  const art: Html =
    face && print
      ? cardFace(h, {
          face,
          width: metrics.cardW,
          height: metrics.cardH,
          className: artClass,
          style: cardBoxStyle,
        })
      : h.div(
          [
            h.Class(
              "flex items-center justify-center rounded-game bg-forest-shadow p-1 text-center text-caption text-snow shadow-hand transition-[filter,opacity] duration-[80ms] ease-state group-data-[drag-source=true]/hand-tile:opacity-25 group-hover/hand-tile:group-data-[playable=true]/hand-tile:brightness-110",
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
      width: `${metrics.peek}px`,
      height: `${metrics.visibleH}px`,
      transform: fanTransform(index, count),
      "--raise-y": `${raiseY}px`,
      "--hand-z": String(index + 1),
    }),
    h.DataAttribute("hand-index", String(index)),
    h.DataAttribute("playable", String(playable)),
    h.DataAttribute("drag-source", String(dragSource)),
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
      [h.Class(faceClass), h.Style({ width: `${metrics.cardW}px` })],
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

function section(name: string, overlap: number, tiles: ReadonlyArray<Html>, h: HtmlBuilder<Message>): Html | null {
  if (tiles.length === 0) return null;
  return h.fieldset(
    [
      h.Class("m-0 flex min-w-0 items-end overflow-visible border-none p-0"),
      h.Style({ paddingLeft: `${overlap}px` }),
      h.Attribute("aria-label", name),
    ],
    tiles,
  );
}

export type HandViewInputs = {
  /** Board viewport in CSS px — the bar scales with it. */
  viewport: { width: number; height: number };
  state: VisibleState;
  hiddenId: number | null;
  flyingIds: ReadonlySet<number>;
  /** Ids to drop from the bar entirely (mid-flight to the battlefield / stack). Union of
   * `board.handHidden` and any external hide set. */
  hiddenIds: ReadonlySet<number>;
  handDrag: HandDragState | null;
  /** Type line and rules text by catalog card id — the words the wire doesn't send. */
  cardText?: ReadonlyMap<string, CardText | null>;
  /** Object ids legal for the live local discard cost; null when not discarding. */
  discardCostIds?: ReadonlySet<number> | null;
  /** Object ids currently selected for discard cost / pending discard pick. */
  discardSelectedIds?: ReadonlySet<number> | null;
};

export function handView(inputs: HandViewInputs, h: HtmlBuilder<Message>): Html {
  const {
    viewport,
    state,
    hiddenId,
    flyingIds,
    hiddenIds,
    handDrag,
    cardText = new Map(),
    discardCostIds = null,
    discardSelectedIds = null,
  } = inputs;
  const metrics = handMetrics(viewport);
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

  /** The face to draw, with the catalog's words folded in once its lookup lands. */
  const faceOf = (object: ObjectView): FaceData => {
    const text = object.card_id != null ? cardText.get(object.card_id) : null;
    const face = faceDataFrom(object);
    if (text == null) return face;
    return { ...face, typeLine: text.typeLine, oracle: text.oracle, flavor: text.flavor };
  };

  const slotInert = (id: number) => id === hiddenId || flyingIds.has(id);

  const metaFor = (id: number | undefined | null) => {
    const obj = id != null ? objectsById.get(id) : undefined;
    return {
      print: obj?.print ?? "",
      face: obj ? faceOf(obj) : null,
      cardId: obj?.card_id,
      kind: obj?.kind?.kind,
      manaCost: obj?.mana_cost ?? emptyCost(),
    };
  };

  const commandVisible = commandCards.filter((c) => !hiddenIds.has(c.id));
  const draggingActionId = handDrag?.action.id ?? null;

  const commandTiles = commandVisible.map((c, index) =>
    tile(
      {
        metrics,
        name: c.name,
        print: c.print ?? "",
        face: faceOf(c),
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
      },
      h,
    ),
  );

  type HandSlot = {
    name: string;
    print: string;
    face: FaceData;
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
      face: faceOf(c),
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
    tile(
      {
        metrics,
        ...slot,
        zone: "hand",
        draggingActionId,
        index,
        count: handSlots.length,
      },
      h,
    ),
  );

  // GY/exile peeks are playable-only (unlike command, which always shows the commander).
  // Zone purple/green still layers with mint via barZoneAura when those tiles appear.
  const zoneTiles = (zone: "graveyard" | "exile", actions: ActionView[]) =>
    actions.map((a, index, arr) => {
      const meta = metaFor(a.object);
      const id = a.object ?? undefined;
      return tile(
        {
          metrics,
          name: formatMessage(a.label),
          print: meta.print,
          face: meta.face,
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
        },
        h,
      );
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
          h.Style({ height: `${metrics.barH}px` }),
        ],
        [
          section("Command", metrics.overlap, commandTiles, h),
          section("Hand", metrics.overlap, handTiles, h),
          section("Graveyard", metrics.overlap, zoneTiles("graveyard", grouped.graveyard), h),
          section("Exile", metrics.overlap, zoneTiles("exile", grouped.exile), h),
        ].filter((child): child is Html => child !== null),
      ),
    ].filter((child): child is Html => child !== null),
  );
}
