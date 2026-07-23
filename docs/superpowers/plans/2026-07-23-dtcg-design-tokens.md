# DTCG Design Tokens Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a repo-root DTCG JSON file the sole design-token source of truth, generate Tailwind `@theme` CSS and a canvas TS module via Style Dictionary under existing `bun run gen`, and strip YAML/component maps from `DESIGN.md`.

**Architecture:** `design.tokens.json` (DTCG, constrained hex/dimension/css-string subset) → Style Dictionary (`client/scripts/gen-tokens.mjs`) → committed `client/styles/tokens.generated.css` + `client/lib/design-tokens.generated.ts`. `global.css` imports the generated `@theme` vars and keeps hand-authored keyframes/interaction CSS. Foldkit UI stays TypeScript recipes; canvas named colors import the TS module. Drift check runs inside `client-check`.

**Tech Stack:** Style Dictionary `5.5.0`, Bun, DTCG JSON (`$value` / `$type`), Tailwind v4 `@theme`, Vitest, existing `just` / `bun run gen` codegen path.

**Spec:** [docs/superpowers/specs/2026-07-23-dtcg-design-tokens-design.md](../specs/2026-07-23-dtcg-design-tokens-design.md)

## Global Constraints

- DTCG JSON is the only place token *values* are authored; never hand-edit `*.generated.*`.
- Constrained DTCG subset (v1): hex/`#RRGGBBAA` color strings; dimension strings (`12px`); CSS pass-through strings for shadow / ease / animate shorthands (`$type: "css"`). No `colorSpace` objects, no component recipes.
- Token gen is part of `bun run gen` (with proto wire gen), not a parallel island.
- Generated CSS + TS are **committed**; CI proves they match the JSON.
- Components remain TypeScript (`client/lib/ui`); do not reintroduce a components token map.
- Canvas: named design colors come from the generated module; unnamed one-offs may stay literal.
- TDD: failing test → implement → pass → commit per task.
- Angular commits (`feat:`, `fix:`, `test:`, `docs:`); continue branch `cursor/dtcg-design-tokens-2dae`.
- Update `2026-07-20-client-shell-deck-builder-and-observability.md` in the same change (same code target).

---

## File map

| File | Responsibility |
|------|----------------|
| `design.tokens.json` (new, repo root) | Canonical DTCG token source |
| `client/scripts/gen-tokens.mjs` (new) | Style Dictionary build + `--check` drift mode |
| `client/styles/tokens.generated.css` (new, committed) | Generated `@theme { --color-* … }` |
| `client/lib/design-tokens.generated.ts` (new, committed) | Generated `colors` / etc. for canvas |
| `client/lib/design-tokens.test.ts` (new) | Required color keys + hex shape; generator smoke |
| `client/package.json` | `style-dictionary` devDep; split `gen:wire` / `gen:tokens`; `gen` runs both |
| `justfile` | `client-tokens-check`; wire into `client-check` |
| `client/styles/global.css` | `@import` generated theme; drop hand-mirrored token values; keep keyframes + interaction CSS |
| `client/lib/cn.test.ts` | Read `@theme` keys from `tokens.generated.css` |
| `client/app/board/chrome.ts` | Re-export named colors from generated module |
| `client/app/board/action/targeting.ts` | `TARGET_COLOR` from generated module |
| `client/app/board/geometry/stackLayout.ts` | Same `TARGET_COLOR` source (or re-export from targeting) |
| Board paint call sites | Replace matching hex literals with imports where they equal named tokens |
| `DESIGN.md` | Prose only; remove YAML frontmatter + `components` |
| Specs README, client-shell spec, `AGENTS.md` | Token SoT pointers |
| Design spec status | `Draft` → `Done` when implementation lands |

---

### Task 1: DTCG source + Style Dictionary CSS/TS generator

**Files:**
- Create: `design.tokens.json`
- Create: `client/scripts/gen-tokens.mjs`
- Create: `client/styles/tokens.generated.css` (via generator)
- Create: `client/lib/design-tokens.generated.ts` (via generator)
- Create: `client/lib/design-tokens.test.ts`
- Modify: `client/package.json` (add `style-dictionary@5.5.0` devDependency only in this task; wire scripts in Task 2)

