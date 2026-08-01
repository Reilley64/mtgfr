# Arena Card Frame Renderer
**Status:** Design input (as of 2026-08-01)
**Module (planned):** `client/app/domain/card-render/`, `client/app/board/bitmap/paint-cards.ts`, `client/app/board/html/hand.ts`, `client/app/board/html/stack.ts`, `client/app/board/geometry/layout.ts`, `crates/server/src/catalog*.rs`, `proto/mtgfr/v1/catalog.proto`, `proto/mtgfr/v1/stream.proto`

## Problem Statement

Every card surface paints the printed Scryfall image, so the board cannot present a card any way
but whole. Three presentations we want are impossible with a printed image:

- Battlefield permanents should be Arena-style square tiles — name slot and art, no oracle text —
  so a four-seat table reads at a glance. Tokens should be distinguishable from cards at a glance.
- Hand cards should not carry a mana cost on the face; the cost belongs to the pip tray under
  the card, which already exists.
- A stack entry for an activated or triggered ability should show *that ability's sentence*, not
  the whole card's text.

Cropping or masking the printed image was rejected: crop fractions are tuned to one frame and
misbehave on old borders, Sagas, and split cards, and a patch panel cannot match a hundred frame
treatments.

## Solution

Draw the card ourselves. A card renderer composites vendored frame assets, the art crop, and text
into a bitmap, and the three board surfaces fill different slots of the same template. Fidelity
target is a small version of the actual card: real frame art, real typefaces, real mana glyphs.

## User Stories

- As a player, I read my board at a glance: every permanent is a square tile with a name slot and
  its art, and a token is obviously a token.
- As a player, my hand cards read as real cards, and I read their cost from the pip tray.
- As a player, when an ability goes on the stack I see the ability that is actually resolving,
  not the card's whole text box.

## Behavior

### Renderer

`renderFace(data, variant)` draws to an OffscreenCanvas and returns a bitmap cached by
`(print, variant, faceIndex, textHash)`.

| Variant | Slots drawn | Consumers |
|---------|-------------|-----------|
| `permanent` | frame, title bar, art window; square. Tokens: arched top, no title bar | battlefield Mount paint, flights, drag ghost, exit FX |
| `full` | frame, title bar, art window, type line, text box, P/T or loyalty. **Mana cost is never drawn** | hand bar tiles (hand, command, graveyard, exile) |
| `stack` | as `full`, but the text box holds only what is on the stack — a spell's own oracle text, or a single ability's sentence — plus its declared targets | stack overlay |

`full` and `stack` render at 745 × 1040 (the printed card's proportions at Scryfall `normal`
size); `permanent` renders at 745 × 745. Both scale down at paint time. The cache is an LRU over
bitmap count.

### Battlefield

- A permanent is a square tile: frame, title bar with the card name, art window, P/T box. No type
  line, no text box — card inspect remains the read-the-card surface.
- A token draws with an arched top and no title bar.
- Chrome layered over a permanent is unchanged: tap rotation, keyword glyph badges, counters,
  commander gold, target dashes, cluster count badge, attach offsets, seat aura.
- A face-down permanent keeps the card back and fetches no print render data.

### Hand and zone bar

- Hand, command, graveyard, and exile tiles paint the `full` face. The resting pip row remains the
  only place a cost appears.
- Tile geometry, playable aura, `data-*` chrome, hover raise, drag threshold, and hit widths are
  unchanged; only the pixels inside the tile change.
- A play flight paints the `full` face for the whole flight; the square `permanent` tile appears
  when the card comes to rest. No mid-flight morph.

### Stack

- A spell shows its own oracle text in the text box.
- An activated or triggered ability shows only that ability's printed sentence, with declared
  targets listed beneath it.
- The sentence comes from a new `oracle` field on the ability in the card TOML, projected onto
  `StackObjectView`. When a card has no ability oracle recorded, the existing `label` message ref
  renders instead.
- A spell's text box is the printing's oracle text, unchanged.

### Frame coverage

- Every printed **layout** is templated: normal, creature, planeswalker, saga, class, adventure,
  leveler, split, flip, battle, and double-faced backs.
- Non-standard **frame treatments** — borderless, showcase, extended art, retro, textless — are
  bespoke per-card art and cannot be reproduced by a template set. Those printings render in their
  nearest standard frame; the special treatment is lost.

### Data

- New `PrintRenderView` message: printed mana cost string, type line, oracle text, flavor text,
  power, toughness, loyalty, rarity, set code, artist, collector line, layout, frame,
  frame effects, border color, full-art flag, watermark, and per-face data for double-faced cards.
