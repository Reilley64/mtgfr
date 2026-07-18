// Table-driven prompt forms — the kind list used to be copied twice (once in the wire type, once
// as a hand-written <Match> chain in Board.tsx's PromptModal). FORMS is the single place it now
// lives: `Record<PendingChoiceView["kind"], ...>` means adding a wire kind without adding a form
// here is a TypeScript build failure, not a silent "Unhandled choice" caught only at runtime.

import { type Component, createMemo, createSignal, For, type JSX, onCleanup, onMount, Show } from "solid-js";
import { Button, Field } from "~/components/atoms";
import { InspectDock } from "~/components/molecules/card-preview";
import { seatColor } from "~/layout";
import type { AnswerInput } from "~/lib/choice";
import { chooseTargetIsCardPick } from "~/lib/choice";
import { cn } from "~/lib/cn";
import { type InspectPin, pinFromHit } from "~/lib/inspect";
import { modeAvailable } from "~/lib/modal";
import { openModalWhenReady } from "~/lib/modalDialog";
import { cardPickIsSearchable, filterChoiceItems, searchableChoiceItems } from "~/lib/promptForm";
import { imageUrlByPrint } from "~/lib/scryfall";
import {
  choiceItemPrint,
  mayYesNoTitle,
  objectName,
  objectPrint,
  payCostTitle,
  payEchoTitle,
  payOrCounterTitle,
  sourceHint,
  spellTargetsTitle,
} from "~/lib/targetPrompt";
import type { ChoiceItem, ModeView, PendingChoiceView, VisibleState, WireCost, WireTarget } from "~/wire/types";

export const PROMPT_TITLE = cn("mb-sm font-bold");
export const PROMPT_ROW = cn("my-1 flex flex-wrap gap-xs");

export function costText(cost: WireCost): string {
  const pips = ["W", "U", "B", "R", "G"]
    .map((c, i) => (cost.colored[i] > 0 ? `${c}×${cost.colored[i]}` : ""))
    .filter(Boolean)
    .join(" ");
  return `{${cost.generic}} ${pips}`.trim();
}

/** Props every table-driven form receives. `pc` stays the full union — Solid components can't
 * destructure props (breaks reactivity), so each form narrows it to its own kind with a small,
 * localized `as`/`Extract` cast instead of PromptModal narrowing it up front. */
export type FormProps = {
  pc: PendingChoiceView;
  state: VisibleState;
  onAnswer: (a: AnswerInput) => void;
};

type Narrow<K extends PendingChoiceView["kind"]> = Extract<PendingChoiceView, { kind: K }>;

const OrderForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"order_triggers">;
  return <OrderPrompt labels={pc().labels} onSubmit={(order) => props.onAnswer({ kind: "order", order })} />;
};

function OrderPrompt(props: { labels: string[]; onSubmit: (order: number[]) => void }) {
  // `order` holds original-effect indices in stacking order: the last row is pushed onto the
  // stack last, so it resolves first (CR 603.3b).
  const [order, setOrder] = createSignal(props.labels.map((_, i) => i));
  const move = (pos: number, delta: number) => {
    const target = pos + delta;
    if (target < 0 || target >= order().length) return;
    setOrder((o) => {
      const next = [...o];
      [next[pos], next[target]] = [next[target], next[pos]];
      return next;
    });
  };
  return (
    <div>
      <div class={PROMPT_TITLE}>Order these triggers — the last one resolves first</div>
      <div class="my-1 flex flex-col items-stretch gap-xs">
        <For each={order()}>
          {(effectIndex, pos) => (
            <div class="flex items-center gap-xs">
              <Button type="button" disabled={pos() === 0} onClick={() => move(pos(), -1)} variant="ghost">
                ↑
              </Button>
              <Button
                type="button"
                disabled={pos() === order().length - 1}
                onClick={() => move(pos(), 1)}
                variant="ghost"
              >
                ↓
              </Button>
              <span>{props.labels[effectIndex]}</span>
            </div>
          )}
        </For>
      </div>
      <Button type="button" onClick={() => props.onSubmit(order())}>
        Submit
      </Button>
    </div>
  );
}

const ChooseTargetForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_target">;
  const hint = () => sourceHint(props.state, pc().source, pc().label);
  const seatLabel = (seat: number, fallback: string) => {
    const name = props.state.players.find((p) => p.player === seat)?.username?.trim();
    return name || fallback;
  };
  return (
    <Show
      when={chooseTargetIsCardPick(pc().items)}
      fallback={
        // Player seats (and mixed player+object lists) need a full-screen dialog: choose_target
        // skips PromptModal's panel chrome, so a bare button row would never appear on screen.
        <PickDialog label={pc().label}>
          <div class={PICK_COLUMN}>
            <div class="text-snow text-title">{pc().label}</div>
            <Show when={hint()}>
              <div class="text-label text-mist">From {hint()}</div>
            </Show>
            <div class="flex max-w-[min(90vw,1040px)] flex-wrap justify-center gap-3">
              <For each={pc().items}>
                {(it) => (
                  <button
                    type="button"
                    aria-label={it.label}
                    onClick={() =>
                      props.onAnswer({
                        kind: "target",
                        id: it.id,
                        player: it.player ?? undefined,
                      })
                    }
                    class="relative cursor-pointer rounded-[9px] p-0 shadow-hand transition-transform duration-150 ease-out hover:-translate-y-2"
                  >
                    {/* Wrap seat in an object so Show doesn't treat seat 0 as falsy. */}
                    <Show
                      when={it.player != null ? { seat: it.player as number } : null}
                      fallback={
                        <img
                          src={imageUrlByPrint(choiceItemPrint(props.state, it))}
                          alt={it.label}
                          draggable={false}
                          width={150}
                          class="block rounded-[9px]"
                        />
                      }
                    >
                      {(p) => (
                        <div
                          style={{ "--seat": seatColor(p().seat, 0.9) }}
                          class="flex aspect-[150/209] w-[150px] flex-col items-center justify-center rounded-[9px] border-(--seat) border-4 bg-morph-slate font-bold text-snow text-title"
                        >
                          {seatLabel(p().seat, it.label)}
                        </div>
                      )}
                    </Show>
                  </button>
                )}
              </For>
            </div>
          </div>
        </PickDialog>
      }
    >
      <CardPickPrompt
        state={props.state}
        title={pc().label}
        hint={hint() ? `From ${hint()}` : undefined}
        submitLabel="Choose"
        items={pc().items}
        count={1}
        onSubmit={(ids) => props.onAnswer({ kind: "target", id: ids[0] })}
      />
    </Show>
  );
};

const ChooseSpellTargetsForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_spell_targets">;
  const title = () => spellTargetsTitle(pc().label, pc().min, pc().max);
  const fixed = () => pc().min === pc().max;
  const hint = () =>
    fixed()
      ? undefined
      : pc().max >= 99
        ? `Select at least ${pc().min} distinct target${pc().min === 1 ? "" : "s"}`
        : `Select ${pc().min}–${pc().max} distinct targets`;
  return (
    <CardPickPrompt
      state={props.state}
      title={title()}
      hint={hint()}
      submitLabel="Choose targets"
      items={pc().items}
      count={fixed() ? pc().min : null}
      minCount={fixed() ? undefined : pc().min}
      maxCount={fixed() ? undefined : pc().max}
      onSubmit={(ids) => props.onAnswer({ kind: "targets", ids })}
    />
  );
};

const MayForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"may_yes_no">;
  const title = () => mayYesNoTitle(objectName(props.state, pc().source), pc().label);
  return (
    <div>
      <div class={PROMPT_TITLE}>{title()}</div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "may", yes: true })}>
          Yes
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "may", yes: false })} variant="ghost">
          No
        </Button>
      </div>
    </div>
  );
};

const PayForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"pay_cost">;
  const title = () => payCostTitle(objectName(props.state, pc().source), costText(pc().cost), pc().label);
  return (
    <div>
      <div class={PROMPT_TITLE}>{title()}</div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: true })}>
          Pay {costText(pc().cost)}
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: false })} variant="ghost">
          Decline
        </Button>
      </div>
    </div>
  );
};

