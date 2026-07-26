import { Effect, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import { ClosedAccountMenu } from "./messages";

const ACCOUNT_MENU_ROOT_SELECTOR = '[data-testid="account-menu-root"]';

/** Window-level Escape while the account menu is open. */
export const BindAccountMenuEscape = Mount.defineStream(
  "BindAccountMenuEscape",
  ClosedAccountMenu,
)((_element) =>
  Stream.callback<typeof ClosedAccountMenu.Type>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          const onKeyDown = (event: Event): void => {
            if (!(event instanceof KeyboardEvent)) return;
            if (event.key !== "Escape") return;
            if (document.querySelector(ACCOUNT_MENU_ROOT_SELECTOR) == null) return;
            event.preventDefault();
            Queue.offerUnsafe(queue, ClosedAccountMenu());
          };
          window.addEventListener("keydown", onKeyDown);
          return onKeyDown;
        }),
        (onKeyDown) =>
          Effect.sync(() => {
            window.removeEventListener("keydown", onKeyDown);
          }),
      );
      return yield* Effect.never;
    }),
  ),
);
