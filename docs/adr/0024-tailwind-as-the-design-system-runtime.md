# 0024 — Tailwind v4 as design system runtime

Status: **Accepted**; depends on [0023](0023-biome-as-the-client-toolchain.md).

## Decision

- `client/src/global.css`: `@theme` mirrors DESIGN.md YAML tokens.
- DESIGN.md §5 surfaces are Solid wrappers + utility recipes in `client/src/ui/` (no `@apply`, no `@layer components` class vocabulary).
- Delete `ui.ts`. `style` only for CSS variables (`style={{ "--x": … }}`); classes carry appearance.
- Canvas draw calls keep hex literals; DOM uses CSS vars where colors overlap.

## Consequences

- Preflight active (load-bearing for buttons/borders). `index.html` inline `#0b1310` background prevents flash.
- `STACK_OVERLAY_PAD` coupled to `hudClass` padding — keep in sync.