// Pay-or-counter reuses PayForm's shape (same `pay` answer → pay_optional_cost intent; the engine
// routes it to the PayOrCounter handler based on the pending choice), only the copy differs.
const PayOrCounterForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"pay_or_counter">;
  const title = () => payOrCounterTitle(objectName(props.state, pc().spell), costText(pc().cost));
  return (
    <div>
      <div class={PROMPT_TITLE}>{title()}</div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: true })}>
          Pay {costText(pc().cost)}
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: false })} variant="ghost">
          Let it be countered
        </Button>
      </div>
    </div>
  );
};

// Mirrors SearchLibraryForm: pick one offered land to put onto the battlefield, or decline.
const PutLandForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"put_land_from_hand">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Put a land onto the battlefield?"
      submitLabel="Put onto the battlefield"
      declineLabel="Decline"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "put_land", choice: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "put_land", choice: null })}
    />
  );
};

// Mirrors PutLandForm: pick one offered creature card from hand to put onto the battlefield, or
// decline (Cauldron Dance's "you may put a creature card from your hand onto the battlefield").
const PutCreatureForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"put_creature_from_hand">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Put a creature onto the battlefield?"
      submitLabel="Put onto the battlefield"
      declineLabel="Decline"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "put_creature", choice: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "put_creature", choice: null })}
    />
  );
};

// Mirrors PutLandForm: pick one card exiled with the source (Currency Converter's cash-out) to
// put into its owner's graveyard, or decline.
const ChooseExiledForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_exiled_with_card">;
  const title = () => {
    const source = objectName(props.state, pc().source);
    return `Put a card exiled with ${source} into its owner's graveyard?`;
  };
  return (
    <CardPickPrompt
      state={props.state}
      title={title()}
      submitLabel="Put into graveyard"
      declineLabel="Decline"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "choose_exiled", choice: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "choose_exiled", choice: null })}
    />
  );
};

const AssignDamageForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"assign_combat_damage">;
  const power = () => props.state.objects.find((o) => o.id === pc().source)?.power ?? 0;
  return (
    <AssignDamagePrompt
      items={pc().items}
      power={power()}
      onSubmit={(assignment) => props.onAnswer({ kind: "assign", assignment })}
    />
  );
};

function AssignDamagePrompt(props: {
  items: ChoiceItem[];
  power: number;
  onSubmit: (assignment: { blocker: number; amount: number }[]) => void;
}) {
  // Default: pile all the damage onto the first blocker — a valid split (sums to power) so the
  // player can just click Assign, then tweak if they want to spread it.
  const [amounts, setAmounts] = createSignal<Record<number, number>>(
    props.items.length > 0 ? { [props.items[0].id]: props.power } : {},
  );
  const assigned = () => Object.values(amounts()).reduce((s, n) => s + n, 0);
  return (
    <div>
      <div class={PROMPT_TITLE}>Divide {props.power} damage among blockers</div>
      <For each={props.items}>
        {(it) => (
          <div class={PROMPT_ROW}>
            <span class="w-[120px]">{it.label}</span>
            <Field
              type="number"
              min="0"
              value={amounts()[it.id] ?? 0}
              onInput={(e) => setAmounts((a) => ({ ...a, [it.id]: parseInt(e.currentTarget.value, 10) || 0 }))}
              class="w-14"
            />
          </div>
        )}
      </For>
      <div class={cn("my-1 text-caution-amber", assigned() === props.power && "text-assign-clover")}>
        assigned {assigned()} / {props.power}
      </div>
      <Button
        type="button"
        disabled={assigned() !== props.power}
        onClick={() => props.onSubmit(props.items.map((it) => ({ blocker: it.id, amount: amounts()[it.id] ?? 0 })))}
      >
        Assign
      </Button>
    </div>
  );
}

// Scry/surveil share one form: click a card to keep it on top (in click order); unclicked cards
// go to the bottom (scry) or graveyard (surveil) — the wire kind alone tells the server which.
const ArrangeTopForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"scry" | "surveil">;
  return (
    <CardPickPrompt
      state={props.state}
      title={pc().kind === "scry" ? `Scry ${pc().items.length}` : `Surveil ${pc().items.length}`}
      hint={`Click cards to keep on top, in that order — the rest go to ${pc().kind === "scry" ? "the bottom of your library" : "your graveyard"}.`}
      submitLabel="Done"
      items={pc().items}
      count={null}
      ordered
      onSubmit={(top) =>
        props.onAnswer({
          kind: "arrange",
          top,
          bottom: pc()
            .items.map((it) => it.id)
            .filter((id) => !top.includes(id)),
        })
      }
    />
  );
};

const SearchLibraryForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"search_library">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Search your library"
      submitLabel="Choose"
      declineLabel="Fail to find"
      items={pc().items}
      count={1}
      searchable={cardPickIsSearchable("search_library")}
      onSubmit={(ids) => props.onAnswer({ kind: "search", choice: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "search", choice: null })}
    />
  );
};

const SacrificeForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"sacrifice_edict">;
  const keepOne = () => pc().keep_one ?? false;
  const count = () => (keepOne() ? Math.max(0, pc().items.length - 1) : 1);
  return (
    <CardPickPrompt
      state={props.state}
      title={keepOne() ? "Choose permanents to sacrifice (keep one)" : "Choose a permanent to sacrifice"}
      submitLabel="Sacrifice"
      items={pc().items}
      count={count()}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

const DiscardForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"discard">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`Discard ${pc().count} card${pc().count === 1 ? "" : "s"}`}
      submitLabel="Discard"
      items={pc().items}
      count={pc().count}
      onSubmit={(cards) => props.onAnswer({ kind: "discard", cards })}
    />
  );
};

const SelectFromTopForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"select_from_top">;
  const upTo = () => pc().up_to;
  return (
    <CardPickPrompt
      state={props.state}
      title={`Select up to ${upTo()} from the top`}
      hint="Click cards to take — the rest go to the bottom."
      submitLabel="Done"
      items={pc().items}
      count={null}
      maxCount={upTo()}
      onSubmit={(cards) => props.onAnswer({ kind: "select_top", cards })}
    />
  );
};

const ChooseModeForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_mode">;
  const [mode, setMode] = createSignal<number | null>(null);
  const title = () => {
    const source = objectName(props.state, pc().source);
    return source ? `${source}: choose one` : "Choose one";
  };
  return (
    <div>
      <div class={PROMPT_TITLE}>{title()}</div>
      <div class="my-1 flex flex-col items-stretch gap-xs">
        <For each={pc().labels}>
          {(label, i) => (
            <Button
              type="button"
              aria-pressed={mode() === i()}
              onClick={() => setMode(i())}
              variant="ghost"
              class={cn("text-left", mode() === i() && "border-llanowar bg-llanowar/25")}
            >
              {label}
            </Button>
          )}
        </For>
      </div>
      <Button
        type="button"
        disabled={mode() === null}
        onClick={() => {
          const m = mode();
          if (m === null) return;
          props.onAnswer({ kind: "mode", mode: m });
        }}
      >
        Choose
      </Button>
    </div>
  );
};

/** Every full-screen decision surface, on the native `<dialog>` element — the same reason
 * `ConfirmDialog` uses one: `showModal()` supplies the focus trap, the inert background and the
 * top-layer stacking, so none of that is hand-rolled here. Nothing can float over the question the
 * game is waiting on, and a keyboard user can't tab out behind it.
 *
 * `onEscape` is Escape *and* the only way this closes itself. A prompt the game is genuinely parked
 * on (a pending choice — the engine will not proceed without an answer) passes none, and Escape then
 * does nothing; a prompt the player opened themselves (targeting, mode picking) passes its cancel.
 *
 * Always `dialog.close()` on cleanup: cost-pick prompts (discard / exile / sacrifice) unmount via
 * parent state, and removing an open `showModal()` dialog without closing it can leave the document
 * inert. Opening is deferred (`openModalWhenReady`) so a chained second picker — discard/exile
 * then an off-board cast target — is not racing that close in the same Solid flush.
 *
 * Rendered only while open — the `flex` display would otherwise override the UA's `display: none`
 * on a closed dialog and leave it on screen.
 *
 * `overflow-y-auto` so a big pick (15+ candidates) scrolls inside the backdrop rather than clipping,
 * and `m-auto` on the column centers it while it fits but scrolls from the top when it doesn't. */
