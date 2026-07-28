# UI Component Layer
**Status:** Current (as of 2026-07-28)
**Module:** `client/app/domain/ui/recipe.ts`, `client/app/domain/ui/button.ts`, `client/app/domain/ui/input.ts`, `client/app/domain/ui/dialog.ts`, `client/app/domain/ui/confirmDialog.ts`, `client/app/domain/ui/menu.ts`, `client/app/domain/ui/windowedGrid.ts`, `client/app/domain/ui/surfaces.ts`, `client/app/domain/cn.ts`

## Problem Statement

Buttons and text fields appear on every shell route and in most board HTML overlays. Assembled per view, each one is a hand-written attribute list — a class string, a `type`, a `data-testid`, an `aria-label`, and whatever disabled wiring the author remembered — which scatters design-system decisions and accessibility decisions across dozens of files where they drift apart independently.

## Solution

`client/app/domain/ui/` exports Foldkit-shaped component functions that own markup and classes and delegate behavior and ARIA to a headless `@foldkit/ui` primitive. Each component holds a module-private cva recipe. Recipes import `cva` only from `recipe.ts`, the single seam where cva's `onComplete` hook is wired to this project's `cn`. Chrome that has no variants stays a plain `cn` class helper in `surfaces.ts`.

## User Stories

- As a view author, I render a button or field by naming a variant and passing props, not by composing a class string.
- As a designer, I change a variant's chrome in one file and every call site follows.
- As a player, a disabled control is genuinely inert to pointer, keyboard, and form submission — not merely styled as if it were.
- As a reviewer, one rule tells me whether a new styled element belongs in a component or in a class helper.

## Behavior

### Wrapper-vs-class-helper rule

- A styled element **with variants** is a component: a function in `client/app/domain/ui/` that wraps the matching headless `@foldkit/ui` primitive and applies a module-private cva recipe. `button` and `input` are the components.
- A styled element a headless primitive renders *for* you — it takes class **strings**, not markup — is a class-string helper next to the surface it dresses: `menu.ts`'s `menuPanelClass` / `menuItemClass`, and `input.ts`'s `inputClass`. This is the one place views legitimately receive an assembled class string, because `@foldkit/ui`'s `Menu` and `Combobox` emit their own trigger, input, container, and rows and accept only `buttonClassName` / `inputClassName` / `itemsClassName` / `ItemConfig.className`.
- A styled element **with no variants** is a class helper on `cn` in `surfaces.ts` — `panelClass`, `modalClass`, `listRowClass`, `alertClass`, `appVersionClass`. `cva({ base: X })({ class: extra })` emits exactly what `cn(X, ...extra)` already emits, so the helper form carries the same behavior with less ceremony. A helper becomes a component the day it grows its first variant.
- Components are exported; their recipes are not. There is no way to reach a variant string and paint button chrome onto a `div`.

### Recipe seam (`recipe.ts`)

- `recipe.ts` is the only module under `client/app/` that imports from `cva`. It calls `defineConfig({ hooks: { onComplete } })` with `cn` as the hook and re-exports the configured `cva`.
- Every recipe's output therefore passes through `clsx` plus the `THEME_SCALES`-extended tailwind-merge in `cn.ts`: a variant utility overrides a base utility for the same CSS property, and this project's `text-*`, `rounded-*`, and spacing scales are classified correctly instead of collapsing into stock tailwind-merge's colour group.
- Recipes take `class` last, so a call-site utility wins over the variant it conflicts with.

### `button`

`button(h, props, children)`.

