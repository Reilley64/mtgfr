// Priority context bar: Next / Resolve card / Resolve stack / End Turn / turn yield (always bottom-right, above stack z).
// MTGA-style pass cluster: hierarchy by silhouette — primary owns the corner; no tray.

import { Show } from "solid-js";
import { Button } from "~/components/atoms";
import { HAND_BAR_H } from "~/components/molecules/hand";
import type { PrimaryAction } from "~/lib/interaction";
import { priorityPrimaryClass } from "~/lib/priorityContextChrome";
import type { StackChrome } from "~/lib/stackResponse";
import { turnYieldRockerClass, turnYieldThumbClass, turnYieldTrackClass } from "~/lib/turnYieldChrome";

export function PriorityContextBar(props: {
  action: PrimaryAction;
  yours: boolean;
  chrome: StackChrome;
  reject: string | null;
  staged: string | null;
  /** The staged spell may target a player, so the hint must mention their life orb. */
  stagedPlayers: boolean;
  onRun: () => void;
  onPass: () => void;
  onArmStackYield: () => void;
  onTurnYield: (enabled: boolean) => void;
  /** Clear a staged targeting cast (arrow mode). */
  onCancelTarget: (() => void) | null;
}) {
  const showNext = () => props.chrome.showPrimary(props.action.kind);
  return (
    <div
      style={{ "--b": `${HAND_BAR_H + 10}px` }}
      class="fixed right-md bottom-(--b) z-25 flex flex-col items-end gap-sm"
      data-testid="priority-context-bar"
    >
      <div class="flex flex-row-reverse flex-wrap items-center justify-end gap-md">
        {/* Primary first in reverse row so it sits rightmost (Arena-style pass cluster). */}
        <Show when={showNext()}>
          <Button
            type="button"
            data-testid="board-primary"
            disabled={!props.yours}
            onClick={props.onRun}
            variant="game"
            class={priorityPrimaryClass(props.yours)}
          >
            {props.action.label}
          </Button>
        </Show>
        {/* Companions share a tighter gap so the primary keeps corner ownership. */}
        <div class="flex flex-row-reverse flex-wrap items-center justify-end gap-sm">
          <Show when={props.chrome.showEndTurn}>
            <Button
              type="button"
              data-testid="board-end-turn"
              aria-pressed={props.chrome.turnYielded}
              title={props.chrome.turnYielded ? "Cancel end turn" : "End turn (Enter)"}
              onClick={() => props.onTurnYield(!props.chrome.turnYielded)}
              variant={props.chrome.turnYielded ? "game-yielded" : "game-quiet"}
            >
              {props.chrome.turnYielded ? "Ending turn…" : "End Turn"}
            </Button>
          </Show>
          <Show when={props.chrome.pass}>
            <Button type="button" data-testid="board-pass" onClick={props.onPass} variant="game" class="shadow-glow">
              Resolve card
            </Button>
          </Show>
          <Show when={props.chrome.stackYieldArm}>
            <Button type="button" data-testid="board-stack-yield" onClick={props.onArmStackYield} variant="game-quiet">
              Resolve stack
            </Button>
          </Show>
          <Show when={props.chrome.stackYieldArmed}>
            <Button type="button" data-testid="board-stack-yield-armed" disabled variant="game-yielded">
              Resolve stack
            </Button>
          </Show>
          {/* Arena-style pass-turn rocker: icon + sliding switch, not a form checkbox. */}
          <Show when={props.chrome.showTurnYield}>
            <button
              type="button"
              role="switch"
              data-testid="board-turn-yield"
              aria-checked={props.chrome.turnYielded}
              aria-label="Auto-pass until my turn"
              title="Auto-pass until my turn"
              onClick={() => props.onTurnYield(!props.chrome.turnYielded)}
              class={turnYieldRockerClass(props.chrome.turnYielded)}
            >
              <span class={turnYieldTrackClass(props.chrome.turnYielded)}>
                <span class={turnYieldThumbClass(props.chrome.turnYielded)} aria-hidden="true">
                  ≫
                </span>
              </span>
            </button>
          </Show>
          <Show when={props.onCancelTarget}>
            <Button
              type="button"
              data-testid="board-cancel-target"
              onClick={() => props.onCancelTarget?.()}
              variant="game-quiet"
            >
              Cancel
            </Button>
          </Show>
        </div>
      </div>
      <Show when={props.staged}>
        <div data-testid="board-staged-hint" class="max-w-[280px] text-right text-caption text-caution-amber">
          {props.staged}: click a highlighted {props.stagedPlayers ? "card or life orb" : "card"}
        </div>
      </Show>
      <Show when={props.reject}>
        <div data-testid="board-reject" class="text-burn-red text-caption">
          {props.reject}
        </div>
      </Show>
    </div>
  );
}