function PickDialog(props: { label: string; onEscape?: () => void; children: JSX.Element }) {
  let dialog!: HTMLDialogElement;
  onMount(() => onCleanup(openModalWhenReady(dialog)));
  return (
    <dialog
      ref={dialog}
      aria-label={props.label}
      onCancel={(e) => {
        e.preventDefault(); // `open` is owned by the caller's state; never self-close behind its back
        props.onEscape?.();
      }}
      class="fixed inset-0 m-0 flex h-full max-h-none w-full max-w-none overflow-y-auto bg-black/55 p-0 backdrop:bg-transparent"
    >
      {props.children}
    </dialog>
  );
}

const PICK_COLUMN = cn("m-auto flex flex-col items-center gap-xl py-xxl");

/** Pick one target — a card in any zone, or a *player*. Distinct from `CardPickPrompt` because a
 * seat has no card image: it renders as its own life-orb-coloured tile. The board uses this wherever
 * the targeting arrow can't reach (a graveyard pile, the stack overlay) and for every mode of a
 * modal spell, whose targets travel per mode (CR 700.2).
 *
 * Object art resolves via `objectPrint(state, id)` — targets are always in `state.objects`
 * (battlefield / hand / GY / exile / stack), never library-private ids. */
export function TargetPickPrompt(props: {
  title: string;
  targets: WireTarget[];
  state: VisibleState;
  /** Name a player target; defaults to P{n}. */
  playerName?: (seat: number) => string;
  onPick: (target: WireTarget) => void;
  onCancel: () => void;
}) {
  const seatLabel = (seat: number) => props.playerName?.(seat) ?? `P${seat}`;
  return (
    <PickDialog label={props.title} onEscape={props.onCancel}>
      <div class={PICK_COLUMN}>
        <div class="text-snow text-title">{props.title}</div>
        <div class="flex max-w-[min(90vw,1040px)] flex-wrap justify-center gap-3">
          <For each={props.targets}>
            {(t) => (
              <button
                type="button"
                aria-label={
                  t.kind === "player"
                    ? `Player ${seatLabel((t as Extract<WireTarget, { kind: "player" }>).player)}`
                    : objectName(props.state, t.id)
                }
                onClick={() => props.onPick(t)}
                class="relative cursor-pointer rounded-[9px] p-0 shadow-hand transition-transform duration-150 ease-out hover:-translate-y-2"
              >
                <Show
                  when={t.kind === "object" && t}
                  fallback={
                    // A seat, drawn as its life orb is on the canvas — same colour, same name.
                    <div
                      style={{ "--seat": seatColor((t as Extract<WireTarget, { kind: "player" }>).player, 0.9) }}
                      class="flex aspect-[150/209] w-[150px] flex-col items-center justify-center rounded-[9px] border-(--seat) border-4 bg-morph-slate font-bold text-snow text-title"
                    >
                      {seatLabel((t as Extract<WireTarget, { kind: "player" }>).player)}
                    </div>
                  }
                >
                  {(obj) => (
                    <img
                      src={imageUrlByPrint(objectPrint(props.state, obj().id))}
                      alt=""
                      draggable={false}
                      class="block aspect-[150/209] w-[150px] rounded-[9px] bg-morph-slate"
                    />
                  )}
                </Show>
              </button>
            )}
          </For>
        </div>
        <Button type="button" onClick={props.onCancel} variant="ghost">
          Cancel
        </Button>
      </div>
    </PickDialog>
  );
}

/** Choose the modes of a modal spell (CR 700.2): between `choose` and `chooseMax` distinct modes.
 * A mode that wants a target but has none legal right now can't be chosen, and says so. */
export function ModePickPrompt(props: {
  name: string;
  choose: number;
  chooseMax: number;
  modes: ModeView[];
  onSubmit: (indices: number[]) => void;
  onCancel: () => void;
}) {
  const [picked, setPicked] = createSignal<number[]>([]);
  const toggle = (i: number) =>
    setPicked((p) => {
      if (p.includes(i)) return p.filter((x) => x !== i);
      if (p.length >= props.chooseMax) return p; // at the ceiling: unselect one first
      return [...p, i];
    });
  const ready = () => picked().length >= props.choose && picked().length <= props.chooseMax;
  const countHint = () =>
    props.choose === props.chooseMax ? `Choose ${props.choose}` : `Choose ${props.choose}–${props.chooseMax}`;
  return (
    <PickDialog label={props.name} onEscape={props.onCancel}>
      <div class={cn(PICK_COLUMN, "max-w-[560px]")}>
        <div class="text-snow text-title">{props.name}</div>
        <div class="-mt-2 text-label text-mist">{countHint()} —</div>
        <div class="flex w-full flex-col gap-sm">
          <For each={props.modes}>
            {(m, i) => (
              <button
                type="button"
                disabled={!modeAvailable(m)}
                aria-pressed={picked().includes(i())}
                onClick={() => toggle(i())}
                class={cn(
                  "flex cursor-pointer items-center gap-md rounded-hud border border-hud-edge bg-glass-dim px-lg py-md text-left text-snow",
                  picked().includes(i()) && "border-llanowar bg-llanowar/25",
                  !modeAvailable(m) && "cursor-not-allowed opacity-40",
                )}
              >
                <span class="font-bold text-lichen">{picked().indexOf(i()) >= 0 ? "✓" : "•"}</span>
                <span>
                  {m.label}
                  <Show when={!modeAvailable(m)}>
                    <span class="ml-sm text-caption text-caution-amber">(no legal target)</span>
                  </Show>
                </span>
              </button>
            )}
          </For>
        </div>
        <div class="flex gap-md">
          <Button type="button" disabled={!ready()} onClick={() => props.onSubmit(picked())}>
            Cast
          </Button>
          <Button type="button" onClick={props.onCancel} variant="ghost">
            Cancel
          </Button>
        </div>
      </div>
    </PickDialog>
  );
}

/** Full-screen card picker shared by every card-selection choice: the candidates as real card
 * faces in one flat centered row over a dimmed backdrop. Click (or Enter/Space) toggles a card in
 * and out of the selection; Submit sits at the bottom.
 *
 * - `count: n` — Submit unlocks at exactly n picked; a pick-one prompt swaps the selection on the
 *   next click instead of demanding an unselect first.
 * - `count: null` — any number is valid (scry/surveil keep-on-top); Submit is always live.
 * - `ordered` — selected cards wear a 1-based pick-order badge (click order = stacking order).
 * - `declineLabel`/`onDecline` — the "no thanks" path (fail to find, decline).
 * - `searchable` — autofocused name filter; also dedupes by face when `count === 1` (library
 *   tutors). Multi-pick searchable surfaces filter only, so two Forests stay distinct.
 *
 * Art is always `choiceItemPrint` → `imageUrlByPrint` (ADR 0031). Pass `state` so an empty
 * `item.print` can fall back to a visible object (rolling deploy); client-built items should set
 * `print` themselves. */