- `variant` is `primary` | `ghost` | `danger` | `link` | `game` | `game-quiet` | `game-yielded`, defaulting to `primary`. The first four are shell chrome; the three `game*` variants are the board's chunky pressed chrome.
- Shared props: `onClick`, `testId` (emitted as `data-testid`), `ariaLabel`, `class` (any `ClassValue`), and `attrs` — extra Foldkit attributes appended after the component's own.
- The element-specific props are a discriminated union rather than optional fields on one shape: `{ as?: "button"; type?; disabled? }` or `{ as: "a"; href }`. `type` defaults to `"button"`, so a button inside a form cannot submit it by accident.
- `as: "a"` renders `h.a` directly and bypasses `@foldkit/ui`'s `Button.view` entirely, because that primitive emits `type`, which is invalid on an anchor. The anchor branch attaches `onClick` itself, since `Button.view` is what wires it on the button branch.
- `Button.view` marks a disabled button only with `aria-disabled` and `data-disabled`. `button.ts` additionally sets the native `disabled` DOM property, which is what makes the browser block focus, click, and form submission, and what the variants' `disabled:` and `hover:enabled:` Tailwind selectors key off. `Button.view` also drops `onClick` from a disabled button rather than relying on the browser alone.
- `Button.view` sets `tabIndex` to `0` on every rendered button. A `<button>` is already in the tab order and a natively disabled one is out of it regardless of `tabindex`, so focus behavior is unchanged; the attribute is visible in the DOM and in attribute-level assertions.

### `modalDialog`

`modalDialog(h, props, children)` in `dialog.ts` — the shared modal frame, and the only modal chrome in the client.

- Behavior — open/close, focus trap, focus restore, Escape, page scroll lock, backdrop click — comes from `@foldkit/ui`'s `Dialog`. The `<dialog>` element, the dimmed backdrop, and the panel come from here.
- `Dialog` is a **submodel**, not a pure view. The owner holds a `Dialog.Model`, delegates to `Dialog.update`, and acts on its `Closed` OutMessage. Escape, a backdrop click, and a spread of `render.closeButton` all arrive on that one path, so there is no `onDismiss` prop.
- Props: `model`, `toDialogMessage`, optional `panel` (sizing and inner layout classes), `testId` (also the submodel slot id), and optional `backdropTestId` (defaults to `<testId>-backdrop`).
- `children` is a function of `Dialog.RenderInfo`, so a caller spreads `title`, `description`, `closeButton`, and `initialFocus` onto its own elements. The panel's contents are entirely the caller's — headings, buttons, and grids differ per modal, and only the frame is shared.
- `render.isVisible` gates the backdrop and panel. A closed dialog still renders an empty `<dialog>` element, which has to stay in the DOM for `Dialog` to open and close it.
- `render.isVisible` also gates the centring classes (`flex items-center justify-center`). A closed `<dialog>` is hidden only by the UA rule `dialog:not([open]) { display: none }`, and `Dialog` sizes the element `width: 100%; height: 100%` unconditionally, so any `display` of its own leaves a closed modal as a full-viewport `pointer-events-auto` layer that swallows clicks on the page behind it.
- The `<dialog>` carries `pointer-events-auto`. `pointer-events` inherits, and the board composes its overlays under a `pointer-events-none` root; a modal is always meant to take clicks.
- It stays a wrapper function rather than a `Submodel.defineView` because the caller's `children` carry the *parent's* messages: `h.submodel` auto-wraps top-level `viewInputs` functions to the parent's boundary, but not what `toView` returns, so building the body inside `toView` is what lets a parent message dispatch unwrapped.

### `confirmDialog`

`confirmDialog(h, props)` — a question and two choices in `modalDialog`'s frame.

- Props: `model`, `toDialogMessage`, `title`, optional `body`, `confirmLabel`, optional `danger` (picks the `danger` button variant), `onConfirm`, and optional `testId` (defaults to `confirm-dialog`).
- Escape, a backdrop click, and Cancel all reach the owner as `Dialog`'s `Closed`, so there is no `onCancel` prop — only `onConfirm` carries a parent message.
- Cancel spreads `render.closeButton` and `render.initialFocus`, so a destructive confirm is never one Enter away and a plain dismiss needs no parent message.

### `menu.ts` chrome

