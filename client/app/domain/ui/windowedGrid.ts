// Windowed tile grid — renders only the rows inside the viewport, so a grid of thousands of
// tiles mounts a couple of dozen DOM nodes instead of thousands. Behaviour (scroll tracking,
// container measurement, the spacers that keep the scrollbar honest) comes from @foldkit/ui's
// VirtualList; the chunking into grid rows and the row's classes come from here.
//
// VirtualList is a one-item-per-row list, so a grid becomes a list of rows: `columns` items are
// chunked into one row, and each row renders as the caller's `rowClass` grid. The rendered
// pixels are the same as an unwindowed grid.
//
// Two constraints VirtualList puts on callers, both load-bearing:
//
//  - **Rows must be uniform height.** `rowHeightPx` is one number for the whole list, and the
//    row `<li>` gets it as an inline height. A taller row overflows into the next. Callers make
//    their tiles uniform (truncate, or reserve space) rather than reaching for VirtualList's
//    variable-height path, which rebuilds prefix sums over every item on *every scroll event*.
//  - **`rowHeightPx` includes the row gap.** The `<li>` has an exact height and no margin, so
//    the vertical gap between rows is the space left under a top-aligned row. `rowClass` should
//    not set a bottom margin.
//
// Scroll and resize arrive through VirtualList's `containerEvents` Subscription, not through the
// view, so an owner must also wire that subscription for the grid to ever leave its unmeasured
// state.

import * as VirtualList from "@foldkit/ui/virtualList";
import { childAttributes, type html as createHtml, type Html } from "foldkit/html";
import { cn } from "../cn";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

export type WindowedGridProps<Item, Msg> = {
  /** The owner's grid state. Create with `VirtualList.init({ id, rowHeightPx })`. */
  model: VirtualList.Model;
  /** Lifts a `VirtualList.Message` into the owner's message union. */
  toGridMessage: (message: VirtualList.Message) => Msg;
  items: ReadonlyArray<Item>;
  /** Items per row. The caller owns this number — a fixed column count, or one measured from
   *  the container's width when the grid is responsive. */
  columns: number;
  itemToKey: (item: Item) => string;
  itemToView: (item: Item) => Html;
  /** Classes on the row: the grid columns and the column gap. */
  rowClass: string;
  /** Classes on the scroll container — max height, width, overscroll. */
  containerClass?: string;
  /** Emitted as the container's `data-testid`; also the submodel slot id. */
  testId: string;
};

/** Chunks `items` into rows of `columns`. A trailing partial row is kept as-is, so the last row
 *  can hold fewer than `columns` items. */
function toRows<Item>(items: ReadonlyArray<Item>, columns: number): ReadonlyArray<ReadonlyArray<Item>> {
  if (columns < 1) return items.map((item) => [item]);

  const rows: Array<ReadonlyArray<Item>> = [];
  for (let index = 0; index < items.length; index += columns) {
    rows.push(items.slice(index, index + columns));
  }
  return rows;
}

/** Renders `items` as a windowed grid of `columns`-wide rows. */
export function windowedGrid<Item, Msg>(h: HtmlFactory<Msg>, props: WindowedGridProps<Item, Msg>): Html {
  const { model, toGridMessage, items, columns, itemToKey, itemToView, rowClass, containerClass, testId } = props;
  const rows = toRows(items, columns);

  return h.submodel({
    slotId: testId,
    model,
    view: VirtualList.view<ReadonlyArray<Item>>(),
    viewInputs: {
      items: rows,
      // The first item's key identifies the row: rows are a pure chunking of `items`, so it is
      // as stable as the caller's own keys.
      itemToKey: (row: ReadonlyArray<Item>, index: number) => (row[0] ? itemToKey(row[0]) : `${testId}-row-${index}`),
      itemToView: (row: ReadonlyArray<Item>) =>
        h.div(
          // `content-start` keeps tiles at the top of the row, so the leftover height inside the
          // fixed-height `<li>` reads as the gap between rows.
          [h.Class(cn("grid content-start", rowClass))],
          row.map((item) => itemToView(item)),
        ) as Html,
      containerClassName: cn("overscroll-contain", containerClass),
      containerAttributes: childAttributes([h.DataAttribute("testid", testId)]),
    },
    toParentMessage: toGridMessage,
  }) as Html;
}
