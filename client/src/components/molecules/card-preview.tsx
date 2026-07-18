// Card reading UI: docked inspect during play (Alt-pin), cursor-follow hover in the deck builder.
//
// Do not use `useAtomResource` here — it suspends under the app-root Suspense and blanks the whole
// screen while catalog text loads. Hover and inspect must stay non-suspending (`useAtomValue`).

import { useAtomValue } from "@effect/atom-solid";
import * as Effect from "effect/Effect";
import * as Option from "effect/Option";
import * as AsyncResult from "effect/unstable/reactivity/AsyncResult";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { Button } from "~/components/atoms";
import { cn } from "~/lib/cn";
import {
  type InspectFace,
  type InspectPin,
  inspectRootChanged,
  playFace,
  popInspectHistory,
  pushInspectSource,
  shownName,
} from "~/lib/inspect";
import { lookupCardsByIds } from "~/lib/lookupCards";
import { splitOracleText } from "~/lib/oracleText";
import { imageUrlByPrint } from "~/lib/scryfall";
import type { CatalogCard, ModifierSourceView } from "~/wire/types";

// Keyed by Card (oracle) id — ADR 0031. An empty id (no id known for this pin/hover yet) skips
// the fetch rather than looking anything up by name; there is no name-based lookup anymore.
const cardTextFamily = Atom.family((id: string) =>
  Atom.make(id === "" ? Effect.succeed(null) : lookupCardsByIds([id]).pipe(Effect.map((cards) => cards[0] ?? null))),
);

/** Success value (`CatalogCard | null`), or `undefined` while the lookup is still in flight. */
function catalogCard(result: AsyncResult.AsyncResult<CatalogCard | null, unknown>): CatalogCard | null | undefined {
  return Option.getOrUndefined(AsyncResult.value(result));
}

const W = 320;
const H = Math.round(W / 0.716);
const PANEL = 300;
const GAP = 24;
/** Inspect dock / column wrap ceiling — keep in sync with `max-h-(--dock-h)` classes below. */
const DOCK_H = "min(90vh, 720px)";
const PANEL_CARD = cn(
  "w-(--w) shrink-0 rounded-panel border border-vine bg-forest-surface px-xl py-lg text-preview-ash leading-[1.4]",
);

/** Oracle / approximates prose with `{T}` / `{G}` etc. as mana-font glyphs. */
function OracleRichText(props: { text: string }) {
  return (
    <For each={splitOracleText(props.text)}>
      {(part) =>
        part.kind === "text" ? (
          part.text
        ) : (
          <span role="img" class={cn("ms", "ms-cost", "ms-oracle", `ms-${part.ms}`)} aria-label={`{${part.code}}`} />
        )
      }
    </For>
  );
}

function TextPanel(props: {
  oracle?: string | null;
  approximates?: string | null;
  /** Cap + internal scroll. CSS length (`447px`, `min(90vh, 720px)`, `var(--dock-h)`). */
  maxH?: string;
}) {
  const hasText = () => !!(props.oracle || props.approximates);
  return (
    <Show when={hasText()}>
      <div
        style={{
          "--w": `${PANEL}px`,
          ...(props.maxH != null ? { "--max-h": props.maxH } : {}),
        }}
        class={cn(PANEL_CARD, "text-body", props.maxH != null && "max-h-(--max-h) overflow-y-auto")}
      >
        <Show when={props.oracle}>
          {(oracle) => (
            <div class="whitespace-pre-wrap">
              <OracleRichText text={oracle()} />
            </div>
          )}
        </Show>
        <Show when={props.approximates}>
          {(approx) => (
            <div class={cn("text-label text-note-gold italic", props.oracle && "mt-3 border-vine/40 border-t pt-3")}>
              <span class="font-semibold not-italic">Approximation: </span>
              <OracleRichText text={approx()} />
            </div>
          )}
        </Show>
      </div>
    </Show>
  );
}

