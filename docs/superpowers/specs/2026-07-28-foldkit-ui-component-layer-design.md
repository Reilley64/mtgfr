# Foldkit UI component layer — cva recipes + `@foldkit/ui` headless primitives

**Status:** Design input (2026-07-28) on branch `feat/foldkit-ui-component-layer`.
Waves land as separate PRs; each wave updates its living surface specs in the same
change. The component layer itself is a **new code target** (`client/app/domain/ui/`)
and gets its own indexed surface spec when W1 lands. Related living docs:
[`DESIGN.md`](../../../DESIGN.md) (components / tokens prose),
[`2026-07-27-dtcg-token-architecture-design.md`](2026-07-27-dtcg-token-architecture-design.md)
(token tiers house rule), and the per-surface specs named in each wave below.

**Dependencies:** [`cva@1.0.0-beta.7`](https://cva.style) (already in
`client/package.json`, currently unused), `@foldkit/ui@0.132.0` (new — peer-requires
`effect 4.0.0-beta.101` and `foldkit ^0`, matching today's pins exactly).

---

## Problem Statement

`client/app/domain/ui/` holds hand-written class recipes — `buttonClass` (7 variants),
`panelClass`, `modalClass`, `listRowClass`, `fieldClass`, `alertClass`. They work, but:

- **Variant strings duplicate their base.** `primary` / `ghost` / `danger` repeat
  `px-lg py-sm text-button … transition-colors duration-150 ease-state
  disabled:opacity-50`; the `game` / `game-quiet` / `game-yielded` trio repeats a
  longer shared run. There is no base/variant factoring, so a change to shared
  chrome is a 3–7 site edit.
- **Styling is decoupled from behavior.** A recipe returns a `string`; every call
  site re-assembles `h.button([h.Type("button"), h.OnClick(…), h.Disabled(…),
  h.Class(buttonClass(…))], …)` by hand across 47 sites. Nothing enforces that a
  thing styled as a button *is* one.
- **No accessibility layer exists.** `client/app/` contains zero `role="dialog"` and
  zero `aria-modal`. Overlays (`confirmDialog`, `prompt-modal`, `result-overlay`,
  `mulligan-overlay`) are plain divs; menus (`account-chrome`, decks-list context
  menu) hand-roll `aria-expanded` / `aria-haspopup` plus bespoke Escape-key Mount
  streams. Focus trapping, focus restore, and `inert` siblings are absent
  everywhere.
- **`cva` is a declared dependency with zero imports.** `VARIANTS` in
  `buttonClass.ts` is a hand-rolled subset of it.

## Goal

One answer to "how do I render a styled, accessible control": a component function
in `client/app/domain/ui/` that wraps the matching `@foldkit/ui` headless primitive
and applies its cva recipe automatically.

## Non-goals

- Moving component styling into `design.tokens.json`. The DTCG house rule stands
  (`2026-07-27-dtcg-token-architecture-design.md` Non-goals, "recipes stay in
  `client/app/domain/ui/`"). Only the recipes' internal form changes; the token file
  keeps its `primitive` + `semantic` tiers with **no** component tier.
- Generating component code from token JSON. DTCG describes values, not variants,
  states, or slots.
- Changing the visual design. Waves preserve today's rendered chrome; new behavior
  is additive (focus management, ARIA), not a restyle.
- Adopting `@foldkit/ui` submodel components for non-modal board chrome — see the
  board boundary below.
- Renaming semantic token names or Tailwind class names.

## Approaches considered

1. **One wrapper module per primitive (chosen)** — `domain/ui/button.ts`,
   `input.ts`, `dialog.ts`, …; each owns a module-private cva recipe and exports a
   Foldkit-shaped function. Matches the existing flat `domain/ui/` layout; files stay
   small and independently testable.
2. **A generic `defineComponent(recipe, render)` factory** — fewer files, but each
   `@foldkit/ui` primitive returns a different attribute-group shape (button
   `{button}`, input `{input,label,description}`, dialog a 7-field `RenderInfo`).
   A shared factory collapses to a lowest-common-denominator that call sites then
   work around. Rejected: one-implementation abstraction.
3. **Parallel `domain/ui/components/` tree beside the existing `*Class.ts` files** —
   clean on paper, but leaves two supported ways to style a button indefinitely.
   Rejected.

## Locked decisions

- **Wrapper vs class helper.** If `@foldkit/ui` provides a primitive, `domain/ui/`
  exports a **component function** and its recipe is module-private. If it does not —
  `panelClass`, `listRowClass`, `alertClass`, `appVersionClass` style plain
  containers with no interactive behavior — it stays a **class helper** on `cn`, and
  reaches for cva only once it grows a variant. One answer per question, not per file.
- **`buttonClass` / `gameButtonClass` are deleted** in W1, not deprecated, so call
  sites cannot drift back. Anchors that borrow button chrome
  (`shell/coverage/view.ts:116`, `shell/leaderboard/view.ts:71`) use an `as: "a"`
  option on the wrapper.
- **Board boundary.** Pure-view primitives go everywhere. Submodel components
  (dialog, menu, popover, combobox — which lock scroll, mark siblings `inert`, and
  trap focus) are taken on the board **only** for overlays that already block board
  interaction. Canvas-adjacent chrome that must coexist with camera gestures and
  hand-drag Mounts — `activation-menu`, `inspect`, `log-panel`, `discoverability` —
  keeps its current behavior and adopts W1 wrappers only.
- **Delivery is a waved program, one PR per wave.** Each wave ships green and is
  revertible alone.

## Design

### 1. Recipe seam

`client/app/domain/ui/recipe.ts` wires cva's completion hook to the repo's existing
`cn` (clsx + `extendTailwindMerge` configured for this project's scales), so every
recipe inherits Tailwind-aware merging at one place:

```ts
import { defineConfig } from "cva";
import { cn } from "../cn";

export const { cva, cx, compose } = defineConfig({ hooks: { onComplete: (c) => cn(c) } });
```

All recipes import `cva` from this module, never from `cva` directly. `cn` itself
stays exported for the remaining non-recipe call sites.

### 2. Component wrapper shape

Recipe private, class applied automatically, behavior delegated to `@foldkit/ui`:

```ts
import * as Button from "@foldkit/ui/button";
import { cva } from "./recipe";

const recipe = cva({
  base: "cursor-pointer",
  variants: { variant: { primary: "…", ghost: "…", danger: "…", link: "…", game: "…", … } },
  defaultVariants: { variant: "primary" },
});

export function button<Msg>(h: HtmlFactory<Msg>, props: ButtonProps<Msg>, children: ReadonlyArray<Child>): Html {
  return Button.view<Msg>({
    onClick: props.onClick,
    isDisabled: props.disabled,
    type: props.type,
    toView: (a) => {
      const attrs = [...a.button, h.Class(recipe({ variant: props.variant, class: props.class })), …];
      return props.as === "a" ? h.a([h.Href(props.href ?? ""), ...attrs], children) : h.button(attrs, children);
    },
  });
}
```

`testId` is a first-class prop and always emits `data-testid`, since Scene tests key
off it.

### 3. Component inventory

`@foldkit/ui@0.132.0` splits into two integration shapes, which is what sets wave
boundaries:

| Shape | Components | Integration |
|---|---|---|
| Pure view (`view` only) | button, input, textarea, select, checkbox, switch, radioGroup, disclosure, fieldset, nav | Drop-in wrapper; no parent state |
| Submodel (`init` / `update` / `Model`) | dialog, menu, popover, tooltip, tabs, combobox, listbox, virtualList, toast, dragAndDrop, fileDrop, animation, calendar, datePicker, slider | Needs `Model` field + `update` branch + `h.submodel` in the owning surface |

Dialog renders **inline** through `h.submodel` on a native `<dialog>` opened with
`show()` (not `showModal()`), with a component-supplied backdrop, a focus trap, and a
`cancel` event on Esc. Nothing is portalled, so Scene assertions stay structural.

### 4. Implementation waves (for the later plan)

**W1 — Recipe seam + pure-view primitives that have call sites.**
Add `@foldkit/ui@0.132.0`. Add `recipe.ts` plus `button` and `input` wrappers.
The other eight pure-view primitives wait for a consumer: `textarea`, `select`,
`checkbox`, `switch`, `radioGroup`, and `disclosure` have zero call sites today,
and `nav` / `fieldset` have one each. `surfaces.ts` stays on `cn`: its helpers
have no variants, so a cva wrapper would produce exactly what `cn` already
produces. Migrate 47 `buttonClass` / `gameButtonClass` sites
across 19 files and the 12 `h.input` sites
(`shell/auth/view.ts`, `shell/decks/builder/view.ts`, `board/html/prompts.ts`).
Delete `buttonClass` / `gameButtonClass`. New indexed surface spec for
`client/app/domain/ui/`; update `DESIGN.md` components prose.

**W2 — Shell submodel components. Shipped.**
`dialog` → `domain/ui/confirmDialog.ts`, wired to the deck-list delete and deck-builder
discard prompts. `menu` → `shell/account-chrome/view.ts`, replacing hand-rolled
`aria-haspopup` + `BindAccountMenuEscape` and hoisting the open flag from three
duplicated `accountMenuOpen` booleans to one `Menu.Model` on the root model.

Two planned adoptions did not land. The decks-list context menu **stays hand-rolled**:
it is pointer-positioned, and `Menu` anchors its panel to a trigger button and returns
focus there. It takes the shared `menuPanelClass` / `menuItemClass` chrome instead.
`tooltip` / `popover` / `tabs` found no hand-rolled surface to replace — the only
tooltip-shaped code is the native `title=` attribute — so none were adopted. Updates
[`shell-routes-and-auth`](2026-07-20-shell-routes-and-auth.md) and
[`deck-list-and-builder`](2026-07-20-deck-list-and-builder.md).

**W3 — One modal chrome. Shipped.**
The planned W3 primitives had no shell surface to adopt. `listbox` / `combobox` are
anchored trigger+popup submodels, but the shell's three searches — deck-list, card
pool, coverage — are plain inputs whose results render **inline** in the page, so
adopting either would move results into a floating panel: a restyle, which the
Non-goals forbid. `virtualList` windows fixed-height rows, and the grids
that would want it were reserved for W4. `toast` found zero transient-notification surfaces (errors are
inline `alertClass` panels), so it was not introduced. `@floating-ui/dom` anchoring
therefore stays unexercised beyond W2's `Menu`.

W3 shipped the remaining `dialog` adoption instead. `domain/ui/dialog.ts` factors the
modal frame — `<dialog>`, backdrop, panel — out of `confirmDialog` into `modalDialog`,
and the deck builder's print picker moves onto it, gaining a focus trap, focus restore,
Escape, and a managed close. That emptied the hand-rolled `native-dialog.ts`, so it and
its `ModalOpened` / `OpenDialogAsModal` plumbing are gone and every modal in the client
is one chrome. Updates [`ui-component-layer`](2026-07-28-ui-component-layer.md) and
[`deck-list-and-builder`](2026-07-20-deck-list-and-builder.md).

**W4 — Windowed grids. Shipped.**
`domain/ui/windowedGrid.ts` wraps `@foldkit/ui`'s `virtualList` as a tile grid: rows of
`columns` items, only the rows near the viewport in the DOM. Both of the deck builder's
grids adopt it — the print picker, where a basic land's hundreds of printings each
fetched art, and the card pool, which is heading for tens of thousands of cards. Each
grid is a `VirtualList` submodel with a lifted subscription; the pool measures its own
column width, since `virtualList` reports height only, and its paging moves from an
IntersectionObserver sentinel to the window's own `endIndex` — a windowed grid has no
bottom element to observe. Windowing exposed the picker's other wait: Scryfall paginates
printings at 175, and the picker fetched every page before showing any, so a basic land
sat on skeletons. `searchPrints` splits into a single-page `searchPrintPage`, and the
update re-issues its own command for `nextPage` until there is none — foldkit `Command`s
emit exactly one message each, so a self-rechaining command is what stands in for a
stream. Updates [`ui-component-layer`](2026-07-28-ui-component-layer.md)
and [`deck-list-and-builder`](2026-07-20-deck-list-and-builder.md).

**W5 — Board dismissible modals.**
The result overlay and the concede confirm → `dialog`, as a `Dialog` submodel each in
`board/submodel.ts`; the result one is raised by `raiseResultDialog` on the fold that
ends the game, and `resultRaised` latches so a dismissed result stays dismissed. The
card-name typeahead (`game/intents.ts`, `board/messages.ts`, covered by
`board/card-name-typeahead.test.ts`) → `combobox`: it renders inside
`pending-card-name-modal`, so it clears the blocking-modal bar. `Combobox` is another
class-string primitive, so the `domain/ui/` contribution is chrome, not a wrapper — a
`hud` variant on `menuPanelClass` / `menuItemClass`, which already dress `Menu`'s
identically-shaped panel and rows, plus `inputClass` for the input the combobox renders
itself. The typeahead gains arrow-key navigation of its suggestions and loses
Enter-to-submit: `Combobox` bakes its own keydown handler into the input and snabbdom
merges attributes by event name, so a second one would overwrite it. Enter now commits
the highlighted name into the draft and the Name button submits.

`prompt-modal` and `mulligan-overlay` are a **deliberate exclusion**, not a deferral.
`Dialog` bundles the Escape handler into `render.dialog` and the outside-click handler
into `render.backdrop` with no way to drop either, so any prompt on that frame can be
dismissed — and a dismissed pending choice leaves the engine waiting on an answer the
player can no longer give. They stay hand-rolled under the board boundary rule. Non-modal
board chrome is otherwise explicitly untouched. Updates
[`prompts-and-pending-choices`](2026-07-20-prompts-and-pending-choices.md) and
[`system-overlays`](2026-07-20-system-overlays.md).

## Components & data flow

- `design.tokens.json` → `gen-tokens.mjs` → Tailwind `@theme` (`tokens.generated.css`)
  — unchanged by this program.
- `domain/ui/recipe.ts` (`cva` + `cn`) → module-private recipes → `domain/ui/<name>.ts`
  wrappers → view code. Views pass props; they never assemble class strings.
- `@foldkit/ui` supplies ARIA attributes and behavior into the wrapper's `toView`
  callback; the wrapper supplies markup and classes. Submodel components add a
  `Model` field and `update` branch in the owning surface's submodel
  (`shell/decks/submodel.ts`, `board/submodel.ts` and siblings).

## Testing Decisions

Per [`client-interaction-test-policy`](2026-07-22-client-interaction-test-policy-design.md),
every wave asserts user-visible outcomes and is named for product behavior — never as
"parity" or migration checks.

- Wrapper unit tests: variant token appears in the rendered class; `as: "a"` emits an
  anchor with `href`; disabled suppresses the click message.
- Scene coverage extends `client/app/shell/surfaces.test.ts` and
  `client/app/board/html/surfaces.test.ts` per wave.
- New behavior gets explicit assertions: dialog exposes `role="dialog"`, focus lands
  on the `initialFocus` marker, Esc closes and focus returns to the trigger, menu
  trigger `aria-expanded` flips, arrow keys move active descendant.
- `just client-check` (tokens check, lint, typecheck, vitest) green per wave.

## Out of Scope

- A component tier in `design.tokens.json`.
- Replacing the canvas/bitmap board rendering path or its Mount hosts.
- `dragAndDrop` / `fileDrop` for hand-drag (canvas-integrated; not a DOM drag).
- `calendar` / `datePicker` / `slider` — no surface needs them today.
- Light mode / theming (still deferred per the DTCG spec).

## Further Notes

- `@foldkit/ui` version tracks `foldkit` and `effect` exactly (0.132.0 ↔
  `effect 4.0.0-beta.101`). It joins the existing AGENTS.md lockstep rule: bumping
  `foldkit` or `effect` bumps `@foldkit/ui` in the same change.
- `@floating-ui/dom` arrives as a new runtime transitive dependency via `@foldkit/ui`.
- `cva` is pinned at a `1.0.0-beta`; the `defineConfig` seam keeps the blast radius of
  an API change to `recipe.ts`.