export function CardPickPrompt(props: {
  title: string;
  hint?: string;
  submitLabel: string;
  items: ChoiceItem[];
  /** When set, empty `item.print` falls back via `choiceItemPrint` (expand-only peers). */
  state?: VisibleState;
  count: number | null;
  /** When `count` is null, the minimum picks required before Submit unlocks. */
  minCount?: number;
  /** When `count` is null, the maximum picks allowed. */
  maxCount?: number;
  ordered?: boolean;
  /** Autofocused name filter; pick-one also dedupes by face (see `cardPickIsSearchable`). */
  searchable?: boolean;
  declineLabel?: string;
  onDecline?: () => void;
  onSubmit: (ids: number[]) => void;
}) {
  const artPrint = (it: ChoiceItem) => (props.state ? choiceItemPrint(props.state, it) : (it.print ?? ""));
  const [picked, setPicked] = createSignal<number[]>([]);
  const [query, setQuery] = createSignal("");
  const shown = createMemo(() => {
    if (!props.searchable) return props.items;
    // Pick-one: one face per name. Multi-pick keeps copies distinct.
    if (props.count === 1) return searchableChoiceItems(props.items, query());
    return filterChoiceItems(props.items, query());
  });
  const toggle = (id: number) =>
    setPicked((p) => {
      if (p.includes(id)) return p.filter((x) => x !== id);
      if (props.count === 1) return [id]; // pick-one: clicking another card just moves the pick
      const max = props.count ?? props.maxCount;
      if (max !== undefined && max !== null && p.length >= max) return p;
      return [...p, id];
    });
  const ready = () => {
    const n = picked().length;
    if (props.count !== null) return n === props.count;
    const min = props.minCount ?? 0;
    const max = props.maxCount ?? Number.POSITIVE_INFINITY;
    return n >= min && n <= max;
  };
  // Alt-pin inspect (same dock as the board). Pin on Alt-down over a hovered card; release clears.
  const [inspectPin, setInspectPin] = createSignal<InspectPin | null>(null);
  const [hover, setHover] = createSignal<{ name: string; print?: string } | null>(null);
  const onAltDown = (e: KeyboardEvent) => {
    if (e.key !== "Alt") return;
    e.preventDefault();
    const h = hover();
    const pin = pinFromHit(true, h ? { name: h.name, print: h.print } : null, 2);
    if (pin) setInspectPin(pin);
  };
  const onAltUp = (e: KeyboardEvent) => {
    if (e.key === "Alt") setInspectPin(null);
  };
  const onEsc = (e: KeyboardEvent) => {
    if (e.key === "Escape" && inspectPin()) {
      e.stopImmediatePropagation();
      setInspectPin(null);
    }
  };
  window.addEventListener("keydown", onAltDown);
  window.addEventListener("keydown", onEsc, true);
  window.addEventListener("keyup", onAltUp);
  onCleanup(() => {
    window.removeEventListener("keydown", onAltDown);
    window.removeEventListener("keydown", onEsc, true);
    window.removeEventListener("keyup", onAltUp);
  });
  let searchInput: HTMLInputElement | undefined;
  onMount(() => {
    if (!props.searchable || !searchInput) return;
    // PickDialog opens via deferred `showModal()` (a microtask). rAF is after that, so the
    // filter is ready to type into even when the UA skips dialog autofocus handling.
    const id = requestAnimationFrame(() => searchInput?.focus());
    onCleanup(() => cancelAnimationFrame(id));
  });
  return (
    // No `onEscape`: this answers a *pending choice* — the engine will not proceed until it's
    // answered, so there is nothing to escape to. Decline, where the choice allows one, is a button.
    <PickDialog label={props.title}>
      <div class={PICK_COLUMN} data-testid="pick-prompt">
        <div class="text-snow text-title" data-testid="pick-title">
          {props.title}
        </div>
        <Show when={props.hint}>
          <div class="-mt-2 text-label text-mist">{props.hint}</div>
        </Show>
        <Show when={props.searchable}>
          <label for="card-pick-search" class="sr-only">
            Filter cards by name
          </label>
          <Field
            id="card-pick-search"
            type="search"
            autofocus
            ref={searchInput}
            placeholder="Filter by name…"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
            class="w-[min(90vw,320px)]"
          />
        </Show>
        {/* Flat row — deliberately no hand-bar fan: this is a decision surface, not the hand. */}
        <div class="flex max-w-[min(90vw,1040px)] flex-wrap justify-center gap-3">
          <For each={shown()}>
            {(it) => {
              const selected = () => picked().includes(it.id);
              const order = () => picked().indexOf(it.id);
              return (
                // Selected = vine ring + a lift; llanowar-family green is DESIGN.md's "selected".
                // `relative` anchors the pick-order badge; `p-0` sheds the UA chrome so only the
                // card art shows.
                <button
                  type="button"
                  data-testid={`pick-card-${it.id}`}
                  aria-pressed={selected()}
                  aria-label={it.label}
                  onClick={() => toggle(it.id)}
                  onPointerMove={() => setHover({ name: it.label, print: artPrint(it) })}
                  onPointerLeave={() => setHover((h) => (h?.name === it.label ? null : h))}
                  class={cn(
                    "relative cursor-pointer rounded-[9px] p-0 shadow-hand transition-[transform,box-shadow] duration-150 ease-out",
                    selected() && "-translate-y-2 shadow-pick",
                  )}
                >
                  {/* Fixed aspect ratio + card-back slate behind the loading image, so the row never
                      reflows and a still-loading card reads as a card, not a hole. */}
                  <img
                    src={imageUrlByPrint(artPrint(it))}
                    alt=""
                    draggable={false}
                    class="block aspect-[150/209] w-[150px] rounded-[9px] bg-morph-slate"
                  />
                  <Show when={props.ordered && selected()}>
                    {/* 1-based pick-order chip (scry/surveil): click order = stacking order. */}
                    <div class="absolute top-1.5 left-1.5 flex h-[22px] min-w-[22px] items-center justify-center rounded-full bg-llanowar font-bold text-caption text-snow-mint">
                      {order() + 1}
                    </div>
                  </Show>
                </button>
              );
            }}
          </For>
          <Show when={props.searchable && query().trim() !== "" && shown().length === 0}>
            <div class="text-label text-mist">No cards match.</div>
          </Show>
        </div>
        <Show when={props.count !== null}>
          <div class="text-caption text-mist" data-testid="pick-count">
            {picked().length} / {props.count} selected
          </div>
        </Show>
        <Show when={props.count === null && (props.minCount !== undefined || props.maxCount !== undefined)}>
          <div class="text-caption text-mist" data-testid="pick-count">
            {picked().length}
            {props.maxCount !== undefined ? ` / ${props.maxCount}` : ""} selected
            {props.minCount !== undefined && props.minCount > 0 ? ` (need at least ${props.minCount})` : ""}
          </div>
        </Show>
        <div class="flex gap-md">
          <Button type="button" data-testid="pick-submit" disabled={!ready()} onClick={() => props.onSubmit(picked())}>
            {props.submitLabel}
          </Button>
          <Show when={props.declineLabel}>
            <Button type="button" data-testid="pick-decline" onClick={() => props.onDecline?.()} variant="ghost">
              {props.declineLabel}
            </Button>
          </Show>
        </div>
      </div>
      <InspectDock pin={inspectPin()} onDismiss={() => setInspectPin(null)} />
    </PickDialog>
  );
}

// Proliferate (CR 701.27): any subset (0..all) of permanents you have counters on — Submit is
// always live (a 0-pick proliferate declines). Reuses the `sacrifice` answer; routing is by
// pending kind, not answer kind.
const ProliferateForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"proliferate">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Proliferate — choose any number"
      submitLabel="Proliferate"
      items={pc().items}
      count={null}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// Phase out any number of other creatures you control (CR 702.26) — empty declines.
const PhaseOutForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"phase_out">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: phase out any number of creatures`}
      submitLabel="Phase out"
      items={pc().items}
      count={null}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// Forced sacrifice of exactly `count` of your own permanents (no decline).
const ChooseOwnSacrificesForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_own_sacrifices">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`Choose ${pc().count} to sacrifice`}
      submitLabel="Sacrifice"
      items={pc().items}
      count={pc().count}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// Devour N: sacrifice any subset of other creatures as source enters (empty = 0 counters).
const DevourForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"devour">;
  const n = () => pc().multiplier;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: Devour ${n()} — sacrifice any number`}
      hint={`Each sacrificed creature puts ${n()} +1/+1 counter${n() === 1 ? "" : "s"} on it.`}
      submitLabel="Devour"
      items={pc().items}
      count={null}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// Tragic Arrogance-style: keep up to one of each type among target player's nonlands.
const CasterKeepForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"caster_keep_permanents">;
  const seat = () => props.state.players.find((p) => p.player === pc().target_player)?.username?.trim();
  const who = () => seat() || `P${pc().target_player}`;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: choose ${who()}'s permanents to keep`}
      hint="Keep up to one artifact, one creature, and one enchantment. The rest are sacrificed."
      submitLabel="Keep these"
      items={pc().items}
      count={null}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// Mandatory exile of exactly one card from your graveyard — declining is illegal while you have a
