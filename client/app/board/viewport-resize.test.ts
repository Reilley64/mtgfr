/**
 * @vitest-environment happy-dom
 *
 * The canvas layers size their backing store from `board.viewport`, so a viewport that does not
 * track the window leaves the board stretched (blurry, and magnified relative to the HTML hand bar).
 */
import { afterEach, expect, it, vi } from "vitest";
import { emptyGameFold } from "../game/fold";
import { BoardViewportResized } from "./messages";
import { initialBoardModel, updateBoard } from "./submodel";

function resizeWindowTo(width: number, height: number): void {
  vi.stubGlobal("window", { ...window, innerWidth: width, innerHeight: height });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

it("starts the viewport at the real window size", () => {
  resizeWindowTo(2560, 1440);

  expect(initialBoardModel().viewport).toEqual({ width: 2560, height: 1440 });
});

it("tracks the window on resize so the canvas backing store matches its CSS box", () => {
  const [model] = updateBoard(
    initialBoardModel(),
    BoardViewportResized({ width: 2560, height: 1440, dpr: 1 }),
    emptyGameFold(),
    null,
  );

  expect(model.viewport).toEqual({ width: 2560, height: 1440 });
});

it("refits the camera to the new size while the player has not panned or zoomed", () => {
  resizeWindowTo(1440, 900);
  const fitted = { ...initialBoardModel(), cameraFitPlayers: 4, camera: { panX: 10, panY: 20, zoom: 0.7 } };

  const [model] = updateBoard(
    fitted,
    BoardViewportResized({ width: 2560, height: 1440, dpr: 1 }),
    emptyGameFold(),
    null,
  );

  expect(model.camera.zoom).toBeGreaterThan(0.7);
});

it("leaves a camera the player moved alone", () => {
  const moved = {
    ...initialBoardModel(),
    cameraFitPlayers: 4,
    cameraUserMoved: true,
    camera: { panX: 10, panY: 20, zoom: 0.7 },
  };

  const [model] = updateBoard(
    moved,
    BoardViewportResized({ width: 2560, height: 1440, dpr: 1 }),
    emptyGameFold(),
    null,
  );

  expect(model.camera).toEqual({ panX: 10, panY: 20, zoom: 0.7 });
});
