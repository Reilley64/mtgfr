#!/usr/bin/env bun
// Regenerate Tailwind @theme CSS + canvas TS from repo-root design.tokens.json (DTCG).
// Usage:
//   bun scripts/gen-tokens.mjs          # write
//   bun scripts/gen-tokens.mjs --check  # fail if stale
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import StyleDictionary from "style-dictionary";

const here = dirname(fileURLToPath(import.meta.url));
const clientRoot = join(here, "..");
const repoRoot = join(clientRoot, "..");
const tokensPath = join(repoRoot, "design.tokens.json");
const cssOut = join(clientRoot, "styles/tokens.generated.css");
const tsOut = join(clientRoot, "app/domain/design-tokens.generated.ts");

const ALIAS_RE = /^\{([a-z0-9.-]+)\}$/i;

function kebabToCamel(kebab) {
  return kebab.replace(/-([a-z0-9])/g, (_, c) => c.toUpperCase());
}

function isAlias(value) {
  return typeof value === "string" && ALIAS_RE.test(value.trim());
}

/** @returns {string[]} token path segments */
function parseAlias(value) {
  const match = value.trim().match(ALIAS_RE);
  if (!match) {
    throw new Error(`not an alias: ${value}`);
  }
  return match[1].split(".");
}

function getNode(root, path) {
  let cur = root;
  for (const seg of path) {
    if (!cur || typeof cur !== "object" || !(seg in cur)) {
      return null;
    }
    cur = cur[seg];
  }
  return cur;
}

function resolveAliases(root) {
  const clone = structuredClone(root);
  const resolving = new Set();

  function resolveToken(path) {
    const key = path.join(".");
    if (resolving.has(key)) {
      throw new Error(`circular alias: ${key}`);
    }

    const node = getNode(clone, path);
    if (!node || !Object.hasOwn(node, "$value")) {
      throw new Error(`alias target missing: ${key}`);
    }
    if (!isAlias(node.$value)) {
      return node;
    }

    resolving.add(key);
    const targetPath = parseAlias(node.$value);
    const target = resolveToken(targetPath);
    resolving.delete(key);
    if (node.$type && target.$type && node.$type !== target.$type) {
      throw new Error(`alias type mismatch: ${key} -> ${targetPath.join(".")}`);
    }

    node.$type = node.$type ?? target.$type;
    node.$value = structuredClone(target.$value);
    return node;
  }

  function walk(node, path = []) {
    if (!node || typeof node !== "object" || Array.isArray(node)) {
      return;
    }
    if (Object.hasOwn(node, "$value") && isAlias(node.$value)) {
      resolveToken(path);
    }
    for (const [key, value] of Object.entries(node)) {
      if (key.startsWith("$")) {
        continue;
      }
      walk(value, [...path, key]);
    }
  }

  walk(clone);
  return clone;
}

function dim(value, unit = "px") {
  if (value && typeof value === "object" && "value" in value) {
    return `${value.value}${value.unit ?? unit}`;
  }
  return String(value);
}

function shadowDim(value) {
  if (value && typeof value === "object" && "value" in value && Number(value.value) === 0) {
    return "0";
  }
  if (Number(value) === 0) {
    return "0";
  }
  return dim(value);
}

function serializeColor(value) {
  if (typeof value === "string") {
    throw new Error("color tokens must be DTCG color objects with colorSpace");
  }

  const { colorSpace, components, alpha } = value;
  if (colorSpace === "oklch") {
    const [L, C, H] = components;
    if (alpha == null || alpha === 1) {
      return `oklch(${L} ${C} ${H})`;
    }
    return `oklch(${L} ${C} ${H} / ${alpha})`;
  }
  if (colorSpace === "srgb") {
    const [r, g, b] = components.map((c) => Math.round(c * 255));
    if (alpha == null || alpha === 1) {
      return `rgb(${r} ${g} ${b})`;
    }
    return `rgb(${r} ${g} ${b} / ${alpha})`;
  }
  throw new Error(`unsupported colorSpace: ${colorSpace}`);
}

function serializeShadowLayer(layer) {
  const inset = layer.inset ? "inset " : "";
  const x = shadowDim(layer.offsetX);
  const y = shadowDim(layer.offsetY);
  const blur = shadowDim(layer.blur ?? { value: 0, unit: "px" });
  const spreadValue = layer.spread != null ? shadowDim(layer.spread) : null;
  const spread = spreadValue != null && spreadValue !== "0" ? ` ${spreadValue}` : "";
  const color = serializeColor(layer.color);
  return `${inset}${x} ${y} ${blur}${spread} ${color}`.replace(/ {2,}/g, " ").trim();
}

function serializeShadow(value) {
  const layers = Array.isArray(value) ? value : [value];
  return layers.map(serializeShadowLayer).join(", ");
}

function normalizeRgbForCanvas(color) {
  const match = color.trim().match(/^rgb\(\s*(\d+)\s+(\d+)\s+(\d+)\s*\/\s*([0-9.]+)\s*\)$/);
  if (!match) {
    return color;
  }
  const [, r, g, b, alpha] = match;
  return `rgba(${r},${g},${b},${alpha})`;
}