// card, so no decline button.
const ExileFromGraveyardForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"exile_from_graveyard">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Exile a card from your graveyard"
      submitLabel="Exile"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// The "you may pick zero or one card, then do X" family — sacrifice / return / reanimate / discard.
// All share the CardPickPrompt(count 1) + decline shape and the `sacrifice` answer (routing is by
// pending kind); only the title differs, so one factory covers all four.
const mayOneCardForm =
  (title: (props: FormProps) => string): Component<FormProps> =>
  (props) => {
    const pc = () => props.pc as Narrow<"may_sacrifice">; // every variant here carries `items`
    return (
      <CardPickPrompt
        state={props.state}
        title={title(props)}
        submitLabel="Choose"
        declineLabel="Decline"
        items={pc().items}
        count={1}
        onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
        onDecline={() => props.onAnswer({ kind: "sacrifice", ids: [] })}
      />
    );
  };

const MaySacrificeForm = mayOneCardForm(
  (props) => `Sacrifice a permanent (${objectName(props.state, (props.pc as Narrow<"may_sacrifice">).source)})?`,
);
const MayReturnFromGraveyardForm = mayOneCardForm(
  (props) =>
    `Return a card to your hand (${objectName(props.state, (props.pc as Narrow<"may_return_from_graveyard">).source)})?`,
);
const MayDiscardForm = mayOneCardForm(
  (props) => `Discard a card (${objectName(props.state, (props.pc as Narrow<"may_discard">).source)})?`,
);

// Echo (CR 702.28): pay the echo cost to keep the permanent, or sacrifice it. Same `pay` answer as
// PayOrCounterForm (the engine routes it to the echo handler by pending kind), only the copy differs.
const PayEchoOrSacrificeForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"pay_echo_or_sacrifice">;
  const title = () => payEchoTitle(objectName(props.state, pc().source), costText(pc().cost));
  return (
    <div>
      <div class={PROMPT_TITLE}>{title()}</div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: true })}>
          Pay {costText(pc().cost)}
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: false })} variant="ghost">
          Sacrifice it
        </Button>
      </div>
    </div>
  );
};

// Recover (CR 702.59): a creature died, so the recover card in the graveyard asks — pay the
// recover cost to return it to your hand, or exile it. Same `pay` answer as PayEchoOrSacrificeForm
// (the engine routes it to the recover handler by pending kind), only the copy differs.
const PayRecoverOrExileForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"pay_recover_or_exile">;
  return (
    <div>
      <div class={PROMPT_TITLE}>
        {objectName(props.state, pc().source)}: pay recover {costText(pc().cost)} to return it to your hand, or exile
        it?
      </div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: true })}>
          Pay {costText(pc().cost)}
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: false })} variant="ghost">
          Exile it
        </Button>
      </div>
    </div>
  );
};

const DivideSpellDamageForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"divide_spell_damage">;
  return (
    <DivideDamagePrompt
      items={pc().items}
      total={pc().total}
      onSubmit={(assignment) => props.onAnswer({ kind: "assign", assignment })}
    />
  );
};

// Divide N +1/+1 counters among targets (CR 601.2d — Grove's Bounty). Same shape and answer as
// DivideSpellDamageForm: the engine routes the AssignDamage wire to divide_counters by pending kind.
const DivideCountersForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"divide_counters">;
  return (
    <DivideDamagePrompt
      items={pc().items}
      total={pc().total}
      noun="counter"
      onSubmit={(assignment) => props.onAnswer({ kind: "assign", assignment })}
    />
  );
};

// Divide N damage among targets (CR 601.2d): every target gets ≥ 1, all targets covered, sum == N.
// Unlike combat's AssignDamagePrompt (pile-on-first is legal there), the ≥1-each rule means the
// default seeds each target at 1 and puts the remainder on the first. `noun` names what's divided
// (defaults to "damage"; "counter" for divide_counters).
function DivideDamagePrompt(props: {
  items: ChoiceItem[];
  total: number;
  noun?: string;
  onSubmit: (assignment: { blocker: number; amount: number }[]) => void;
}) {
  const seed = (): Record<number, number> => {
    const out: Record<number, number> = {};
    props.items.forEach((it, i) => {
      out[it.id] = i === 0 ? props.total - (props.items.length - 1) : 1;
    });
    return out;
  };
  const [amounts, setAmounts] = createSignal<Record<number, number>>(seed());
  const assigned = () => Object.values(amounts()).reduce((s, n) => s + n, 0);
  const legal = () => assigned() === props.total && props.items.every((it) => (amounts()[it.id] ?? 0) >= 1);
  return (
    <div>
      <div class={PROMPT_TITLE}>
        Divide {props.total} {props.noun ?? "damage"}
        {props.noun ? (props.total === 1 ? "" : "s") : ""} among targets
      </div>
      <For each={props.items}>
        {(it) => (
          <div class={PROMPT_ROW}>
            <span class="w-[120px]">{it.label}</span>
            <Field
              type="number"
              min="1"
              value={amounts()[it.id] ?? 0}
              onInput={(e) => setAmounts((a) => ({ ...a, [it.id]: parseInt(e.currentTarget.value, 10) || 0 }))}
              class="w-14"
            />
          </div>
        )}
      </For>
      <div class={cn("my-1 text-caution-amber", legal() && "text-assign-clover")}>
        assigned {assigned()} / {props.total}
      </div>
      <Button
        type="button"
        disabled={!legal()}
        onClick={() => props.onSubmit(props.items.map((it) => ({ blocker: it.id, amount: amounts()[it.id] ?? 0 })))}
      >
        Assign
      </Button>
    </div>
  );
}

// Choose any-number of target players (Priest of Forgotten Gods): seat tiles, multi-select toggle,
// gated min..max. Player ids ride in ChoiceItem.player (codegen types it `never`; read `as number`).
const ChooseTargetPlayersForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_target_players">;
  const seatLabel = (seat: number, fallback: string) => {
    const name = props.state.players.find((p) => p.player === seat)?.username?.trim();
    return name || fallback;
  };
  const [picked, setPicked] = createSignal<number[]>([]);
  const toggle = (seat: number) => setPicked((p) => (p.includes(seat) ? p.filter((x) => x !== seat) : [...p, seat]));
  const ready = () => {
    const n = picked().length;
    return n >= pc().min && n <= pc().max;
  };
  return (
    <PickDialog label={pc().label}>
      <div class={PICK_COLUMN}>
        <div class="text-snow text-title">{pc().label}</div>
        <div class="-mt-2 text-label text-mist">
          Choose {pc().min === pc().max ? pc().min : `${pc().min}–${pc().max}`} target players
        </div>
        <div class="flex max-w-[min(90vw,1040px)] flex-wrap justify-center gap-3">
          <For each={pc().items}>
            {(it) => {
              // ChoiceItem.player is codegen-typed `never`; every item in this list is a seat.
              const seat = () => it.player as unknown as number;
              const selected = () => picked().includes(seat());
              return (
                <button
                  type="button"
                  aria-pressed={selected()}
                  aria-label={it.label}
                  onClick={() => toggle(seat())}
                  class={cn(
                    "relative cursor-pointer rounded-[9px] p-0 shadow-hand transition-transform duration-150 ease-out",
                    selected() && "-translate-y-2 shadow-pick",
                  )}
                >
                  <div
                    style={{ "--seat": seatColor(seat(), 0.9) }}
                    class="flex aspect-[150/209] w-[150px] flex-col items-center justify-center rounded-[9px] border-(--seat) border-4 bg-morph-slate font-bold text-snow text-title"
                  >
                    {seatLabel(seat(), it.label)}
                  </div>
                </button>
              );
            }}
          </For>
        </div>
        <div class="text-caption text-mist">{picked().length} selected</div>
        <Button
          type="button"
          disabled={!ready()}
          onClick={() => props.onAnswer({ kind: "target_players", players: picked() })}
        >
          Confirm
        </Button>
      </div>
    </PickDialog>
  );
};

// Shuffle any subset of your graveyard back into your library (Gaea's Blessing-style).
const ShuffleFromGraveyardForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"shuffle_from_graveyard">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Shuffle cards from your graveyard into your library"
      submitLabel="Shuffle in"
      items={pc().items}
      count={null}
      onSubmit={(cards) => props.onAnswer({ kind: "shuffle_gy", cards })}
    />
  );
};