- New `CatalogService.GetPrintRenders(prints[])` RPC.
- Server holds a `printings` table in Postgres `mtgfr`, filled lazily from Scryfall `/cards/{id}`
  the first time a printing is requested and cached from then on. No bulk ingest.
- The client fetches render data per printing when a seat's deck loads, alongside the art warming
  the lobby already performs.

`WireCost` cannot express hybrid, X, or Phyrexian costs, which is why the printed mana cost string
is carried as text rather than derived from the existing cost message.

`ObjectView` also gains `is_token` and `legendary`. The renderer needs both to pick a frame — a
token draws arched with no title bar, a legendary permanent draws the crown — and neither is
derivable from the fields on the message today.

## Implementation Decisions

- One renderer, three variants — not three renderers. A single frame/slot model keeps the surfaces
  from drifting, and the same cached bitmap serves canvas paint and DOM tiles.
- Battlefield blits the bitmap exactly where it blits printed art today. Hand and stack place the
  bitmap in a `<canvas>` inside their existing markup, so pointer, drag, keyboard, and `data-*`
  contracts are untouched.
- Frame images, set-symbol SVGs, and the Beleren / MPlantin typefaces are vendored under
  `client/public/`. Mana glyphs come from the icon font `paint-cards.ts` already loads.
- Module split under `client/app/domain/card-render/`: `assets.ts` (frames, fonts), `text.ts`
  (wrapping, mana-glyph inlining, auto-shrink), `frame.ts` (template selection and slot rects per
  layout), `render.ts` (the draw), `cache.ts` (bitmap LRU). `frame.ts` and `text.ts` are pure.
- Card inspect, the deck builder, and hover preview keep the printed image. They exist to read the
  actual card, and the printed image is the most faithful thing available.
- A square permanent tile changes `CARD_W` / `CARD_H` (96 × 134 today), which moves row packing,
  seat band width, attach offsets, and `fitCamera`. Those constants and their geometry locks move
  in the battlefield slice.

## Testing Decisions

- `frame.ts` and `text.ts` are unit-tested as pure functions: slot rects per layout at a fixed
  canonical size, wrap and auto-shrink behavior for long text boxes, mana-symbol inlining.
- `render.ts` is tested against a fake canvas context by asserting the draw-op sequence. No pixel
  snapshots.
- Scene tests cover the battlefield tile, hand tile, and stack face rendering through the real
  surfaces; `handBarHit.test.ts` and the layout geometry locks take the square tile's numbers.
- Server tests cover a cold printing fetching once from Scryfall, caching, and serving the second
  request from Postgres.
- Schema projection tests lock the ability oracle sentence onto `StackObjectView`, including the
  fallback to `label`.

## Slices

1. `PrintRenderView` RPC and the printings cache, plus `is_token` / `legendary` on `ObjectView`.
   Frame selection needs the printing's layout, frame, and rarity, so no variant can be drawn
   before this lands.
2. Renderer, vendored assets, and the `permanent` variant behind the battlefield (includes the
   square-tile geometry change).
3. The `full` variant in the hand bar.
4. Per-ability oracle text in the card DSL and the `stack` variant on the stack.

Each slice ships on its own.

## Out of Scope

- Reproducing borderless, showcase, extended-art, retro, or textless treatments.
- Replacing the printed image in card inspect, the deck builder, or hover preview.
- Server-side or build-time card rendering; faces render in the browser.

## Further Notes

- Surface specs this design changes at implementation time: [`battlefield`](2026-07-20-battlefield.md),
  [`hand-and-zone-bar`](2026-07-20-hand-and-zone-bar.md), [`stack`](2026-07-20-stack.md),
  [`board-camera-and-layout`](2026-07-20-board-camera-and-layout.md), [`flights`](2026-07-20-flights.md),
  [`wire-protocol-and-visibility`](2026-07-20-wire-protocol-and-visibility.md),
  [`accounts-decks-and-catalog`](2026-07-20-accounts-decks-and-catalog.md), and
  [`card-dsl-and-card-pool`](2026-07-20-card-dsl-and-card-pool.md).
- Companion docs that move with it: [`docs/client-canvas-map.md`](../../client-canvas-map.md) (the
  renderer joins the Mount bitmap layer) and `.agents/skills/card-dsl/DSL_REFERENCE.md` (the
  ability `oracle` field).
- The card TOML already quotes each ability's printed sentence in a comment above its
  `[[abilities]]` block, so the last slice promotes an existing convention to a field.
