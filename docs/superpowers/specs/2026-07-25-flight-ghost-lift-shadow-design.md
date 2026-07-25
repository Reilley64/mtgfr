# Flight & hand-drag lift shadow — design

**Status:** Approved for planning (2026-07-25)  
**Living surface specs to update at implement time:**
[`2026-07-20-flights.md`](2026-07-20-flights.md),
[`2026-07-20-hand-and-zone-bar.md`](2026-07-20-hand-and-zone-bar.md)

## Problem

Canvas in-flight cards use a soft black lift shadow (`FLIGHT_SHADOW_*` in
`paint-flights.ts`: blur 28, offsetY 12, alpha 0.55). Against the forest table
that shadow reads green-tinted rather than a normal black lift. The hand drag
ghost already carries `--drop-shadow-drag` (`0 16px 36px rgb(0 0 0 / 0.72)`), so
flight and ghost lifts are also out of sync.

## Goal

Flights and the hand drag ghost share one black lift recipe: the existing
`--drop-shadow-drag` token. Flights should read as a normal elevated card shadow,
not a green cast on the felt.

## Non-goals

- Deep lift on the **stack staged ghost** — Arena treats the targeting-time stack
  face like a normal stack card (`shadow-hand` stays).
- Changing playable mint ring / glow from `barZoneAura` (border chrome, not lift).
- Resting hand tile shadows, battlefield permanent shadows, or `shadow-glow`.
- Expanding Style Dictionary TS codegen beyond colors (optional later).

## Approach

**Shared lift mapping from the drag token** (rejected: nudge flight constants only
and let them drift; rejected: hand-drawn under-card ellipse instead of canvas
`shadow*`).

### Source of truth

- Token: `design.tokens.json` → `drop-shadow.drag` → CSS `--drop-shadow-drag`:
  `0 16px 36px rgb(0 0 0 / 0.72)`.
- Hand drag ghost keeps the Tailwind/`drop-shadow-drag` filter class.
- Canvas flights consume a small pure helper that maps that recipe to
  `shadowOffsetY = 16`, `shadowBlur = 36`, `shadowColor` equivalent to
  `rgb(0 0 0 / 0.72)` (e.g. `rgba(0,0,0,0.72)`).

### Shared helper

- Pure module next to board paint/chrome (e.g. exports used by `paintFlightCard`).
- Replaces today’s `FLIGHT_SHADOW_BLUR` / `OFFSET_Y` / `COLOR` literals (28 / 12 / 0.55).
- Unit test asserts the helper’s canvas fields stay equal to the drag token string
  in `design.tokens.json` or `tokens.generated.css` so CSS and canvas cannot drift.

### Surfaces

| Surface | Lift |
|---------|------|
| Canvas flight (`paint-flights.ts`) | Canvas shadow from shared helper (= drag token) |
| Hand drag ghost (`hand.ts`) | Keep `drop-shadow-drag`; no second competing lift recipe |
| Stack staged ghost (`stack.ts`) | Unchanged — `shadow-hand` only (Arena parity) |

## Testing

- Update `paint-flights.test.ts` for the new lift constants.
- Unit-assert shared helper ↔ `--drop-shadow-drag` / token string parity.
- Keep existing hand ghost coverage that expects `drop-shadow-drag`; add only if
  the ghost class string changes.
- No engine/server work; focused client tests are enough.

## Spec updates at implement time

- **Flights:** document that in-flight lift matches `--drop-shadow-drag` (black,
  offset 16 / blur 36 / alpha 0.72); canvas flights still omit the playable ring.
- **Hand-and-zone-bar:** document that the drag ghost lift is the same shared
  drag token used by flights.

## Out of scope / follow-ups

- If soft canvas shadows still tint on the felt after the opacity match, consider
  a manual under-card shadow paint (approach rejected for v1).
- Generating drop-shadow values into `design-tokens.generated.ts` via
  `gen-tokens.mjs` if more canvas consumers appear.