function serializeCubicBezier(value) {
  const [a, b, c, d] = value;
  return `cubic-bezier(${a}, ${b}, ${c}, ${d})`;
}

function serializeDuration(value) {
  return `${value.value}${value.unit}`;
}

/** Map DTCG path → public CSS var name, or null to skip primitives. */
function publicCssVarName(path) {
  if (path[0] === "primitive") {
    return null;
  }

  const p = path[0] === "semantic" ? path.slice(1) : path;
  if (p.length >= 3 && (p[2] === "font-weight" || p[2] === "line-height")) {
    return `--${p[0]}-${p[1]}--${p[2]}`;
  }
  return `--${p.join("-")}`;
}

/** Allowlisted animate recipes (keyframe names live in global.css). */
const ANIMATE_RECIPES = {
  breathe: {
    keyframe: "breathe",
    durationPath: ["semantic", "duration", "breathe"],
    easing: "ease-in-out",
    iteration: "infinite",
  },
  skeleton: {
    keyframe: "breathe",
    durationPath: ["semantic", "duration", "skeleton"],
    easing: "ease-in-out",
    iteration: "infinite",
  },
  "shell-enter": {
    keyframe: "shell-enter",
    durationPath: ["semantic", "duration", "shell-enter"],
    easing: "var(--ease-state)",
  },
};

function serializeTokenValue($type, $value) {
  switch ($type) {
    case "color":
      return serializeColor($value);
    case "shadow":
      return serializeShadow($value);
    case "cubicBezier":
      return serializeCubicBezier($value);
    case "duration":
      return serializeDuration($value);
    case "fontFamily":
    case "dimension":
    case "number":
    case "fontWeight":
      return typeof $value === "object" && $value && "value" in $value ? dim($value) : String($value);
    case "typography":
      throw new Error("typography must be expanded before serializeTokenValue");
    default:
      throw new Error(`unsupported $type: ${$type}`);
  }
}

function serializeTokenCss(token) {
  return serializeTokenValue(token.$type ?? token.type, token.$value ?? token.value);
}

function linearToSrgb(value) {
  if (value <= 0.0031308) {
    return 12.92 * value;
  }
  return 1.055 * value ** (1 / 2.4) - 0.055;
}

function toHexByte(value) {
  const clamped = Math.max(0, Math.min(255, Math.round(value * 255)));
  return clamped.toString(16).padStart(2, "0").toUpperCase();
}

/** OKLCH → #RRGGBB for meta/PWA (compact conversion; no new dependency). */
function oklchToHex(components) {
  const [L, C, H] = components;
  const hRad = (H * Math.PI) / 180;
  const a = Math.cos(hRad) * C;
  const b = Math.sin(hRad) * C;
  const lPrime = L + 0.3963377774 * a + 0.2158037573 * b;
  const mPrime = L - 0.1055613458 * a - 0.0638541728 * b;
  const sPrime = L - 0.0894841775 * a - 1.291485548 * b;
  const l = lPrime ** 3;
  const m = mPrime ** 3;
  const s = sPrime ** 3;
  const r = linearToSrgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s);
  const g = linearToSrgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s);
  const blue = linearToSrgb(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s);
  return `#${toHexByte(r)}${toHexByte(g)}${toHexByte(blue)}`;
}

// Formats render from this raw DTCG walk, not dictionary.allTokens. Style Dictionary is mostly a
// build harness (transforms, platform wiring); SD alone omitted nested composite children such as
// text.*.font-weight and text.*.line-height. The direct JSON walk is intentional for v1 literals.
function collectDtcgTokens(node, path = []) {
  if (!node || typeof node !== "object" || Array.isArray(node)) {
    return [];
  }

  const tokens = [];
  if (Object.hasOwn(node, "$value")) {
    if (node.$type === "typography") {
      if (node.$value.fontSize) {
        tokens.push({ path, $type: "dimension", $value: node.$value.fontSize });
      }
      if (node.$value.fontWeight) {
        tokens.push({ path: [...path, "font-weight"], $type: "fontWeight", $value: node.$value.fontWeight });
      }
      if (node.$value.lineHeight) {
        const lineHeightType =
          typeof node.$value.lineHeight === "object" && node.$value.lineHeight ? "dimension" : "number";
        tokens.push({ path: [...path, "line-height"], $type: lineHeightType, $value: node.$value.lineHeight });
      }
    } else {
      tokens.push({
        path,
        $type: node.$type,
        $value: node.$value,
      });
    }
  }

  for (const [key, value] of Object.entries(node)) {
    if (key.startsWith("$")) {
      continue;
    }
    tokens.push(...collectDtcgTokens(value, [...path, key]));
  }

  return tokens;
}

const rawRoot = JSON.parse(readFileSync(tokensPath, "utf8"));
const root = resolveAliases(rawRoot);
const sourceTokens = collectDtcgTokens(root);

