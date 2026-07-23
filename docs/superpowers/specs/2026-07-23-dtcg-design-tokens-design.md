# DTCG design tokens

**Status:** Done  
**Date:** 2026-07-23

## Goal

Align mtgfr design tokens with the [Design Tokens Community Group (DTCG)](https://www.designtokens.org/) format so that:

1. **Interop** — the token file can be imported into design tools (Figma / Tokens Studio / Penpot) without a bespoke exporter.
2. **Single source of truth** — Style Dictionary generates Tailwind `@theme` CSS and a canvas TS module; hand-mirroring between `DESIGN.md` YAML and `global.css` ends.
3. **House standard** — DTCG is the authored format; prose in `DESIGN.md` no longer carries token values.

## Prior state

- Token values lived in `DESIGN.md` YAML frontmatter (`colors`, `typography`, `rounded`, `spacing`, `components`).
- `client/styles/global.css` hand-mirrored those into a Tailwind v4 `@theme` block; drift was a documented failure mode.
- Some theme values existed only in CSS (`hud-edge`, shadows, motion).
- Canvas paint used hard-coded hexes in `client/app/board/` and was exempt from DOM tokens.
- Component recipes lived in YAML `{colors.x}` maps but were realized as Foldkit/`client/lib/ui` TypeScript — a dual representation nobody wanted.

This was a project-local token convention, not DTCG.

## Decisions (locked)

| Decision | Choice |
|---|---|
| Approach | Style Dictionary over DTCG JSON → CSS + TS |
| Canonical file | Repo DTCG JSON (e.g. `design.tokens.json` at repo root) |
| `DESIGN.md` | Prose only (north star, rules, layout, motion intent); no YAML values; no `components` map |
| Components | TypeScript only (`client/lib/ui`); not tokenized |
| Canvas | In scope — generated TS module; named design colors import from it |
| Design tools | Tokens-file-first; export into tools as needed; no live bidirectional sync in v1 |
| Codegen | Token generation is part of existing `bun run gen` / `just server-codegen` (and `predev` / `prebuild` / `pretest` / `pretypecheck`); drift check is part of `client-check` / `just check` |

## Source layout & pipeline

### Canonical source

One DTCG JSON file at repo root (sibling of prose `DESIGN.md`), e.g. `design.tokens.json`.

Groups cover today’s primitives: color, typography (size / weight / line-height / family), dimension (spacing + radius), shadow, easing, duration / animation names as already present in `@theme`. No component or interaction-state recipes.

### Constrained DTCG subset (v1)

- **Colors:** hex (or hex+alpha) string `$value` with `$type: "color"` — not full 2025.10 `colorSpace` objects yet (Style Dictionary 2025.10 coverage is still maturing; hex keeps tool interop simple).
- **Spacing / radius:** dimension strings (`"12px"`) or Style Dictionary–compatible dimension objects.
- **Typography:** individual size / weight / line-height / font-family tokens — not composite “button-game” recipes.
- **`$description`:** optional semantic notes (e.g. Gold Means Act; seat hues ≠ combat semantics).

### Outputs (generated, committed)

Style Dictionary (Bun-runnable config) emits:

1. `client/styles/tokens.generated.css` — Tailwind v4 `@theme { … }` block, imported by `global.css`.
2. `client/lib/design-tokens.generated.ts` — flat typed exports for canvas / non-Tailwind paint.

**Ownership:** edit JSON → run codegen → commit source and outputs. Never hand-edit generated files. Runtime does not depend on Style Dictionary (outputs are committed); CI proves they match the JSON.

### Pipeline under codegen

Token gen is not a parallel island:

- `bun run gen` runs **proto wire gen and** Style Dictionary token gen.
- `just server-codegen` (which invokes `bun run gen`) therefore covers tokens.
- `predev` / `prebuild` / `pretest` / `pretypecheck` already call `gen`, so local workflows stay one command.
- A tokens drift check (regenerate and diff, or `--check`) is wired into `client-check` / `just check`, same failure posture as other generated artifacts (e.g. mana-oracle check).

## Consumers

### DOM / Tailwind

Call sites stay as Tailwind utilities (`bg-forest-floor`, `text-snow`, `rounded-game`, etc.). Only authoring of the CSS variables moves into `tokens.generated.css`. Hand-written rules in `global.css` (mana font, keyframe bodies, hover recipes) remain; they may reference theme vars but must not redefine token values.

### Foldkit UI

Component look (button variants, panels, inputs) lives entirely in TypeScript class recipes (`buttonClass`, surfaces, etc.) using Tailwind token classes / CSS vars. The former `DESIGN.md` `components` map is deleted; any gap is fixed in TS, not reintroduced as token recipes.

### Canvas / board paint

Named design colors (priority gold, combat outlines, seat hues, zone outlines, oracle ivory, morph slate, etc.) import from `design-tokens.generated.ts`. Discoverability / legend chrome uses the same module so DOM legend and canvas stay aligned.

One-off paint values that are **not** named design decisions may stay literal. Naming a token is the bar for “this is a design decision.”

## Migration (implementation plan deliverable)

1. Port YAML maps + CSS-only theme values (`hud-edge`, shadows, motion tokens) into DTCG JSON.
2. Add Style Dictionary config + custom formats for `@theme` CSS and canvas TS.
3. Fold token gen into `bun run gen`; add drift check to `client-check`.
4. Generate outputs; strip duplicated `@theme` token values from `global.css` (keep keyframes / interaction CSS); `@import` the generated file.
5. Point canvas named colors at the TS module.
6. Strip YAML / `components` from `DESIGN.md`; update companion docs:
   - `docs/superpowers/specs/README.md` — `DESIGN.md` row becomes prose/rules; DTCG JSON is token SoT.
   - `2026-07-20-client-shell-deck-builder-and-observability.md` — design-system section.
   - `AGENTS.md` — design tokens pointer.
7. Confirm Foldkit UI covers former component recipes; delete dead map references.

Invalid DTCG / unknown `$type` fails the Style Dictionary build; no silent fallback to old hand-maintained CSS.

## Testing

- Existing `cn.test.ts` / theme-key assertions keep reading `@theme` from the generated CSS path.
- Add a small test that the canvas module exports required named colors (keys present, hex shape).
- No Scene tests for the pipeline itself — UI surfaces that already use classes keep behavior coverage.
- CI: `tokens` drift check inside `client-check` fails on stale committed outputs.

## Spec / doc targets

| Doc | Change |
|---|---|
| This design | Approved intent for the work |
| `client-shell-deck-builder-and-observability` | Update “current behavior” design-system section when implementation lands (same code target) |
| `DESIGN.md` | Prose-only; point at DTCG JSON + generated outputs |
| Specs README companion table | Token SoT → DTCG JSON; `DESIGN.md` = rules / north star |
| `AGENTS.md` | Same pointer update |

## Out of scope (v1)

- Live bidirectional Figma / Tokens Studio sync
- Multi-theme / mode collections
- Full DTCG 2025.10 structured color objects (`colorSpace` / components)
- Generating component CSS or Foldkit recipes from tokens
- Pulling unnamed one-off canvas literals into the token file without a design-decision name

## Further notes

- Style Dictionary v4+ has first-class DTCG support; full 2025.10 module coverage is still evolving — the constrained hex/dimension subset is intentional for v1.
- Wire proto codegen remains gitignored under `client/lib/wire/generated/`; token outputs are **committed** (different consumers: canvas + CSS at build without SD). Both still run under the same `gen` entrypoint.
- Reference: [DTCG](https://www.designtokens.org/), [Style Dictionary DTCG notes](https://styledictionary.com/info/dtcg/).