**Interfaces:**
- Consumes: none
- Produces:
  - `design.tokens.json` groups: `color`, `font`, `text`, `radius`, `spacing`, `shadow`, `ease`, `animate`
  - CLI: `node client/scripts/gen-tokens.mjs` writes both outputs; `--check` exits 1 if either would change
  - CSS: `@theme { --color-forest-floor: #0b1310; … }` (lowercase hex ok; preserve alpha suffixes)
  - TS: `export const colors = { forestFloor: "#0B1310", … } as const` (preserve authored casing from JSON `$value`)
  - Name mapping: path `["color","forest-floor"]` → CSS `--color-forest-floor` / TS `colors.forestFloor`; path `["text","title","font-weight"]` → `--text-title--font-weight`

- [ ] **Step 1: Write the failing test**

Create `client/lib/design-tokens.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const cssPath = new URL("../styles/tokens.generated.css", import.meta.url);
const tsPath = new URL("./design-tokens.generated.ts", import.meta.url);

describe("tokens.generated.css", () => {
  it("exists and defines forest-floor in @theme", () => {
    const css = readFileSync(cssPath, "utf8");
    expect(css).toContain("@theme");
    expect(css).toMatch(/--color-forest-floor\s*:\s*#0b1310/i);
  });
});

describe("design-tokens.generated.ts", () => {
  it("exports required named colors for canvas", async () => {
    const mod = await import("./design-tokens.generated");
    const required = [
      "forestFloor",
      "priorityGold",
      "playableBorder",
      "commanderGold",
      "graveyardOutline",
      "exileOutline",
      "oracleIvory",
      "morphSlate",
      "mountainRed",
      "wallGreen",
      "islandBlue",
      "llanowar",
      "llanowarDeep",
      "reconnectRust",
      "damageCrimson",
      "phaseMint",
    ] as const;
    for (const key of required) {
      expect(mod.colors[key]).toMatch(/^#[0-9A-Fa-f]{6,8}$/);
    }
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run lib/design-tokens.test.ts`

Expected: FAIL (missing generated files or missing exports)

- [ ] **Step 3: Add Style Dictionary dependency**

```bash
cd client && bun add -d style-dictionary@5.5.0
```

- [ ] **Step 4: Author `design.tokens.json`**

Create at repo root. Port **every** value currently in `DESIGN.md` YAML colors/typography/rounded/spacing **and** every CSS-only `@theme` token in `global.css` (`hud-edge`, shadows, ease, animate, font-sans). No `components` group.

Structure (abbreviated — implementer must include the full color list from `DESIGN.md` lines 4–55 plus `hud-edge`; full text/radius/spacing/shadow/ease/animate from `global.css`):

