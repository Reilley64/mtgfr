# Foldkit DevTools tooling + Arena playable chrome

**Status:** Draft  
**Date:** 2026-07-22  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74) (`cursor/foldkit-migration-design-1ef0`)

## Goal

Land Foldkit agent tooling first (DevTools MCP + vendored skills) so board work can be debugged live, then fix activation radial centering and replace always-on permanent borders with Arena-style playable / zone outline language.

## Approach

**B — Tooling first, then board chrome** (chosen): MCP + skills commit before radial/selection/border work so implementers can use `foldkit_*` tools while fixing UI.

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
2. Board: radial center → selection + disabled wedges → strip always-on borders → playable borders + commander/GY/exile outlines → remove hand/battlefield dim-for-unplayable.  
3. Use MCP while implementing board tasks.  
4. Outcome Scene/unit tests + Interaction checklist for radial/select/borders.

## Testing

- Radial SVG center coincides with selected card screen center.
- Non-ability permanent: click does not set `selectedId`.
- Illegal activate: wedge present and disabled; pointer-up does not commit.
- Playable border on castable hand / activatable permanent; absent on tap-only land.
- No default seat stroke on resting battlefield cards.
- Commander gold outline can coexist with playable border.
- GY bar tile purple outline / exile green outline when those zones have playable actions.
- MCP: `foldkit_list_runtimes` sees a connected tab with `just dev` + open client (manual/agent check).

## Out of scope

- Relitigating engine activate legality beyond client disable of illegal wedges.
- Full Foldkit git subtree by default.
- Playwright CI matrix.

## Success criteria

- Agents have Foldkit DevTools MCP + vendored skills on PR #74.
- Radial sits on the card; selection and chrome match Arena-style playable language with commander/GY/exile outline colors as specified.
- Always-on permanent borders and unplayable dim veil are gone.

## References

- https://foldkit.dev/ai/mcp  
- https://foldkit.dev/ai/skills  
- `client/app/board/html/activation-radial.ts`  
- `client/app/board/geometry/radial.ts`  
- `client/app/board/bitmap/paint-cards.ts`  
- `client/app/board/html/hand.ts`  
- `DESIGN.md`  
- `docs/client-canvas-map.md` (layer stack — chrome must stay on the correct layer)
