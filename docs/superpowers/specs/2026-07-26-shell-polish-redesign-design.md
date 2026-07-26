# Shell polish redesign (design)

**Status:** Approved design input (2026-07-26).
**Surfaces:** `shell-routes-and-auth`, `deck-list-and-builder`, `lobby-entry-ui`, `coverage-by-set`, `system-overlays`, `board-composition` (Landscape Rule), `DESIGN.md`, `design.tokens.json`, shared shell UI helpers (`client/app/shell/**`, `client/app/domain/ui/**`).

---

## Problem Statement

Non-board shell screens (auth, deck list, builder, lobby, leaderboard, coverage) work, but each reinvents felt, headers, and chrome. The result reads closer to quiet admin panels than to walking into a Commander table. Portrait phones are blocked by a rotate dialog instead of remaining playable. Shell typography is locked to `system-ui`, which limits hierarchy and brand presence on arrival surfaces.

## Goal

Polish and reflow every non-board shell surface under one shared shell frame and a stepped-up table atmosphere — geometric shell type pair, richer felt/depth/motion, consistent header and meta chrome — without adding persistent nav, changing routes/actions, or redesigning the live board HUD.

## Locked decisions

| Decision | Choice |
|---|---|
| Scope depth | Visual + shared chrome rethink; routes and actions stay |
| Navigation | No persistent nav; contextual header links + account menu only |
| Atmosphere | Step up felt/texture/brand/motion; not F2P reward glow |
| Surfaces | All non-board shell: auth, deck list, builder, lobby entry + seated, leaderboard, coverage, shared chrome |
| Layout freedom | Reflow allowed inside surfaces; same routes and actions |
| Typography | Shell: geometric UI sans + sharper display sans (self-hosted). Board: keep denser/`system-ui` chrome |
| Type character | Game-client clean (MTGA-adjacent), not fantasy serif |
| Portrait | CSS viewport rotate — landscape layouts run sideways; remove rotate dialog; no `orientation.lock` |
| Delivery | Design-system-first foundation PR, then surface waves |
| Board | Out of scope for shell frame and type change (except portrait rotate wrapper must not break board mount) |

## Approaches considered

1. **Design-system-first, then surface waves (chosen)** — Define tokens, type, `shellFrame`, and portrait rotate once; reflow each surface to that grammar in waves. Consistency by construction; matches one-spec-per-surface.
2. **Flagship screen, then propagate** — Redesign home fully, copy outward. Faster proof; higher thrash when builder/lobby force redesign.
3. **Chrome/type pass, then reflow pass** — Two full tours of every surface. Safer first ship; feels half-done against a full polish goal.

## Design

### North star

Shell screens feel like entering a Commander table: polished game-client energy, shared frame, atmospheric felt. The live board remains the hero and stays chrome-light. Gold still means a decision is owed in-game — shell does not reuse priority gold as brand decoration. Primary shell CTAs stay Llanowar.

### Shared shell frame

Every non-board route renders through one Foldkit composition helper (working name `shellFrame`) so surfaces stop redeclaring full-bleed felt + ad-hoc headers + badge placement.

**Layers (top → bottom)**

1. **Atmosphere** — Full-bleed `forest-floor` plus subtle felt texture and soft vignette (CSS only; no inset hero image cards). Variants: `auth` (strongest) and `shell` (quieter).
2. **Header row** — Fixed three-slot geometry:
   - **Leading:** optional back/secondary (e.g. Play → `/` on leaderboard/coverage)
   - **Title:** display-sans title; optional short subtitle
   - **Trailing:** primary CTA when the surface has one + shared `accountChrome` when signed in
3. **Stage** — Surface-owned content. Consistent horizontal gutters and max content width so landscape phones neither sprawl nor crush.
4. **Meta dock** — Existing bottom-left two-line `% faithful` / `API {version}` badge (`appVersionBadge`), restyled to the frame, same behavior (coverage link when counts exist). Not shown on the board.

**Behavioral rules**

- Auth omits account chrome; brand lives in the main stage (display wordmark).
- Account menu remains the shared avatar menu in the trailing slot on signed-in shell surfaces. After an audit, any signed-in shell surface that currently omits it gains it for consistency (auth excepted).
- Cross-links stay contextual (header secondary, account menu). No new nav IA and no new destinations required by this design.
- Route enter: short stage transition (~150–250ms ease-out). Honor `prefers-reduced-motion` (skip decorative enter motion).
- Leaf primitives stay `accountChrome`, `appVersionBadge`, `buttonClass`, `panelClass`, `fieldClass`, list-row helpers. The frame only composes them.

### Typography, tokens, atmosphere

**Typography**

- Self-host two open-licensed woff2 families for shell routes:
  - **UI sans** — geometric; body, labels, buttons, fields, tables
  - **Display sans** — sharper/tighter; brand wordmark, screen titles, lobby table-code emphasis
- Concrete family names are chosen at implementation-plan time against the “game-client clean” character constraint; this design does not freeze vendor names.
- Update `DESIGN.md`: replace global “system-ui only” with an explicit shell vs board type policy.
- Board HUD/canvas chrome remains on the current denser/`system-ui` path unless a later board pass says otherwise.

**Tokens / surfaces**

- Keep the forest palette; extend `design.tokens.json` rather than invent a second theme.
- New or extended token concerns: felt atmosphere strengths (`auth` vs `shell`), shell header height, stage max-width, stage gutter, named shell-enter motion recipe within the existing 150–250ms ease-out range.
- Component recipes remain TypeScript helpers in `client/app/domain/ui/` (not token component maps; never `@apply`).

