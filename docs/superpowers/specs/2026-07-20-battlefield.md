# Battlefield

**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/canvas/`, `client/app/board/bitmap/`, `client/app/board/chrome.ts`, `client/app/board/geometry/layout.ts`, `client/app/board/geometry/density.ts`

---

## Problem Statement

The battlefield must show a crowded Commander table with stable seat furniture, readable permanents, live combat/targeting arrows, and MTGA-style chrome without requiring every permanent to be a DOM node. It must also avoid misleading presentation: unplayable cards should not look disabled; playable actions should be called out with borders.

## Solution

Battlefield paint is split across Canvas vector shapes and Mount bitmap card layers. Canvas handles felt, seats, avatars, and arrows. The bitmap layer paints resting permanent faces and permanent chrome using the shared image cache. Flights are documented separately in [`2026-07-20-flights.md`](2026-07-20-flights.md).

The board layer stack authority is [`docs/client-canvas-map.md`](../../client-canvas-map.md). The battlefield paint order is felt → seats → resting cards → avatars → arrows → flights.

## User Stories

- As a player, I can read each permanent and see relevant battlefield chrome.
- As a player with priority, I can tell which battlefield permanents have playable actions from their outline.
- As a player declaring combat or targeting, arrows and target highlights stay above resting cards.
- As a player on a crowded board, packing and clusters keep permanents inside their seat bands.

## Behavior

### Paint order

The battlefield paints bottom to top:

1. Felt.
2. Seat bands and zone furniture.
3. Resting battlefield cards.
4. Avatars and life-orb paint.
5. Combat, block, spell-targeting, and drag-aim arrows.
6. Card flights.

Flights are always above resting cards and are covered in [`2026-07-20-flights.md`](2026-07-20-flights.md). HTML life-orb hit targets sit above this paint so combat drops can be targeted reliably.

### Felt and seats

`canvas/felt.ts` paints the table background and speckles. Seat geometry comes from `layout.ts`: seat bands reserve space for battlefield rows, zone columns, avatars, and mana. Packing must not cover avatar paint or move cards outside their seat band.

### Resting permanents

`bitmap/mount.ts` paints resting battlefield permanents through `paintCard`. Card faces use `sharedImageCache` with fallback art/name paint when images are not decoded yet. The resting layer skips ids in `hideCardIds` so a flying card is not double-drawn.

Resting permanent chrome includes:

- Base resting outline.
- Commander gold outline.
- Playable border when the object has a current battlefield action.
- Target highlight for staged object targets.
- Auto-tap preview glyphs.
- Summoning-sick, keyword, goaded, prepared, owner-strip, P/T, loyalty, counter, and marked-damage badges where the wire exposes those values.

Unplayable permanents are not darkened. Castability and activation availability are represented by playable borders and action affordances.

### Selection

- Permanents that have activatable abilities (including ones that are presently illegal) are selectable so the radial can list them with disabled wedges.
- Permanents with no activates are not selectable.
- Tap-only mana lands (tap-for-mana only) are selectable so the tap wedge can open.
- Always-on seat/controller borders on every permanent are not used.

### Playable outlines

`chrome.ts` defines battlefield outline colors:

- `CARD_RESTING_OUTLINE = "#1a1a1a"`
- `PLAYABLE_BORDER = "#EAFFF0"`
- `COMMANDER_GOLD = "#E9B84A"`
- `GRAVEYARD_OUTLINE = "#7B5CFF"`
- `EXILE_OUTLINE = "#3DDC97"`

Battlefield playable borders are derived from current `ActionView` data. Tap-only mana lands remain selectable for their tap wedge but do not get a playable border unless they have another action. Commander gold can coexist with a playable border as an outer halo.

### Avatars

Avatars are painted from `canvas/avatars.ts` using the same camera transform as cards. The priority player uses a gold stroke. Lost players render with muted fill. Player life, name, and hand count paint inside the avatar group. Targetable player highlights use Island Blue.

### Arrows and target highlights

`canvas/arrows.ts` paints combat and targeting arrows above resting permanents. Attack arrows are Mountain Red (`#ff6b6b`), block arrows are Wall Green (`#66ff99`), and spell/object target highlights use Island Blue (`#77CCFF`). Declare-attackers drag aim uses the same arrow layer as committed arrows.

### Canvas hex colors

Canvas and bitmap paint use explicit hex and rgba literals rather than Tailwind classes. Important values include:

- Felt base `#0B1310`; felt speckles `#1a2a22`.
- Priority gold `#ffd76a`.
- Attack red `#ff6b6b`; block green `#66ff99`; target blue `#77CCFF`.
- Face-up fallback `#e8e4d8`; face-down fallback `#2a3742`.
- Badge examples: summoning sick `#e8b24a`, goaded `#7a3b13`, prepared `#55cc99`, counters `#2f7d46`, marked damage `#8f2f2f`.

When badge or outline meaning changes, update [`DESIGN.md`](../../DESIGN.md) and the board legend together.

### Packing and clusters

Density overlays affect where cards paint and how topmost ordering works:

- Row packing compresses crowded rows inside the seat band.
- Clusters can replace indistinguishable groups with one face and a count.
- Fanning a cluster paints members in an arc and keeps them inside the seat band.
- Hover raise moves the hovered card and its attachment stack to the top of the resting-card order.

These are visual/layout rules only; they do not collapse engine objects.

## Implementation Decisions

- Keep battlefield cards on the Mount bitmap layer; do not turn every permanent into HTML.
- Paint playable availability with outlines, not unplayable darkening.
- Keep arrows above resting cards so combat and targeting remain legible.
- Keep canvas colors as code literals and sync user-facing meaning through `DESIGN.md`.
- Keep avatar paint below HTML life-orb hit targets.

## Testing Decisions

- Canvas scene tests assert felt, seat, avatar, and arrow ordering.
- Bitmap paint tests assert playable, commander, target, auto-tap, P/T, loyalty, counter, and damage chrome on the resting layer.
- Scene tests assert arrows and interactive life-orb hit targets remain layered correctly.
- Density tests assert packing, cluster fan, and hover raise order.

## Out of Scope

- WebGL or worker-based paint.
- Under-card resting name labels.
- New counter kinds not exposed by the wire.
- Changing combat or targeting legality; this spec covers presentation only.

## Further Notes

- Sibling specs: [`2026-07-20-board-composition.md`](2026-07-20-board-composition.md), [`2026-07-20-board-camera-and-layout.md`](2026-07-20-board-camera-and-layout.md), [`2026-07-20-flights.md`](2026-07-20-flights.md).
