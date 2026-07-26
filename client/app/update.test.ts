import { expect, test } from "vitest";
import { StreamTerminalError } from "./game/messages";
import { init, update } from "./main-exports";
import { GotAuthMessage, GotGameMessage } from "./messages";
import { emptyGameSlice } from "./model";
import { ChangedAuthEmail } from "./shell/auth/messages";

test("terminal stream errors store user-facing reconnect reasons", () => {
  const [base] = init();

  const [expired] = update(
    { ...base, game: emptyGameSlice("T1") },
    GotGameMessage({ message: StreamTerminalError({ status: 401 }) }),
  );
  expect(expired.game?.connected).toBe(false);
  expect(expired.game?.reject).toBe("Session expired — sign in again.");

  const [missing] = update(
    { ...base, game: emptyGameSlice("T1") },
    GotGameMessage({ message: StreamTerminalError({ status: 404 }) }),
  );
  expect(missing.game?.connected).toBe(false);
  expect(missing.game?.reject).toBe("Table no longer available.");
});

test("GotAuthMessage updates auth email through the parent update", () => {
  const [base] = init();

  const [next] = update(
    base,
    GotAuthMessage({
      message: ChangedAuthEmail({ email: "a@b.c" }),
    }),
  );

  expect(next.auth.email).toBe("a@b.c");
});
