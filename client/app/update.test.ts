import { Effect, Layer, Option } from "effect";
import { expect, test, vi } from "vitest";
import { LeaveGame } from "./board/messages";
import { imageUrlByPrint } from "./domain/deck-builder/scryfall";
import { sharedImageCache } from "./domain/image-cache";
import type { Client } from "./domain/rpc-client";
import { StreamTerminalError } from "./game/messages";
import { init, update } from "./main-exports";
import { GotAuthMessage, GotBoardMessage, GotGameMessage, GotLobbyMessage, UrlChanged } from "./messages";
import { emptyGameSlice } from "./model";
import { RpcClient } from "./resources";
import { GameTableRoute, routePath } from "./routes";
import * as Auth from "./shell/auth";
import { ChangedAuthEmail } from "./shell/auth/messages";
import { ReceivedLobbyView } from "./shell/lobby/messages";
import { warmDeckArt } from "./update";

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

test("seating at a pregame table warms that deck's card art", () => {
  const [base] = init(url("/play/7/ABC123"));
  const [, commands] = update(base, GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) }));

  const warm = commands.find((command) => command.name === "WarmDeckArt") as { args?: { deckId?: number } } | undefined;
  expect(warm?.args?.deckId).toBe(7);
});

test("warming a deck hands its fetched prints to the shared image cache at low priority", async () => {
  const preload = vi.spyOn(sharedImageCache, "preload").mockImplementation(() => {});
  const deck = {
    id: 7,
    name: "Atraxa",
    commander: "oracle-cmd",
    commander_print: "print-cmd",
    cards: [{ id: "oracle-a", count: 1, print: "print-a" }],
  };
  const rpc = { getDeck: () => Effect.succeed(deck) } as unknown as Client;

  await Effect.runPromise(warmDeckArt(7).pipe(Effect.provide(Layer.succeed(RpcClient, rpc))));

  expect(preload).toHaveBeenCalledWith([imageUrlByPrint("print-cmd"), imageUrlByPrint("print-a")], "low");
  preload.mockRestore();
});

test("entering a live table route does not warm deck art mid-game", () => {
  const [base] = init(url("/play/ABC123"));
  const [, commands] = update(base, GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) }));

  expect(commands.find((command) => command.name === "WarmDeckArt")).toBeUndefined();
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
