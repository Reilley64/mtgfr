# Deck Builder Scroll Design

**Status:** Accepted  
**Date:** 2026-07-25  
**Surface:** Deck builder (`client/app/shell/decks/builder/**`) — catalog / decklist scrollports and print-picker modal  
**Living module spec:** [deck-list-and-builder](2026-07-20-deck-list-and-builder.md) (update Behavior in the same implementation change)

## Goal

Make catalog and decklist scroll independently inside the split-pane builder, and freeze both background scrollports while the choose-printing dialog is open so only the print tile grid scrolls.

## Design

### Independent pane scroll

- The builder page shell (`deck-builder-page`) stays viewport-bounded (`h-dvh` + `grid-rows-[minmax(0,1fr)]`) and non-scrolling (`overflow-hidden`). Document/page scroll is not the browsing mechanism — without a definite viewport height the grid grows with the pool and the catalog never overflows.
- **Catalog** (left pool grid): the pool's own scroll host, with `overscroll-contain` so wheel/trackpad does not chain to the document or the decklist. (It is now a `windowedGrid`, which owns `overflow: auto` itself and pages off its scroll position — see [deck-list-and-builder](2026-07-20-deck-list-and-builder.md).)
- **Decklist** (right list): keep its own `overflow-y-auto` scrollport. Add `overscroll-contain`. Ensure the aside participates in the grid height constraint (`min-h-0` / flex column) so the list scrolls inside the panel instead of expanding the page.
- Wheel/trackpad over one pane must not move the other pane.

### Print picker scroll lock

- When `printPicker` is non-null, the existing modal `<dialog>` (`showModal`) is open.
- While open, catalog and decklist scrollports are frozen (e.g. `overflow-hidden` on those hosts, driven from `printPicker != null` in the view — no new submodel field unless that proves awkward).
- Wheel over the dimmed page must not move either list.
- The print tile grid inside the dialog keeps `overflow-y-auto` + `overscroll-contain` so long printing lists remain usable.
- On close (`printPicker` cleared), both panes restore independent scrolling with no leftover body/page lock.

## Non-goals

- Changing print-picker UX, art loading, or catalog paging / search RPCs.
- A global app-wide scroll-lock utility beyond this surface.
- Making the decklist non-scrollable.

## Tests

- Extend builder Scene / story coverage to assert outcomes: with the print picker open, catalog and decklist hosts are non-scrolling; the print grid remains the scroll host.
- Keep existing print-picker and builder Scene coverage green; extend rather than replace.
- Follow [client interaction test policy](2026-07-22-client-interaction-test-policy-design.md): assert scroll-host contracts (classes / overflow), not only that the picker `data-testid` exists.

## Implementation note

Update [deck-list-and-builder](2026-07-20-deck-list-and-builder.md) Behavior in the same PR that lands the CSS/view changes so the living module spec describes independent panes and the print-picker background lock.
