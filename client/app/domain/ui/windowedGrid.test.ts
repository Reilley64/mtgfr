import * as VirtualList from "@foldkit/ui/virtualList";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import { windowedGrid } from "./windowedGrid";

// A stand-in owner: the smallest thing that holds a VirtualList.Model and renders tiles.
type Tile = { id: string };
type HostModel = { grid: VirtualList.Model; tiles: ReadonlyArray<Tile> };
type HostMessage = { _tag: "GotGridMessage"; message: VirtualList.Message };

const h = html<HostMessage>();

const ROW_HEIGHT = 100;
const toGridMessage = (message: VirtualList.Message): HostMessage => ({ _tag: "GotGridMessage", message });

/** The grid's own messages arrive from a Subscription, not the view, so a scene seeds them here
 *  instead of dispatching: an unmeasured container until `containerHeight` says otherwise. */
function hostModel(tileCount: number, seen: ReadonlyArray<VirtualList.Message> = []): HostModel {
  const grid = seen.reduce(
    (model, message) => VirtualList.update(model, message)[0],
    VirtualList.init({ id: "tile-grid", rowHeightPx: ROW_HEIGHT }),
  );
  return { grid, tiles: Array.from({ length: tileCount }, (_, index) => ({ id: `tile-${index}` })) };
}

const measured = (containerHeight: number) => VirtualList.MeasuredContainer({ containerHeight });
const scrolled = (scrollTop: number) => VirtualList.ScrolledContainer({ scrollTop });

const view = (model: HostModel) =>
  windowedGrid(h, {
    model: model.grid,
    toGridMessage,
    items: model.tiles,
    columns: 2,
    itemToKey: (tile: Tile) => tile.id,
    itemToView: (tile: Tile) => h.div([h.DataAttribute("testid", tile.id)], [tile.id]),
    rowClass: "grid-cols-2 gap-md",
    containerClass: "max-h-[400px]",
    testId: "tile-grid",
  });

const update = (model: HostModel): readonly [HostModel, ReadonlyArray<unknown>] => [model, []];

const program = { update, view } as never;

test("a grid whose height is not known yet renders its container and no tiles", () => {
  Scene.scene(
    program,
    Scene.with(hostModel(100)),
    Scene.expect(Scene.testId("tile-grid")).toExist(),
    Scene.expect(Scene.testId("tile-0")).toBeAbsent(),
  );
});

test("a long grid mounts only the tiles near the viewport", () => {
  Scene.scene(
    program,
    // 400px of viewport over 100px rows: a handful of rows plus overscan, never all 500.
    Scene.with(hostModel(1000, [measured(400)])),
    Scene.expect(Scene.testId("tile-0")).toExist(),
    Scene.expect(Scene.testId("tile-1")).toExist(),
    Scene.expect(Scene.testId("tile-999")).toBeAbsent(),
  );
});

test("scrolling swaps which tiles are mounted", () => {
  Scene.scene(
    program,
    Scene.with(hostModel(1000, [measured(400), scrolled(20_000)])),
    Scene.expect(Scene.testId("tile-0")).toBeAbsent(),
    Scene.expect(Scene.testId("tile-400")).toExist(),
  );
});

test("two columns put two tiles in every row", () => {
  Scene.scene(
    program,
    Scene.with(hostModel(4, [measured(400)])),
    Scene.expect(Scene.selector('[data-virtual-list-item-index="0"] [data-testid="tile-0"]')).toExist(),
    Scene.expect(Scene.selector('[data-virtual-list-item-index="0"] [data-testid="tile-1"]')).toExist(),
    Scene.expect(Scene.selector('[data-virtual-list-item-index="1"] [data-testid="tile-2"]')).toExist(),
    Scene.expect(Scene.selector('[data-virtual-list-item-index="1"] [data-testid="tile-3"]')).toExist(),
  );
});

test("a trailing partial row keeps the tiles it has", () => {
  Scene.scene(
    program,
    Scene.with(hostModel(3, [measured(400)])),
    Scene.expect(Scene.selector('[data-virtual-list-item-index="1"] [data-testid="tile-2"]')).toExist(),
    Scene.expect(Scene.testId("tile-3")).toBeAbsent(),
  );
});