```json
{
  "color": {
    "forest-floor": {
      "$type": "color",
      "$value": "#0B1310",
      "$description": "Canvas / app background"
    },
    "forest-surface": { "$type": "color", "$value": "#101816FA" },
    "forest-hud": { "$type": "color", "$value": "#0C1412EB" },
    "llanowar": { "$type": "color", "$value": "#2F7D46" },
    "llanowar-deep": { "$type": "color", "$value": "#276B3C" },
    "vine": { "$type": "color", "$value": "#22CC44" },
    "vine-dim": { "$type": "color", "$value": "#11CC33" },
    "snow": { "$type": "color", "$value": "#EEFFFF" },
    "seafoam": { "$type": "color", "$value": "#DDFFEE" },
    "mist": { "$type": "color", "$value": "#CCDDEE" },
    "lichen": { "$type": "color", "$value": "#99CCBB" },
    "fog": { "$type": "color", "$value": "#8899AA" },
    "snow-mint": { "$type": "color", "$value": "#EAFFF0" },
    "glass": { "$type": "color", "$value": "#FFFFFF0F" },
    "glass-dim": { "$type": "color", "$value": "#FFFFFF0D" },
    "priority-gold": {
      "$type": "color",
      "$value": "#FFD76A",
      "$description": "Gold Means Act — a decision is owed"
    },
    "playable-border": { "$type": "color", "$value": "#EAFFF0" },
    "commander-gold": { "$type": "color", "$value": "#E9B84A" },
    "graveyard-outline": { "$type": "color", "$value": "#7B5CFF" },
    "exile-outline": { "$type": "color", "$value": "#3DDC97" },
    "oracle-ivory": { "$type": "color", "$value": "#E8E4D8" },
    "morph-slate": { "$type": "color", "$value": "#2A3742" },
    "mountain-red": { "$type": "color", "$value": "#FF5555" },
    "wall-green": { "$type": "color", "$value": "#66FF99" },
    "island-blue": { "$type": "color", "$value": "#77CCFF" },
    "burn-red": { "$type": "color", "$value": "#FF8888" },
    "caution-amber": { "$type": "color", "$value": "#FFEE88" },
    "damage-crimson": { "$type": "color", "$value": "#8F2F2F" },
    "tapped-out": { "$type": "color", "$value": "#24312B" },
    "tapped-ink": { "$type": "color", "$value": "#66786E" },
    "seat-forest": { "$type": "color", "$value": "#5AC88C" },
    "seat-island": { "$type": "color", "$value": "#5A96F0" },
    "seat-mountain": { "$type": "color", "$value": "#F0785A" },
    "seat-arcane": { "$type": "color", "$value": "#C88CF0" },
    "turn-mint": { "$type": "color", "$value": "#8FDCAA" },
    "turn-ember": { "$type": "color", "$value": "#E0A878" },
    "phase-mint": { "$type": "color", "$value": "#55CC99" },
    "phase-ember": { "$type": "color", "$value": "#CC8855" },
    "note-gold": { "$type": "color", "$value": "#F0C674" },
    "auto-moss": { "$type": "color", "$value": "#3A7D52" },
    "reconnect-rust": { "$type": "color", "$value": "#7A3B13" },
    "assign-clover": { "$type": "color", "$value": "#99FF99" },
    "watch-sage": { "$type": "color", "$value": "#9FB4A8" },
    "watch-flare": { "$type": "color", "$value": "#F87171" },
    "ready-sprout": { "$type": "color", "$value": "#8FE3A8" },
    "phase-fern": { "$type": "color", "$value": "#8FA398" },
    "preview-ash": { "$type": "color", "$value": "#E8E8EA" },
    "hud-edge": { "$type": "color", "$value": "#5A786966" },
    "yielded": { "$type": "color", "$value": "#6B5A1F" },
    "yielded-hover": { "$type": "color", "$value": "#7D6A26" },
    "yielded-ink": { "$type": "color", "$value": "#FFE9A8" },
    "quiet-hover": { "$type": "color", "$value": "#2C3F35" }
  },
  "font": {
    "sans": {
      "$type": "fontFamily",
      "$value": "system-ui, sans-serif",
      "$description": "One Face Rule"
    }
  },
  "text": {
    "title": {
      "$type": "dimension",
      "$value": "18px",
      "font-weight": { "$type": "number", "$value": 700 }
    },
    "body": {
      "$type": "dimension",
      "$value": "14px",
      "line-height": { "$type": "number", "$value": 1.5 }
    },
    "button": {
      "$type": "dimension",
      "$value": "14px",
      "font-weight": { "$type": "number", "$value": 600 }
    },
    "label": { "$type": "dimension", "$value": "13px" },
    "caption": { "$type": "dimension", "$value": "12px" },
    "chip": { "$type": "dimension", "$value": "11px" },
    "micro": { "$type": "dimension", "$value": "10px" },
    "game": {
      "$type": "dimension",
      "$value": "15px",
      "font-weight": { "$type": "number", "$value": 600 }
    },
    "display": {
      "$type": "dimension",
      "$value": "22px",
      "font-weight": { "$type": "number", "$value": 700 }
    }
  },
  "radius": {
    "panel": { "$type": "dimension", "$value": "12px" },
    "modal": { "$type": "dimension", "$value": "10px" },
    "game": { "$type": "dimension", "$value": "10px" },
    "hud": { "$type": "dimension", "$value": "8px" },
    "control": { "$type": "dimension", "$value": "6px" },
    "focus": { "$type": "dimension", "$value": "4px" }
  },
  "spacing": {
    "xs": { "$type": "dimension", "$value": "6px" },
    "sm": { "$type": "dimension", "$value": "8px" },
    "md": { "$type": "dimension", "$value": "10px" },
    "lg": { "$type": "dimension", "$value": "14px" },
    "xl": { "$type": "dimension", "$value": "16px" },
    "xxl": { "$type": "dimension", "$value": "24px" }
  },
  "shadow": {
    "table": { "$type": "css", "$value": "0 12px 40px rgb(0 0 0 / 0.6)" },
    "hand": { "$type": "css", "$value": "0 6px 18px rgb(0 0 0 / 0.5)" },
    "hud": { "$type": "css", "$value": "0 6px 20px rgb(0 0 0 / 0.45)" },
    "press": {
      "$type": "css",
      "$value": "inset 0 1px 0 rgb(255 255 255 / 0.16), 0 6px 16px rgb(0 0 0 / 0.4)"
    },
    "press-active": {
      "$type": "css",
      "$value": "inset 0 1px 0 rgb(255 255 255 / 0.1), 0 3px 10px rgb(0 0 0 / 0.4)"
    },
    "glow": {
      "$type": "css",
      "$value": "inset 0 1px 0 rgb(255 255 255 / 0.18), 0 0 0 2px rgb(90 200 140 / 0.55), 0 6px 20px rgb(47 140 90 / 0.45)"
    },
    "pick": {
      "$type": "css",
      "$value": "0 0 0 3px #22cc44, 0 10px 24px rgb(0 0 0 / 0.6)"
    }
  },
  "drop-shadow": {
    "drag": { "$type": "css", "$value": "0 10px 24px rgb(0 0 0 / 0.6)" }
  },
  "ease": {
    "state": { "$type": "css", "$value": "cubic-bezier(0.22, 1, 0.36, 1)" }
  },
  "animate": {
    "stack-in": { "$type": "css", "$value": "stack-in 0.25s ease-out" },
    "stack-return": { "$type": "css", "$value": "stack-return 0.22s ease-in" },
    "stack-resolve": { "$type": "css", "$value": "stack-resolve 0.22s ease-out" },
    "breathe": { "$type": "css", "$value": "breathe 1.8s ease-in-out infinite" },
    "skeleton": { "$type": "css", "$value": "breathe 1.2s ease-in-out infinite" }
  }
}
```

