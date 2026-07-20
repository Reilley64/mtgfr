# Choose-X cast prompt redesign

**Date:** 2026-07-20  
**Status:** Approved for planning  
**Context:** Casting Hangarback Walker (`{X}{X}`) with 7 mana available felt broken — players expect either “pay 7” or an affordable X, but the client offered an unbounded number field and no cost preview. Rules-correct: no integer X satisfies `2X = 7`; max with 7 mana is X=3 (pay 6).

## Goals

1. **Only allow valid X values** — clamp the chooser to the engine’s legal range for this cast/activation right now.
2. **Show the resulting mana cost** — live preview of what will be paid after X is folded in (so `{X}{X}` never looks like “type 7”).
3. **Min / Max affordances** — one-click jump to bounds; − / + and typeable field, all clamped.
4. **Arena-aligned contract** — server sends min/max; client only renders and submits X.

## Non-goals (v1)

- Denylist of sparse illegal X values (Arena’s Living Breakthrough extension). Revisit when we have effects that forbid specific mana values.
- Client-side mana planning that duplicates the engine’s payment solver.
- Changing cast order for optional additional costs (sacrifice/discard) beyond today’s flow.
- Free-cast / “without paying mana cost” UX beyond whatever `max_x`/`min_x` the engine already projects (typically X must be 0).

## Arena reference

From WotC GRE writeups ([Living Breakthrough](https://magic.wizards.com/en/news/mtg-arena/on-whiteboards-naps-and-living-breakthrough)) and Neon Dynasty SotG:

- GRE asks the duel client to pick X with a **minimum and maximum legal value** (later: plus specifically disallowed values).
- Player chooses **X**, not total mana spent; cost is derived (`{X}{X}` → pay `2×X`).
- Cast flow: choose X → optional additional costs → pay.
- Accessible client overlays describe a bounded value control (±1 / ±5), not an unbounded text box.
- Magarena’s X panel defaults the stepper to **maximum X**.

We match the GRE contract and Magarena default; Min/Max buttons are our Commander-scale convenience on top of Arena.

## Approach

**Server-authoritative bounds** (chosen over client estimates or reject-and-nudge).

The engine already knows `Cost.x` (symbol count) and can compute the largest payable X via the same affordability path used for playability / auto-tap. The client must not invent that number.

## Wire & projection

### `WireCost`

- Add `x_symbols: u8` — engine `Cost.x` (0 = no `{X}`). Source of truth for preview math.
- Keep existing `has_x` as an expand-only-compatible boolean (`x_symbols > 0`) so older projection/client paths keep working during the roll; new UI reads `x_symbols` + action `min_x`/`max_x`. Do not remove `has_x` in this change ([WIRE_COMPAT.md](../../WIRE_COMPAT.md)).

### Legal actions that need an X choice

For cast, cast_prepared / adventure, and activate (any action that today sets `ActionView.has_x`):

| Field | Meaning |
|-------|---------|
| `min_x` | Lowest legal X (almost always `0`; may be higher if a card forbids X=0 later) |
| `max_x` | Highest X payable *right now* with the same planner the engine uses for affordability / auto-tap |
| `x_symbols` | On the paid cost / `WireCost` for preview math |

Projection notes:

- `max_x` must respect cost reducers, colored pips, hybrid, pool, and untapped producers the auto-tap path would use — same truth as “can I cast this at all?”
- If even X = `min_x` is unaffordable (e.g. `{X}{R}` with no red), the action should not be offered (existing playability); the prompt must not open in a dead state.
- Free-cast / alt costs that force X=0: project `min_x = max_x = 0`.

### Client prompt payload

`XPromptModal` (or successor) receives at least:

- `name` — card/ability label  
- `x_symbols`  
- `min_x`, `max_x`  
- Fixed (non-X) cost pips for preview — from the object/action’s paid `WireCost` after stripping X symbols into `x_symbols`

Chosen X continues to ride the cast/activate intent’s existing `x` field. Engine remains the final payment check.

## UI

**Control model:** stepper (approved).

| Element | Behavior |
|---------|----------|
| Title | “Choose X for {name}” |
| Cost preview | Live mana pips for total cost at current X (`generic += x * x_symbols`, plus fixed colored/hybrid) |
| Optional hint | When cheap: effect of X (e.g. Hangarback “Enters as 3/3”) — only if we already have a simple mapping; skip clever oracle parsing in v1 if costly |
| Row | **Min** · **−** · **[editable number]** · **+** · **Max** |
| Default | `max_x` (Arena/Magarena muscle memory) |
| Clamp | All inputs stay in `[min_x, max_x]`; invalid keystrokes snap on blur/input |
| Keys | Enter = Cast, Esc = Cancel |
| Actions | Cast · Cancel |

Visual language: reuse existing prompt atoms (`Modal`, `Button`, `Field`, `PROMPT_TITLE` / `PROMPT_ROW`) and design tokens from `DESIGN.md` / `global.css` — forest HUD, no new card chrome.

## Data flow

```
player starts cast/activate with X
  → takeCastAction sees has_x / x_symbols > 0 and x unset
  → open XPromptModal(min_x, max_x, x_symbols, fixed cost, name)
  → user confirms X
  → takeCastAction(..., x) → take_action intent
  → engine verifies payment (unchanged authority)
```

## Error handling

- If `max_x < min_x` (should not happen): treat as bug; do not open prompt; surface existing reject path.
- Engine reject after submit (stale state / race): keep existing humanized reject; optional follow-up to refresh bounds — out of scope for v1 polish.

## Testing

- **Engine / schema:** project `x_symbols` and `max_x` for Hangarback with known mana (e.g. 7 available → `max_x = 3`); Fireball `{X}{R}`; Astral Cornucopia-style `x_symbols = 3` if in pool.
- **Client unit:** clamp helpers (type 99 → max; Min/Max; default = max); cost preview math for `x_symbols = 1` and `2`.
- **Regression:** existing Hangarback / X cast engine tests still pass; cast intent still carries `x`.

## Implementation sketch (for planning)

1. Schema/proto: `x_symbols` on cost; `min_x` / `max_x` on X-bearing actions (or a nested `x_choice` message).
2. Engine helper: largest affordable X for a cost + player (binary search or linear up from 0 using existing `available_mana` / pay planner).
3. Snapshot projection wires the fields.
4. Client codegen + `XPromptModal` redesign + `actionExecution` passes new fields.
5. Tests as above; `just check` / focused nextest + client tests.

## Success criteria

- With 7 mana and Hangarback in hand, the prompt’s Max is **3**, cost preview shows **{6}**, and typing **7** snaps to **3** (or is rejected as input).
- Min jumps to **0**; Cast at 0 remains legal when affordable.
- `{X}{R}` shows colored pips in the preview and Max respects the colored requirement.
