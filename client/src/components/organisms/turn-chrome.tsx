// Turn banner, priority watch, and phase-track styling.

import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import { createEffect, createSignal, For, onCleanup, Show } from "solid-js";
import { Hud } from "~/components/atoms";
import { PHASES, phaseOf, STEP_NAMES } from "~/layout";
import { cn } from "~/lib/cn";
import { playerLabel } from "~/lib/players";
import { playAttentionPriority, playAttentionYourTurn } from "~/lib/tableAudio";
import { type Heat, heatOf, watchElapsed } from "~/lib/watch";
import { SPECTATOR_VIEWER } from "~/store";
import type { VisibleState } from "~/wire/types";

export function TurnBanner(props: { me: number; state: VisibleState }) {
  const s = () => props.state;
  const yourTurn = () => s().active_player === props.me;
  const current = () => phaseOf(s().step);
  // Show the specific step name under the band when it's more precise than the band label
  // (e.g. Combat → "Declare Attackers"); a single-step band like Main 1 adds nothing.
  const stepDetail = () => {
    const band = PHASES[current()];
    const name = STEP_NAMES[s().step] ?? String(s().step);
    return band && band.steps.length > 1 && band.name !== name ? name : null;
  };

  // Attention cues: your-turn wins when both flip in the same update (client-game-board-and-interaction spec).
  // Skip watchers and eliminated seats — they cannot hold priority.
  createEffect((prev: { turn: boolean; priority: boolean } | undefined) => {
    const viewer = s().viewer;
    const seat = s().players.find((p) => p.player === props.me);
    const canHearAttention = viewer !== SPECTATOR_VIEWER && seat != null && !seat.lost;
    const turn = yourTurn();
    const priority = s().priority === props.me;
    if (canHearAttention && prev !== undefined) {
      const gainedTurn = turn && !prev.turn;
      const gainedPriority = priority && !prev.priority;
      if (gainedTurn) playAttentionYourTurn();
      else if (gainedPriority) playAttentionPriority();
    }
    return { turn, priority };
  });

  return (
    <Hud
      data-testid="board-turn-banner"
      data-step={String(s().step)}
      data-active-player={String(s().active_player)}
      data-priority={String(s().priority)}
      data-stack-len={String(s().stack.length)}
      class="fixed top-md left-1/2 z-20 flex -translate-x-1/2 flex-col items-center gap-xs shadow-hud"
    >
      <div
        data-testid="board-turn-label"
        class={cn("font-bold text-label text-turn-ember", yourTurn() && "text-turn-mint")}
      >
        {yourTurn() ? "Your turn" : `${playerLabel(s().players, s().active_player)}'s turn`}
      </div>
      <div class="flex gap-xs">
        <For each={PHASES}>
          {(band, i) => {
            const state = () => (i() < current() ? "past" : i() === current() ? "now" : "future");
            return (
              <div class={phaseSegment(state(), yourTurn())}>
                {band.name}
                <Show when={i() === current() && stepDetail()}>
                  {(d) => <div class="mt-px text-micro text-snow-mint/85">{d()}</div>}
                </Show>
              </div>
            );
          }}
        </For>
      </div>
      <PriorityWatch me={props.me} state={props.state} />
    </Hud>
  );
}

// Visual escalation for the slowpoke, per `heatOf`'s thresholds.
const HEAT_INK: Record<Heat, string> = {
  sage: "text-watch-sage",
  ember: "text-turn-ember",
  flare: "text-watch-flare",
};

// Names whose priority it is with a live elapsed timer — so you can shame whoever's dawdling.
// Client-local clock (the engine is deterministic and carries no wall time).
function PriorityWatch(props: { me: number; state: VisibleState }) {
  const holder = () => props.state.priority;
  const [elapsed, setElapsed] = createSignal(0);
  // Priority changing hands restarts the shame clock: the effect re-runs, its cleanup interrupts
  // the counting fiber, and a fresh `watchElapsed` starts again from 1. There is no wall-clock
  // read here — the tick sleeps on the Effect Clock, which is what makes `heatOf`'s escalation
  // testable (lib/watch.test.ts) instead of a thirty-second wait.
  createEffect(() => {
    holder(); // tracked: a new holder restarts the count
    setElapsed(0);
    const fiber = Effect.runFork(watchElapsed(setElapsed));
    onCleanup(() => Effect.runFork(Fiber.interrupt(fiber)));
  });

  const yours = () => holder() === props.me;
  return (
    <div
      class={cn(
        "font-semibold text-caption tracking-[0.01em]",
        HEAT_INK[heatOf(elapsed())],
        yours() && "text-turn-mint",
      )}
    >
      {yours() ? "You have priority" : `Waiting on ${playerLabel(props.state.players, holder())}`}
      {/* Suppressed below 10s — a 1s "· 1s" flicker reads as noise, not signal. */}
      <Show when={elapsed() >= 10}>
        <span class="text-fog"> · {elapsed()}s</span>
      </Show>
    </div>
  );
}

// A single band in the phase track: past bands read as done, the current one lights up (green on
// your turn, amber on an opponent's), future bands stay faint. Phase Fern on the future band's
// fill computes to ~6.5:1, clearing the 4.5:1 contrast floor while staying quieter than past/now's
// Snow Mint (~16.6:1).
function phaseSegment(state: "past" | "now" | "future", yourTurn: boolean): string {
  return cn(
    // Fixed equal width — sized for the longest step detail ("First Strike Damage" at text-micro).
    "w-[7.5rem] rounded-control border border-transparent px-md py-xs text-center font-semibold text-caption",
    "bg-tapped-out/60 text-phase-fern", // future: the resting band
    state === "past" && "bg-quiet-hover text-snow-mint",
    state === "now" && "text-snow-mint",
    state === "now" && yourTurn && "border-phase-mint bg-llanowar/90",
    state === "now" && !yourTurn && "border-phase-ember bg-phase-ember/90",
  );
}
