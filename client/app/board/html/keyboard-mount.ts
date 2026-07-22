// Global keyboard event mount for the board.
// Registers window keydown/keyup listeners while the board element is mounted,
// emitting board Messages for the shortcuts Foldkit's built-in OnKeyDown cannot
// cover (they only fire on focused elements, not globally).

import { Effect, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import { AltDown, AltUp, KeyboardEnterPressed, KeyboardEscape, KeyboardSpacePressed } from "../messages";

type KeyMessage =
  | typeof AltDown.Type
  | typeof AltUp.Type
  | typeof KeyboardEscape.Type
  | typeof KeyboardEnterPressed.Type
  | typeof KeyboardSpacePressed.Type;

/**
 * Attach this to any long-lived board element. Emits keyboard Messages for the
 * board-global shortcuts: Alt (inspect pin), Space (primary/pass), Enter (end turn),
 * Escape (cancel / dismiss).
 *
 * The element receiving this mount must be non-interactive itself so Space / Enter
 * don't fire while the player is typing in an input.  The handlers guard against
 * interactive controls (inputs, buttons, textareas, selects).
 */
export const MountBoardKeyboard = Mount.defineStream(
  "MountBoardKeyboard",
  AltDown,
  AltUp,
  KeyboardEscape,
  KeyboardEnterPressed,
  KeyboardSpacePressed,
)((_element) =>
  Stream.callback<KeyMessage>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          const onKeyDown = (e: Event): void => {
            if (!(e instanceof KeyboardEvent)) return;
            // Don't intercept board shortcuts while typing in an interactive control.
            if (isInteractiveControl(e.target)) return;

            if (e.key === "Alt") {
              e.preventDefault();
              Queue.offerUnsafe(queue, AltDown());
              return;
            }
            if (e.key === "Escape") {
              Queue.offerUnsafe(queue, KeyboardEscape());
              return;
            }
            if (e.key === "Enter") {
              e.preventDefault();
              Queue.offerUnsafe(queue, KeyboardEnterPressed());
              return;
            }
            if (e.key === " ") {
              e.preventDefault();
              Queue.offerUnsafe(queue, KeyboardSpacePressed());
              return;
            }
          };

          const onKeyUp = (e: Event): void => {
            if (!(e instanceof KeyboardEvent)) return;
            if (e.key === "Alt") {
              Queue.offerUnsafe(queue, AltUp());
            }
          };

          window.addEventListener("keydown", onKeyDown);
          window.addEventListener("keyup", onKeyUp);
          return { onKeyDown, onKeyUp };
        }),
        ({ onKeyDown, onKeyUp }) =>
          Effect.sync(() => {
            window.removeEventListener("keydown", onKeyDown);
            window.removeEventListener("keyup", onKeyUp);
          }),
      );

      return yield* Effect.never;
    }),
  ),
);

function isInteractiveControl(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  const tag = target.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select" || tag === "button";
}
