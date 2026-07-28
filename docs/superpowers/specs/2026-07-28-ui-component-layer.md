# UI Component Layer
**Status:** Current (as of 2026-07-28)
**Module:** `client/app/domain/ui/recipe.ts`, `client/app/domain/ui/button.ts`, `client/app/domain/ui/input.ts`, `client/app/domain/ui/surfaces.ts`, `client/app/domain/cn.ts`

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
- A styled element **with no variants** is a class helper on `cn` in `surfaces.ts` — `panelClass`, `modalClass`, `listRowClass`, `alertClass`, `appVersionClass`. `cva({ base: X })({ class: extra })` emits exactly what `cn(X, ...extra)` already emits, so the helper form carries the same behavior with less ceremony. A helper becomes a component the day it grows its first variant.
- Components are exported; their recipes are not. There is no way to reach a variant string and paint button chrome onto a `div`.

### Recipe seam (`recipe.ts`)

- `recipe.ts` is the only module under `client/app/` that imports from `cva`. It calls `defineConfig({ hooks: { onComplete } })` with `cn` as the hook, and re-exports `cva`, `cx`, and `compose`.
- Every recipe's output therefore passes through `clsx` plus the `THEME_SCALES`-extended tailwind-merge in `cn.ts`: a variant utility overrides a base utility for the same CSS property, and this project's `text-*`, `rounded-*`, and spacing scales are classified correctly instead of collapsing into stock tailwind-merge's colour group.
- Recipes take `class` last, so a call-site utility wins over the variant it conflicts with.

### `button`

`button(h, props, children)`.

- `variant` is `primary` | `ghost` | `danger` | `link` | `game` | `game-quiet` | `game-yielded`, defaulting to `primary`. The first four are shell chrome; the three `game*` variants are the board's chunky pressed chrome.
- Shared props: `onClick`, `testId` (emitted as `data-testid`), `ariaLabel`, `class` (any `ClassValue`), and `attrs` — extra Foldkit attributes appended after the component's own.
- The element-specific props are a discriminated union rather than optional fields on one shape: `{ as?: "button"; type?; disabled? }` or `{ as: "a"; href }`. `type` defaults to `"button"`, so a button inside a form cannot submit it by accident.
- `as: "a"` renders `h.a` directly and bypasses `@foldkit/ui`'s `Button.view` entirely, because that primitive emits `type`, which is invalid on an anchor. The anchor branch attaches `onClick` itself, since `Button.view` is what wires it on the button branch.
- `Button.view` marks a disabled button only with `aria-disabled` and `data-disabled`. `button.ts` additionally sets the native `disabled` DOM property, which is what makes the browser block focus, click, and form submission, and what the variants' `disabled:` and `hover:enabled:` Tailwind selectors key off.

### `input`

`input(h, props)`.

