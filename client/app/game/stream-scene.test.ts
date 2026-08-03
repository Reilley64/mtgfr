/**
 * @vitest-environment happy-dom
 *
 * What a seated player sees when the game stream drops and comes back. The stream is a
 * Subscription, so these drive the Messages in through `Scene.Subscription.emit` rather than
 * handing the board an already-disconnected model: the path from stream frame through the app
 * update into the reconnect banner is the part that breaks.
 */
import { Scene } from "foldkit/test";
import { beforeAll, test } from "vitest";
import type { VisibleState } from "~/wire/types";
import { MountHintAutoHide } from "../board/html/audio-mount";
import { resolveLiveBoardMounts } from "../board/html/scene-helpers";
import { init as appInit, update as appUpdate } from "../main-exports";
import { GotGameMessage } from "../messages";
import type { Model } from "../model";
import { emptyGameSlice } from "../model";
import { GameTableRoute } from "../routes";
import { initialLobbySlice } from "../shell/lobby/submodel";
import { view as appView } from "../view";
import { StreamStatus, StreamTerminalError } from "./messages";

beforeAll(() => {
  class MockImage {
    onload: (() => void) | null = null;
    onerror: (() => void) | null = null;
    src = "";
    addEventListener(type: string, fn: () => void): void {
      if (type === "load") this.onload = fn;
    }
  }
  // @ts-expect-error test stub
  globalThis.Image = MockImage;
});

function player(seat: number) {
  return {
    commander_tax: 0,
    hand_count: 0,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: seat,
    username: seat === 0 ? "Alice" : "Bob",
  };
}

const state: VisibleState = {
  active_player: 0,
  can_act: true,
  combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
  objects: [],
  pending_choice: null,
  players: [player(0), player(1)],
  priority: 0,
  stack: [],
  step: 3,
  viewer: 0,
};

function seatedAtTable(): Model {
  const [base] = appInit();
  return {
    ...base,
    route: GameTableRoute({ table: "ABC123" }),
    currentPath: "/play/ABC123",
    landscapeRotate: { active: false },
    sessionLoaded: true,
    session: { me: { id: 1, email: "alice@example.com", username: "alice" }, meGravatarHash: null },
    game: { ...emptyGameSlice("ABC123"), seq: 1, state },
    lobby: { ...initialLobbySlice(), started: true, tableId: "ABC123" },
  };
}

test("a dropped game stream raises the reconnect banner, and reconnecting clears it", () => {
  Scene.scene(
    { update: appUpdate, view: appView },
    Scene.given(seatedAtTable()),
    resolveLiveBoardMounts(),
    // The hint hides itself on its first tick, so its mount is gone by the time the stream speaks.
    Scene.Mount.expectEnded(MountHintAutoHide),
    Scene.expect(Scene.testId("board-reconnecting")).not.toExist(),
    Scene.Subscription.emit(GotGameMessage({ message: StreamStatus({ connected: false }) })),
    Scene.expect(Scene.testId("board-reconnecting")).toExist(),
    Scene.expect(Scene.text("Connection lost — reconnecting…")).toExist(),
    Scene.Subscription.emit(GotGameMessage({ message: StreamStatus({ connected: true }) })),
    Scene.expect(Scene.testId("board-reconnecting")).not.toExist(),
  );
});

test("an expired session on the game stream says so instead of 'reconnecting'", () => {
  Scene.scene(
    { update: appUpdate, view: appView },
    Scene.given(seatedAtTable()),
    resolveLiveBoardMounts(),
    // The hint hides itself on its first tick, so its mount is gone by the time the stream speaks.
    Scene.Mount.expectEnded(MountHintAutoHide),
    Scene.Subscription.emit(GotGameMessage({ message: StreamTerminalError({ status: 401 }) })),
    Scene.expect(Scene.testId("board-reconnecting")).toExist(),
    Scene.expect(Scene.text("Session expired — sign in again.")).toExist(),
    Scene.expect(Scene.text("Connection lost — reconnecting…")).not.toExist(),
  );
});