Note: `button-label` in DESIGN.md maps to `@theme` `--text-button` (already the Tailwind name) — use path `text.button`, not `text.button-label`.

- [ ] **Step 5: Implement `client/scripts/gen-tokens.mjs`**

```js
#!/usr/bin/env node
// Regenerate Tailwind @theme CSS + canvas TS from repo-root design.tokens.json (DTCG).
// Usage:
//   node scripts/gen-tokens.mjs          # write
//   node scripts/gen-tokens.mjs --check  # fail if stale
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import StyleDictionary from "style-dictionary";

const here = dirname(fileURLToPath(import.meta.url));
const clientRoot = join(here, "..");
const repoRoot = join(clientRoot, "..");
const tokensPath = join(repoRoot, "design.tokens.json");
const cssOut = join(clientRoot, "styles/tokens.generated.css");
const tsOut = join(clientRoot, "lib/design-tokens.generated.ts");

function kebabToCamel(kebab) {
  return kebab.replace(/-([a-z0-9])/g, (_, c) => c.toUpperCase());
}

function cssVarName(path) {
  // ["text","title","font-weight"] → --text-title--font-weight
  // ["color","forest-floor"] → --color-forest-floor
  // ["drop-shadow","drag"] → --drop-shadow-drag
  if (path.length >= 3 && (path[2] === "font-weight" || path[2] === "line-height")) {
    return `--${path[0]}-${path[1]}--${path[2]}`;
  }
  return `--${path.join("-")}`;
}

function buildConfig(buildPath) {
  return {
    source: [tokensPath],
    usesDtcg: true,
    hooks: {
      transforms: {
        "name/mtgfr-css": {
          type: "name",
          transform: (token) => cssVarName(token.path),
        },
        "value/css-passthrough": {
          type: "value",
          filter: (token) => (token.$type ?? token.type) === "css",
          transform: (token) => token.$value ?? token.value,
        },
      },
      formats: {
        "mtgfr/tailwind-theme": ({ dictionary }) => {
          const lines = dictionary.allTokens.map((t) => {
            const name = cssVarName(t.path);
            const value = t.$value ?? t.value;
            return `  ${name}: ${value};`;
          });
          return (
            `/* GENERATED by client/scripts/gen-tokens.mjs — do not edit.\n` +
            ` * Source: design.tokens.json. Regenerate: bun run gen:tokens\n */\n` +
            `@theme {\n${lines.join("\n")}\n}\n`
          );
        },
        "mtgfr/ts-colors": ({ dictionary }) => {
          const colorTokens = dictionary.allTokens.filter((t) => t.path[0] === "color");
          const entries = colorTokens.map((t) => {
            const key = kebabToCamel(t.path.slice(1).join("-"));
            const value = t.$value ?? t.value;
            return `  ${key}: ${JSON.stringify(value)},`;
          });
          return (
            `/* GENERATED by client/scripts/gen-tokens.mjs — do not edit.\n` +
            ` * Source: design.tokens.json. Regenerate: bun run gen:tokens\n */\n` +
            `export const colors = {\n${entries.join("\n")}\n} as const;\n` +
            `export type ColorToken = keyof typeof colors;\n`
          );
        },
      },
    },
    platforms: {
      css: {
        transforms: ["name/mtgfr-css", "value/css-passthrough"],
        buildPath: `${buildPath}/`,
        files: [
          {
            destination: "tokens.generated.css",
            format: "mtgfr/tailwind-theme",
            options: { showFileHeader: false },
          },
        ],
      },
      ts: {
        transforms: ["name/mtgfr-css", "value/css-passthrough"],
        buildPath: `${buildPath}/`,
        files: [
          {
            destination: "design-tokens.generated.ts",
            format: "mtgfr/ts-colors",
            filter: (token) => token.path[0] === "color",
            options: { showFileHeader: false },
          },
        ],
      },
    },
  };
}

async function generateTo(dir) {
  mkdirSync(dir, { recursive: true });
  // SD 5.5: prefer constructor+config; if that throws, use `new StyleDictionary()` + `await sd.extend(config)`.
  const sd = new StyleDictionary(buildConfig(dir));
  await sd.buildAllPlatforms();
  return {
    css: readFileSync(join(dir, "tokens.generated.css"), "utf8"),
    ts: readFileSync(join(dir, "design-tokens.generated.ts"), "utf8"),
  };
}

const check = process.argv.includes("--check");
const tmp = mkdtempSync(join(tmpdir(), "mtgfr-tokens-"));
try {
  const out = await generateTo(tmp);
  if (check) {
    const curCss = readFileSync(cssOut, "utf8");
    const curTs = readFileSync(tsOut, "utf8");
    if (curCss !== out.css || curTs !== out.ts) {
      console.error("design tokens outputs are stale — run: cd client && bun run gen:tokens");
      process.exit(1);
    }
    console.log("design tokens outputs up to date");
  } else {
    mkdirSync(dirname(cssOut), { recursive: true });
    mkdirSync(dirname(tsOut), { recursive: true });
    writeFileSync(cssOut, out.css);
    writeFileSync(tsOut, out.ts);
    console.log(`wrote ${cssOut}\nwrote ${tsOut}`);
  }
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
```