**Atmosphere rules**

- Texture, vignette, and panel depth only — no purple glow, reward badges, floating promo chips, or storefront chrome.
- Auth: strongest felt + display wordmark as the brand hero above the form.
- Other shell routes: quieter atmosphere; hierarchy from type and spacing.
- Anti-references in `PRODUCT.md` still apply (no F2P chrome, no generic SaaS dashboard, no hobby-tool jank).

### Portrait: CSS landscape render

Replace the native `<dialog>` rotate gate with an app-root **CSS viewport rotate** when `(orientation: portrait) and (max-width: 900px)` (same breakpoint family as today’s gate):

- Rotate and width/height-swap so existing landscape compositions run sideways.
- No `screen.orientation.lock`.
- This is layout, not decoration: `prefers-reduced-motion` does not disable the rotate.
- Safe-area insets applied as best-effort in the rotated coordinate space.
- Portrait gate Mount/`showModal`/Escape-swallow path is removed.
- Board mounts inside the same wrapper so play remains possible without a blocking dialog.

### Per-surface reflows

Same routes and actions; composition may change. Each surface mounts inside `shellFrame`.

| Surface | Reflow intent |
|---|---|
| **Auth (`/login`)** | Full-bleed atmosphere; display wordmark as hero brand; title + form secondary; centered stage (panel allowed, brand must still read without nav chrome). Mode toggle and errors stay in-form. |
| **Deck list (`/`)** | Header: title beat + primary New deck + account chrome. Stage: searchable deck grid with stronger commander-art hierarchy; empty state points to create/play. Context menus and delete confirm stay. |
| **Deck builder (`/decks/…`)** | Keep split-pane job; header owns name/save/back; panes get frame gutters and type ramp; legality/errors use shared alert recipe. Short-landscape tightening remains; no dialog dependency. |
| **Lobby entry (`/play/:deckId`)** | Selected deck card as visual anchor; Host primary; Join secondary with code field — “about to sit,” not two equal admin buttons. |
| **Lobby seated** | Table code in display emphasis; four-seat strip with ready state; Ready/Start as owed primary actions (Llanowar, not priority gold). Copy-link and errors stay, quieter. |
| **Leaderboard** | Frame header with Play-back + account; clearer rank/username/rating hierarchy; load-more/error behavior unchanged functionally. |
| **Coverage** | Frame header with Play-back + global `% faithful` context; clearer table/numeric hierarchy; search and try-again behavior preserved. |

### Delivery waves

| Wave | Ships |
|---|---|
| **1 — Foundation** | Fonts + theme wiring; `shellFrame` + atmosphere variants; CSS portrait→landscape rotate (remove dialog); `DESIGN.md` + living updates for `shell-routes-and-auth`, `system-overlays`, and `board-composition` Landscape Rule; Scene tests for frame + no dialog |
| **2 — Arrival** | Auth, deck list, lobby entry + seated reflows + living specs (`shell-routes-and-auth` / `deck-list-and-builder` / `lobby-entry-ui` as touched) |
| **3 — Tools** | Builder, leaderboard, coverage reflows + living specs (`deck-list-and-builder`, `shell-routes-and-auth`, `coverage-by-set`) |
| **4 — Pass** | Motion/enter polish, empty states, visual consistency sweep, deslop |

This document is design input only. Each implementation wave **must** update the corresponding living module spec(s) in the same change so Behavior / Implementation / Testing describe what ships. Index this file under Process/policy in `docs/superpowers/specs/README.md`.

### Testing

- Extend `client/app/shell/surfaces.test.ts` and focused surface tests for: shared header slots, meta badge behavior, account menu on signed-in shell surfaces, portrait media showing shell content **without** the rotate dialog.
- Keep outcome assertions (search filters, delete confirm, host/join, ready/start, load-more, coverage filter) — assert product behavior, not markup parity with the pre-polish UI.
- Run the Interaction checklist in `.agents/skills/verify/SKILL.md` before claiming UI waves done.
- Portrait rotate wrapper must not break board mount; no broader board HUD redesign tests required by this program.

### Error / degradation

| Condition | Behavior |
|---|---|
| Font files fail to load | Fall back to `system-ui` / existing sans stack; layout remains usable |
| Coverage meta incomplete | Meta dock shows version line only (unchanged) |
| Portrait rotate unsupported edge case | Still no blocking dialog; prefer readable landscape composition via the rotate wrapper. If a browser cannot transform, content may be cramped — acceptable vs hard gate |
| Route transition + reduced motion | Skip enter animation; final layout identical |

## Out of Scope

- Persistent shell nav or new routes/IA
- New RPC/product features unrelated to presentation
- Board canvas/HUD visual redesign (beyond living under the portrait rotate wrapper)
- `screen.orientation.lock` / fullscreen orientation forcing
- Offline PWA caching, marketing SEO, storefront/F2P chrome
- Freezing specific commercial font SKUs in this design doc

## Further Notes

- Concrete font pair selection belongs in the Wave 1 implementation plan (license, glyph coverage, file size, tracking with display titles).
- `PRODUCT.md` “auth, lobby, and builder are quiet surfaces” is relaxed for shell atmosphere and auth brand beat; the board remains the hero and shell still must not invent reward chrome.
- Landscape-first composition rules remain; portrait no longer blocks with a dialog — it renders those landscape compositions rotated.