function publicColorKey(path) {
  const p = path[0] === "semantic" ? path.slice(1) : path;
  if (p[0] !== "color" || p.length < 2) {
    return null;
  }
  return kebabToCamel(p.slice(1).join("-"));
}

function buildShadowDragExport(root) {
  const node = getNode(root, ["semantic", "drop-shadow", "drag"]);
  if (node?.$type !== "shadow") {
    return null;
  }

  const layer = Array.isArray(node.$value) ? node.$value[0] : node.$value;
  if (!layer || typeof layer !== "object") {
    return null;
  }

  return {
    css: serializeShadow(node.$value),
    offsetY: Number(layer.offsetY?.value ?? layer.offsetY ?? 0),
    blur: Number(layer.blur?.value ?? layer.blur ?? 0),
    color: normalizeRgbForCanvas(serializeColor(layer.color)),
  };
}

function buildHexFallbacksExport(root) {
  const node = getNode(root, ["semantic", "color", "forest-floor"]);
  if (node?.$type !== "color") {
    return {};
  }
  if (typeof node.$value === "string") {
    throw new Error("hexFallbacks.forestFloor must be derived from an OKLCH color token");
  }
  if (node.$value?.colorSpace === "oklch") {
    return { forestFloor: oklchToHex(node.$value.components) };
  }
  return {};
}

function buildConfig(buildPath) {
  return {
    source: [tokensPath],
    usesDtcg: true,
    hooks: {
      transforms: {
        "name/mtgfr-css": {
          type: "name",
          transform: (token) => publicCssVarName(token.path) ?? token.name,
        },
      },
      formats: {
        "mtgfr/tailwind-theme": ({ dictionary }) => {
          // Prefer sourceTokens (see collectDtcgTokens); dictionary.allTokens is SD fallback only.
          const tokens = sourceTokens.length > 0 ? sourceTokens : dictionary.allTokens;
          const lines = [];
          for (const t of tokens) {
            const name = publicCssVarName(t.path);
            if (!name) {
              continue;
            }
            lines.push(`  ${name}: ${serializeTokenCss(t)};`);
          }
          for (const [animName, recipe] of Object.entries(ANIMATE_RECIPES)) {
            const durNode = getNode(root, recipe.durationPath);
            if (!durNode?.$value) {
              continue;
            }
            const dur = serializeDuration(durNode.$value);
            const iter = recipe.iteration ? ` ${recipe.iteration}` : "";
            lines.push(`  --animate-${animName}: ${recipe.keyframe} ${dur} ${recipe.easing}${iter};`);
          }
          return (
            `/* GENERATED by client/scripts/gen-tokens.mjs — do not edit.\n` +
            ` * Source: design.tokens.json. Regenerate: bun run gen:tokens\n */\n` +
            `@theme {\n${lines.join("\n")}\n}\n`
          );
        },
        "mtgfr/ts-colors": ({ dictionary }) => {
          const tokens = sourceTokens.length > 0 ? sourceTokens : dictionary.allTokens;
          const colorTokens = tokens.filter((t) => publicColorKey(t.path));
          const entries = colorTokens.map((t) => {
            const key = publicColorKey(t.path);
            const value = serializeColor(t.$value ?? t.value);
            return `  ${key}: ${JSON.stringify(value)},`;
          });
          const shadowDrag = buildShadowDragExport(root);
          const shadowLines = shadowDrag
            ? [
                "export const shadowDrag = {",
                `  css: ${JSON.stringify(shadowDrag.css)},`,
                `  offsetY: ${shadowDrag.offsetY},`,
                `  blur: ${shadowDrag.blur},`,
                `  color: ${JSON.stringify(shadowDrag.color)},`,
                "} as const;",
              ]
            : ["export const shadowDrag = null;"];
          const hexFallbackEntries = Object.entries(buildHexFallbacksExport(root)).map(
            ([key, value]) => `  ${key}: ${JSON.stringify(value)},`,
          );
          return (
            `/* GENERATED by client/scripts/gen-tokens.mjs — do not edit.\n` +
            ` * Source: design.tokens.json. Regenerate: bun run gen:tokens\n */\n` +
            `export const colors = {\n${entries.join("\n")}\n} as const;\n` +
            `export type ColorToken = keyof typeof colors;\n` +
            `${shadowLines.join("\n")}\n` +
            `export const hexFallbacks = {\n${hexFallbackEntries.join("\n")}\n} as const;\n`
          );
        },
      },
    },
    platforms: {
      css: {
        transforms: ["name/mtgfr-css"],
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
        transforms: ["name/mtgfr-css"],
        buildPath: `${buildPath}/`,
        files: [
          {
            destination: "design-tokens.generated.ts",
            format: "mtgfr/ts-colors",
            filter: (token) => publicColorKey(token.path) != null,
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

async function main() {
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
}

export {
  ANIMATE_RECIPES,
  buildShadowDragExport,
  getNode,
  isAlias,
  parseAlias,
  publicCssVarName,
  resolveAliases,
  serializeTokenCss,
  serializeTokenValue,
};

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  await main();
}
