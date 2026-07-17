// The bottom action bar as a DOM overlay on the canvas board. It is driven by the viewer's
// legal-action list (`game.state.actions`): the Hand and Command sections show every card in
// those zones (a card with a play action is draggable, an actionless one dims — the commander is
// always on show while it sits in the command zone), and the Graveyard / Exile sections show
// one card per action there. Battlefield activates live on the selection radial, not in this bar.
// Every playable card is physically dragged — a ghost follows the cursor — and on release the
// Board takes the action by its id (drag above the play threshold → take_action; back in the bar
// → snap back). Combat declarations are NOT cards here; they keep the board's existing
// drag-to-avatar UI.

import { createMemo, createSignal, For, type JSX, onCleanup, Show } from "solid-js";
import { ZONE } from "~/layout";
import { byObject, bySection, handExtras } from "~/lib/actions";
import { cn } from "~/lib/cn";
import { imageUrlByPrint } from "~/lib/scryfall";
import { game } from "~/store";
import type { ActionView, ObjectView } from "~/wire/types";

export interface ActionDrop {
  action: ActionView;
  x: number;
  y: number;
}

/** MTGA-style fan for a card row: tilt grows linearly with distance from the row's center and
 * the card sinks quadratically, tracing a slight arc. A single card sits flat. */
function fanTransform(index: number, count: number): string {
  const off = index - (count - 1) / 2;
  const angle = Math.max(-12, Math.min(12, off * 3));
  const drop = Math.min(28, off * off * 2.2);
  return `rotate(${angle}deg) translateY(${drop}px)`;
}

/** Height of the bottom action bar; the Board uses it to place the play threshold. */
export const HAND_BAR_H = 210;

// A card face, in the bar and as the drag ghost. The bar card overlaps its left neighbour.
const CARD_FACE = cn("block w-[150px] rounded-[9px]");