If `$type: "css"` causes SD to warn/fail, keep the identity `value/css-passthrough` transform and ensure unknown types are not rejected (SD 5 typically allows custom types with a value transform).

- [ ] **Step 6: Run generator and tests**

```bash
cd client && node scripts/gen-tokens.mjs
cd client && bunx vitest run lib/design-tokens.test.ts
```

Expected: PASS. Confirm generated CSS includes `--color-hud-edge`, `--shadow-table`, `--text-title--font-weight`, `--drop-shadow-drag`, `--font-sans`.

- [ ] **Step 7: Commit**

```bash
git add design.tokens.json client/scripts/gen-tokens.mjs \
  client/styles/tokens.generated.css client/lib/design-tokens.generated.ts \
  client/lib/design-tokens.test.ts client/package.json client/bun.lock
git commit -m "feat: add DTCG tokens and Style Dictionary generator"
```

---

### Task 2: Fold token gen into `bun run gen` + CI check

**Files:**
- Modify: `client/package.json` scripts
- Modify: `justfile` (`client-tokens-check`, `client-check`)

**Interfaces:**
- Consumes: `client/scripts/gen-tokens.mjs` from Task 1
- Produces: `bun run gen` runs wire **and** tokens; `just client-tokens-check` / `client-check` fail on drift