function ModifierLedger(props: {
  modifiers: ModifierSourceView[];
  onSource: (source: { name: string; cardId?: string }) => void;
}) {
  return (
    <For each={props.modifiers}>
      {(group) => (
        <div style={{ "--w": `${PANEL}px` }} class={cn(PANEL_CARD, "text-label")}>
          <button
            type="button"
            class="cursor-pointer text-seafoam underline decoration-mist/50 underline-offset-2 hover:decoration-seafoam"
            onClick={() =>
              props.onSource({
                name: group.source_name,
                cardId: group.source_card_id || undefined,
              })
            }
          >
            {group.source_name}
          </button>
          <div class="mt-0.5 text-preview-ash/80">{group.contributions.join(", ")}</div>
        </div>
      )}
    </For>
  );
}

/** Deck-builder / list hover: large face follows the cursor. `id` is the Card (oracle) id;
 * `print` (a Printing UUID) picks the art when known, else falls back to the catalog's
 * `default_print` once the oracle-text fetch resolves it. */
export function HoverPreview(props: { id: string | null; print?: string | null; x: number; y: number }) {
  const result = useAtomValue(() => cardTextFamily(props.id ?? ""));
  const card = () => catalogCard(result());
  const oracle = () => card()?.oracle;
  const approximates = () => card()?.approximates;
  const hasText = () => !!(oracle() || approximates());
  const width = () => (hasText() ? W + 12 + PANEL : W);
  const flipped = () => props.x + GAP + width() > window.innerWidth;
  const left = () => (flipped() ? Math.max(GAP, props.x - GAP - width()) : props.x + GAP);
  const top = () => Math.min(Math.max(GAP, props.y - H / 2), window.innerHeight - H - GAP);
  const print = () => props.print || card()?.default_print || "";
  return (
    <Show when={props.id}>
      {(_id) => (
        <div
          style={{ "--x": `${left()}px`, "--y": `${top()}px` }}
          class={cn(
            "pointer-events-none fixed top-(--y) left-(--x) z-40 flex flex-row items-start gap-3",
            flipped() && "flex-row-reverse",
          )}
        >
          <img
            src={imageUrlByPrint(print())}
            alt={card()?.name ?? ""}
            style={{ "--w": `${W}px` }}
            class="w-(--w) flex-none rounded-[14px] shadow-table"
          />
          <TextPanel oracle={oracle()} approximates={approximates()} maxH={`${H}px`} />
        </div>
      )}
    </Show>
  );
}