export default function Hand(props: {
  viewer: number;
  hiddenId: number | null; // the staged card, dimmed in place while it awaits a target
  /** Current face-up bar card under the cursor (for Alt-pin inspect owned by Board). */
  onHoverCard?: (card: { name: string; cardId?: string; print?: string } | null) => void;
  /** Action under the cursor (or being dragged) — Board paints its `auto_tap` preview. */
  onHoverAction?: (action: ActionView | null) => void;
  onDrop: (d: ActionDrop) => void;
}) {
  const grouped = createMemo(() => bySection(game.state?.actions));
  const handCards = createMemo<ObjectView[]>(() =>
    game.state ? game.state.objects.filter((o) => o.zone === ZONE.Hand && o.owner === props.viewer) : [],
  );
  const handActionByObject = createMemo(() => byObject(grouped().hand));
  const objectMeta = (id: number | undefined | null): { print: string; cardId?: string; kind?: string } => {
    const obj = id != null ? game.state?.objects.find((o) => o.id === id) : undefined;
    return {
      print: obj?.print ?? "",
      cardId: obj?.card_id || undefined,
      kind: obj?.kind?.kind,
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

  const onDown = (action: ActionView, name: string, print: string, e: PointerEvent) => {
    e.preventDefault();
    teardown(); // clear any listeners from a drag whose pointerup was missed
    setDrag({ action, name, print, x: e.clientX, y: e.clientY });
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
  // (dropped at the viewport center — comfortably above the bar threshold, so a targeted action
  // stages exactly as it would from a drag, and a plain one takes immediately).
  const activate = (action: ActionView) =>
    props.onDrop({ action, x: window.innerWidth / 2, y: window.innerHeight / 2 });
  /** Later conditions win (twMerge resolves the `opacity-*` conflict), so this reads as a cascade:
   * lit by default, dimmed when inert, nearly invisible while the drag ghost carries the card. */
  const dimmedness = (p: { action: ActionView | null; dimmed?: boolean }) =>
    cn("opacity-100", p.dimmed && "opacity-55", p.action && drag()?.action.id === p.action.id && "opacity-25");
  const BarCard = (p: {
    name: string;
    print: string;
    cardId?: string;
    objectId?: number;
    objectKind?: string;
    action: ActionView | null;
    dimmed?: boolean;
    caption?: string;
    fan?: string;
  }) => (
    // Not a <button>: `onDown` drops the card on pointerup, so a native button's click would fire
    // `onDrop` a second time. The div carries the full button contract instead — tabIndex, role,
    // aria-label and Enter/Space — but all four are conditional on `p.action` together, and Biome
    // can't see that they move as a set.
    // biome-ignore lint/a11y/noStaticElementInteractions: keyboard-operable exactly when actionable
    // biome-ignore lint/a11y/useAriaPropsSupportedByRole: aria-label is set iff role is "button"
    <div
      tabIndex={p.action ? 0 : undefined}
      role={p.action ? "button" : undefined}
      data-testid={p.objectId != null ? `hand-card-${p.objectId}` : undefined}
      data-action-kind={p.action?.kind ?? undefined}
      data-action-id={p.action != null ? String(p.action.id) : undefined}
      data-needs-target={p.action?.needs_target ? "1" : "0"}
      data-has-player-target={p.action?.targets?.some((t) => t.kind === "player") ? "1" : "0"}
      data-has-object-target={p.action?.targets?.some((t) => t.kind === "object") ? "1" : "0"}
      data-object-kind={p.objectKind}
      aria-label={p.action ? (p.caption ? `${p.name}: ${p.caption}` : p.name) : undefined}
      onKeyDown={(e) => {
        if (!p.action || (e.key !== "Enter" && e.key !== " ")) return;
        e.preventDefault(); // Space must not scroll the page
        activate(p.action);
      }}
      style={{ "--fan": p.fan }}
      // `transition-transform` re-settles the fan smoothly when a card leaves the row.
      class="pointer-events-auto relative origin-bottom transition-transform duration-[120ms] [transform:var(--fan,none)]"
    >
      <img
        src={imageUrlByPrint(p.print)}
        alt={p.name}
        draggable={false}
        onPointerDown={(e) => p.action && onDown(p.action, p.name, p.print, e)}
        onPointerMove={() => {
          setHoverCard({ name: p.name, cardId: p.cardId, print: p.print });
          setHoverAction(p.action);
        }}
        onPointerLeave={() => {
          if (hover() === p.name) setHoverCard(null);
          if (!drag()) setHoverAction(null);
        }}
        class={cn(
          CARD_FACE,
          "-ml-6 cursor-default touch-none shadow-hand transition-transform duration-[80ms]",
          p.action && "cursor-grab",
          dimmedness(p),
        )}
      />
      <Show when={p.caption}>
        <div class="pointer-events-none absolute right-0 bottom-2 -left-6 mx-1.5 overflow-hidden text-ellipsis whitespace-nowrap rounded-[5px] bg-[#0a100edb] px-1 py-0.5 text-center font-semibold text-micro text-snow">
          {p.caption}
        </div>
      </Show>
    </div>
  );

  return (
    <>
      <div
        data-testid="hand-bar"
        style={{ "--bar-h": `${HAND_BAR_H}px` }}
        class="pointer-events-none fixed right-0 bottom-0 left-0 flex h-(--bar-h) items-end justify-center gap-lg px-3 pb-2"
      >
        <Section label="Hand">
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
                    action={slot.action}
                    caption={actionCaption(slot.action.kind)}
                    fan={fanTransform(i(), count())}
                  />
                );
              }
              const action = () => handActionByObject().get(slot.card.id) ?? null;
              const dimmed = () => !action() || slot.card.id === props.hiddenId;
              const caption = () => actionCaption(action()?.kind ?? "");
              return (
                <BarCard
                  name={slot.card.name}
                  print={slot.card.print ?? ""}
                  cardId={slot.card.card_id || undefined}
                  objectId={slot.card.id}
                  objectKind={slot.card.kind.kind}
                  action={action()}
                  dimmed={dimmed()}
                  caption={caption()}
                  fan={fanTransform(i(), count())}
                />
              );
            }}
          </For>
        </Section>
        <Show when={commandCards().length > 0}>
          <Section label="Command" divider>
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
                    action={action()}
                    dimmed={!action()}
                    caption={card.is_commander && commanderTax() > 0 ? `Tax +{${commanderTax()}}` : undefined}
                    fan={fanTransform(i(), commandCards().length)}
                  />
                );
              }}
            </For>
          </Section>
        </Show>
        <ZoneSection label="Graveyard" actions={grouped().graveyard} name={(a) => a.label} />
        <ZoneSection label="Exile" actions={grouped().exile} name={(a) => a.label} />
      </div>
      <Show when={drag()}>
        {(d) => (
          <img
            src={imageUrlByPrint(d().print)}
            alt={d().name}
            draggable={false}
            style={{ "--x": `${d().x}px`, "--y": `${d().y}px` }}
            class={cn(
              CARD_FACE,
              "pointer-events-none fixed top-(--y) left-(--x) z-20 -translate-x-1/2 -translate-y-1/2 drop-shadow-drag",
            )}
          />
        )}
      </Show>
    </>
  );

  // A non-hand section: one draggable card per action, shown only when it has any. `name` picks the
  // image (card name for zone casts, source-permanent name for abilities); `caption` overlays the
  // ability label. Declared inside the component so it closes over BarCard/onDown.
  function ZoneSection(p: {
    label: string;
    actions: ActionView[];
    name: (a: ActionView) => string;
    caption?: boolean;
  }) {
    return (
      <Show when={p.actions.length > 0}>
        <Section label={p.label} divider>
          <For each={p.actions}>
            {(a, i) => {
              const meta = objectMeta(a.object);
              return (
                <BarCard
                  name={p.name(a)}
                  print={meta.print}
                  cardId={meta.cardId}
                  action={a}
                  caption={p.caption ? a.label : undefined}
                  fan={fanTransform(i(), p.actions.length)}
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

// A labelled group of bar cards with a small all-caps caption; `divider` draws the thin separator
// that visually splits it from the section to its left.
function Section(props: { label: string; divider?: boolean; children: JSX.Element }) {
  return (
    <div class={cn("flex flex-col items-center justify-end gap-1", props.divider && "border-white/14 border-l pl-lg")}>
      <div class="flex items-end">{props.children}</div>
      <div class="pointer-events-none font-semibold text-lichen text-micro uppercase tracking-[0.09em]">
        {props.label}
      </div>
    </div>
  );
}