// Cast one card exiled with this for free, or decline. Two near-identical prompts (cast vs. dig)
// share this factory — only the title and answer tag differ.
const chooseExiledCastForm =
  (titleOf: (props: FormProps) => string, tag: "choose_exiled_cast" | "choose_exiled_dig"): Component<FormProps> =>
  (props) => {
    const pc = () => props.pc as Narrow<"choose_exiled_with_card_to_cast">;
    return (
      <CardPickPrompt
        state={props.state}
        title={titleOf(props)}
        submitLabel="Cast for free"
        declineLabel="Decline"
        items={pc().items}
        count={1}
        onSubmit={(ids) => props.onAnswer({ kind: tag, choice: ids[0] })}
        onDecline={() => props.onAnswer({ kind: tag, choice: null })}
      />
    );
  };

const ChooseExiledToCastForm = chooseExiledCastForm(
  (props) =>
    `Cast a card exiled with ${objectName(props.state, (props.pc as Narrow<"choose_exiled_with_card_to_cast">).source)} for free?`,
  "choose_exiled_cast",
);
const ChooseExiledDigForm = chooseExiledCastForm(
  (props) =>
    `Cast a card exiled with ${objectName(props.state, (props.pc as Narrow<"choose_exiled_dig_to_cast_free">).source)} for free?`,
  "choose_exiled_dig",
);

const COLOR_PIPS = ["W", "U", "B", "R", "G"];

// Add N mana of one color (CR 106.4): pick a single WUBRG index (0=W … 4=G, per Color::index).
const ChooseManaColorForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_mana_color">;
  return (
    <div>
      <div class={PROMPT_TITLE}>Add {pc().amount} mana of one color</div>
      <div class={PROMPT_ROW}>
        <For each={COLOR_PIPS}>
          {(c, i) => (
            <Button type="button" onClick={() => props.onAnswer({ kind: "mana_color", color: i() })}>
              {c}
            </Button>
          )}
        </For>
      </div>
    </div>
  );
};

// Choose one creature type from a bounded list (Xenograft-style). Plain button column — the option
// list is short, so no search box (YAGNI).
const ChooseCreatureTypeForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_creature_type">;
  const [sel, setSel] = createSignal<string | null>(null);
  return (
    <div>
      <div class={PROMPT_TITLE}>Choose a creature type</div>
      <div class="my-1 flex max-h-[40vh] flex-col items-stretch gap-xs overflow-y-auto">
        <For each={pc().options}>
          {(opt) => (
            <Button
              type="button"
              aria-pressed={sel() === opt}
              onClick={() => setSel(opt)}
              variant="ghost"
              class={cn("text-left", sel() === opt && "border-llanowar bg-llanowar/25")}
            >
              {opt}
            </Button>
          )}
        </For>
      </div>
      <Button
        type="button"
        disabled={sel() === null}
        onClick={() => {
          const s = sel();
          if (s === null) return;
          props.onAnswer({ kind: "creature_type", subtype: s });
        }}
      >
        Choose
      </Button>
    </div>
  );
};

// As-enters choose a color (Flickering Ward) — same WUBRG buttons as ChooseManaColorForm.
const ChooseColorForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_color">;
  return (
    <div>
      <div class={PROMPT_TITLE}>{objectName(props.state, pc().source)}: choose a color</div>
      <div class={PROMPT_ROW}>
        <For each={COLOR_PIPS}>
          {(c, i) => (
            <Button type="button" onClick={() => props.onAnswer({ kind: "color", color: i() })}>
              {c}
            </Button>
          )}
        </For>
      </div>
    </div>
  );
};

// Dance with Calamity push-your-luck: exile another top card or stop. Reuses AnswerMay.
const DanceExileMoreForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"dance_exile_more">;
  return (
    <div>
      <div class={PROMPT_TITLE}>
        {objectName(props.state, pc().source)}: exile another? ({pc().total_mv}/{pc().budget} MV)
      </div>
      <Show when={pc().items.length > 0}>
        <div class="my-1 text-caption text-mist">
          Exiled so far:{" "}
          {pc()
            .items.map((it) => it.label)
            .join(", ")}
        </div>
      </Show>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "may", yes: true })}>
          Exile another
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "may", yes: false })} variant="ghost">
          Stop
        </Button>
      </div>
    </div>
  );
};

// Abstract Performance: opponent picks which exile pile goes to the graveyard.
const OpponentChoosesPileForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"opponent_chooses_pile">;
  const pileLabel = (items: ChoiceItem[]) => (items.length === 0 ? "(empty)" : items.map((it) => it.label).join(", "));
  return (
    <div>
      <div class={PROMPT_TITLE}>{objectName(props.state, pc().source)}: choose a pile for the graveyard</div>
      <div class="my-1 flex flex-col items-stretch gap-xs">
        <Button
          type="button"
          onClick={() => props.onAnswer({ kind: "opponent_pile", pile: 0 })}
          variant="ghost"
          class="text-left"
        >
          Pile A — {pileLabel(pc().pile_a)}
        </Button>
        <Button
          type="button"
          onClick={() => props.onAnswer({ kind: "opponent_pile", pile: 1 })}
          variant="ghost"
          class="text-left"
        >
          Pile B — {pileLabel(pc().pile_b)}
        </Button>
      </div>
    </div>
  );
};

// Plargg and Nassari: opponent must pick one exiled nonland (no decline).
const OpponentChoosesExiledNonlandForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"opponent_chooses_exiled_nonland">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: choose an exiled nonland`}
      submitLabel="Choose"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "choose_exiled", choice: ids[0] })}
    />
  );
};

// Cast up to `count` of these exiled cards for free; the rest route per the card.
const ChooseExiledToCastFreeForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_exiled_to_cast_free">;
  const count = () => pc().count;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: cast up to ${count()} for free`}
      submitLabel="Cast for free"
      items={pc().items}
      count={null}
      maxCount={count()}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// Just-revealed library card: put onto the battlefield, or into hand instead.
const RevealedCardForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"revealed_card_to_battlefield_or_hand">;
  const item = () => pc().item;
  return (
    <div>
      <div class={PROMPT_TITLE}>Put {item().label} onto the battlefield?</div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "revealed", choice: item().id })}>
          Battlefield
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "revealed", choice: null })} variant="ghost">
          Put into hand
        </Button>
      </div>
    </div>
  );
};

// CR 303.4f: choose a host for an Aura put onto the battlefield without casting.
// Choose the host for an Aura/Equipment put onto the battlefield without casting it. An Aura's host
// is mandatory (`optional` false — no decline); Equipment's is optional (declining leaves it
// unattached, host null).
const ChooseAttachHostForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_attach_host">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`Attach ${objectName(props.state, pc().attachment)} to…`}
      submitLabel="Attach"
      declineLabel={pc().optional ? "Leave unattached" : undefined}
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "attach_host", host: ids[0] })}
      onDecline={pc().optional ? () => props.onAnswer({ kind: "attach_host", host: null }) : undefined}
    />
  );
};

// "You may have this permanent enter as a copy of a creature on the battlefield" (CR 706/707.2 —
// Altered Ego, Cursed Mirror). Pick one creature or decline the "you may".
const ChooseCopyTargetForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_copy_target">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`Have ${objectName(props.state, pc().source)} enter as a copy of…?`}
      submitLabel="Enter as a copy"
      declineLabel="Enter as itself"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "copy_target", copy: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "copy_target", copy: null })}
    />
  );
};

// Put a +1/+1 counter on up to one of a target player's creatures, or decline (Nils, Discipline
// Enforcer). The answering seat is the chooser (pc.player), not target_player. Answered like the
// 0-or-1 sacrifice choices (the `sacrifice` answer names the chosen creature; [] declines).
const ChooseCounterTargetForForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_counter_target_for_player">;
  const who = () =>
    props.state.players.find((p) => p.player === pc().target_player)?.username?.trim() || `P${pc().target_player}`;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: put a +1/+1 counter on one of ${who()}'s creatures?`}
      submitLabel="Add counter"
      declineLabel="Decline"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
      onDecline={() => props.onAnswer({ kind: "sacrifice", ids: [] })}
    />
  );
};