/** In-game Alt-pin inspect: left dock + modal dialog. */
export function InspectDock(props: {
  pin: InspectPin | null;
  /** Live modifiers for a battlefield object id (empty / omit off-battlefield). */
  modifiersFor?: (objectId: number) => ModifierSourceView[];
  onDismiss: () => void;
}) {
  let dialog!: HTMLDialogElement;
  const [history, setHistory] = createSignal<InspectPin[]>([]);
  const current = createMemo(() => {
    const stack = history();
    if (stack.length === 0) return null;
    return stack[stack.length - 1] ?? null;
  });
  const result = useAtomValue(() => cardTextFamily(current()?.cardId ?? ""));
  const card = () => catalogCard(result());
  const back = () => card()?.back ?? null;
  const hasBack = () => !!back()?.name;
  const [face, setFace] = createSignal<InspectFace>("front");
  const catalogReady = () => card() !== undefined;

  createEffect(() => {
    const pin = props.pin;
    if (!pin) {
      setHistory([]);
      if (dialog.open) dialog.close();
      return;
    }
    setHistory((prev) => (inspectRootChanged(prev[0], pin) ? [pin] : prev));
    // Defer open like PickDialog, but don't close on cleanup when the pin merely changes —
    // inspect history updates while the same dialog stays up.
    if (dialog.open) return;
    let cancelled = false;
    queueMicrotask(() => {
      if (cancelled || !dialog.isConnected || dialog.open || !props.pin) return;
      dialog.showModal();
    });
    onCleanup(() => {
      cancelled = true;
    });
  });

  createEffect(() => {
    const pin = current();
    if (!pin) return;
    // Prepared DFCs stay on the back face until catalog says otherwise (avoids a front-face flash).
    if (pin.prepared && !catalogReady()) {
      setFace("back");
      return;
    }
    setFace(playFace(!!pin.prepared, hasBack()));
  });

  const displayName = () => shownName(current()?.name ?? "", back()?.name, face());
  const oracle = () => (face() === "back" ? (back()?.oracle ?? null) : (card()?.oracle ?? null));
  const approximates = () => (face() === "back" ? (back()?.approximates ?? null) : (card()?.approximates ?? null));
  const modifiers = createMemo(() => {
    const id = current()?.objectId;
    if (id == null || !props.modifiersFor) return [];
    return props.modifiersFor(id);
  });
  const canGoBack = () => history().length > 1;
  const goBack = () => setHistory(popInspectHistory);
  const openSource = (source: { name: string; cardId?: string }) => setHistory((h) => pushInspectSource(h, source));
  const hasOracle = () => !!(oracle() || approximates());
  const hasMods = () => modifiers().length > 0;
  const artPrint = () => current()?.print || card()?.default_print || "";

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: Escape dismisses via showModal() → onClose.
    <dialog
      ref={dialog}
      onClose={() => props.pin && props.onDismiss()}
      onClick={(e) => e.target === dialog && props.onDismiss()}
      class="fixed inset-0 z-30 m-0 h-full max-h-none w-full max-w-none border-0 bg-black/55 p-0 open:flex open:items-center"
    >
      <Show when={current()}>
        {/* pt-(--back-gutter) reserves room for the top chrome (Back / Flip) so the card never
         * jumps when either appears, and the links can't clip off a centered short viewport.
         * Sized for game-quiet Flip (incl. coarse 44px floor), not just the Back link. */}
        <div
          style={{ "--back-gutter": "2.75rem", "--dock-h": DOCK_H }}
          class="pointer-events-auto relative m-lg flex max-h-(--dock-h) w-full max-w-[calc(100vw-2*var(--spacing-lg))] flex-row items-start gap-3 pt-(--back-gutter)"
        >
          <div class="relative flex shrink-0 flex-col items-center">
            {/* Same top chrome row: Back left, Flip top-right of the card. */}
            <Show when={canGoBack() || hasBack()}>
              <div class="absolute top-0 right-0 left-0 flex -translate-y-full items-center justify-between pb-2">
                <Show when={canGoBack()}>
                  <Button
                    type="button"
                    onClick={goBack}
                    variant="link"
                    class="flex items-center gap-1 text-label text-fog no-underline hover:text-mist hover:underline"
                  >
                    <svg
                      aria-hidden="true"
                      viewBox="0 0 16 16"
                      class="size-3.5 shrink-0 fill-none stroke-current"
                      stroke-width="1.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M10 3.5 5.5 8 10 12.5" />
                      <path d="M5.5 8h6" />
                    </svg>
                    Back
                  </Button>
                </Show>
                <Show when={hasBack()}>
                  <Button
                    type="button"
                    onClick={() => setFace((f) => (f === "front" ? "back" : "front"))}
                    variant="game-quiet"
                    class="ml-auto"
                  >
                    Flip
                  </Button>
                </Show>
              </div>
            </Show>
            <Show
              when={!current()?.prepared || catalogReady()}
              fallback={
                <div
                  style={{ "--w": `${W}px`, "--h": `${H}px` }}
                  class="h-(--h) w-(--w) animate-skeleton rounded-[14px] bg-white/10"
                />
              }
            >
              <img
                src={imageUrlByPrint(artPrint(), "large", face())}
                alt={displayName()}
                style={{ "--w": `${W}px` }}
                class="w-(--w) flex-none rounded-[14px] shadow-table"
              />
            </Show>
          </div>
          <Show when={hasOracle() || hasMods()}>
            {/* Fixed-width cards wrap into columns across the remaining viewport width. */}
            <div class="flex max-h-(--dock-h) min-w-0 flex-1 flex-col flex-wrap content-start gap-3 overflow-x-auto">
              <TextPanel oracle={oracle()} approximates={approximates()} maxH="var(--dock-h)" />
              <Show when={hasMods()}>
                <ModifierLedger modifiers={modifiers()} onSource={openSource} />
              </Show>
            </div>
          </Show>
        </div>
      </Show>
    </dialog>
  );
}

export default HoverPreview;
