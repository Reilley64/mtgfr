# Design System: edh.reilley.dev

**North star:** "Arena, Unplugged" — MTGA game-client polish, zero storefront chrome (see `PRODUCT.md` anti-references). Dark green felt (`forest-floor`), translucent panels, cream cards (`oracle-ivory`) on felt. **Gold = a decision is owed** (priority gold on the orb; commander gold on commanders only). **Playable border** means a card has a current action. **Zone outlines** use graveyard purple and exile green for playable bar tiles. **Seat hues = identity** (forest/island/mountain/arcane), never semantics.

**Layout:** Canvas board (96×134 world-unit cards, shared camera) + thin DOM HUD. Z-order: board → HUD → backdrop (29) → modal (30) → pinned board inspect dock (topmost, above system modals). No persistent nav chrome.

**The Landscape Rule.** Mobile and tablet are **landscape-first**. Auth, lobby, decks, builder, and board assume horizontal space. Portrait phones (`orientation: portrait` and `max-width: 900px`) open a native `<dialog showModal>` rotate gate (top-layer, inert background, focus trap; Escape disabled) — never a stacked vertical reflow of the builder or board. Short landscape (phone on its side) keeps side-by-side columns and tightens padding; it does not flip the axis. Safe-area insets apply on notched devices (`viewport-fit=cover`).

**Typography:** `system-ui` only. Screen ramp: title 18/700, body 14/400, button 14/600, label 13, caption 12. Game chrome: `game` 15/600 (`Button variant="game"`), `display` 22/700 (lobby table code). HUD density below caption: `chip` 11, `micro` 10 — board/hand chrome only, never prose. No display fonts.

**Semantic colors (combat):** Mountain Red = attack, Wall Green = block, Island Blue = targeting.

**Priority readability:** Gold = a decision is owed. During **instant-priority focus**, playable hand cards and battlefield permanents with real action-list activates get a playable border; tap-only mana lands remain selectable without that border. The former dim-for-unplayable veil is retired. Empty-stack main and declare attackers/blockers stay fully lit (see turn-priority-and-stack spec / `CONTEXT.md`). Yielded pass uses amber earth (`yielded` / `yielded-ink`) — not priority gold (The Gold Means Act Rule).

**Components:** Chunky buttons (Llanowar Deep → Llanowar on hover, inset highlight, shortens on press) via Foldkit `client/lib/ui/` helpers (`buttonClass`, surfaces) — never `@apply`. Panels: Forest Surface 98% + 1px Vine border; fluid width (`max-w` capped, no hard `min-w` that overflows landscape phones). HUD: Forest HUD 92%. Inputs: Glass fill + Vine border. Cards on canvas: tapped = 90° rotation; combat outlines red/green; life orbs are combat drop targets. Quiet HUD dismiss controls use `.hit-quiet` so coarse pointers still hit ≥44×44.

**Motion:** 150–250ms ease-out, state-only, `prefers-reduced-motion` fallback — never celebration.

**Tokens:** Values live in [`design.tokens.json`](design.tokens.json) (DTCG). Codegen (`bun run gen` via Style Dictionary) writes `client/styles/tokens.generated.css` (Tailwind `@theme`) and `client/lib/design-tokens.generated.ts` (canvas). Component recipes live in TypeScript (`client/lib/ui`), not in the token file. Canvas named colors import the generated TS module; unnamed paint one-offs may stay literal.