- `menuPanelClass(extra?, variant?)` is the shared dropdown panel; `menuItemClass(extra?, variant?)` is a single row: transparent, borderless, with hover and `focus-visible` highlight plus a `ring-vine` focus ring.
- `variant` is `shell` (page chrome: `bg-forest-surface`, `shadow-table`, `text-label` rows) | `hud` (the board's translucent prompt chrome: `bg-forest-hud`, `shadow-hud`, `text-body` rows), defaulting to `shell`.
- Positioning (`absolute` / `fixed`, z-index, min-width) differs per site and is passed as `extra`.
- Both back module-private cva recipes, so they merge through the same `cn` seam as the components. They dress `Menu` and `Combobox` alike — both primitives render the same panel-and-rows shape.

### `input`

`input(h, props)`.

- `variant` is `field` (shell chrome: panels, forms, search bars) | `hud` (the board's prompt chrome), defaulting to `field`.
- `id` is required. `@foldkit/ui`'s `Input.view` derives the element id, the `for` target a label points at, and the field's ARIA wiring from it.
- Remaining props: `value`, `onInput`, `type` (the primitive defaults to `text`), `placeholder`, `autofocus`, `testId`, `ariaLabel`, `class`, and `attrs`.
- `inputClass(extra?, variant?)` is the same recipe as a class string, for primitives that render their own `<input>` and take only an `inputClassName` — `Combobox`.
- `Input.view` unconditionally emits `aria-describedby="<id>-description"` referencing an element nothing renders. Per the accessible name and description spec an unresolvable IDREF is skipped, so no screen reader announces anything extra and this is not a WCAG failure; it does surface as `aria-valid-attr-value` needs-review noise in axe-core. A call site that wants it gone passes a later `h.AriaDescribedBy` through `attrs`, which overrides the value.

### `windowedGrid`

`windowedGrid(h, props)` — a tile grid that renders only the rows inside its viewport, so a grid of thousands of tiles costs a couple of dozen DOM nodes and a couple of dozen art requests.

- Scroll tracking, container measurement, and the spacers that keep the scrollbar honest come from `@foldkit/ui`'s `VirtualList`. The chunking into rows, the row element, and its classes come from here.
- `VirtualList` is a one-item-per-row list, so a grid is modelled as a list of rows: `columns` items chunk into one row, and each row renders as a `grid content-start` div wearing the caller's `rowClass`. A trailing partial row keeps whatever items are left. Rendered pixels match the unwindowed grid.
- Props: `model` (the owner's `VirtualList.Model`), `toGridMessage`, `items`, `columns`, `itemToKey`, `itemToView`, `rowClass` (the row's columns and column gap), optional `rowStyle`, optional `containerClass` (max height, width), and `testId` — emitted as the container's `data-testid` and used as the submodel slot id.
- `rowStyle` is for what a class cannot express: a responsive grid measures its column count at runtime, and Tailwind has no `grid-cols-N` class generated ahead of time for a number that did not exist at build time.
- Like `Dialog`, `VirtualList` is a **submodel**: the owner holds the model, delegates to `VirtualList.update`, and — because `VirtualList` has no view handlers — must also lift `VirtualList.subscriptions`. Without that subscription the grid never learns its height and paints nothing.
- Two constraints the caller carries. **Rows must be uniform height**: `rowHeightPx` is one number fixed at `VirtualList.init`, applied as the row's inline height, so callers truncate or reserve space rather than reach for the variable-height path, which rebuilds prefix sums over every item on every scroll event. **`rowHeightPx` includes the row gap**: the row element has an exact height and no margin, so the gap between rows is the space left under a top-aligned row, and `rowClass` sets no bottom margin.
- `VirtualList` writes `overflow: auto` on the container as an inline style, before `containerClassName`, so freezing a windowed grid takes Tailwind's important modifier (`overflow-hidden!`) rather than a plain class.
- `itemToKey` reads the row's first item, since rows are a pure chunking of `items` and are therefore as stable as the caller's own keys.

### Call sites

- Every text field in the shell routes and the board HTML overlays renders through `input`, and standard button chrome renders through `button`. View files pass props; they do not compose variant class strings.
- `confirmDialog` renders the deck-list delete prompt, the deck-builder discard prompt, and the board's concede confirmation; the deck builder's print picker and the board's game-result overlay render on `modalDialog` directly, supplying their own headings and controls — see [deck-list-and-builder](2026-07-20-deck-list-and-builder.md) and [system-overlays](2026-07-20-system-overlays.md).
- Board prompt modals and the mulligan overlay deliberately stay hand-rolled. `Dialog` bundles Escape into `render.dialog` and outside-click into `render.backdrop` with no way to drop either, and a dismissible pending choice leaves the engine waiting on an answer the player can no longer give — see [prompts-and-pending-choices](2026-07-20-prompts-and-pending-choices.md).
- `windowedGrid` renders both of the deck builder's tile grids — the print picker at two tiles per row, and the card pool at a column count measured from its container. The builder owns each `VirtualList.Model`, its row height, and its subscription lift; the pool additionally pages the catalog off the window's `endIndex` — see [deck-list-and-builder](2026-07-20-deck-list-and-builder.md).
- `menuPanelClass` / `menuItemClass` dress every dropdown: the account chrome's `Menu` panel and rows (see [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)), the hand-rolled deck-list right-click context menu, which supplies its own pointer positioning as `extra` (see [deck-list-and-builder](2026-07-20-deck-list-and-builder.md)), and — at `hud` — the card-name typeahead's suggestion list.
- The board's `choose_card_name` prompt renders `@foldkit/ui`'s `Combobox` (`CardNameCombobox` in `board/card-name-combobox.ts`), dressed with `inputClass(…, "hud")` and `menuPanelClass` / `menuItemClass` at `hud`. Like `Dialog` it is a submodel: the board holds the model and delegates through a `GotCardNameComboboxMessage` branch that mirrors the input text into the string draft — see [prompts-and-pending-choices](2026-07-20-prompts-and-pending-choices.md).
- Controls whose chrome is not button chrome stay hand-written `h.button` elements with their own classes: the prompt HUD rows and submit/cancel in `prompts.ts`, the radial scrim and wedge rows in `activation-menu.ts`, the turn-yield rocker in `priority-bar.ts`, selectable pile cards in `pile-overlay.ts`, and the tile chrome in the deck and app-shell views.
- Canvas and Mount board surfaces are unaffected — they paint pixels rather than emitting DOM.

## Implementation Decisions

- Wire cva to `cn` at one seam rather than per recipe, so no recipe can be authored against unconfigured tailwind-merge by accident.
- Keep recipes module-private and export only the component. The variant vocabulary is the public surface; the class strings behind it are not.
- Type `ButtonProps` as a union so `{ as: "a", disabled: true }` and an anchor missing `href` are rejected. Excess-property checking covers inline object literals and values assigned into a `ButtonProps`-annotated position; props built through an unannotated intermediate still pass structurally and the extra field is ignored at render.
- Delegate behavior and ARIA to `@foldkit/ui` and supply markup and classes locally through its `toView` callback, so the primitive never dictates element structure or class names.
- Type the Foldkit factory parameter as `ReturnType<typeof createHtml<Msg>>`, matching `seat-face.ts` and `deck-card.ts`.
- Pin `@foldkit/ui` exactly, in lockstep with `foldkit` and `effect`: `@foldkit/ui@0.132.0` peer-requires `effect 4.0.0-beta.101` and `foldkit ^0`.
- Keep component recipes in TypeScript rather than a component-token tier in `design.tokens.json`, per `DESIGN.md`.
- Give `windowedGrid` no cva recipe. It owns structure, not chrome — the row and container classes are the caller's, because the grid's columns, gaps, and sizing differ per surface and there is no variant vocabulary to name.

## Testing Decisions

- `client/app/domain/ui/recipe.test.ts` asserts a variant overrides the base for the same CSS property, that `THEME_SCALES` entries such as `text-caption` survive alongside a colour, and that call-site classes merge last.
- `client/app/domain/ui/button.test.ts` asserts each variant's chrome tokens, call-site override, array and null `class` values, the `type="button"` default, the native `disabled` property, `testId` / `ariaLabel`, `attrs` pass-through on both the button and anchor branches, anchor `href`, and click dispatch on both branches.
- `client/app/domain/ui/input.test.ts` asserts both variants' chrome, call-site override with `hud` sizing preserved, array and null `class` values, the `id` a label points at, `value` / `placeholder` / `type` / `autofocus` bindings, keystroke dispatch, `testId` / `ariaLabel`, and `attrs` pass-through. It pins `inputClass` against the classes the component itself paints, so the two cannot drift.
- Component tests read snabbdom's `data.class`, `data.attrs`, `data.props`, and `data.on` maps directly, so a native property (`disabled`, `value`) is distinguished from an attribute rather than conflated.
- `client/app/domain/ui/confirmDialog.test.ts` is a Scene suite over a stand-in host, and covers `modalDialog`'s frame through its one wrapper rather than duplicating it: it asserts the closed dialog renders an empty `<dialog>` and carries no `flex` (so it cannot lay itself out over the page) while an open one does, that opening paints backdrop / title / body / both buttons, that Cancel and a backdrop click both reach the owner as `Closed`, that Cancel carries the initial-focus marker, that `danger` picks the danger variant, and that `onConfirm` dispatches the parent's message unwrapped.
- `client/app/domain/ui/windowedGrid.test.ts` is a Scene suite over a stand-in host: an unmeasured grid renders its container but no tiles, a 1000-tile grid mounts only the tiles near the viewport, scrolling swaps which tiles are mounted, two columns put two tiles in one row, and a trailing partial row keeps its tiles. Scene has no message step, so grid state is seeded by folding `VirtualList.update` over the messages the grid would have heard.
- `client/app/domain/ui/menu.test.ts` asserts each helper's chrome tokens per variant and that a call-site class merges last — the panel's positioning `extra` and a row's `no-underline`.
- Surface-level coverage of the rendered controls stays in `client/app/shell/surfaces.test.ts` and `client/app/board/html/surfaces.test.ts`; the component suites do not duplicate it.
- Scene suites that render an open `Combobox` must resolve its `AnchorCombobox` and `PortalComboboxBackdrop` Mounts (`resolveCardNameComboboxMounts` in `board/html/scene-helpers.ts`). Its other Mounts are conditional — `AttachComboboxPreventBlur` renders only with a toggle button and `AttachComboboxSelectOnFocus` only under `selectInputOnFocus`, neither of which the card-name typeahead configures. `Combobox.init` ignores an `isOpen` field, so an open combobox is seeded through `open(init(…))[0]`.
- Scene suites that open a `Menu` must resolve its `FocusItems` command and its `PortalMenuBackdrop` / `AnchorMenu` Mounts, and acknowledge those Mounts with `expectEnded` once a row commits. `Menu.update` builds its `InertOthers` command eagerly, whose factory calls `CSS.escape`, so `client/vitest-setup.ts` shims `CSS.escape` for the repo's `environment: "node"` config.

## Out of Scope

- Wrappers for `@foldkit/ui` entry points with no call site in `client/app/` — `textarea`, `select`, `checkbox`, `switch`, `radioGroup`, `disclosure`, `tabs`, `popover`, and `tooltip`. The only tooltip-shaped code in the client is the native `title=` attribute on a few board buttons and the builder's print badge, which needs no component.
- A component function around `@foldkit/ui`'s `Menu` or `Combobox`. Each renders its own trigger, input, items container, and rows and takes class strings, so there is no markup left for a wrapper to own; `menu.ts` and `inputClass` supply the classes instead.
- Converting the `surfaces.ts` class helpers to cva while they have no variants.
- Canvas-drawn board chrome, which has no DOM element to wrap.
- Rendering `@foldkit/ui`'s label and description slots; components emit the control only, and call sites own their own labels.
- A component-token tier in `design.tokens.json`.
- The other `client/app/domain/ui/` modules, which are not primitive wrappers: `card-art.ts`, `seat-face.ts`, and `app-version.ts` belong to the specs for the surfaces that render them.

## Further Notes

- Design input remains in [2026-07-28-foldkit-ui-component-layer-design.md](2026-07-28-foldkit-ui-component-layer-design.md); this file documents the shipped surface.
- Token values the recipes spend come from `design.tokens.json` through Tailwind `@theme`; the design-system rules they encode are in [`DESIGN.md`](../../../DESIGN.md).
- Shell chrome composed from these components is described in [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md); board overlay composition in [board-composition](2026-07-20-board-composition.md).
