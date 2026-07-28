# Lobby entry redesign (design)

**Status:** Design input (2026-07-27). Shipped in Layout C entry reflow; living behavior in [lobby-entry-ui](2026-07-20-lobby-entry-ui.md).
**Surfaces:** `lobby-entry-ui` (Host/Join entry); shared shell ghost button recipe (`client/app/domain/ui/buttonClass.ts`) used by lobby Back and other shell secondary/back nav. Living update target at implement time: [lobby-entry-ui](2026-07-20-lobby-entry-ui.md). Seated lobby layout is out of scope.

---

## Problem Statement

The Host/Join lobby entry (`/play/:deckId`) still reads as a centered admin panel after shell polish: empty felt around one enclosing panel, a weak dashed Join `#` card that needs a mode switch, and a deck card that does not carry enough visual weight. Players should feel they are about to sit at a Commander table with a clear Host path and an obvious Join path — without redesigning the seated table chrome.

## Goal

Reflow lobby **entry only** into a deck-anchored stage composition (Layout C): larger deck-card chrome as left visual anchor, Host as primary Llanowar CTA, soft-inline Join (code field + Join table) on the same surface, and a stronger shared ghost recipe for Back / secondary shell nav. Same routes and Host/Join actions; preserve the home→play deck-card FLIP morph.

## Locked decisions

| Decision | Choice |
|---|---|
| Scope | Host/Join entry (`surface: "entry"`) only; seated lobby layout unchanged |
| Composition | Drop the full-stage enclosing panel; deck + action stack on the `shellFrame` stage (Layout C) |
| Deck hero | Keep existing deck-card chrome; scale/place it as the left anchor (preserve FLIP morph) |
| Host | Primary Llanowar “Host a table”; same `RequestedLobbyHost` path |
| Join | Soft-inline — always-visible code field + “Join table”; Join is not a second solid Llanowar primary (ghost / non-filled); no choose→join mode switch / dashed `#` card |
| `entryMode` | Remove `entryMode` and choose→join messages (`RequestedLobbyOpenJoin`, `RequestedLobbyCancelJoin`, join-cancel UI) from the lobby submodel; single entry surface when a deck is selected |
| Back | Ghost control, strengthened recipe so it stays readable on felt; still secondary to Host |
| Ghost shell-wide | Strengthen `buttonClass("ghost")` once; all shell Back / secondary nav that already use ghost pick it up (lobby Back, builder ghosts, coverage/leaderboard “Play” leading links, etc.) |
| Atmosphere | Existing shell felt/vignette; no full-bleed commander backdrop; no reward glow |
| Motion | Keep short stage enter + deck FLIP; honor `prefers-reduced-motion`; no new idle loops |
| Routes / actions | Unchanged |

## Approaches considered

1. **Panel rebalance** — Keep one enclosing panel; re-grid Host/Join/deck inside. Least churn; leaves the “box on felt” composition problem half-solved.
2. **Deck-anchored stage + soft-inline Join (chosen)** — Stage is the composition: deck left, Host then Join in one action column; no mode switch. Best fix for empty felt, Host primacy, and Join clarity while keeping the deck morph.
3. **Full-bleed commander backdrop** — Strongest atmosphere; fights the deck-card morph and risks marketing-hero chrome the shell polish avoided.

Layout variants under the chosen approach: **A** split stage, **B** centered hero, **C** deck + action stack — **C chosen**.

## Design

### Stage composition (Layout C)

- `shellFrame` with `atmosphere: "shell"`, title “Lobby”, trailing account chrome — unchanged frame geometry.
- **Entry** stage content is **not** wrapped in a single max-width `panelClass` that owns the whole story. Deck card and action column sit directly on the stage with shared gutters / max width so short landscape stays usable. **Seated** (`surface: "table"`) may keep today’s panel wrapper until a later redesign.
- **Left:** selected deck-card chrome (`lobby-deck-card` / `lobby-deck-card-{id}`), larger than today’s ~280px max so it reads as the hero.
- **Right (action stack):**
  1. Display beat: “Ready to play?” + short Host supporting line
  2. Primary **Host a table** (`lobby-host`) — only solid Llanowar CTA on the surface
  3. Quiet “Have a code?” + `lobby-join-code` field + **Join table** (`lobby-join`) using the shared ghost (non-filled) recipe
  4. Strengthened ghost **Back** (`lobby-back`) to Your decks, below the Join row
- Empty / no-deck states keep amber `lobby-empty` copy pointing home; no Host/Join chrome.
- Errors stay burn-red `lobby-error` under the action column.

### Join interaction

- Code field and Join are always visible when a deck is selected — no `entryMode === "join"` panel, no `lobby-open-join`, no Cancel-back-to-choose step.
- Join still uses `RequestedLobbyJoin` / existing `parseTableCode` behavior (bare codes and pasted play URLs).
- Submitting / disabled states unchanged functionally.

### Shared ghost Back recipe

- Strengthen the shared `ghost` variant in `buttonClass` (clearer border and/or label contrast on felt — e.g. brighter label than `text-mist`, keep `border-vine`, transparent fill). Exact token classes chosen at implement time against DESIGN.md shell type ramp; no new button variant unless ghost would break a non-back use that must stay quieter (prefer one stronger ghost for all current ghost call sites).
- Lobby entry Back uses that recipe. Other shell surfaces that already call `buttonClass("ghost")` for back/secondary nav inherit the same emphasis without per-surface one-offs.
- Ghost remains secondary: never solid Llanowar; Host/primary CTAs stay primary.

### Motion & atmosphere

- No new atmosphere variant; no commander art as full-bleed stage backdrop.
- Existing shell enter motion and home→`/play/:deckId` deck-card FLIP remain; honor `prefers-reduced-motion`. No additional entry-only motion beyond that.

### Spec updates at implement time

- Update [lobby-entry-ui](2026-07-20-lobby-entry-ui.md) Behavior for entry composition, soft-inline Join, removal of choose/join mode, and Back ghost emphasis.
- If DESIGN.md or shell-routes docs describe the ghost recipe explicitly, align the one-line description there; otherwise the living lobby spec + `buttonClass` tests are enough.

## Testing Decisions

- Update `client/app/shell/lobby/entry.test.ts` (and related lobby Scene coverage) for Layout C: deck + Host + join code + Join visible together; assert absence of `lobby-open-join` / join-only panel testids on the happy path.
- Keep `lobby-back`, `lobby-host`, `lobby-join`, `lobby-join-code`, `lobby-deck-card` testids (or document renames in the same change).
- Extend `buttonClass` unit tests for the strengthened ghost classes.
- Spot-check existing shell Scene coverage for surfaces that render ghost Back/Play leading links; no new shell destinations.

## Out of Scope

- Seated lobby / claim-seat layout redesign.
- Server/BFF lobby tables, seed, affinity, drain ([lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md)).
- Auth wordmark, deck list grid, builder pane job, board HUD.
- New navigation IA or routes.
- Full-bleed commander hero / Approach 3 atmosphere.

## Further Notes

- Brainstorm mockups (Layout A/B/C) live under `.superpowers/brainstorm/` (gitignored); Layout C was selected in session.
- Companion to shipped [shell-polish-redesign](2026-07-26-shell-polish-redesign-design.md): this tightens lobby entry composition beyond Wave 2’s panel-based Host/Join choose flow.
