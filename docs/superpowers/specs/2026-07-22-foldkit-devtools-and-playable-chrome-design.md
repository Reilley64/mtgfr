# Foldkit DevTools tooling + Arena playable chrome

**Status:** Draft  
**Date:** 2026-07-22  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74) (`cursor/foldkit-migration-design-1ef0`)

## Goal

Land Foldkit agent tooling first (DevTools MCP + vendored skills) so board work can be debugged live, then fix activation radial centering, in-game Alt/Option inspect (still broken), top-left HUD control layout, pending-choice prompts shown to non-deciders, restore battlefield permanent chrome (badges / P/T / counters / planeswalker loyalty), and always-on permanent borders → Arena-style playable / zone outline language.

## Approach

**B — Tooling first, then board chrome** (chosen): MCP + skills commit before radial/selection/border/inspect/HUD/prompt-visibility/permanent-chrome work so implementers can use `foldkit_*` tools while fixing UI.

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

### Pending-choice prompts visible to everyone (investigate and fix)

**Symptom:** Interactive prompt modals (Yes/No, pay cost, card pick, etc.) appear on every seated player’s client whenever `pending_choice` is set, not only for the player who must answer.

**Likely cause (client):** `boardOverlays` gates `promptsView` with `isActivePlayer` (anyone still playing, not lost/spectating) and `promptsView` renders whenever `state.pending_choice != null`. Wire projection deliberately ships `pending_choice` to all viewers (private *items* redacted for non-awaited seats — scry/search/etc.), but the **interactive formulator** must only mount for the awaited seat.

**Fix:**

1. Render engine pending-choice formulators only when `pending_choice.player === state.viewer` (and the viewer is still an active player). Non-deciders and spectators: **no** interactive prompt DOM.
2. Client-local pre-submit prompts (`xPrompt`, modal modes, discard/sacrifice/gy-exile picks, staged targeting) stay local to the acting client’s `BoardModel` — confirm they never appear from shared state alone.
3. Do **not** hide `pending_choice` on the wire for non-deciders in this wave (projection already redacts private items; other clients may still need the fact for passive chrome later). No passive “waiting for seat N” banner required unless it already exists and is wrong.
4. Submissions from a non-awaited seat must not be offerable in the UI (server already rejects wrong-player answers; client must not present the buttons).

Regression: seated non-decider Scene has no `prompt-yes` / formulator test ids while a `may_yes_no` (or similar) is pending for another seat; the awaited seat still gets the full prompt.

### Top-left HUD controls (stacked — investigate and fix)

**Symptom:** Legend (`?`), sound, and related controls pile on the same top-left point (user reports concede in the stack too — verify layout; concede is authored `top-md right-md` and may be a separate bug or misread).

**Likely cause:** `discoverabilityView` uses its own `fixed top-md left-md` while `boardOverlays` also wraps legend + sound in a `fixed top-md left-md` flex row — nested `fixed` children leave the flex flow and stack at the viewport corner.

**Fix:** One top-left **toolbar** cluster: a single `fixed top-md left-md` flex row owns legend toggle, sound toggle, and any peer controls that belong there. Inner views are **not** independently `fixed` (in-flow flex items only). Concede stays **top-right** (Solid layout); confirm live that it is not in the top-left stack. Scene/layout tests assert distinct positions / non-overlapping toolbars.

### Battlefield permanent chrome (Solid parity — badges, P/T, counters, planeswalkers)

**Symptom:** Resting battlefield permanents show card art but not the Solid-era overlay chrome: summoning-sickness / keyword ability chips, effective power/toughness (including modified toughness), +1/+1 counter badge, marked damage, and planeswalker loyalty in the P/T slot.

**Likely cause:** Layout still builds `RenderCard` with `pt`, `keywords`, `summoningSick`, `counters`, `loyalty`-via-`pt`, etc., and `paintCard` / `paintFaceUp` still draw that chrome — but the live resting bitmap layer (`paintBitmapLayer` in `bitmap/mount.ts`) calls **`paintCardArt` only** (image + name fallback), which never invokes badge/P/T drawing. Scene/text paths that still assert `"2/2"` are not what the player sees on the bitmap board.

**Restore (product language — match Solid board):**

