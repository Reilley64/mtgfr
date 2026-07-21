// The bottom action bar as a DOM overlay on the canvas board. It is driven by the viewer's
// legal-action list (`game.state.actions`): the Command and Hand sections show every card in
// those zones (a card with a play action is draggable, an actionless one dims — the commander is
// always on show while it sits in the command zone), and the Graveyard / Exile sections show
// one card per action there. Battlefield activates live on the selection radial, not in this bar.
// Zone order left→right: command → hand → graveyard → exile. Zone groups follow Arena: gap +
// aura colour, no section captions. Hand tiles are a dense fan (faces hang left of a peek-wide
// slot) with cast-cost pips on the top-right; hover raises the full card (paint translate only —
// layout footprint stays peek × visible so the hit box cannot thrash). Arena stacking: left
// lowest, right highest, so resting cards show a left-edge name strip. Hits stay on that left
// peek only (`~/lib/handBarHit`) so a raised face cannot steal a neighbor. Centre of the fan
// rises toward the board. Faces tuck under the screen edge at rest (viewport clips them —
// no mid-card overflow cut).
// Every playable card is physically dragged — a ghost follows the cursor — and on release the
// Board takes the action by its id (drag above the play threshold → take_action; back in the bar
// → snap back). Combat declarations are NOT cards here; they keep the board's existing
// drag-to-avatar UI.

import { createMemo, createSignal, For, type JSX, onCleanup, Show } from "solid-js";
import { CardArt } from "~/components/atoms";
import { ZONE } from "~/layout";
import { type BarZone, barZoneAura, byObject, bySection, handExtras } from "~/lib/actions";
import { HAND_FACE_W } from "~/lib/cardFlight";
import { cn } from "~/lib/cn";
import { costPipPlate, costPips } from "~/lib/costPips";
import { HAND_BAR_PEEK, handBarHitHeight, handBarHitWidth, handBarRaiseTranslateY } from "~/lib/handBarHit";
import { game } from "~/store";
import type { ActionView, ObjectView, WireCost } from "~/wire/types";

export interface ActionDrop {
  action: ActionView;
  x: number;
  y: number;
}

/**
 * Face width — Arena-scale (~stack overlay size) so hand/command/graveyard/exile tiles
 * read as real cards, not chrome thumbnails. Every bar tile uses this same locked box.
 * Sourced from `HAND_FACE_W` so flight scale can't drift.
 */
export const HAND_CARD_W = HAND_FACE_W;
/** Visible strip width at rest — left edge of the face (card name), Arena-style. */
export const HAND_CARD_PEEK = HAND_BAR_PEEK;
export const HAND_CARD_OVERLAP = HAND_CARD_W - HAND_CARD_PEEK;
export const HAND_CARD_H = Math.round(HAND_CARD_W / 0.716);
/**
 * How much of each tucked card sticks into the viewport at rest. The rest hangs past the
 * bottom edge so the *screen* clips the face (Arena), not an overflow:hidden mid-card cut.
 */
export const HAND_VISIBLE_H = 130;
/** @deprecated Alias — prefer `HAND_VISIBLE_H`. */
export const HAND_STRIP_H = HAND_VISIBLE_H;
/** Room above each face for cast-cost pips (reserved band outside the card). */
const HAND_PIP_ROW_H = 20;

/** MTGA fan: left/right tilt out; centre rises toward the board (edges sit lower). */
function fanTransform(index: number, count: number): string {
  const off = index - (count - 1) / 2;
  const angle = Math.max(-10, Math.min(10, off * 2.5));
  const rise = Math.max(0, 14 - off * off * 1.2);
  return `rotate(${angle}deg) translateY(${-rise}px)`;
}

/** Height of the bottom action bar; the Board uses it to place the play threshold.
 * Matches the on-screen tuck + pip row above the faces. */
export const HAND_BAR_H = HAND_VISIBLE_H + HAND_PIP_ROW_H + 12;

