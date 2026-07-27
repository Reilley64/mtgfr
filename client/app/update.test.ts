import { Option } from "effect";
import { expect, test } from "vitest";
import { LeaveGame } from "./board/messages";
import { StreamTerminalError } from "./game/messages";
import { init, update } from "./main-exports";
import { GotAuthMessage, GotBoardMessage, GotGameMessage, GotLobbyMessage, UrlChanged } from "./messages";
import { emptyGameSlice } from "./model";
import { GameTableRoute, routePath } from "./routes";
import * as Auth from "./shell/auth";
import { ChangedAuthEmail } from "./shell/auth/messages";
import { ReceivedLobbyView } from "./shell/lobby/messages";

const me = { id: 1, email: "alice@example.com", username: "alice" };

const url = (pathname: string, search = "") => ({
  protocol: "http:",
  host: "localhost",
  port: Option.none<string>(),
  pathname,
  search: search === "" ? Option.none<string>() : Option.some(search),
  hash: Option.none<string>(),
});

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

test("LeaveGame redirects home from the result overlay", () => {
  const [base] = init();
  const [, commands] = update(base, GotBoardMessage({ message: LeaveGame() }));
  const redirect = commands.find((command) => command.name === "Redirect") as { args?: { path?: string } } | undefined;
  expect(redirect?.args?.path).toBe("/");
});

test("lobby start redirect followed by UrlChanged keeps the active GameTableRoute game slice", () => {
  const [base] = init(url("/play/7/ABC123"));
  const [authed] = update(base, GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) }));
  const [started, commands] = update(
    authed,
    GotLobbyMessage({
      message: ReceivedLobbyView({
        view: {
          table_id: "ABC123",
          seats: [],
          you: 0,
          started: true,
          start_error: null,
          error: null,
        },
      }),
    }),
  );

  const redirect = commands.find((command) => command.name === "Redirect") as { args?: { path?: string } } | undefined;
  expect(redirect?.args?.path).toBe(routePath(GameTableRoute({ table: "ABC123" })));

  const startedGame = started.game;
  expect(startedGame).not.toBeNull();
  if (startedGame == null) {
    throw new Error("expected started lobby handoff to seed a game slice");
  }

  const [tableRoute] = update(
    {
      ...started,
      game: { ...startedGame, seq: 7, connected: false, reject: "Preserve me" },
    },
    UrlChanged({ url: url("/play/ABC123") }),
  );

  expect(tableRoute.route).toEqual(GameTableRoute({ table: "ABC123" }));
  expect(tableRoute.game).not.toBeNull();
  expect(tableRoute.game?.tableId).toBe("ABC123");
  expect(tableRoute.game?.active).toBe(true);
  expect(tableRoute.game?.seq).toBe(7);
  expect(tableRoute.game?.connected).toBe(false);
  expect(tableRoute.game?.reject).toBe("Preserve me");
});
