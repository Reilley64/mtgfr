# Foldkit remaining bugs + board layer lock

**Status:** Done  
**Date:** 2026-07-22  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74) (`cursor/foldkit-migration-design-1ef0`)

## Goal

Close the remaining Foldkit **product** bugs on PR #74 — not more process-only work. Live-triage claimed fixes, finish Lobby Bring UX, restore Solid-parity board inspect via a shared preview **dock** mode, tune hand drag-to-play, remove under-card name labels, fix declare-attackers arrow stacking, and **lock a single board layer stack** in spec so paint/DOM order stops drifting.

## Approach

**Live triage, then known UX, then board** (chosen). Execution order is listed under Delivery order below: live triage (Alt/Option first) → dock preview → drag threshold + under-card names → layer-stack lock with arrow/flight fixes → layout collisions → Lobby Bring + Back.

## Scope

| # | Item | Notes |
|---|------|--------|
| 1 | Alt/Option card preview | Known broken; hold pins, release clears; prefer hand/stack aux over battlefield hit |
| 2 | Live triage of other claimed fixes | Host/`WEB_DATABASE_URL`, hand drag-hide, builder hover, session gate — code only if still broken |
| 3 | Hand drag-to-play sensitivity | Lower commit threshold (too far today) |
| 4 | Lobby Bring + **Back** | Pre-pick → locked Bring text, no `<select>`; **Back** → Your decks (`/`) |
| 5 | Board layout | Mats, overlapping zone text, command-zone collisions, HUD garble |
| 6 | No under-card name labels | Remove names under/on resting battlefield cards |
| 7 | Declare-attackers arrows under cards | Declaring aim must use same above-cards arrow layer as committed arrows |
| 8 | Board layer stack lockdown | Enumerate layers; commit to canvas map + cross-link board feature spec |
| 9 | Shared card-preview **dock** mode | Board inspect = dock mode of builder hover preview; left art + backdrop; oracle/effects to the right |

### Out of scope

- Playwright / cold-env CI matrix
- ~47 prompt-stub / CardArt ImageCache debt
- Logout 408 unless it blocks triage

## Lobby Bring

When `selectedDeckId` is set (Play → Host/Join with a pick, or `?deck=`):

- Show locked **Bring: `<deck name>`** (bold name). No `<select>`.
- Show **Back** → navigate to Your decks (`HomeRoute` `/`). Leaves `/play`; does not invent a change-deck picker.
- Host / Join / table-code controls unchanged.

When `selectedDeckId == null` (bare `/play`): keep today’s deck `<select>` + Host / Join.

Claim-seat with a pre-picked deck: same locked Bring copy + **Back** → `/`.

**Tests:** ≥2 decks, pre-pick non-first → Bring text shows that name; `#lobby-deck` absent; Back navigates to `/`.

## Card preview modes (builder + board inspect)

Unify on `client/lib/deck-builder/card-hover-preview.ts` (today’s cursor-follow builder/list hover). Add a second mode; board inspect uses it instead of a divergent one-off.

### Mode `follow` (unchanged)

Deck builder / deck list: large face + optional oracle panel, cursor-follow, no scrim.

### Mode `dock` (board Alt/Option — Solid parity)

- Full-board dim **backdrop**.
- Content **docked left**: card art on the left; **oracle + approximates** to the right; for battlefield permanents, **modifier ledger / effects** in that right column.
- DFC flip; dismiss via Alt/Option up, Esc, Close, backdrop as today.
- Prefer hand/stack aux hover over battlefield hit when pinning.

**Inspect is topmost** in the board layer stack — above prompts, HUD, and system modals (concede / result / portrait gate). Reading a card to decide an action must not sit under action chrome.

**Wiring:** shared view API `mode: "follow" | "dock"` (+ pin/card/face/modifiers for dock). Board `inspectView` becomes a thin wrapper. Deduplicate oracle/text-panel markup into the shared module.

**Live + tests:** Alt/Option on battlefield shows left dock + backdrop + right-side oracle/effects. Scene/unit covers dock layout + backdrop; `follow` still cursor-positions.

## Hand drag-to-play

Play commits when the pointer crosses a threshold above the hand bar. **Lower** how far above the bar is required (closer to Solid). Cancel below threshold restores the tile. Keep hide-on-commit + flight seeding when still correct after triage. Spell/payment mana tray stays on the **hand layer** (above hand cards within that layer).