/** Locked width+height so every face is identical regardless of art intrinsic size. */
const CARD_FACE = cn("block h-(--card-h) w-(--card-w) rounded-game object-cover");
const emptyCost = (): WireCost => ({ generic: 0, colored: [0, 0, 0, 0, 0] });

/**
 * Arena cost disk: opaque plate is inline (not only `.ms-cost` background) so a buried or
 * filter-composited glyph can never read as a hollow outline on the felt.
 */
function CostPip(props: { ms: string; code: string; sizePx?: number }) {
  const size = props.sizePx ?? 12;
  return (
    <span
      class="inline-flex shrink-0 items-center justify-center rounded-full shadow-[0_1px_2px_rgb(0_0_0/0.9)]"
      style={{
        width: `${size}px`,
        height: `${size}px`,
        "background-color": costPipPlate(props.code),
        color: "#111",
        "font-size": `${Math.round(size * 0.82)}px`,
      }}
    >
      <i class={cn("ms", `ms-${props.ms}`)} />
    </span>
  );
}

export default function Hand(props: {
  viewer: number;
  hiddenId: number | null; // the staged card, dimmed in place while it awaits a target
  /** Hand/command ids owned by the canvas flight layer — dim the resting slot (client-game-board-and-interaction spec). */
  flyingIds?: ReadonlySet<number>;
  /** Current face-up bar card under the cursor (for Alt-pin inspect owned by Board). */
  onHoverCard?: (card: { name: string; cardId?: string; print?: string } | null) => void;
  /** Action under the cursor (or being dragged) — Board paints its `auto_tap` preview. */
  onHoverAction?: (action: ActionView | null) => void;
  onDrop: (d: ActionDrop) => void;
}) {
  const slotDimmed = (id: number) => id === props.hiddenId || (props.flyingIds?.has(id) ?? false);
  const grouped = createMemo(() => bySection(game.state?.actions));
  const handCards = createMemo<ObjectView[]>(() =>
    game.state ? game.state.objects.filter((o) => o.zone === ZONE.Hand && o.owner === props.viewer) : [],
  );
  const handActionByObject = createMemo(() => byObject(grouped().hand));
  const objectMeta = (
    id: number | undefined | null,
  ): { print: string; cardId?: string; kind?: string; manaCost: WireCost } => {
    const obj = id != null ? game.state?.objects.find((o) => o.id === id) : undefined;
    return {
      print: obj?.print ?? "",
      cardId: obj?.card_id || undefined,
      kind: obj?.kind?.kind,
      manaCost: obj?.mana_cost ?? emptyCost(),
    };
  };
  // Hand cards plus overshadowed alternative-action tiles (cycle / suspend / discard-ability) share
  // one fan so extras sit in the same arc.
  const handSlots = createMemo(() => {
    const cards = handCards().map((card) => ({ kind: "card" as const, card }));
    const extras = handExtras(grouped().hand).map((action) => ({
      kind: "extra" as const,
      action,
    }));
    return [...cards, ...extras];
  });
  // The command zone renders from the *objects*, not the actions: the commander stays visible
  // (dimmed, inert) whenever it sits in the command zone — even when unaffordable or out of
  // sorcery timing — and leaves the bar only by leaving the zone (cast, graveyard, exile).
  const commandCards = createMemo<ObjectView[]>(() =>
    game.state ? game.state.objects.filter((o) => o.zone === ZONE.Command && o.owner === props.viewer) : [],
  );
  const commandActionByObject = createMemo(() => byObject(grouped().command));
  // What recasting your commander costs on top of its mana cost, {2} per previous cast (CR 903.8).
  // The engine folds it into the cast, but until now the bar never said what you were about to pay.
  const commanderTax = createMemo(() => game.state?.players.find((p) => p.player === props.viewer)?.commander_tax ?? 0);

  // The drag rides an action + the image name/print to draw its ghost. Driven from window listeners
  // (not per-card handlers) so a stream delta arriving mid-drag — which re-renders the `<For>` and
  // would destroy the grabbed element, losing its pointer capture — can't strand the drag.
  const [drag, setDrag] = createSignal<{
    action: ActionView;
    name: string;
    print: string;
    manaCost: WireCost;
    kind?: string;
    x: number;
    y: number;
  } | null>(null);
  // Track which bar card is under the cursor so Board can Alt-pin inspect it.
  const [hover, setHover] = createSignal<string | null>(null);
  const setHoverCard = (card: { name: string; cardId?: string; print?: string } | null) => {
    setHover(card?.name ?? null);
    props.onHoverCard?.(card);
  };
  const setHoverAction = (action: ActionView | null) => {
    props.onHoverAction?.(action);
  };

  let move: ((e: PointerEvent) => void) | null = null;
  let up: ((e: PointerEvent) => void) | null = null;
  let cancel: ((e: PointerEvent) => void) | null = null;
  const teardown = () => {
    if (move) window.removeEventListener("pointermove", move);
    if (up) window.removeEventListener("pointerup", up);
    if (cancel) window.removeEventListener("pointercancel", cancel);
    move = null;
    up = null;
    cancel = null;
  };
  onCleanup(() => {
    teardown();
    // StackOverlay clears aux hover on unmount; do the same so a sticky hand name can't
    // steal Alt-inspect after Hand is torn down (eliminate / spectate).
    setHoverCard(null);
    setHoverAction(null);
  });

  const onDown = (
    action: ActionView,
    name: string,
    print: string,
    manaCost: WireCost,
    kind: string | undefined,
    e: PointerEvent,
  ) => {
    e.preventDefault();
    teardown(); // clear any listeners from a drag whose pointerup was missed
    setDrag({ action, name, print, manaCost, kind, x: e.clientX, y: e.clientY });
    setHoverAction(action);
    move = (ev) => setDrag((d) => (d ? { ...d, x: ev.clientX, y: ev.clientY } : d));
    up = (ev) => {
      teardown();
      const d = drag();
      setDrag(null);
      setHoverAction(null);
      if (d) props.onDrop({ action: d.action, x: ev.clientX, y: ev.clientY });
    };
    cancel = () => {
      teardown();
      setDrag(null);
      setHoverAction(null);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
  };

  // A single bar card: draggable-to-play when `action` is set, otherwise dimmed and inert (still
  // previews on Alt). `caption` overlays an ability label for battlefield tiles; `fan` is the
  // MTGA-style arc transform for the card's slot in its section. Actionable cards are also
  // keyboard-operable: focusable, labelled for AT, Enter/Space plays them the same as a drag-out
  // (dropped at the viewport center — comfortably above the play threshold, so a targeted action
  // stages exactly as it would from a drag, and a plain one takes immediately).
  const activate = (action: ActionView) =>
    props.onDrop({ action, x: window.innerWidth / 2, y: window.innerHeight / 2 });
  /** Inert tiles darken in place; the drag source fades so the ghost carries the face.
   * Prefer brightness over opacity for inert — opacity punches holes through the dense fan. */
  const dimmedness = (p: { action: ActionView | null; dimmed?: boolean }) =>
    cn(p.dimmed && "brightness-[0.55]", p.action && drag()?.action.id === p.action.id && "opacity-25");
  const BarCard = (p: {
    name: string;
    print: string;
    cardId?: string;
    objectId?: number;
    objectKind?: string;
    manaCost: WireCost;
    action: ActionView | null;
    dimmed?: boolean;
    caption?: string;
    fan?: string;
    zone: BarZone;
    /** Slot index in the section — later (right) cards stack above for Arena name peeks. */
    index: number;
    count: number;
  }) => {
    const zoneWord = p.zone === "hand" ? null : p.zone;
    const ariaName = () => {
      if (!p.action) return undefined;
      const base = p.caption ? `${p.name}: ${p.caption}` : p.name;
      return zoneWord ? `${base} (${zoneWord})` : base;
    };
    const pips = () => costPips(p.manaCost, { showZero: p.objectKind != null && p.objectKind !== "land" });
    const [raised, setRaised] = createSignal(false);
    // Arena: left lowest, right highest — resting fans show left-edge name strips.
    const stackZ = () => (raised() ? 30 : p.index + 1);
    const onHitEnter = () => {
      setRaised(true);
      setHoverCard({ name: p.name, cardId: p.cardId, print: p.print });
      setHoverAction(p.action);
    };
    const onHitMove = () => {
      setHoverCard({ name: p.name, cardId: p.cardId, print: p.print });
      setHoverAction(p.action);
    };
    const onHitLeave = () => {
      setRaised(false);
      if (hover() === p.name) setHoverCard(null);
      if (!drag()) setHoverAction(null);
    };
    // Buried cards: left peek only. Rightmost (and single-card sections like the commander):
    // full face — nothing to the right to protect (`handBarHitWidth`).
    const hitW = () => handBarHitWidth(p.index, p.count, HAND_CARD_PEEK, HAND_CARD_W);
    const hitH = () => handBarHitHeight(raised(), HAND_VISIBLE_H, HAND_CARD_H);
    const raiseY = () => handBarRaiseTranslateY(raised(), HAND_VISIBLE_H, HAND_CARD_H);
    return (
      // Layout slot only — hit strip is a sibling of the paint face (below), not inside it.
      <div
        style={{
          "--fan": p.fan,
          "--peek": `${HAND_CARD_PEEK}px`,
          "--visible": `${HAND_VISIBLE_H}px`,
          "--card-w": `${HAND_CARD_W}px`,
          "--card-h": `${HAND_CARD_H}px`,
          "z-index": stackZ(),
        }}
        // Flex footprint stays peek-wide *and* visible-tall always. Growing width or height on
        // raise moved the hit box out from under the cursor → enter/leave thrash. Raise is
        // paint-only translateY on the face; the hit strip bottom-anchors and grows upward.
        class="pointer-events-none relative h-(--visible) w-(--peek) origin-bottom [transform:var(--fan,none)]"
      >
        {/* Face + pips share a fixed column (right-aligned in the peek slot). Pips live in a
            reserved band *above* the face top — Arena cast disks, not overlaid on the art. */}
        <div
          class="pointer-events-none absolute top-0 right-0 w-(--card-w) transition-transform duration-[120ms] ease-state"
          style={{ transform: `translateY(${raiseY()}px)` }}
        >
          <Show when={pips().length > 0}>
            <div
              data-testid="hand-cost-pips"
              class="absolute right-0 left-0 z-20 flex items-end justify-end gap-px pb-0.5"
              style={{ top: `-${HAND_PIP_ROW_H}px`, height: `${HAND_PIP_ROW_H}px` }}
              aria-hidden="true"
            >
              <For each={pips()}>{(pip) => <CostPip ms={pip.ms} code={pip.code} sizePx={raised() ? 16 : 14} />}</For>
            </div>
          </Show>
          <div
            class="relative h-(--card-h) origin-bottom rounded-game"
            data-testid={p.objectId != null ? `hand-card-${p.objectId}` : undefined}
          >
            <CardArt
              print={p.print}
              alt={p.name}
              draggable={false}
              class={cn(
                CARD_FACE,
                // Paint only — default `auto` on <img> would re-enable hits under a
                // pointer-events-none ancestor and steal the right neighbor's left peek.
                "pointer-events-none touch-none shadow-hand transition-[filter] duration-[80ms] ease-state",
                p.action && raised() && "brightness-110",
                barZoneAura(p.zone),
                dimmedness(p),
              )}
            />
            <Show when={p.caption}>
              <div class="pointer-events-none absolute right-0 bottom-2 left-0 mx-1.5 overflow-hidden text-ellipsis whitespace-nowrap rounded-control bg-forest-hud px-1 py-0.5 text-center font-semibold text-micro text-snow">
                {p.caption}
              </div>
            </Show>
          </div>
        </div>
        {/* Hit target — left peek for buried cards; full face for the section's rightmost
            (`handBarHitWidth` / `hitHandBarSlot`). Bottom-anchored so raise grows upward and a
            cursor on the resting visible bottom never leaves. Face art stays paint-only.
            Not a <button>: onDown drops on pointerup, so a native button click would double-fire. */}
        {/* biome-ignore lint/a11y/noStaticElementInteractions: keyboard-operable exactly when actionable */}
        {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: aria-label is set iff role is "button" */}
        <div
          tabIndex={p.action ? 0 : undefined}
          role={p.action ? "button" : undefined}
          data-action-kind={p.action?.kind ?? undefined}
          data-action-id={p.action != null ? String(p.action.id) : undefined}
          data-needs-target={p.action?.needs_target ? "1" : "0"}
          data-has-player-target={p.action?.targets?.some((t) => t.kind === "player") ? "1" : "0"}
          data-has-object-target={p.action?.targets?.some((t) => t.kind === "object") ? "1" : "0"}
          data-object-kind={p.objectKind}
          data-bar-zone={p.zone}
          aria-label={ariaName()}
          onKeyDown={(e) => {
            if (!p.action || (e.key !== "Enter" && e.key !== " ")) return;
            e.preventDefault(); // Space must not scroll the page
            activate(p.action);
          }}
          onPointerEnter={onHitEnter}
          onPointerMove={onHitMove}
          onPointerLeave={onHitLeave}
          onPointerDown={(e) => p.action && onDown(p.action, p.name, p.print, p.manaCost, p.objectKind, e)}
          style={{
            width: `${hitW()}px`,
            height: `${hitH()}px`,
            // Face is right-aligned in the peek slot; hit starts at the face's left edge
            // (name strip), not the slot's left — same as the old `left-0` on the face.
            right: `${HAND_CARD_W - hitW()}px`,
          }}
          class={cn("pointer-events-auto absolute bottom-0", p.action ? "cursor-grab" : "cursor-default")}
        />
      </div>
    );
  };

  return (
    <>
      <div
        data-testid="hand-bar"
        style={{ "--bar-h": `${HAND_BAR_H}px` }}
        // Above world-anchored mana trays (z-18); level with other action chrome (log / turn).
        class="pointer-events-none fixed right-0 bottom-0 left-0 z-20 flex h-(--bar-h) items-end justify-center gap-xl overflow-visible px-md"
      >
        <Show when={commandCards().length > 0}>
          <Section name="Command">
            <For each={commandCards()}>
              {(card, i) => {
                const action = () => commandActionByObject().get(card.id) ?? null;
                return (
                  <BarCard
                    name={card.name}
                    print={card.print ?? ""}
                    cardId={card.card_id || undefined}
                    objectId={card.id}
                    objectKind={card.kind.kind}
                    manaCost={card.mana_cost}
                    action={action()}
                    dimmed={!action() || slotDimmed(card.id)}
                    caption={card.is_commander && commanderTax() > 0 ? `Tax +{${commanderTax()}}` : undefined}
                    fan={fanTransform(i(), commandCards().length)}
                    zone="command"
                    index={i()}
                    count={commandCards().length}
                  />
                );
              }}
            </For>
          </Section>
        </Show>
        <Section name="Hand">
          <For each={handSlots()}>
            {(slot, i) => {
              const count = () => handSlots().length;
              if (slot.kind === "extra") {
                // Alternative-action labels are "Cycle: name" / "Suspend: name" / "Discard: name":
                // the prefix is the caption, the rest is the card image name.
                const meta = objectMeta(slot.action.object);
                return (
                  <BarCard
                    name={slot.action.label.replace(/^[^:]+:\s*/, "")}
                    print={meta.print}
                    cardId={meta.cardId}
                    objectId={slot.action.object ?? undefined}
                    objectKind={meta.kind}
                    manaCost={meta.manaCost}
                    action={slot.action}
                    caption={actionCaption(slot.action.kind)}
                    fan={fanTransform(i(), count())}
                    zone="hand"
                    index={i()}
                    count={count()}
                  />
                );
              }
              const action = () => handActionByObject().get(slot.card.id) ?? null;
              const dimmed = () => !action() || slotDimmed(slot.card.id);
              const caption = () => actionCaption(action()?.kind ?? "");
              return (
                <BarCard
                  name={slot.card.name}
                  print={slot.card.print ?? ""}
                  cardId={slot.card.card_id || undefined}
                  objectId={slot.card.id}
                  objectKind={slot.card.kind.kind}
                  manaCost={slot.card.mana_cost}
                  action={action()}
                  dimmed={dimmed()}
                  caption={caption()}
                  fan={fanTransform(i(), count())}
                  zone="hand"
                  index={i()}
                  count={count()}
                />
              );
            }}
          </For>
        </Section>
        <ZoneSection zone="graveyard" actions={grouped().graveyard} name={(a) => a.label} />
        <ZoneSection zone="exile" actions={grouped().exile} name={(a) => a.label} />
      </div>
      <Show when={drag()}>
        {(d) => {
          const ghostPips = () => costPips(d().manaCost, { showZero: d().kind != null && d().kind !== "land" });
          return (
            <div
              style={{
                "--x": `${d().x}px`,
                "--y": `${d().y}px`,
                "--card-w": `${HAND_CARD_W}px`,
                "--card-h": `${HAND_CARD_H}px`,
              }}
              class="pointer-events-none fixed top-(--y) left-(--x) z-20 -translate-x-1/2 -translate-y-1/2"
            >
              <CardArt print={d().print} alt={d().name} draggable={false} class={cn(CARD_FACE, "drop-shadow-drag")} />
              <Show when={ghostPips().length > 0}>
                <div
                  class="pointer-events-none absolute right-0 left-0 flex items-end justify-end gap-px pb-0.5"
                  style={{ top: `-${HAND_PIP_ROW_H}px`, height: `${HAND_PIP_ROW_H}px` }}
                  aria-hidden="true"
                >
                  <For each={ghostPips()}>{(pip) => <CostPip ms={pip.ms} code={pip.code} sizePx={17} />}</For>
                </div>
              </Show>
            </div>
          );
        }}
      </Show>
    </>
  );

  // A non-hand section: one draggable card per action, shown only when it has any. `name` picks the
  // image (card name for zone casts, source-permanent name for abilities); `caption` overlays the
  // ability label. Declared inside the component so it closes over BarCard/onDown.
  function ZoneSection(p: {
    zone: "graveyard" | "exile";
    actions: ActionView[];
    name: (a: ActionView) => string;
    caption?: boolean;
  }) {
    const groupName = p.zone === "graveyard" ? "Graveyard" : "Exile";
    return (
      <Show when={p.actions.length > 0}>
        <Section name={groupName}>
          <For each={p.actions}>
            {(a, i) => {
              const meta = objectMeta(a.object);
              return (
                <BarCard
                  name={p.name(a)}
                  print={meta.print}
                  cardId={meta.cardId}
                  objectId={a.object ?? undefined}
                  objectKind={meta.kind}
                  manaCost={meta.manaCost}
                  action={a}
                  caption={p.caption ? a.label : undefined}
                  fan={fanTransform(i(), p.actions.length)}
                  zone={p.zone}
                  index={i()}
                  count={p.actions.length}
                />
              );
            }}
          </For>
        </Section>
      </Show>
    );
  }
}

// The all-caps caption an alternative hand action shows on its tile (a plain cast/land drop has
// none). Keeps the "this tile isn't just playing the card" cue consistent whether the action is the
// card's only one or an overshadowed extra beside a cast.
function actionCaption(kind: string): string | undefined {
  if (kind === "cycle") return "Cycle";
  if (kind === "suspend") return "Suspend";
  if (kind === "activate_hand_ability") return "Discard";
  return undefined;
}

/** A group of bar cards. No visual caption — Arena uses gap + aura; `name` is for AT only.
 * Left pad equals one overlap so the first face's left overhang stays inside the group. */
function Section(props: { name: string; children: JSX.Element }) {
  return (
    <fieldset
      aria-label={props.name}
      style={{ "--overlap": `${HAND_CARD_OVERLAP}px` }}
      class="m-0 flex min-w-0 items-end overflow-visible border-none p-0 pl-(--overlap)"
    >
      {props.children}
    </fieldset>
  );
}
