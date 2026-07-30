// Global keyboard event mount for the board.
// Registers window keydown/keyup listeners while the board element is mounted,
// emitting board Messages for the shortcuts Foldkit's built-in OnKeyDown cannot
// cover (they only fire on focused elements, not globally).

import { Effect, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import {
  AltDown,
  AltUp,
  KeyboardEnterPressed,
  KeyboardEscape,
  KeyboardSpacePressed,
  ShiftDown,
  ShiftUp,
} from "../messages";

type KeyMessage =
  | typeof AltDown.Type
  | typeof AltUp.Type
  | typeof ShiftDown.Type
  | typeof ShiftUp.Type
  | typeof KeyboardEscape.Type
  | typeof KeyboardEnterPressed.Type
  | typeof KeyboardSpacePressed.Type;

export function isAltKeyEvent(e: KeyboardEvent): boolean {
  if (e.code === "AltLeft" || e.code === "AltRight") return true;
  return e.key === "Alt";
}

export function isShiftKeyEvent(e: KeyboardEvent): boolean {
  if (e.code === "ShiftLeft" || e.code === "ShiftRight") return true;
  return e.key === "Shift";
}

/**
 * The window listeners `MountBoardKeyboard` installs, factored out of the Stream so the modifier
 * lifecycle is unit-testable — a Shift keyup that lands in another window must not leave the
 * whole-pile drop armed, so `onBlur` releases it too.
 *
 * Alt is deliberately keyup-only (no `onBlur` release): blur-releasing Alt would dismiss the
 * inspect pin it arms, a behavior change to that surface rather than a fix to this one.
 */
export function boardKeyListeners(offer: (message: KeyMessage) => void): {
  onKeyDown: (e: Event) => void;
  onKeyUp: (e: Event) => void;
  onBlur: () => void;
} {
  const onKeyDown = (e: Event): void => {
    if (!(e instanceof KeyboardEvent)) return;
    // Don't intercept board shortcuts while typing in an interactive control.
    if (shouldIgnoreBoardShortcut(e)) return;

    if (isAltKeyEvent(e)) {
      e.preventDefault();
      offer(AltDown());
      return;
    }
    // No preventDefault: Shift stays a live modifier for text selection and native shortcuts.
    if (isShiftKeyEvent(e)) {
      offer(ShiftDown());
      return;
    }
    if (e.key === "Escape") {
      offer(KeyboardEscape());
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      offer(KeyboardEnterPressed());
      return;
    }
    if (e.key === " ") {
      e.preventDefault();
      offer(KeyboardSpacePressed());
      return;
    }
  };

  const onKeyUp = (e: Event): void => {
    if (!(e instanceof KeyboardEvent)) return;
    if (isAltKeyEvent(e)) offer(AltUp());
    if (isShiftKeyEvent(e)) offer(ShiftUp());
  };

  const onBlur = (): void => offer(ShiftUp());

  return { onKeyDown, onKeyUp, onBlur };
}

/**
 * Attach this to any long-lived board element. Emits keyboard Messages for the
 * board-global shortcuts: Alt (inspect pin), Shift (whole-pile combat drop), Space (primary/pass),
 * Enter (end turn), Escape (cancel / dismiss).
 *
 * Alt-down pins the card under the cursor (or hand/stack aux hover); Alt-up dismisses —
 * same as the Solid board. The element receiving this mount must be non-interactive itself
 * so Space / Enter don't fire while typing; handlers also guard interactive controls.
 */
export const MountBoardKeyboard = Mount.defineStream(
  "MountBoardKeyboard",
  AltDown,
  AltUp,
  ShiftDown,
  ShiftUp,
  KeyboardEscape,
  KeyboardEnterPressed,
  KeyboardSpacePressed,
)((_element) =>
  Stream.callback<KeyMessage>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          const listeners = boardKeyListeners((message) => Queue.offerUnsafe(queue, message));
          window.addEventListener("keydown", listeners.onKeyDown);
          window.addEventListener("keyup", listeners.onKeyUp);
          window.addEventListener("blur", listeners.onBlur);
          return listeners;
        }),
        ({ onKeyDown, onKeyUp, onBlur }) =>
          Effect.sync(() => {
            window.removeEventListener("keydown", onKeyDown);
            window.removeEventListener("keyup", onKeyUp);
            window.removeEventListener("blur", onBlur);
          }),
      );

      return yield* Effect.never;
    }),
  ),
);

export function shouldIgnoreBoardShortcut(e: KeyboardEvent): boolean {
  const target = e.target;
  if (!(target instanceof Element)) return false;

  const tag = target.tagName.toLowerCase();
  if (tag === "input" || tag === "textarea" || tag === "select") return true;
  if (tag !== "button") return false;

  return e.key === " " || e.key === "Enter";
}
