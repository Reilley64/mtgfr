// Right-edge / expanded / full stack presentation. Pass / yield live on the priority context bar.

import { createEffect, createMemo, createSignal, For, Index, onCleanup, onMount, Show } from "solid-js";
import { Button, CardArt } from "~/components/atoms";
import type { StagedAction } from "~/controllers/actionExecution";
import {
  STACK_CARD_W,
  STACK_HORIZONTAL_MARGIN,
  STACK_OVERLAY_RIGHT,
  STACK_STRIP_MIN_PEEK,
  STACK_VERTICAL_RESERVED,
  shouldAutoCollapseStackExpand,
  stackCardH,
  stackExpandAvailable,
  stackFullPerRow,
  stackPeekFor,
  stackPresentation,
  stackStripPeek,
  TARGET_COLOR,
} from "~/lib/boardDraw";
import { cn } from "~/lib/cn";
import { playerLabel } from "~/lib/players";
import type { ObjectView, PlayerView, VisibleState, WireTarget } from "~/wire/types";

function targetLabel(target: WireTarget, name: (id: number) => string, players: PlayerView[]): string {
  return target.kind === "object" ? name(target.id) : playerLabel(players, target.player);
}

export function StackOverlay(props: {
  state: VisibleState;
  /** Local preview of a hand card awaiting a target — visual top when shown. */
  staged: ObjectView | null;
  /** Staged card flying back to hand after cancel. */
  returningStaged: StagedAction | null;
  /** CSS translate delta for staged play-in from drop/hand. */
  stagedPlayIn: { dx: number; dy: number } | null;
  /** CSS translate delta for cancel fly-back (stack → hand). */
  stagedReturn: { dx: number; dy: number } | null;
  /** Pile shows the staged ghost only while arrow aiming (suspended in expand/full). */
  showPileStaged: boolean;
  /** Ids owned by canvas flight — hide resting stack faces until settle (ADR 0035). */
  hideFaceIds: ReadonlySet<number>;
  /** From shared boardChrome — do not recompute with a divergent staged/mana policy. */
  allowDwell: boolean;
  viewportW: number;
  viewportH: number;
  expanded: boolean;
  /** Per stack-object id play-in deltas (opponent / non-flight entrances). */
  entranceDeltas: Map<number, { dx: number; dy: number }>;
  onExpand: () => void;
  onCollapse: () => void;
  onHoverCard: (card: { name: string; cardId?: string; print?: string } | null) => void;
  onDwell: (dwelling: boolean) => void;
}) {
  const names = createMemo(() => new Map(props.state.objects.map((o) => [o.id, o.name])));
  const name = (id: number) => names().get(id) ?? `#${id}`;
  const byId = createMemo(
    () => new Map(props.state.objects.map((o) => [o.id, { print: o.print ?? "", cardId: o.card_id || undefined }])),
  );
  const [holdMs, setHoldMs] = createSignal(props.state.stack_hold_remaining_ms ?? 0);
  const [holdTotal, setHoldTotal] = createSignal(0);
  const [stackHover, setStackHover] = createSignal(false);
  const [dwelling, setDwelling] = createSignal(false);
  const [resolveFlash, setResolveFlash] = createSignal(false);
  createEffect(() => {
    const serverMs = props.state.stack_hold_remaining_ms ?? 0;
    if (serverMs <= 0) {
      setHoldMs(0);
      setHoldTotal(0);
      return;
    }
    setHoldTotal((t) => Math.max(t, serverMs));
    setHoldMs(serverMs);
    const started = performance.now();
    const id = window.setInterval(() => {
      const left = Math.max(0, serverMs - (performance.now() - started));
      setHoldMs(left);
      if (left <= 0) window.clearInterval(id);
    }, 100);
    onCleanup(() => window.clearInterval(id));
  });
  createEffect((prev: number | undefined) => {
    const len = props.state.stack.length;
    if (prev !== undefined && len < prev) {
      setResolveFlash(true);
      const t = window.setTimeout(() => setResolveFlash(false), 220);
      onCleanup(() => window.clearTimeout(t));
    }
    return len;
  });
  const holdPct = () => {
    const total = holdTotal();
    if (total <= 0) return 0;
    return Math.min(100, (holdMs() / total) * 100);
  };
  const cardH = () => Math.round(stackCardH());
  const visualCount = () => props.state.stack.length + (props.staged ? 1 : 0);
  const peek = () => stackPeekFor(visualCount(), props.viewportH, STACK_VERTICAL_RESERVED);
  const presentation = () =>
    stackPresentation({
      count: visualCount(),
      expandedOpen: props.expanded,
      viewportW: props.viewportW,
      viewportH: props.viewportH,
    });
  // Auto-collapse when both expand thresholds clear (or stack empties) — not while staged.
  createEffect(() => {
    if (
      !shouldAutoCollapseStackExpand({
        expanded: props.expanded,
        count: visualCount(),
        peek: peek(),
        staged: props.staged != null,
      })
    ) {
      return;
    }
    props.onCollapse();
  });
  const pileH = () => cardH() + Math.max(0, visualCount() - 1) * peek();
  const top = () => props.state.stack[props.state.stack.length - 1];
  const showMagnifier = () => stackExpandAvailable(visualCount(), peek());
  let hoveredRow: number | null = null;
  const endDwell = () => {
    if (!dwelling()) return;
    setDwelling(false);
    props.onDwell(false);
  };
  const clearHover = (row: number) => {
    if (hoveredRow !== row) return;
    hoveredRow = null;
    props.onHoverCard(null);
    endDwell();
  };
  const leaveStack = () => {
    setStackHover(false);
    if (hoveredRow !== null) clearHover(hoveredRow);
    endDwell();
  };
  const hoverEntry = (row: number, imageName: string | null, meta: { cardId?: string; print?: string }) => {
    if (!imageName) return;
    hoveredRow = row;
    props.onHoverCard({ name: imageName, cardId: meta.cardId, print: meta.print });
    if (props.allowDwell) {
      setDwelling(true);
      props.onDwell(true);
    }
  };
  onMount(() => {
    const onHide = () => {
      leaveStack();
    };
    const onVis = () => {
      if (document.hidden) leaveStack();
    };
    window.addEventListener("pagehide", onHide);
    document.addEventListener("visibilitychange", onVis);
    onCleanup(() => {
      window.removeEventListener("pagehide", onHide);
      document.removeEventListener("visibilitychange", onVis);
      leaveStack();
    });
  });

  const stackFace = (opts: {
    row: number;
    imageName: string | null;
    print: string;
    cardId?: string;
    label: string;
    isTop: boolean;
    staged?: boolean;
    returning?: boolean;
    /** When set, play CSS stack-in from that delta; otherwise appear at rest (flight promote). */
    entranceDelta?: { dx: number; dy: number } | null;
    style: Record<string, string>;
  }) => (
    // biome-ignore lint/a11y/noStaticElementInteractions: hover reveals art / dwell
    <div
      onMouseEnter={() => hoverEntry(opts.row, opts.imageName, { cardId: opts.cardId, print: opts.print })}
      style={opts.style}
      class={cn(
        "absolute rounded-game shadow-[0_4px_14px_rgb(0_0_0/0.55)]",
        opts.returning ? "animate-stack-return" : opts.entranceDelta ? "animate-stack-in" : null,
        opts.staged && "ring-(--target) ring-2",
        opts.isTop && holdMs() > 0 && stackHover() && "shadow-[0_0_16px_rgba(255,215,106,0.4)]",
      )}
    >
      <Show
        when={opts.imageName}
        fallback={
          <div
            style={{ "--h": `${cardH()}px`, "--w": `${STACK_CARD_W}px` }}
            class="flex h-(--h) w-(--w) items-center justify-center rounded-game bg-forest-hud px-1 text-center font-semibold text-caption text-seafoam"
          >
            {opts.label}
          </div>
        }
      >
        {(n) => <CardArt print={opts.print} alt={n()} width={STACK_CARD_W} class="block rounded-game" />}
      </Show>
    </div>
  );

  const pileBody = () => (
    <div
      style={{ "--r": `${STACK_OVERLAY_RIGHT}px`, "--w": `${STACK_CARD_W}px`, height: `${pileH()}px` }}
      class={cn("fixed top-1/2 right-(--r) z-15 w-(--w) -translate-y-1/2", resolveFlash() && "animate-stack-resolve")}
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: hover drives hold UI / dwell */}
      <div class="relative h-full w-full" onMouseEnter={() => setStackHover(true)} onMouseLeave={leaveStack}>
        <Index each={props.state.stack}>
          {(entry, row) => {
            onCleanup(() => clearHover(row));
            createEffect((prev: string | undefined) => {
              const identity = `${entry().kind}:${entry().source}`;
              if (prev !== undefined && prev !== identity) clearHover(row);
              return identity;
            });
            const imageName = () => (entry().kind === "spell" ? entry().label : (names().get(entry().source) ?? null));
            const meta = byId().get(entry().source) ?? { print: "", cardId: undefined };
            const isTop = () => row === props.state.stack.length - 1 && !(props.staged && props.showPileStaged);
            const delta = () => props.entranceDeltas.get(entry().source) ?? null;
            return (
              <Show when={!props.hideFaceIds.has(entry().source)}>
                {stackFace({
                  row,
                  imageName: imageName(),
                  print: meta.print,
                  cardId: meta.cardId,
                  label: entry().label,
                  isTop: isTop(),
                  entranceDelta: delta(),
                  style: {
                    "--w": `${STACK_CARD_W}px`,
                    width: `${STACK_CARD_W}px`,
                    bottom: `${row * peek()}px`,
                    "z-index": String(row),
                    left: "0",
                    ...(delta()
                      ? {
                          "--stack-from-dx": `${delta()?.dx ?? 0}px`,
                          "--stack-from-dy": `${delta()?.dy ?? 0}px`,
                        }
                      : {}),
                  },
                })}
              </Show>
            );
          }}
        </Index>
        <Show when={props.showPileStaged ? props.staged : null}>
          {(card) =>
            stackFace({
              row: props.state.stack.length,
              imageName: card().name,
              print: card().print ?? "",
              cardId: card().card_id || undefined,
              label: card().name,
              isTop: true,
              staged: true,
              entranceDelta: props.stagedPlayIn,
              style: {
                "--w": `${STACK_CARD_W}px`,
                "--target": TARGET_COLOR,
                width: `${STACK_CARD_W}px`,
                bottom: `${props.state.stack.length * peek()}px`,
                "z-index": String(props.state.stack.length),
                left: "0",
                ...(props.stagedPlayIn
                  ? {
                      "--stack-from-dx": `${props.stagedPlayIn.dx}px`,
                      "--stack-from-dy": `${props.stagedPlayIn.dy}px`,
                    }
                  : {}),
              },
            })
          }
        </Show>
        <Show when={props.returningStaged}>
          {(card) =>
            stackFace({
              row: props.state.stack.length,
              imageName: card().card.name,
              print: card().card.print ?? "",
              cardId: card().card.card_id || undefined,
              label: card().card.name,
              isTop: true,
              returning: true,
              entranceDelta: props.stagedReturn,
              style: {
                "--w": `${STACK_CARD_W}px`,
                width: `${STACK_CARD_W}px`,
                bottom: `${props.state.stack.length * peek()}px`,
                "z-index": String(props.state.stack.length + 1),
                left: "0",
                ...(props.stagedReturn
                  ? {
                      "--stack-from-dx": `${props.stagedReturn.dx}px`,
                      "--stack-from-dy": `${props.stagedReturn.dy}px`,
                    }
                  : {}),
              },
            })
          }
        </Show>
        <Show when={showMagnifier()}>
          <Button
            type="button"
            aria-label={`Expand stack (${visualCount()} objects)`}
            onClick={props.onExpand}
            variant="ghost"
            class="absolute -top-9 right-0 flex items-center gap-1 px-2 py-1 text-chip text-seafoam"
          >
            Expand · {visualCount()}
          </Button>
        </Show>
      </div>
      <div class="absolute top-full right-0 left-0 mt-sm flex flex-col items-center gap-sm">
        <Show when={stackHover() && holdMs() > 0 && !props.staged}>
          <div
            style={{ "--w": `${STACK_CARD_W}px` }}
            class="h-1.5 w-(--w) overflow-hidden rounded-full bg-white/15"
            aria-hidden="true"
          >
            <div
              style={{ width: `${holdPct()}%` }}
              class="h-full rounded-full bg-vine transition-[width] duration-150 ease-linear"
            />
          </div>
        </Show>
        <Show when={props.staged && props.showPileStaged}>
          <div
            style={{ "--w": `${STACK_CARD_W}px`, "--target": TARGET_COLOR }}
            class="max-w-(--w) text-center text-(--target) text-chip"
          >
            Choose a target
          </div>
        </Show>
        <Show when={!props.staged && top()}>
          {(t) => (
            <div style={{ "--w": `${STACK_CARD_W}px` }} class="max-w-(--w) text-center text-chip text-seafoam">
              <Show when={t().kind === "ability"}>
                <div class="font-semibold">{t().label}</div>
              </Show>
              <Show when={t().target}>
                {(target) => <div>→ {targetLabel(target(), name, props.state.players)}</div>}
              </Show>
            </div>
          )}
        </Show>
      </div>
    </div>
  );

  const stripOrFullBody = () => {
    const mode = presentation();
    const items = () => {
      const list = props.state.stack.map((entry, row) => {
        const meta = byId().get(entry.source) ?? { print: "", cardId: undefined };
        return {
          row,
          source: entry.source,
          imageName: entry.kind === "spell" ? entry.label : (names().get(entry.source) ?? null),
          print: meta.print,
          cardId: meta.cardId,
          label: entry.label,
          staged: false as boolean,
        };
      });
      if (props.staged && props.showPileStaged) {
        list.push({
          row: props.state.stack.length,
          source: props.staged.id,
          imageName: props.staged.name,
          print: props.staged.print ?? "",
          cardId: props.staged.card_id || undefined,
          label: props.staged.name,
          staged: true,
        });
      }
      return list;
    };
    const n = () => items().length;
    const hPeek = () =>
      mode === "full" ? STACK_STRIP_MIN_PEEK : Math.max(STACK_STRIP_MIN_PEEK, stackStripPeek(n(), props.viewportW));
    const perRow = () => (mode === "full" ? stackFullPerRow(props.viewportW) : n());
    const rows = () => Math.ceil(n() / perRow());
    const stripW = () => {
      const cols = Math.min(n(), perRow());
      return STACK_CARD_W + Math.max(0, cols - 1) * hPeek();
    };
    const stripH = () => cardH() + Math.max(0, rows() - 1) * (cardH() * 0.35);
    return (
      <div
        class={cn(
          "fixed z-15 flex flex-col items-center gap-sm",
          mode === "full" ? "top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2" : "top-1/2 right-4 -translate-y-1/2",
          resolveFlash() && "animate-stack-resolve",
        )}
        style={{
          width:
            mode === "full" ? `${Math.min(props.viewportW - STACK_HORIZONTAL_MARGIN, stripW())}px` : `${stripW()}px`,
          "max-width": `${props.viewportW - STACK_HORIZONTAL_MARGIN}px`,
        }}
      >
        <div class="flex w-full items-center justify-between gap-sm">
          <span class="text-chip text-seafoam">
            Stack · {visualCount()}
            {mode === "full" ? " · full" : ""}
          </span>
          <Button
            type="button"
            aria-label="Collapse stack"
            onClick={props.onCollapse}
            variant="ghost"
            class="px-2 py-1 text-chip"
          >
            ✕
          </Button>
        </div>
        {/* biome-ignore lint/a11y/noStaticElementInteractions: hover drives hold UI / dwell */}
        <div
          class="relative"
          style={{ width: `${stripW()}px`, height: `${stripH()}px` }}
          onMouseEnter={() => setStackHover(true)}
          onMouseLeave={leaveStack}
        >
          <For each={items()}>
            {(item) => {
              const col = () => item.row % perRow();
              const rowY = () => Math.floor(item.row / perRow());
              const isTop = () => item.row === n() - 1;
              const delta = () => (item.staged ? props.stagedPlayIn : (props.entranceDeltas.get(item.source) ?? null));
              onCleanup(() => clearHover(item.row));
              return (
                <Show when={!props.hideFaceIds.has(item.source)}>
                  {stackFace({
                    row: item.row,
                    imageName: item.imageName,
                    print: item.print,
                    cardId: item.cardId,
                    label: item.label,
                    isTop: isTop(),
                    staged: item.staged,
                    entranceDelta: delta(),
                    style: {
                      "--w": `${STACK_CARD_W}px`,
                      "--target": TARGET_COLOR,
                      width: `${STACK_CARD_W}px`,
                      left: `${col() * hPeek()}px`,
                      top: `${rowY() * cardH() * 0.35}px`,
                      "z-index": String(item.row),
                      ...(delta()
                        ? {
                            "--stack-from-dx": `${delta()?.dx ?? 0}px`,
                            "--stack-from-dy": `${delta()?.dy ?? 0}px`,
                          }
                        : {}),
                    },
                  })}
                </Show>
              );
            }}
          </For>
        </div>
        <Show when={stackHover() && holdMs() > 0 && !props.staged}>
          <div
            style={{ "--w": `${STACK_CARD_W}px` }}
            class="h-1.5 w-(--w) overflow-hidden rounded-full bg-white/15"
            aria-hidden="true"
          >
            <div
              style={{ width: `${holdPct()}%` }}
              class="h-full rounded-full bg-vine transition-[width] duration-150 ease-linear"
            />
          </div>
        </Show>
        <Show when={!props.staged && top()}>
          {(t) => (
            <div class="max-w-[280px] text-center text-chip text-seafoam">
              <Show when={t().kind === "ability"}>
                <div class="font-semibold">{t().label}</div>
              </Show>
              <Show when={t().target}>
                {(target) => <div>→ {targetLabel(target(), name, props.state.players)}</div>}
              </Show>
            </div>
          )}
        </Show>
      </div>
    );
  };

  return (
    <Show when={visualCount() > 0}>
      <Show when={presentation() === "pile"} fallback={stripOrFullBody()}>
        {pileBody()}
      </Show>
    </Show>
  );
}
