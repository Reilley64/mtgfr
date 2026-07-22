# Foldkit DevTools tooling + Arena playable chrome

**Status:** Draft  
**Date:** 2026-07-22  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74) (`cursor/foldkit-migration-design-1ef0`)

## Goal

Land Foldkit agent tooling first (DevTools MCP + vendored skills) so board work can be debugged live, then fix activation radial centering, in-game Alt/Option inspect (still broken), top-left HUD control layout, and always-on permanent borders → Arena-style playable / zone outline language.

## Approach

**B — Tooling first, then board chrome** (chosen): MCP + skills commit before radial/selection/border/inspect/HUD work so implementers can use `foldkit_*` tools while fixing UI.

## Workstream 1 — Foldkit MCP + skills

### DevTools MCP

- Add `@foldkit/devtools-mcp` as a client `devDependency`.
- Register the server in `.cursor/mcp.json` alongside `scryfall` (follow upstream Cursor/`npx @foldkit/devtools-mcp init` recipe).
- Vite: pass `devToolsMcpPort: 9988` to `foldkit()` in `client/vite.config.ts`.
- Runtime: keep `devTools: { Message }` in `client/app/entry.ts` (already present) so dispatch tools work.
- Bridge only sees a runtime while the app tab is open — note in `AGENTS.md`.

### Skills (vendored)

- Copy upstream `skills/{foldkit,generate-program,audit-program}/` from [foldkit/foldkit](https://github.com/foldkit/foldkit) into `.agents/skills/`.
- Register in `skills-lock.json` if this repo pins external skills that way.
- Upstream text may assume `repos/foldkit/`; retarget “where to look” to this repo’s Foldkit install (`client/node_modules/foldkit` / project docs). Do **not** add a full Foldkit git subtree unless a skill is unusable without it.

## Workstream 2 — Board chrome

### Radial centering

SVG center = selected card’s screen-space center (`worldToScreen` of card center, including tapped / cluster layout). Fix offsets from wrong corner, zoom, or stacking. Outside click still dismisses.

### Selection

- Select a permanent that **has activatable abilities** (on the card / engine activates), even if none are currently legal.
- Permanents with **no** activates: not selectable.
- Tap-only lands (only tap-for-mana): **selectable** so the tap wedge can open.

### Radial options

List known activates for that permanent; **disable** illegal wedges (no commit). Tap-for-mana remains a wedge when applicable (enabled/disabled by tap / `can_act`).

### In-game card preview / inspect (still broken — investigate and fix)

Prior wave landed dock-mode wiring and AltLeft/AltRight detection, but **live Alt/Option inspect still fails**. Treat as an open investigation, using Foldkit DevTools MCP once tooling lands:

1. Confirm AltDown / pin / FetchInspectCard / InspectCardFetched / dock render in message history and model (`inspectPin`, `inspectCard`).
2. Fix the failing layer (keyboard capture, hit→pin, catalog fetch, dock z-order / pointer-events, or layout).
3. Success: hold Alt/Option over a face-up board, hand, or stack card → left dock + backdrop + oracle/effects; release or Esc clears. Topmost per board layer stack.

Regression tests must cover the live failure class, not only model-level AltDown.

### Top-left HUD controls (stacked — investigate and fix)

**Symptom:** Legend (`?`), sound, and related controls pile on the same top-left point (user reports concede in the stack too — verify layout; concede is authored `top-md right-md` and may be a separate bug or misread).

**Likely cause:** `discoverabilityView` uses its own `fixed top-md left-md` while `boardOverlays` also wraps legend + sound in a `fixed top-md left-md` flex row — nested `fixed` children leave the flex flow and stack at the viewport corner.

**Fix:** One top-left **toolbar** cluster: a single `fixed top-md left-md` flex row owns legend toggle, sound toggle, and any peer controls that belong there. Inner views are **not** independently `fixed` (in-flow flex items only). Concede stays **top-right** (Solid layout); confirm live that it is not in the top-left stack. Scene/layout tests assert distinct positions / non-overlapping toolbars.

### Arena playable chrome + zone outlines

Remove always-on seat/controller borders on every battlefield permanent. Drop the unplayable **dim veil** in favor of outline/border language.

| Surface | When | Treatment |
|---------|------|-----------|
| Hand | Castable / playable action | Playable **border** (Arena-style) |
| Battlefield permanent | Has activate other than tap-for-mana alone | Same playable **border** |
| Tap-only land | — | No playable border; still selectable for tap wedge |
| Commander | Always (identity) | Gold **outline** (`commander-gold`) — outer halo, distinct from playable border |
| Graveyard bar tile | Playable/castable action from GY | **Purple outline** (new token, e.g. `graveyard-outline`) |
| Exile bar tile | Playable/castable action from exile | **Green outline** (new token, e.g. `exile-outline`) |

Keep: spell/ability **target** highlights and **combat** arrows.

DESIGN.md: add `graveyard-outline` / `exile-outline`; document playable-border vs commander/zone outlines (update the old “dim non-usable” priority note where it conflicts).

## Delivery order

1. Tooling: MCP + Vite port + vendored skills + AGENTS note.  
2. Board investigations (MCP-assisted): **inspect live fix**, **top-left toolbar layout**.  
3. Board chrome: radial center → selection + disabled wedges → strip always-on borders → playable borders + commander/GY/exile outlines → remove dim-for-unplayable.  
4. Outcome Scene/unit tests + Interaction checklist (inspect, HUD, radial, borders).

## Testing

- Radial SVG center coincides with selected card screen center.
- Non-ability permanent: click does not set `selectedId`.
- Illegal activate: wedge present and disabled; pointer-up does not commit.
- Playable border on castable hand / activatable permanent; absent on tap-only land.
- No default seat stroke on resting battlefield cards.
- Commander gold outline can coexist with playable border.
- GY bar tile purple outline / exile green outline when those zones have playable actions.
- Alt/Option inspect: live + unit — dock opens with backdrop and oracle; dismiss on release/Esc.
- Top-left toolbar: legend and sound are siblings in one flex row (not stacked absolutes); concede at top-right.
- MCP: `foldkit_list_runtimes` sees a connected tab with `just dev` + open client (manual/agent check).

## Out of scope

- Relitigating engine activate legality beyond client disable of illegal wedges.
- Full Foldkit git subtree by default.
- Playwright CI matrix.

## Success criteria

- Agents have Foldkit DevTools MCP + vendored skills on PR #74.
- Live Alt/Option inspect works (dock mode).
- Top-left HUD controls are a single non-overlapping toolbar; concede remains top-right.
- Radial sits on the card; selection and chrome match Arena-style playable language with commander/GY/exile outline colors as specified.
- Always-on permanent borders and unplayable dim veil are gone.

## References

- https://foldkit.dev/ai/mcp  
- https://foldkit.dev/ai/skills  
- `client/app/board/html/activation-radial.ts`  
- `client/app/board/geometry/radial.ts`  
- `client/app/board/bitmap/paint-cards.ts`  
- `client/app/board/html/hand.ts`  
- `client/app/board/html/inspect.ts`  
- `client/app/board/html/overlays.ts`  
- `client/app/board/html/discoverability.ts`  
- `client/app/board/html/sound-chrome.ts`  
- `client/app/board/html/concede.ts`  
- `DESIGN.md`  
- `docs/client-canvas-map.md` (layer stack — chrome must stay on the correct layer)  
- Prior inspect design: `docs/superpowers/specs/2026-07-22-foldkit-remaining-bugs-and-board-layers-design.md`
