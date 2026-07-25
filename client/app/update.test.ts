import { expect, test } from "vitest";
import { StreamTerminalError } from "./game/messages";
import { init, update } from "./main-exports";
import { emptyGameSlice } from "./model";

test("terminal stream errors store user-facing reconnect reasons", () => {
  const [base] = init();

  const [expired] = update({ ...base, game: emptyGameSlice("T1") }, StreamTerminalError({ status: 401 }));
  expect(expired.game?.connected).toBe(false);
  expect(expired.game?.reject).toBe("Session expired — sign in again.");

  const [missing] = update({ ...base, game: emptyGameSlice("T1") }, StreamTerminalError({ status: 404 }));
  expect(missing.game?.connected).toBe(false);
  expect(missing.game?.reject).toBe("Table no longer available.");
});