## Under-card names + board layout

- Remove name labels drawn under/on resting battlefield cards (overlapping land-name paint). Names belong in inspect, stack/pile, and aux hover — not permanent under-card captions.
- Fix packing/mat issues: seat mats proportions, zone text collisions (including command-zone label vs art), HUD garble. Layout/camera/density only — no speculative chrome redesign. Changes must respect the locked layer stack.

## Board layer stack (authoritative)

Commit this stack into **`docs/client-canvas-map.md`** (living map) and cross-link from `docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md`. New board visuals must declare which layer they join; no ad-hoc `z-*` without updating the map.

**Bottom → top:**

| # | Layer | Surface | Contents |
|---|--------|---------|----------|
| 1 | Felt / seats | Canvas vector | Table, seat bands |
| 2 | Zone furniture | Canvas / world DOM | Avatar **paint**, library, command zone, **battlefield in-play mana** (left under your seat), GY, exile |
| 3 | Resting battlefield permanents | Mount bitmap (+ card chrome) | Battlefield faces |
| 4 | Arrows | Canvas | Committed attack/block, **declare-attackers drag aim**, spell aim — always above resting permanents |
| 5 | Hand / stack / spell mana | HTML | Resting hand & stack; **spell/payment mana tray** (same layer as hand, above hand cards) |
| 6 | Flights | Mount / motion | In-flight play cards — **above** hand and stack |
| 7 | Combat / life hit targets | HTML | Interactive orbs when needed (paint stays in layer 2; hits here) |
| 8 | Prompts / choice UI | HTML | `pending_choice` and related |
| 9 | Turn HUD | HTML | Phase track, Next / End Turn, discoverability |
| 10 | Inspect dock | HTML | Mode `dock` + backdrop — **topmost** |

### Layer rules

1. **Avatar paint** stays in layer 2 with **clear bands** packing must not cover; **orb hits** stay in layer 7.
2. **Two mana surfaces:** battlefield in-play mana (layer 2) vs spell/payment mana tray on the hand layer (5).
3. No resting permanent paint or DOM card face may sit above layer 4 while combat/spell arrows are active. Declare-drag arrows use the **same arrow layer** as committed arrows.
4. Flights paint above hand/stack (layer 6 over 5).
5. Prompts (8) above combat/life hits (7).
6. Inspect (10) above everything else on the board, including system modals, while pinned.
7. Under-card name labels are forbidden on resting permanents (not a separate layer — deleted).

## Delivery order

1. Live Interaction checklist: Alt/Option inspect first; then Host DB, hand hide, builder hover, session gate — code only failures + outcome tests.
2. Shared card-preview **dock** mode + wire board inspect through it.
3. Hand drag threshold + under-card name removal.
4. Layer-stack spec lock **with** declare-arrow / flight paint-order fix.
5. Remaining board layout collisions.
6. Lobby Bring + **Back**.

## Testing

- Outcome Scene/unit: dock inspect (backdrop, left art, right oracle/mods); drag threshold; Bring text + Back → `/` and no `#lobby-deck` when pre-picked; no under-card name paint; arrow/flight layer invariants (paint order or equivalent).
- Live `verify` Interaction checklist before claiming done (Interaction / UI flagged).

## Success criteria

- Alt/Option inspect docks left with backdrop and right-side oracle/effects; works live.
- Pre-picked lobby shows Bring + Back only (no misleading select).
- Drag-to-play commits without an oversized lift above the bar.
- Declare-attackers aim arrows sit above resting cards; flights clear the hand bar.
- `docs/client-canvas-map.md` lists the locked layer stack; board feature spec points at it.
- Claimed fixes either verified live green or fixed with outcome tests — no silent “already fixed” claims.

## References

- [`docs/client-canvas-map.md`](../../client-canvas-map.md) — code map; becomes layer SoT
- [`docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md`](2026-07-20-client-game-board-and-interaction.md) — board behavior
- [`docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md`](2026-07-22-client-interaction-test-policy-design.md) — outcome tests + verify checklist
- `client/lib/deck-builder/card-hover-preview.ts` — shared preview to extend
- `client/app/board/html/inspect.ts` — current board inspect wrapper
- `client/app/shell/lobby/view.ts` — entry / claim-seat Bring paths
