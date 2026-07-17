// ActionSession — deep module at the seam between Board gestures and WireIntent.
// Cost pipeline, staging, modal, X, and take_action live behind play / aim / cancel / overlay.
// Engine pending_choice stays on PromptHost (same act sink). Pre-submit chrome: ActionChrome.

import { type Accessor, type Component, createMemo } from "solid-js";
import { ActionChrome, type ActionChromeModel } from "~/controllers/action-chrome";
import { type ActionExecutionDeps, type StagedAction, useActionExecution } from "~/controllers/actionExecution";
import type { TargetMode } from "~/lib/targeting";
import type { ActionView, ObjectView, WireTarget } from "~/wire/types";

/** Screen center of a hand-bar card face, or null if not in the DOM. */
export function handCardScreenOrigin(cardId: number): { x: number; y: number } | null {
  if (typeof document === "undefined") return null;
  const el = document.querySelector(`[data-testid="hand-card-${cardId}"]`);
  if (!(el instanceof HTMLElement)) return null;
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

export type SessionOverlay = {
  staged: StagedAction | null;
  returningStaged: StagedAction | null;
  mode: TargetMode;
  objects: ReadonlySet<number>;
  players: ReadonlySet<number>;
};

export type ActionSession = {
  /** Hand drop / radial activate — starts cost pipeline or submits. */
  play(action: ActionView, screen?: { x: number; y: number }): void;
  /** Complete staged targeting. */
  aim(target: WireTarget): void;
  /** Abort local pre-submit state (not engine pending_choice). */
  cancel(): void;
  /** Commander / command-zone cast — ActionView lookup then same pipeline as play. */
  playObjectCast(card: ObjectView, target: WireTarget | null): void;
  /** Canvas facts: ghost, arrow sets, preferPick. */
  overlay: Accessor<SessionOverlay>;
  /** Pre-submit cost/modal/X/staged-pick chrome — mount beside PromptHost. */
  Chrome: Component<{ playerName: (seat: number) => string }>;
};

/** Solid adapter: session API over today's action-execution planners + signals. */
export function useActionSession(deps: ActionExecutionDeps): ActionSession {
  const execution = useActionExecution(deps);

  const overlay = createMemo(
    (): SessionOverlay => ({
      staged: execution.staged(),
      returningStaged: execution.returningStaged(),
      mode: execution.stagedMode(),
      objects: execution.stagedObjects(),
      players: execution.stagedPlayers(),
    }),
  );

  const chromeModel: ActionChromeModel = {
    staged: execution.staged,
    cancelStaged: execution.cancelStagedOnly,
    setStaged: execution.setStaged,
    stagedMode: execution.stagedMode,
    xPrompt: execution.xPrompt,
    setXPrompt: execution.setXPrompt,
    modalCast: execution.modalCast,
    setModalCast: execution.setModalCast,
    sacrificePick: execution.sacrificePick,
    setSacrificePick: execution.setSacrificePick,
    discardPick: execution.discardPick,
    setDiscardPick: execution.setDiscardPick,
    gyExilePick: execution.gyExilePick,
    setGyExilePick: execution.setGyExilePick,
    pendingMode: execution.pendingMode,
    advanceModal: execution.advanceModal,
    answerMode: execution.answerMode,
    continueAfterCostPick: execution.continueAfterCostPick,
    objectName: execution.objectName,
    objectPrint: execution.objectPrint,
    getState: execution.getState,
    aim: (target) => execution.completeTarget(target),
  };

  // JSX (not a function call) so Solid owns the reactive tree for nested Show/memos.
  const Chrome: Component<{ playerName: (seat: number) => string }> = (props) => (
    <ActionChrome model={chromeModel} playerName={props.playerName} />
  );

  return {
    play(action, screen) {
      let x = screen?.x;
      let y = screen?.y;
      // No drag drop — play-in from the hand slot (ADR 0033).
      if ((x == null || y == null) && action.object != null) {
        const slot = handCardScreenOrigin(action.object);
        if (slot) {
          x = slot.x;
          y = slot.y;
        }
      }
      execution.onHandDrop(action, x ?? deps.size().x / 2, y ?? deps.size().y / 2);
    },
    aim(target) {
      execution.completeTarget(target);
    },
    cancel() {
      execution.cancelActionState();
    },
    playObjectCast(card, target) {
      void execution.castFromCommandZone(card, target);
    },
    overlay,
    Chrome,
  };
}