// Partition the looked-at cards into three exact-size buckets (hand / bottom / exile-may-play). Each
// card row carries a 3-way segmented control; Submit unlocks only when every bucket hits its slot
// size (a full partition — the three counts sum to items.length; `distribute_top` in the engine).
const DistributeTopForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"distribute_top">;
  type Slot = "hand" | "bottom" | "exile";
  const [slots, setSlots] = createSignal<Record<number, Slot>>({});
  const chosen = (slot: Slot) => pc().items.filter((it) => slots()[it.id] === slot);
  const target: Record<Slot, () => number> = {
    hand: () => pc().to_hand,
    bottom: () => pc().to_bottom,
    exile: () => pc().to_exile_may_play,
  };
  const ready = () =>
    (["hand", "bottom", "exile"] as Slot[]).every((s) => chosen(s).length === target[s]()) &&
    pc().items.every((it) => slots()[it.id] != null);
  const label: Record<Slot, string> = { hand: "Hand", bottom: "Bottom", exile: "Exile (may play)" };
  return (
    <PickDialog label="Distribute cards">
      <div class={PICK_COLUMN}>
        <div class="text-snow text-title">Distribute the looked-at cards</div>
        <div class="-mt-2 text-label text-mist">
          Hand {chosen("hand").length}/{pc().to_hand} · Bottom {chosen("bottom").length}/{pc().to_bottom} · Exile{" "}
          {chosen("exile").length}/{pc().to_exile_may_play}
        </div>
        <div class="flex max-w-[min(90vw,1040px)] flex-wrap justify-center gap-3">
          <For each={pc().items}>
            {(it) => (
              <div class="flex flex-col items-center gap-xs">
                <img
                  src={imageUrlByPrint(choiceItemPrint(props.state, it))}
                  alt={it.label}
                  draggable={false}
                  class="block aspect-[150/209] w-[150px] rounded-[9px] bg-morph-slate"
                />
                <div class="flex gap-xs">
                  <For each={["hand", "bottom", "exile"] as Slot[]}>
                    {(s) => (
                      <Button
                        type="button"
                        aria-pressed={slots()[it.id] === s}
                        onClick={() => setSlots((cur) => ({ ...cur, [it.id]: s }))}
                        variant="ghost"
                        class={cn("text-caption", slots()[it.id] === s && "border-llanowar bg-llanowar/25")}
                      >
                        {label[s]}
                      </Button>
                    )}
                  </For>
                </div>
              </div>
            )}
          </For>
        </div>
        <Button
          type="button"
          disabled={!ready()}
          onClick={() =>
            props.onAnswer({
              kind: "distribute",
              to_hand: chosen("hand").map((it) => it.id),
              to_bottom: chosen("bottom").map((it) => it.id),
              to_exile_may_play: chosen("exile").map((it) => it.id),
            })
          }
        >
          Confirm
        </Button>
      </div>
    </PickDialog>
  );
};

// Choose exactly `choose` distinct modes (or none, if `optional`), each paired with a DISTINCT
// player target (Shadrix Silverquill: "each mode targets a different player"). Every mode here
// needs a target and the target set is the living players, so each picked mode gets an inline seat
// row and Submit enforces pairwise-distinct seats. (Flagged ambiguous in the brief — went with the
// inline-per-mode-target layout; two-phase was the other option.)
const ChooseTriggerModesForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_trigger_modes">;
  // Chosen player seat per mode index (undefined = mode not picked).
  const [picks, setPicks] = createSignal<Record<number, number | undefined>>({});
  const pickedModes = () =>
    Object.keys(picks())
      .map(Number)
      .filter((i) => picks()[i] !== undefined);
  const seatLabel = (seat: number) => {
    const name = props.state.players.find((p) => p.player === seat)?.username?.trim();
    return name || `P${seat}`;
  };
  const toggleMode = (i: number) =>
    setPicks((cur) => {
      const next = { ...cur };
      if (i in next) delete next[i];
      else next[i] = undefined;
      return next;
    });
  const setSeat = (i: number, seat: number) => setPicks((cur) => ({ ...cur, [i]: seat }));
  const seats = () => pickedModes().map((i) => picks()[i]);
  const distinctSeats = () => new Set(seats()).size === seats().length;
  const ready = () => {
    const modes = pickedModes();
    if (modes.length === 0) return pc().optional;
    return modes.length === pc().choose && seats().every((s) => s !== undefined) && distinctSeats();
  };
  return (
    <PickDialog label="Choose modes">
      <div class={cn(PICK_COLUMN, "max-w-[620px]")}>
        <div class="text-snow text-title">
          Choose {pc().choose} mode{pc().choose === 1 ? "" : "s"}
        </div>
        <div class="-mt-2 text-label text-mist">Each chosen mode targets a different player</div>
        <div class="flex w-full flex-col gap-sm">
          <For each={pc().modes}>
            {(m, i) => {
              const on = () => i() in picks();
              return (
                <div
                  class={cn(
                    "rounded-hud border border-hud-edge bg-glass-dim p-md",
                    on() && "border-llanowar bg-llanowar/25",
                  )}
                >
                  <button
                    type="button"
                    aria-pressed={on()}
                    onClick={() => toggleMode(i())}
                    class="flex w-full items-center gap-md text-left text-snow"
                  >
                    <span class="font-bold text-lichen">{on() ? "✓" : "•"}</span>
                    <span>{m.label}</span>
                  </button>
                  <Show when={on()}>
                    <div class="mt-sm flex flex-wrap gap-xs">
                      <For each={m.targets}>
                        {(t) => {
                          const seat = () => (t as Extract<WireTarget, { kind: "player" }>).player;
                          return (
                            <Show when={t.kind === "player"}>
                              <Button
                                type="button"
                                aria-pressed={picks()[i()] === seat()}
                                onClick={() => setSeat(i(), seat())}
                                variant="ghost"
                                class={cn("text-caption", picks()[i()] === seat() && "border-llanowar bg-llanowar/25")}
                              >
                                {seatLabel(seat())}
                              </Button>
                            </Show>
                          );
                        }}
                      </For>
                    </div>
                  </Show>
                </div>
              );
            }}
          </For>
        </div>
        <Button
          type="button"
          disabled={!ready()}
          onClick={() =>
            props.onAnswer({
              kind: "trigger_modes",
              // ready() guarantees each picked mode has a seat, so the assertion is sound.
              modes: pickedModes().map((i) => {
                const seat = picks()[i];
                if (seat === undefined) throw new Error("unreachable: picked mode without a target");
                return { index: i, target: { kind: "player" as const, player: seat } };
              }),
            })
          }
        >
          Confirm
        </Button>
      </div>
    </PickDialog>
  );
};

/** kind → form component, one entry per `PendingChoiceView["kind"]`. `Record` over the union is
 * the compile-time exhaustiveness check the old `<Match>` chain lacked: add a wire kind, TS fails
 * the build right here until a form is wired up. */
// "You may choose not to untap this permanent during your untap step" (CR 502.2 — Rubinia):
// pick the permanents to KEEP TAPPED; submitting an empty selection untaps everything.
const DeclineUntapForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"decline_untap">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Untap step: choose permanents to keep tapped"
      hint="Anything not chosen untaps as normal."
      submitLabel="Confirm"
      items={pc().items}
      count={null}
      onSubmit={(ids) => props.onAnswer({ kind: "keep_tapped", ids })}
    />
  );
};

// Dredge (CR 702.52): about to draw with an eligible dredger in your graveyard — pick one to
// mill-and-return instead, or decline to draw normally. The mill count isn't in the wire label
// (the projection drops it), so the hint stays generic. ponytail: widen the projection if a
// per-dredger mill count in the label is wanted.
const ChooseDredgeForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_dredge">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Dredge instead of drawing?"
      hint="Mill this dredger's dredge value and return it from your graveyard instead of drawing, or draw normally."
      submitLabel="Dredge"
      declineLabel="Draw normally"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "dredge", dredger: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "dredge", dredger: null })}
    />
  );
};