- [ ] **Step 1: Write a failing check expectation**

Manually dirty `client/styles/tokens.generated.css` (add a space in a comment), then plan to run check — or add a one-line shell assertion in the step below. Prefer implementing scripts then proving `--check` fails on dirt and passes when clean.

- [ ] **Step 2: Update `client/package.json` scripts**

Replace the single `gen` script with:

```json
"gen:wire": "PATH=\"$PWD/node_modules/.bin:$PATH\" bunx --bun buf generate --template ../proto/buf.gen.yaml ../proto",
"gen:tokens": "node scripts/gen-tokens.mjs",
"gen:tokens:check": "node scripts/gen-tokens.mjs --check",
"gen": "bun run gen:wire && bun run gen:tokens",
```

Keep `predev` / `prebuild` / `pretest` / `pretypecheck` as `"bun run gen"` (unchanged names).

- [ ] **Step 3: Update `justfile`**

After `server-codegen` / near mana-oracle recipes, add:

```just
[group('client')]
[doc("Fail if tokens.generated.* are stale vs design.tokens.json")]
client-tokens-check:
    cd client && bun run gen:tokens:check
```

Change:

```just
client-check: client-tokens-check server-codegen client-format client-lint client-typecheck client-test
```

(`server-codegen` already runs full `bun run gen`, so tokens regenerate before check; `client-tokens-check` then asserts the committed tree matches — if gen rewrites the working tree during check, ensure check compares to git or run check **without** rewriting: `--check` must not write. Task 1's `--check` already generates to temp and diffs.)

- [ ] **Step 4: Verify**

```bash
cd client && bun run gen:tokens:check   # PASS
# dirty the css file, then:
cd client && bun run gen:tokens:check   # FAIL
cd client && bun run gen:tokens         # restore
just server-codegen                     # runs wire + tokens
```

- [ ] **Step 5: Commit**

```bash
git add client/package.json justfile
git commit -m "build: run design token codegen under bun run gen"
```

---

### Task 3: Switch `global.css` + `cn` drift test to generated theme

**Files:**
- Modify: `client/styles/global.css`
- Modify: `client/lib/cn.test.ts`
- Test: `client/lib/cn.test.ts`, `client/lib/design-tokens.test.ts`

**Interfaces:**
- Consumes: `tokens.generated.css` `@theme` block
- Produces: `global.css` no longer hand-defines `--color-*` / `--text-*` / `--radius-*` / `--spacing-*` / `--shadow-*` / `--ease-*` / `--animate-*` / `--font-sans`; keyframes remain

- [ ] **Step 1: Update `cn.test.ts` to read generated CSS (fail if path wrong)**

Change the css read to:

```ts
const css = readFileSync(new URL("../styles/tokens.generated.css", import.meta.url), "utf8");
```

Update the describe/it copy from `global.css` to `tokens.generated.css`. Keep the same `THEME_SCALES` assertions.

- [ ] **Step 2: Run cn tests — should still pass against generated file if scales match**

Run: `cd client && bunx vitest run lib/cn.test.ts`

Expected: PASS if Task 1 ported all text/radius/spacing keys.

- [ ] **Step 3: Rewrite `global.css` theme section**

Near the top (after mana `@import`s is fine), add:

```css
@import "./tokens.generated.css";
```

Delete the hand-authored CSS variable declarations inside `@theme { … }` (colors through `--animate-skeleton`), **but keep** the `@keyframes` blocks. Prefer a second `@theme` block that only contains keyframes:

```css
@import "./tokens.generated.css";

/* Keyframes referenced by --animate-* tokens. Not generated — tied to animation names. */
@theme {
  @keyframes stack-in { /* unchanged bodies from current global.css */ }
  @keyframes stack-return { /* … */ }
  @keyframes stack-resolve { /* … */ }
  @keyframes breathe { /* … */ }
}
```

Update the file header comment to say values come from `design.tokens.json` via `tokens.generated.css`.

- [ ] **Step 4: Run client style/type tests**

```bash
cd client && bunx vitest run lib/cn.test.ts lib/design-tokens.test.ts
cd client && bun run typecheck
```

Expected: PASS. Spot-check a Folkit surface still has utilities (no full browser required if unit green).

- [ ] **Step 5: Commit**

```bash
git add client/styles/global.css client/lib/cn.test.ts
git commit -m "refactor: consume generated design tokens in global.css"
```

---

### Task 4: Canvas named colors from generated TS module

**Files:**
- Modify: `client/app/board/chrome.ts`
- Modify: `client/app/board/action/targeting.ts`
- Modify: `client/app/board/geometry/stackLayout.ts` (re-export or import same source — prefer single definition in `targeting.ts`, stackLayout imports from targeting if it currently duplicates)
- Modify: `client/app/board/bitmap/paint-cards.ts` (`#e8e4d8` → `colors.oracleIvory`, `#2a3742` → `colors.morphSlate`, and other exact token matches)
- Modify: `client/app/board/bitmap/paint-flights.ts` (`#e8e4d8` → `colors.oracleIvory`)
- Modify: `client/app/board/canvas/scene.ts` / `avatars.ts` / `bitmap/mount.ts` (`#ffd76a` → `colors.priorityGold`)
- Modify: `client/app/board/html/activation-radial.ts` (`#276B3C` → `colors.llanowarDeep`, `#FFD76A` → `colors.priorityGold`, `#EEFFFF` → `colors.snow`)
- Modify: `client/app/board/html/discoverability.ts` (replace remaining hexes that match tokens: commander gold, reconnect rust, phase mint, mountain red, damage crimson, forest-hud, llanowar, etc.)
- Test: existing board tests (`chrome` consumers, `scene.test.ts`, `paint-cards.test.ts`, `arrows.test.ts`) + `design-tokens.test.ts`

**Interfaces:**
- Consumes: `import { colors } from "~/design-tokens.generated"`
- Produces: `PLAYABLE_BORDER`, `COMMANDER_GOLD`, `GRAVEYARD_OUTLINE`, `EXILE_OUTLINE`, `TARGET_COLOR` equal to generated values; no behavior change

- [ ] **Step 1: Point chrome + targeting at generated colors**

`chrome.ts`:

```ts
import { colors } from "~/design-tokens.generated";

export const CARD_RESTING_OUTLINE = "#1a1a1a"; // unnamed one-off — stays literal
export const PLAYABLE_BORDER = colors.playableBorder;
export const COMMANDER_GOLD = colors.commanderGold;
export const GRAVEYARD_OUTLINE = colors.graveyardOutline;
export const EXILE_OUTLINE = colors.exileOutline;
```

`targeting.ts`:

```ts
import { colors } from "~/design-tokens.generated";

export const TARGET_COLOR = colors.islandBlue;
```

If `stackLayout.ts` still defines its own `TARGET_COLOR`, delete the local const and `export { TARGET_COLOR } from "../action/targeting"` (or import for local use only).

- [ ] **Step 2: Run board tests (expect PASS — values unchanged)**

```bash
cd client && bunx vitest run app/board/canvas/scene.test.ts app/board/canvas/arrows.test.ts app/board/bitmap/paint-cards.test.ts
```

Expected: PASS

- [ ] **Step 3: Replace exact token hex literals in paint/HTML listed above**

Only when the hex equals a token `$value` (case-insensitive). Leave true one-offs (`#1a1a1a`, `#15241c`, `#26302a`, `#f4efe2` if not tokenized, etc.).

- [ ] **Step 4: Re-run board + token tests**

```bash
cd client && bunx vitest run lib/design-tokens.test.ts app/board
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board client/lib/design-tokens.generated.ts
git commit -m "refactor: canvas named colors import generated design tokens"
```

---

### Task 5: Docs — prose `DESIGN.md`, specs, AGENTS; mark design Done

**Files:**
- Modify: `DESIGN.md` (strip YAML frontmatter and `components`; add pointer to `design.tokens.json` + generated outputs)
- Modify: `docs/superpowers/specs/README.md` companion table
- Modify: `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md` design-system section
- Modify: `AGENTS.md` design-tokens sentence
- Modify: `docs/superpowers/specs/2026-07-23-dtcg-design-tokens-design.md` status → `Done`
- Modify: `client/lib/ui/buttonClass.ts` comment if it says “DESIGN.md §5 button vocabulary” — point at tokens + this file as recipe owner

**Interfaces:**
- Consumes: implemented pipeline
- Produces: docs match current behavior (no TBD)

- [ ] **Step 1: Rewrite `DESIGN.md` header**

Remove the entire YAML frontmatter (`---` … `---`). Start with `# Design System: mtgfr` and keep north-star / rules / layout / typography / motion prose. Replace **Implementation** bullet with:

```markdown
**Tokens:** Values live in [`design.tokens.json`](design.tokens.json) (DTCG). Codegen (`bun run gen` → Style Dictionary) writes `client/styles/tokens.generated.css` (Tailwind `@theme`) and `client/lib/design-tokens.generated.ts` (canvas). Component recipes live in TypeScript (`client/lib/ui`), not in the token file. Canvas named colors import the generated TS module; unnamed paint one-offs may stay literal.
```

Keep semantic rules (Gold Means Act, seat ≠ semantics, combat colors by token **name**).

- [ ] **Step 2: Update companion + feature specs + AGENTS**

Specs README companion row:

| [`DESIGN.md`](../../../DESIGN.md) | Design system rules / north star (prose) |
| [`design.tokens.json`](../../../design.tokens.json) | DTCG design token source of truth |

Client-shell design-system section: replace “YAML frontmatter is the single source of truth” with DTCG JSON + generated CSS/TS + codegen-under-`gen` + no component maps.

`AGENTS.md`: “Design tokens live in `design.tokens.json` (DTCG); Tailwind/`@theme` and canvas TS are generated — see DESIGN.md prose and client-shell spec.”

Design spec: `**Status:** Done`

- [ ] **Step 3: Authoring conventions line in specs README**

Update “Reference `DESIGN.md` token names rather than raw hex; canvas hex literals are the documented exception” → “Reference DTCG token names (`design.tokens.json`); canvas uses `design-tokens.generated.ts` for named colors; unnamed paint literals remain the exception.”

- [ ] **Step 4: Verify docs-only + full client check**

```bash
just client-tokens-check
cd client && bunx vitest run lib/cn.test.ts lib/design-tokens.test.ts
just client-check
```

Expected: all PASS (format/lint may rewrite — include those fixes).

- [ ] **Step 5: Commit**

```bash
git add DESIGN.md AGENTS.md docs/superpowers/specs client/lib/ui/buttonClass.ts
git commit -m "docs: DTCG tokens as SoT; DESIGN.md prose-only"
```

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| DTCG JSON sole SoT | Task 1 |
| Style Dictionary → CSS `@theme` + canvas TS | Task 1 |
| Constrained hex/dimension/css subset | Task 1 |
| No component recipes in tokens | Task 1 / 5 |
| Components in TypeScript | Already true; Task 5 docs |
| Canvas joins SoT via generated module | Task 4 |
| Tokens-file-first (no live Figma sync) | Out of scope — no task |
| Under `bun run gen` / codegen | Task 2 |
| Drift check in `client-check` | Task 2 |
| `DESIGN.md` prose-only | Task 5 |
| Update client-shell spec + README + AGENTS | Task 5 |
| Keyframes stay hand-authored | Task 3 |
| Invalid tokens fail build | Task 1 generator (SD throws) |

No TBD/TODO placeholders remain in tasks. Name mapping (`forestFloor` / `--color-forest-floor`) is consistent across Tasks 1 and 4.