| Chrome | When |
|--------|------|
| Summoning-sick chip | Creature summoning sick and not haste |
| Keyword ability chips | Battlefield keywords (existing badge rail / overflow `+N`) |
| Goaded / prepared / owner-strip / commander marker | Existing `drawStatusBadges` rules |
| P/T badge | Creatures: effective `power`/`toughness` from `ObjectView` (modified stats) |
| Loyalty badge | Planeswalkers: live `loyalty`, else printed starting loyalty — same P/T slot |
| +1/+1 counters | `plus_counters > 0` |
| Marked damage | `marked_damage > 0` |

**Also investigate:**

1. **Counters:** Confirm +1/+1 badge reads live wire and paints on the resting layer. Audit whether Solid showed other counter kinds (time, vow, kind-keyed) that `ObjectView` does not yet expose — restore what the wire supports; flag gaps in this design’s Out of scope / fidelity note rather than inventing client-only counters.
2. **Planeswalkers:** Loyalty in the badge must track `loyalty_changed` / live field; painted starting loyalty only as fallback. Loyalty abilities remain radial/activate chrome (selection workstream), not a second badge invent.
3. **Do not** bring back resting under-card **name labels** (intentionally removed). Names stay in inspect / stack / piles.

**Fix shape:** Resting bitmap paint must run full permanent chrome after art (prefer wiring `paintCard` or splitting “art then chrome” so target highlight / auto-tap preview stay correct). Unit/bitmap tests assert badges and P/T/loyalty/counters on the **bitmap paint path**, not only vestigial `sceneShapes` text.

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
2. Board investigations (MCP-assisted): **inspect live fix**, **top-left toolbar layout**, **prompt visibility (awaited seat only)**.  
3. Board chrome: **restore permanent badges / P/T / counters / planeswalker loyalty** on bitmap layer → radial center → selection + disabled wedges → strip always-on borders → playable borders + commander/GY/exile outlines → remove dim-for-unplayable.  
4. Outcome Scene/unit/bitmap tests + Interaction checklist (inspect, HUD, prompts, permanent chrome, radial, borders).

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
- Pending choice for seat A: only seat A’s Scene mounts the interactive prompt; seat B and spectator do not.
- Bitmap resting layer: summoning-sick / keyword chips, creature P/T (modified), planeswalker loyalty, +1/+1 and marked-damage badges paint; no resting name labels.
- MCP: `foldkit_list_runtimes` sees a connected tab with `just dev` + open client (manual/agent check).

## Out of scope

- Relitigating engine activate legality beyond client disable of illegal wedges.
- Full Foldkit git subtree by default.
- Playwright CI matrix.
- Projecting new counter kinds onto `ObjectView` beyond what wire already carries (flag if Solid showed more than `plus_counters` + loyalty).
- Restoring under-card name labels.

## Success criteria

- Agents have Foldkit DevTools MCP + vendored skills on PR #74.
- Live Alt/Option inspect works (dock mode).
- Top-left HUD controls are a single non-overlapping toolbar; concede remains top-right.
- Interactive pending-choice prompts only for the awaited player (`pending_choice.player === viewer`).
- Battlefield permanent chrome (badges, effective P/T, counters, planeswalker loyalty) matches Solid board parity on the live bitmap layer.
- Radial sits on the card; selection and chrome match Arena-style playable language with commander/GY/exile outline colors as specified.
- Always-on permanent borders and unplayable dim veil are gone.

## References

- https://foldkit.dev/ai/mcp  
- https://foldkit.dev/ai/skills  
- `client/app/board/html/activation-radial.ts`  
- `client/app/board/geometry/radial.ts`  
- `client/app/board/bitmap/paint-cards.ts` (`paintCard` vs `paintCardArt`)  
- `client/app/board/bitmap/mount.ts` (resting layer currently art-only)  
- `client/app/board/geometry/layout.ts` (`pt()`, `toCard`)  
- `client/app/board/html/hand.ts`  
- `client/app/board/html/inspect.ts`  
- `client/app/board/html/overlays.ts`  
- `client/app/board/html/discoverability.ts`  
- `client/app/board/html/sound-chrome.ts`  
- `client/app/board/html/concede.ts`  
- `client/app/board/html/prompts.ts`  
- `client/lib/spectator.ts` (`isActivePlayer` — not sufficient alone for prompt gating)  
- Wire visibility: `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md` (pending_choice broadcast + private-item redaction)  
- `DESIGN.md`  
- `docs/client-canvas-map.md` (layer stack — chrome must stay on the correct layer)  
- Prior inspect design: `docs/superpowers/specs/2026-07-22-foldkit-remaining-bugs-and-board-layers-design.md`