- `variant` is `field` (shell chrome: panels, forms, search bars) | `hud` (the board's prompt chrome), defaulting to `field`.
- `id` is required. `@foldkit/ui`'s `Input.view` derives the element id, the `for` target a label points at, and the field's ARIA wiring from it.
- Remaining props: `value`, `onInput`, `type` (the primitive defaults to `text`), `placeholder`, `autofocus`, `testId`, `ariaLabel`, `class`, and `attrs`.
- `Input.view` unconditionally emits `aria-describedby="<id>-description"` referencing an element nothing renders. Per the accessible name and description spec an unresolvable IDREF is skipped, so no screen reader announces anything extra and this is not a WCAG failure; it does surface as `aria-valid-attr-value` needs-review noise in axe-core. A call site that wants it gone passes a later `h.AriaDescribedBy` through `attrs`, which overrides the value.

### Call sites

- Every text field in the shell routes and the board HTML overlays renders through `input`, and standard button chrome renders through `button`. View files pass props; they do not compose variant class strings.
- Controls whose chrome is not button chrome stay hand-written `h.button` elements with their own classes: the prompt HUD rows and submit/cancel in `prompts.ts`, the radial scrim and wedge rows in `activation-menu.ts`, the turn-yield rocker in `priority-bar.ts`, selectable pile cards in `pile-overlay.ts`, and the tile and menu-item chrome in the deck, account-chrome, and app-shell views.
- Canvas and Mount board surfaces are unaffected — they paint pixels rather than emitting DOM.

## Implementation Decisions

- Wire cva to `cn` at one seam rather than per recipe, so no recipe can be authored against unconfigured tailwind-merge by accident.
- Keep recipes module-private and export only the component. The variant vocabulary is the public surface; the class strings behind it are not.
- Type `ButtonProps` as a union so `{ as: "a", disabled: true }` and an anchor missing `href` are rejected. Excess-property checking covers inline object literals and values assigned into a `ButtonProps`-annotated position; props built through an unannotated intermediate still pass structurally and the extra field is ignored at render.
- Delegate behavior and ARIA to `@foldkit/ui` and supply markup and classes locally through its `toView` callback, so the primitive never dictates element structure or class names.
- Type the Foldkit factory parameter as `ReturnType<typeof createHtml<Msg>>`, matching `seat-face.ts` and `deck-card.ts`.
- Pin `@foldkit/ui` exactly, in lockstep with `foldkit` and `effect`: `@foldkit/ui@0.132.0` peer-requires `effect 4.0.0-beta.101` and `foldkit ^0`.
- Keep component recipes in TypeScript rather than a component-token tier in `design.tokens.json`, per `DESIGN.md`.

## Testing Decisions

- `client/app/domain/ui/recipe.test.ts` asserts a variant overrides the base for the same CSS property, that `THEME_SCALES` entries such as `text-caption` survive alongside a colour, and that call-site classes merge last.
- `client/app/domain/ui/button.test.ts` asserts each variant's chrome tokens, call-site override, array and null `class` values, the `type="button"` default, the native `disabled` property, `testId` / `ariaLabel`, `attrs` pass-through on both the button and anchor branches, anchor `href`, and click dispatch on both branches.
- `client/app/domain/ui/input.test.ts` asserts both variants' chrome, call-site override with `hud` sizing preserved, array and null `class` values, the `id` a label points at, `value` / `placeholder` / `type` / `autofocus` bindings, keystroke dispatch, `testId` / `ariaLabel`, and `attrs` pass-through.
- Component tests read snabbdom's `data.class`, `data.attrs`, `data.props`, and `data.on` maps directly, so a native property (`disabled`, `value`) is distinguished from an attribute rather than conflated.
- Surface-level coverage of the rendered controls stays in `client/app/shell/surfaces.test.ts` and `client/app/board/html/surfaces.test.ts`; the component suites do not duplicate it.

## Out of Scope

- Wrappers for `@foldkit/ui` primitives that have no call site in `client/app/` — `textarea`, `select`, `checkbox`, `switch`, `radioGroup`, `disclosure`.
- Converting the `surfaces.ts` class helpers to cva while they have no variants.
- Canvas-drawn board chrome, which has no DOM element to wrap.
- Rendering `@foldkit/ui`'s label and description slots; components emit the control only, and call sites own their own labels.
- A component-token tier in `design.tokens.json`.
- The other `client/app/domain/ui/` modules, which are not primitive wrappers: `confirmDialog.ts` delegates to the native `<dialog>`, and `card-art.ts`, `seat-face.ts`, and `app-version.ts` belong to the specs for the surfaces that render them.

## Further Notes

- Design input remains in [2026-07-28-foldkit-ui-component-layer-design.md](2026-07-28-foldkit-ui-component-layer-design.md); this file documents the shipped surface.
- Token values the recipes spend come from `design.tokens.json` through Tailwind `@theme`; the design-system rules they encode are in [`DESIGN.md`](../../../DESIGN.md).
- Shell chrome composed from these components is described in [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md); board overlay composition in [board-composition](2026-07-20-board-composition.md).
