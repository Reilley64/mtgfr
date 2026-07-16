---
name: mtgfr
description: A browser Commander table — MTGA polish with the storefront unplugged
colors:
  forest-floor: "#0B1310"
  forest-surface: "#101816FA"
  forest-hud: "#0C1412EB"
  llanowar: "#2F7D46"
  llanowar-deep: "#276B3C"
  vine: "#22CC44"
  vine-dim: "#11CC33"
  snow: "#EEFFFF"
  seafoam: "#DDFFEE"
  mist: "#CCDDEE"
  lichen: "#99CCBB"
  fog: "#8899AA"
  snow-mint: "#EAFFF0"
  glass: "#FFFFFF0F"
  glass-dim: "#FFFFFF0D"
  priority-gold: "#FFD76A"
  commander-gold: "#E9B84A"
  oracle-ivory: "#E8E4D8"
  morph-slate: "#2A3742"
  mountain-red: "#FF5555"
  wall-green: "#66FF99"
  island-blue: "#77CCFF"
  burn-red: "#FF8888"
  caution-amber: "#FFEE88"
  damage-crimson: "#8F2F2F"
  tapped-out: "#24312B"
  tapped-ink: "#66786E"
  seat-forest: "#5AC88C"
  seat-island: "#5A96F0"
  seat-mountain: "#F0785A"
  seat-arcane: "#C88CF0"
  turn-mint: "#8FDCAA"
  turn-ember: "#E0A878"
  phase-mint: "#55CC99"
  phase-ember: "#CC8855"
  note-gold: "#F0C674"
  auto-moss: "#3A7D52"
  reconnect-rust: "#7A3B13"
  assign-clover: "#99FF99"
  watch-sage: "#9FB4A8"
  watch-flare: "#F87171"
  ready-sprout: "#8FE3A8"
  phase-fern: "#8FA398"
  preview-ash: "#E8E8EA"
  yielded: "#6B5A1F"
  yielded-hover: "#7D6A26"
  yielded-ink: "#FFE9A8"
  quiet-hover: "#2C3F35"
typography:
  title:
    fontFamily: "system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 700
  body:
    fontFamily: "system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  button-label:
    fontFamily: "system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 600
  label:
    fontFamily: "system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
  caption:
    fontFamily: "system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
  game:
    fontFamily: "system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 600
  display:
    fontFamily: "system-ui, sans-serif"
    fontSize: "22px"
    fontWeight: 700
  chip:
    fontFamily: "system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 600
  micro:
    fontFamily: "system-ui, sans-serif"
    fontSize: "10px"
    fontWeight: 600
rounded:
  panel: "12px"
  modal: "10px"
  game: "10px"
  hud: "8px"
  control: "6px"
  focus: "4px"
spacing:
  xs: "6px"
  sm: "8px"
  md: "10px"
  lg: "14px"
  xl: "16px"
  xxl: "24px"
components:
  button-game:
    backgroundColor: "{colors.llanowar-deep}"
    textColor: "{colors.snow-mint}"
    typography: "{typography.game}"
    rounded: "{rounded.game}"
    padding: "11px 26px"
  button-game-hover:
    backgroundColor: "{colors.llanowar}"
  button-game-disabled:
    backgroundColor: "{colors.tapped-out}"
    textColor: "{colors.tapped-ink}"
  button-game-yielded:
    backgroundColor: "{colors.yielded}"
    textColor: "{colors.yielded-ink}"
  button-primary:
    backgroundColor: "{colors.llanowar}"
    textColor: "{colors.snow-mint}"
    typography: "{typography.button-label}"
    rounded: "{rounded.control}"
    padding: "8px 14px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.mist}"
    rounded: "{rounded.control}"
    padding: "8px 14px"
  input:
    backgroundColor: "{colors.glass}"
    textColor: "{colors.snow}"
    rounded: "{rounded.control}"
    padding: "8px 10px"
  panel:
    backgroundColor: "{colors.forest-surface}"
    textColor: "{colors.snow}"
    rounded: "{rounded.panel}"
    padding: "24px"
  modal:
    backgroundColor: "{colors.forest-surface}"
    textColor: "{colors.snow}"
    rounded: "{rounded.modal}"
    padding: "16px"
  hud-panel:
    backgroundColor: "{colors.forest-hud}"
    textColor: "{colors.seafoam}"
    rounded: "{rounded.hud}"
    padding: "10px"
  list-row:
    backgroundColor: "{colors.glass-dim}"
    textColor: "{colors.snow}"
    rounded: "{rounded.hud}"
---

# Design System: mtgfr

**North star:** "Arena, Unplugged" — MTGA game-client polish, zero storefront chrome (see `PRODUCT.md` anti-references). Dark green felt (`forest-floor`), translucent panels, cream cards (`oracle-ivory`) on felt. **Gold = a decision is owed** (priority gold on the orb; commander gold on commanders only). **Seat hues = identity** (forest/island/mountain/arcane), never semantics.

**Layout:** Canvas board (96×134 world-unit cards, shared camera) + thin DOM HUD. Z-order: board → HUD → backdrop (29) → modal (30). No persistent nav chrome.

**The Landscape Rule.** Mobile and tablet are **landscape-first**. Auth, lobby, decks, builder, and board assume horizontal space. Portrait phones (`orientation: portrait` and `max-width: 900px`) open a native `<dialog showModal>` rotate gate (top-layer, inert background, focus trap; Escape disabled) — never a stacked vertical reflow of the builder or board. Short landscape (phone on its side) keeps side-by-side columns and tightens padding; it does not flip the axis. Safe-area insets apply on notched devices (`viewport-fit=cover`).

**Typography:** `system-ui` only. Screen ramp: title 18/700, body 14/400, button 14/600, label 13, caption 12. Game chrome: `game` 15/600 (`Button variant="game"`), `display` 22/700 (lobby table code). HUD density below caption: `chip` 11, `micro` 10 — board/hand chrome only, never prose. No display fonts.

**Semantic colors (combat):** Mountain Red = attack, Wall Green = block, Island Blue = targeting.

**Priority readability:** Gold = a decision is owed. During **instant-priority focus**, non-usable battlefield permanents take a black veil; legal activates and untapped mana sources stay bright. Empty-stack main and declare attackers/blockers stay fully lit (see ADR 0027 / `CONTEXT.md`). Yielded pass uses amber earth (`yielded` / `yielded-ink`) — not priority gold (The Gold Means Act Rule).

**Components:** Chunky buttons (Llanowar Deep → Llanowar on hover, inset highlight, shortens on press) via `~/components/atoms` Solid wrappers — never `@apply`. Panels: Forest Surface 98% + 1px Vine border; fluid width (`max-w` capped, no hard `min-w` that overflows landscape phones). HUD: Forest HUD 92%. Inputs: Glass fill + Vine border. Cards on canvas: tapped = 90° rotation; combat outlines red/green; life orbs are combat drop targets. Quiet HUD dismiss controls use `.hit-quiet` so coarse pointers still hit ≥44×44.

**Motion:** 150–250ms ease-out, state-only, `prefers-reduced-motion` fallback — never celebration.

**Implementation:** Token values live in the YAML frontmatter above; Tailwind/theme wiring in `client/src/global.css` (ADR 0024). Canvas paint hexes in `Board.tsx` / `layout.ts` are exempt from DOM tokens; keep the legend list in sync with draw.