// "Sacrifice it unless you pay …" as it enters (Rupture Spire). Same `pay` answer as
// PayEchoOrSacrificeForm; only the copy differs (this isn't echo).
const SacrificeUnlessPayForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"sacrifice_unless_pay">;
  return (
    <div>
      <div class={PROMPT_TITLE}>
        {objectName(props.state, pc().source)}: pay {costText(pc().cost)} or sacrifice it
      </div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: true })}>
          Pay {costText(pc().cost)}
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: false })} variant="ghost">
          Sacrifice it
        </Button>
      </div>
    </div>
  );
};

// Rhystic Study's punisher: the caster pays, or the enchantment's controller may draw.
const PayOrControllerDrawsForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"pay_or_controller_draws">;
  const who = () => {
    const name = props.state.players.find((pl) => pl.player === pc().controller)?.username?.trim();
    return name || `P${pc().controller}`;
  };
  return (
    <div>
      <div class={PROMPT_TITLE}>
        Pay {costText(pc().cost)}? If you don't, {who()} may draw a card.
      </div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: true })}>
          Pay {costText(pc().cost)}
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "pay", pay: false })} variant="ghost">
          Don't pay
        </Button>
      </div>
    </div>
  );
};

// Hinder's counter rider: put the countered spell on top or bottom of its owner's library.
const ChooseCounteredSpellDestinationForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_countered_spell_destination">;
  return (
    <div>
      <div class={PROMPT_TITLE}>Put {objectName(props.state, pc().spell)} on top or bottom of its owner's library?</div>
      <div class={PROMPT_ROW}>
        <Button type="button" onClick={() => props.onAnswer({ kind: "top_or_bottom", top: true })}>
          Top of library
        </Button>
        <Button type="button" onClick={() => props.onAnswer({ kind: "top_or_bottom", top: false })} variant="ghost">
          Bottom of library
        </Button>
      </div>
    </div>
  );
};

// Treva's Ruins: return one of the offered lands to your hand, or sacrifice the source.
const SacrificeUnlessReturnLandForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"sacrifice_unless_return_land">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: return a land to your hand or sacrifice it`}
      submitLabel="Return to hand"
      declineLabel="Sacrifice it"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "return_land", land: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "return_land", land: null })}
    />
  );
};

// Illusionary Mask: cast one of the offered hand creatures face down, or decline (CR 708.2).
const CastCreatureFaceDownForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"cast_creature_face_down">;
  return (
    <CardPickPrompt
      state={props.state}
      title="Cast a creature face down?"
      submitLabel="Cast face down"
      declineLabel="Decline"
      items={pc().items}
      count={1}
      onSubmit={(ids) => props.onAnswer({ kind: "cast_face_down_choice", choice: ids[0] })}
      onDecline={() => props.onAnswer({ kind: "cast_face_down_choice", choice: null })}
    />
  );
};

// Fact or Fiction: the chosen opponent splits the revealed cards — selected cards form one pile,
// the rest form the other. Answered with the same subset shape as a sacrifice pick.
const PartitionRevealedForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"partition_revealed">;
  return (
    <CardPickPrompt
      state={props.state}
      title={`${objectName(props.state, pc().source)}: split the revealed cards into two piles`}
      hint="Selected cards form one pile; the rest form the other. Either pile may be empty."
      submitLabel="Split"
      items={pc().items}
      count={null}
      onSubmit={(ids) => props.onAnswer({ kind: "sacrifice", ids })}
    />
  );
};

// Fact or Fiction: the caster picks which pile goes to hand (the other goes to the graveyard).
// Same two-pile view and `opponent_pile` answer as OpponentChoosesPileForm; only the copy differs.
const ChoosePileForHandForm: Component<FormProps> = (props) => {
  const pc = () => props.pc as Narrow<"choose_pile_for_hand">;
  const pileLabel = (items: ChoiceItem[]) => (items.length === 0 ? "(empty)" : items.map((it) => it.label).join(", "));
  return (
    <div>
      <div class={PROMPT_TITLE}>{objectName(props.state, pc().source)}: choose a pile to put into your hand</div>
      <div class="my-1 flex flex-col items-stretch gap-xs">
        <Button
          type="button"
          onClick={() => props.onAnswer({ kind: "opponent_pile", pile: 0 })}
          variant="ghost"
          class="text-left"
        >
          Pile A — {pileLabel(pc().pile_a)}
        </Button>
        <Button
          type="button"
          onClick={() => props.onAnswer({ kind: "opponent_pile", pile: 1 })}
          variant="ghost"
          class="text-left"
        >
          Pile B — {pileLabel(pc().pile_b)}
        </Button>
      </div>
    </div>
  );
};

export const FORMS: Record<PendingChoiceView["kind"], Component<FormProps>> = {
  order_triggers: OrderForm,
  choose_target: ChooseTargetForm,
  choose_spell_targets: ChooseSpellTargetsForm,
  may_yes_no: MayForm,
  pay_cost: PayForm,
  pay_or_counter: PayOrCounterForm,
  put_land_from_hand: PutLandForm,
  put_creature_from_hand: PutCreatureForm,
  choose_exiled_with_card: ChooseExiledForm,
  assign_combat_damage: AssignDamageForm,
  scry: ArrangeTopForm,
  surveil: ArrangeTopForm,
  search_library: SearchLibraryForm,
  select_from_top: SelectFromTopForm,
  sacrifice_edict: SacrificeForm,
  discard: DiscardForm,
  choose_mode: ChooseModeForm,
  proliferate: ProliferateForm,
  phase_out: PhaseOutForm,
  choose_own_sacrifices: ChooseOwnSacrificesForm,
  devour: DevourForm,
  exile_from_graveyard: ExileFromGraveyardForm,
  caster_keep_permanents: CasterKeepForm,
  may_sacrifice: MaySacrificeForm,
  may_return_from_graveyard: MayReturnFromGraveyardForm,
  may_discard: MayDiscardForm,
  pay_echo_or_sacrifice: PayEchoOrSacrificeForm,
  pay_recover_or_exile: PayRecoverOrExileForm,
  divide_spell_damage: DivideSpellDamageForm,
  divide_counters: DivideCountersForm,
  choose_target_players: ChooseTargetPlayersForm,
  shuffle_from_graveyard: ShuffleFromGraveyardForm,
  choose_exiled_with_card_to_cast: ChooseExiledToCastForm,
  choose_exiled_dig_to_cast_free: ChooseExiledDigForm,
  dance_exile_more: DanceExileMoreForm,
  opponent_chooses_pile: OpponentChoosesPileForm,
  opponent_chooses_exiled_nonland: OpponentChoosesExiledNonlandForm,
  choose_exiled_to_cast_free: ChooseExiledToCastFreeForm,
  revealed_card_to_battlefield_or_hand: RevealedCardForm,
  choose_mana_color: ChooseManaColorForm,
  choose_creature_type: ChooseCreatureTypeForm,
  choose_color: ChooseColorForm,
  choose_attach_host: ChooseAttachHostForm,
  choose_copy_target: ChooseCopyTargetForm,
  choose_counter_target_for_player: ChooseCounterTargetForForm,
  // A triggered ability's second target clause — same "pick min..max distinct targets" shape and
  // `choose_targets` answer as a multi-target spell (Kinetic Ooze); reuse the spell-targets form.
  choose_ability_targets: ChooseSpellTargetsForm,
  distribute_top: DistributeTopForm,
  choose_trigger_modes: ChooseTriggerModesForm,
  decline_untap: DeclineUntapForm,
  choose_dredge: ChooseDredgeForm,
  sacrifice_unless_pay: SacrificeUnlessPayForm,
  sacrifice_unless_return_land: SacrificeUnlessReturnLandForm,
  pay_or_controller_draws: PayOrControllerDrawsForm,
  choose_countered_spell_destination: ChooseCounteredSpellDestinationForm,
  cast_creature_face_down: CastCreatureFaceDownForm,
  // "An opponent" chosen by the controller (Fact or Fiction) — the same items/label/source shape
  // and single-target `choose_targets` answer as choose_target; reuse that form.
  choose_splitting_opponent: ChooseTargetForm,
  partition_revealed: PartitionRevealedForm,
  choose_pile_for_hand: ChoosePileForHandForm,
};
