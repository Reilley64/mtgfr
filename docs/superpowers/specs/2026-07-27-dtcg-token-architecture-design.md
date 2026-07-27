# DTCG token architecture — typed composites, aliases, tiers, CSS Color 4

**Status:** Implemented (2026-07-27) on branch
`cursor/dtcg-token-architecture-design-b959` / PR
[#289](https://github.com/Reilley64/mtgfr/pull/289). This file is historical
design input; living surface specs remain the source of truth:
[`2026-07-20-shell-routes-and-auth.md`](2026-07-20-shell-routes-and-auth.md)
(design-system section), plus prose in [`DESIGN.md`](../../../DESIGN.md) and the
token file [`design.tokens.json`](../../../design.tokens.json). Canvas consumers
and `client/scripts/gen-tokens.mjs` change in the same implement waves.

**Standards:** [DTCG Format / Color / Resolver 2025.10](https://www.designtokens.org/TR/2025.10/),
[CSS Color Module Level 4](https://www.w3.org/TR/css-color-4/).

---

## Problem Statement

`design.tokens.json` is already DTCG-shaped and feeds Style Dictionary into
Tailwind `@theme` + canvas TS. Several seams still fight the standard and make
cross-surface consistency harder than it should be:

1. **Opaque `$type: "css"`** — shadows, easings, and animation strings are
   passthrough blobs. Canvas (`lift-shadow.ts`) re-hand-codes the same numbers.
   Style Dictionary cannot validate structure; CSS and Mount paint can drift.
2. **Duplicated literals instead of aliases** — e.g. `playable-border` and
   `snow-mint` share a hex; semantic roles cannot point at a single decision.
3. **Flat naming** — meaningful names (`priority-gold`, `forest-floor`) sit in
   one bag with no primitive → semantic → component chain, so future density /
   contrast modes would rewrite every leaf.
4. **sRGB hex only** — felt, HUD translucency, and seat hues are tuned in hex.
   Perceptual edits (lighten hover, match gold luminance) are guesswork; CSS
   Color 4 (`oklch`, relative color) is unused even though DTCG Color Module
   2025.10 and modern canvas `fillStyle` both accept it.

## Goal

Harden the existing token pipeline so authored tokens are **valid DTCG 2025.10**,
**alias-driven**, **tiered where it pays off**, and **Color-4-native**, without
changing the visual language (“Arena, Unplugged”), without a second theme
product, and without moving component recipes into the token file.

Success looks like:

- No `$type: "css"` in `design.tokens.json`.
- Semantic colors that reuse another token do so via DTCG aliases, not copy-paste.
- A thin primitive color (and spacing) layer; public Tailwind / canvas names stay
  semantic.
- Colors authored in OKLCH (DTCG color objects); generated CSS/TS emit CSS Color 4
  strings; canvas keeps consuming generated string tokens.
- `bun run gen:tokens` / `--check` and existing token tests remain the gate;
  `lift-shadow` and similar dual CSS/canvas values come from generated structure,
  not parallel literals.

## Non-goals

- Light mode, multi-brand themes, or a checked-in `.resolver.json` product surface
  (Resolver Module stays deferred until a real second context exists).
- Adopting Material / Spectrum / Carbon token packs.
- Figma Variables ↔ repo sync.
- OpenUI web components or a new component library.
- Rewriting Foldkit UI recipes into token “component maps” (house rule stands:
  recipes stay in `client/app/domain/ui/`).
- Relitigating forest palette, Gold Means Act, or seat-vs-semantics rules.
- Forcing every unnamed canvas paint one-off (`#ff6b6b` attack stroke, chip
  fills) into tokens in this change — only named tokens and typed composites.

## Approaches considered

1. **In-place DTCG harden (chosen)** — Keep one `design.tokens.json` (optionally
   regrouped with `primitive` / `semantic` groups). Teach `gen-tokens.mjs` to
   resolve aliases, serialize typed composites to CSS, and emit OKLCH CSS strings
   (+ structured shadow/duration exports for canvas). Lowest churn; matches
   current dual-output pipeline.
2. **Multi-file + full Resolver Module** — Split primitives/semantics into files
   and add `*.resolver.json` for `motion: reduce` / density modifiers now.
   Correct for multi-context products; premature here (single dark game client;
   `prefers-reduced-motion` already handled in CSS).
3. **Typed composites only, skip tiers and OKLCH** — Fixes shadow/ease drift
   quickly but leaves duplicate hexes and perceptual tuning debt. Incomplete
   against the agreed scope (1+2+3+Color 4).

## Locked decisions

| Decision | Choice |
|---|---|
| Spec target | DTCG Format + Color modules **2025.10** |
| Resolver Module | **Defer** full theme resolvers; use Format-module **aliases** now |
| File layout | Single `design.tokens.json` with top-level `primitive` + `semantic` groups (optional `component` group empty / unused) |
| Public token names | Keep today’s semantic CSS var names (`--color-forest-floor`, …) via codegen path mapping — no Tailwind class rename wave |
| `$type: "css"` | **Delete**; replace with `shadow`, `cubicBezier`, `duration`, and (where needed) `transition` / typography composites |
| Animation keyframes | Keyframe **names** stay hand-authored in `global.css`; tokens own duration + easing (and compose `--animate-*` in codegen) |
| Color authoring | DTCG color objects with `"colorSpace": "oklch"` (+ `alpha` when needed) |
| Color emission | CSS Color 4 `oklch(...)` / `oklch(... / a)` strings into `@theme` and `colors.*` TS |
| Hex islands | `theme-color` / PWA `background_color` / favicon fill may keep a **generated hex fallback** derived from the same OKLCH source (or a documented `$extensions` hex mirror) — not a second authored palette |
| Component tier | Still TypeScript recipes; do not invent `button.primary.bg` tokens in JSON |
| Canvas | Keep string consumption; add generated structured exports only where paint needs numbers (shadow offset/blur/color) |
| Delivery | One foundation PR (schema + gen + token rewrite + test updates), then small consumer cleanups (`lift-shadow`, any hex assertions) |

---

## Design

### 1. Replace `$type: "css"` with DTCG types

#### Shadows

Author every `shadow.*` and `drop-shadow.drag` as `$type: "shadow"` composites
(single object or array of layers per Format 2025.10). Example shape:

```json
"table": {
  "$type": "shadow",
  "$value": {
    "offsetX": { "value": 0, "unit": "px" },
    "offsetY": { "value": 12, "unit": "px" },
    "blur": { "value": 40, "unit": "px" },
    "spread": { "value": 0, "unit": "px" },
    "color": { "colorSpace": "srgb", "components": [0, 0, 0], "alpha": 0.6 }
  }
}
```

Multi-layer tokens (`press`, `glow`, `pick`) use a **JSON array** of shadow
layers. Inset layers set `"inset": true` (Format 2025.10 shadow composite).
Codegen owns serialization to CSS (custom format, same as today) so Style
Dictionary gaps cannot force a return to `$type: "css"`.

**Codegen:** emit CSS `box-shadow` / `filter: drop-shadow(...)` strings into the
existing `--shadow-*` / `--drop-shadow-*` names. Also emit a small TS module
(or extend the generated file) with structured `{ offsetY, blur, color }` for
`drop-shadow.drag` so `lift-shadow.ts` imports generated constants and drops
hand-duplicated numbers.

#### Easing

```json
"state": {
  "$type": "cubicBezier",
  "$value": [0.22, 1, 0.36, 1]
}
```

Codegen → `cubic-bezier(0.22, 1, 0.36, 1)` as `--ease-state`.

#### Duration + composed animation recipes

Split today’s opaque animation strings:

- Named **duration** tokens (`duration.stack-in`, `duration.shell-enter`, …)
  with `{ "value": 0.25, "unit": "s" }`.
- Keep keyframe identifiers in CSS (`@keyframes stack-in`, `breathe`, …).
- Codegen composes `--animate-stack-in: stack-in 0.25s ease-out` (or
  `var(--ease-state)` where DESIGN.md’s state ease applies) from typed parts +
  a tiny allowlisted keyframe/easing map in `gen-tokens.mjs` — not free-form
  CSS in the token file.

Rejected: inventing a non-DTCG `$type: "animation"` blob. Rejected: moving
`@keyframes` bodies into JSON.

#### Typography composites (cleanup while here)

Today `text.title` is a dimension with sibling `font-weight` / `line-height`
children — awkward for DTCG. Migrate screen ramp entries that bundle size +
weight/line-height to `$type: "typography"` composites (Format 9.x), still
emitting the same `--text-*` / `--text-*--font-weight` CSS vars Tailwind
already consumes. Pure size steps (`label`, `caption`, …) may remain
`dimension`.

### 2. Aliases (Format references — not Resolver themes)

Use DTCG alias values so one decision has one literal:

```json
"playable-border": {
  "$type": "color",
  "$value": "{semantic.color.snow-mint}",
  "$description": "Playable outline — same ink as snow-mint"
}
```

Rules:

- Aliases only reference **same `$type`** (or a typed sub-value where Format
  allows).
- Semantic tokens may alias primitives or other semantics; primitives never
  alias semantics.
- Component recipes in TS continue to reference **semantic** Tailwind classes /
  generated `colors.*` keys — not primitive paths.
- `gen-tokens.mjs` resolves aliases before emit (Style Dictionary `usesDtcg` +
  explicit resolve pass if the custom walk still bypasses SD’s resolver — today’s
  raw JSON walk must gain alias resolution; do not leave `{…}` strings in
  outputs).
- Circular aliases fail the gen script loudly.

Duplicate-hex audit at implement time (non-exhaustive starters):
`playable-border` ↔ `snow-mint`; any other identical `$value` pairs become
aliases or deliberate documented exceptions.

**Resolver Module:** document as future hook for `motion: reduce` duration
overrides or contrast packs. Do **not** add `.resolver.json` in this work.

### 3. Three-tier naming (thin primitives)

| Tier | Lives in | Role | Example |
|---|---|---|---|
| Primitive | `primitive.color.*`, `primitive.space.*` | Raw scale, no product meaning | `primitive.color.forest-0`, `primitive.space.3` |
| Semantic | `semantic.color.*`, … | Product meaning (today’s names) | `semantic.color.forest-floor`, `priority-gold` |
| Component | TypeScript only | Recipes (`buttonClass`, surfaces) | `bg-llanowar-deep`, not JSON |

Authoring rules:

- Primitives hold OKLCH literals (and the spacing scale numbers).
- Semantics alias primitives (or other semantics) and keep the **ubiquitous
  names** from `DESIGN.md` / CONTEXT.
- Codegen flattens semantic color paths to the **current public CSS names**
  (`--color-forest-floor`) so Tailwind classes (`bg-forest-floor`) and canvas
  `colors.forestFloor` do not churn.
- Primitive tokens are emitted only if needed for debugging/docs; they are not
  required Tailwind utilities. Prefer not to generate `bg-forest-0` classes
  unless a test needs them.
- Do not introduce a parallel naming scheme (`color.bg.canvas`) that renames
  the forest vocabulary.

Spacing: existing `spacing.xs`…`xxl` become primitives; semantic aliases
(`shell-gutter` → a primitive step) where names carry meaning.

### 4. CSS Color 4 + DTCG Color Module

#### Authoring

Replace hex strings with DTCG color objects, preferred space **OKLCH**:

```json
"forest-floor": {
  "$type": "color",
  "$value": {
    "colorSpace": "oklch",
    "components": [0.08, 0.02, 150],
    "alpha": 1
  },
  "$description": "Canvas / app background"
}
```

Translucent panels (`forest-surface`, `forest-hud`, `glass`, `hud-edge`) use
the `alpha` field rather than 8-digit hex.

Optional later: CSS relative color in generated CSS for hover ramps
(`llanowar` → deeper sibling) **only** when a semantic pair today is a manual
darken; not required for v1 if both remain explicit aliased primitives.

#### Emission

- CSS `@theme`: `oklch(L C H)` / `oklch(L C H / a)` (CSS Color 4 syntax).
- TS `colors`: same CSS string values. Canvas2D `fillStyle` / `strokeStyle`
  accept CSS Color 4 strings on supported browsers (our target clients).
- Update `design-tokens.test.ts`: stop requiring `^#[0-9A-Fa-f]{6,8}$`; assert
  `oklch(` (or resolved alias equality) instead.
- Snapshot / Scene tests that compare `colors.forestFloor` keep working if they
  use the generated constant rather than a hard-coded hex.

#### Hex fallbacks (narrow)

Some HTML meta surfaces historically want hex:

- `index.html` `theme-color`
- Vite PWA `theme_color` / `background_color`
- Favicon SVG fill

**Implementation:** `gen-tokens.mjs` emits `hexFallbacks.forestFloor`, an sRGB
serialization of the same OKLCH token, for these call sites. Authored source of
truth remains OKLCH; do not hand-maintain a second palette.

#### Out of band paint

Unnamed literals called out in board code (`ATTACK_STROKE = "#ff6b6b"`, avatar
chip fills) stay literal until a later board fidelity pass promotes them to
named semantic tokens. This design does not require cleaning them.

---

## Components & data flow

```text
design.tokens.json (DTCG 2025.10)
        │
        ▼
client/scripts/gen-tokens.mjs
  - walk + alias resolve
  - serialize color → oklch() CSS
  - serialize shadow / cubicBezier / duration
  - compose --animate-* from duration + keyframe map
  - map semantic.color.* → --color-<name> + colors.<camelName>
        │
        ├─► client/styles/tokens.generated.css   (@theme)
        └─► client/app/domain/design-tokens.generated.ts
              colors + shadowDrag (structured) + …
        │
        ├─► Tailwind utilities / Foldkit HTML
        └─► Canvas / Mount / lift-shadow
```

`DESIGN.md` gains a short “Token tiers & Color 4” subsection pointing at this
design and the DTCG version pin. Shell-routes living spec’s design-system
section updates at implement time to drop `$type: "css"` language and describe
OKLCH emission + alias rules.

---

## Implementation waves (for the later plan)

1. **Gen + schema** — alias resolution, serializers, public name mapping,
   `--check` green on a minimal migrated subset.
2. **Token rewrite** — primitives + semantic aliases; typed shadows/ease/
   durations; OKLCH colors; delete `$type: "css"`.
3. **Consumers** — `lift-shadow` from generated struct; test assertion updates;
   meta/PWA hex from generated fallback; `DESIGN.md` + shell-routes living spec.
4. **Verify** — `just client-check` (token tests, Scene smoke that reads
   `forest-floor` / priority gold via generated constants).

## Testing Decisions

- Extend `client/app/domain/design-tokens.test.ts`:
  - no `$type: "css"` in source JSON
  - alias resolve (`playableBorder === snowMint` string equality)
  - `--shadow-table` / `--ease-state` / `--drop-shadow-drag` still present
  - color values match `oklch(` (or documented hex fallback file)
  - structured drag-shadow export matches CSS serialization
- Keep `gen-tokens.mjs --check` in CI / `bun run gen`.
- Update any brittle hex equality tests; prefer comparing to `colors.*`.
- No new Scene surface required (no user-visible panel); smoke that theme CSS
  still defines forest-floor is enough.

## Out of Scope

- `.resolver.json` themes / density packs.
- Design-tool round-trip.
- Renaming semantic vocabulary or Tailwind class names.
- Board unnamed paint literal cleanup.
- Changing motion timing curves’ *feel* (values preserve today’s numbers;
  format only).

## Further Notes

- Style Dictionary 5.5 remains the build harness; custom formats stay justified
  for Tailwind `@theme` and nested typography children — extend them rather than
  replacing SD.
- Prefer preserving computed visual output (screenshot / constant equality via
  generated strings) over preserving hex spelling.
- If OKLCH round-trip from today’s hex is slightly off in the 4th digit,
  lock converted values once in the PR and treat them as the new SoT after a
  quick felt/HUD eyeball on auth + board.
- Cross-link from shell polish / flight-shadow designs: drop-shadow and shell
  motion tokens must remain named and generated after this land.
