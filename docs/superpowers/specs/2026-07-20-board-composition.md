# Board Composition

**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/view.ts`, `client/app/board/submodel.ts`, `client/app/board/messages.ts`, `client/app/board/html/`, `client/app/board/bitmap/`, `client/app/board/canvas/`

---

## Problem Statement

The in-game Commander table must stay readable while showing four seats, hundreds of permanents, a hand bar, stack, prompts, combat affordances, inspect, reconnect state, and audio/keyboard helpers. A single DOM surface is too expensive for the battlefield, while a single canvas surface is wrong for forms, buttons, hand tiles, stack text, and accessibility.

## Solution

The board is a Foldkit submodel with three coordinated surfaces:

- Foldkit Canvas for vector battlefield furniture, seats, arrows, and vector helpers.
- Foldkit Mount canvases for bitmap resting card art, authoritative avatar life orbs, and in-flight card art.
- HTML overlays for hand, stack, prompts, priority chrome, mana tray, inspect, sound, legend, concede, and result UI.

`view.ts` is the composition root. `submodel.ts` owns board state and update logic, and `messages.ts` defines the board message protocol. The authoritative layer stack lives in [`docs/client-canvas-map.md`](../../client-canvas-map.md); board visual changes must follow that map.

## User Stories

- As a seated player, I see the table from my perspective with my hand and controls available.
- As a spectator, I can watch the table without hand, priority, or action controls.
- As an eliminated player, I can keep watching the game without hand or action controls.
- As a player with a screen reader, I get a spoken summary of the current board state.
- As a player reconnecting to a table, I see clear reconnect status without losing the board.

## Behavior

### Root composition

The board root is `data-testid="board-mount"` in both connecting and live states. It is full-screen, overflow-hidden, and `select-none` so drag gestures do not trigger native browser text selection. This is scoped to the board; lobby, deck builder, forms, and prose remain selectable.

When no `VisibleState` is available, the board renders the same root plus `data-testid="board-connecting"` and a connecting message. Once state is present, the root mounts keyboard, audio, and hint helpers as separate hidden children so each Mount hook remains active.

### Surfaces and overlays

The live board composes:

- `Canvas.view` for vector scene shapes and pointer events.
- `manaTrayView` projected from world coordinates as DOM below resting permanents.
- `board-bitmap-layer` for resting battlefield faces, card chrome, and avatar life orbs.
- `boardOverlays` for hand, stack, prompts, priority, discoverability, concede, pile expand, result, and inspect.
- `board-flight-layer` for in-flight cards above hand and stack.
- `board-reconnecting` when the stream is disconnected.

The layer order is not redefined here; [`docs/client-canvas-map.md`](../../client-canvas-map.md) is the authority.

### Submodel and messages

`BoardModel` carries camera, viewport, pointer state, selected permanent, staged action, combat staging, prompts, modal picks, pile expand, inspect, sound, hand hiding, and flight ownership. `updateBoard` handles `Message` values from pointer events, keyboard, prompts, sound, action planning, flight sync, and game sync.

HTML controls dispatch board messages; board updates either mutate local state, return Effect commands, or submit wire intents through the BFF. Engine choices remain server-authoritative.

### Spectator and eliminated viewers

Spectators render a read-only board with a fixed spectator badge and no hand bar or action affordances. Eliminated players keep the board and result/watch flow, but their hand and action controls are removed. Server-side intent rejection remains the final guard for spectators and inactive seats.

### Reconnect banner

When the game stream is disconnected, `data-testid="board-reconnecting"` appears fixed across the top of the viewport in reconnect-rust styling. It is presentation only; the stream and replay behavior live outside the board view.

### Accessibility

The root includes an `sr-only` `aria-live="polite"` region populated by `boardStatusSummary(state, viewer)`. The canvas is an unlabeled pointer surface. DOM life-orb controls use player/life labels, and quiet controls use enlarged hit targets for coarse pointers.

### Image preload

Board image use goes through `sharedImageCache`. Published bitmap frames preload visible resting-card, flight-card, and card-back URLs before painting so decode work is not tied to a single draw call.

### Prompts

Prompt HTML is local to `client/app/board/html/prompts.ts`. Pending engine choices render interactive forms only for the awaited player; non-deciders and spectators do not receive actionable prompt DOM. Client-local prompts remain local board state.

The X prompt follows the current `boardXPrompt` shape: a clamped Min/−/value/+/Max stepper with a `Pay {…}` cost preview and Confirm/Cancel. Details live in [`prompts-and-pending-choices`](2026-07-20-prompts-and-pending-choices.md).

### Inspect

Alt/Option inspect pins a face-up board, hand, or stack card into the shared dock-style inspect overlay. The overlay includes a backdrop, blocks board/HUD clicks while open, and is topmost in the board layer stack. Releasing Alt/Option, pressing Escape, or clicking the backdrop dismisses it.

### Hand and playable chrome

The hand, command, graveyard, and exile bars use outline language for playable actions. Unplayable hand and command cards remain full brightness with no playable border. Drag sources and cards hidden by a flight may fade or disappear as interaction state, not as unplayable-state styling.

### Table audio

Table audio is synthesized with Web Audio. The happy path unlocks audio synchronously from the lobby Ready click. Turning Sound on in-game also attempts unlock and plays a short confirmation tick when playback is available. Muted or suspended contexts no-op silently.

### Landscape Rule

The board follows the Landscape Rule in [`DESIGN.md`](../../DESIGN.md): portrait phones show the rotate gate; the board is not reflowed into a vertical mobile layout.

### Data-testid markers

Stable markers include `board-mount`, `board-connecting`, `board-keyboard-mount`, `board-audio-mount`, `board-hint-mount`, `board-bitmap-layer`, `board-flight-layer`, `board-reconnecting`, `x-prompt`, `pending-choice-waiting`, `inspect-overlay`, `life-orb-{seat}`, and `bf-card-{id}`. Tests use these markers for cold-load, route-entry, and interaction coverage.

## Implementation Decisions

- Keep the board as Canvas + Mount + HTML; do not merge it into one retained scene graph.
- Treat `docs/client-canvas-map.md` as the layer stack authority.
- Keep required identifiers in route path params; board prompts and local modes are not routes.
- Keep selection prevention on `board-mount`, not globally.
- Keep prompt visibility decider-scoped even though wire state may expose redacted pending-choice facts to other viewers.
- Keep audio unlock tied to Ready and Sound-on gestures; do not add board-wide pointer unlock spam.

## Testing Decisions

- Scene tests cover root mounting, `select-none`, layer order, board keyboard/audio mounts, overlays, prompts, hand chrome, and inspect.
- Unit tests cover board update outcomes for spectator/eliminated behavior, prompt state, sound toggles, and keyboard handling.
- Live board verification should exercise route entry, reconnect banner visibility where feasible, and a seated play path with hand, stack, and priority chrome.

## Out of Scope

- WebGL, Pixi, Konva, or a unified retained scene graph.
- Portrait board reflow.
- Per-card audio files, music, or voice.

## Further Notes

- Sibling specs: [`2026-07-20-board-camera-and-layout.md`](2026-07-20-board-camera-and-layout.md), [`2026-07-20-battlefield.md`](2026-07-20-battlefield.md), [`2026-07-20-flights.md`](2026-07-20-flights.md), [`2026-07-20-prompts-and-pending-choices.md`](2026-07-20-prompts-and-pending-choices.md) (non-decider waiting banner).
