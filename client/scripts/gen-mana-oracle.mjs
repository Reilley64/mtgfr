#!/usr/bin/env bun
// Regenerate `styles/mana-oracle.css` from `mana-font`'s full stylesheet — pip/cost rules plus
// tray extras (multicolor duo + color indicators). Not the full ability glyph sheet.
//
// Also emits `app/domain/mana-glyphs.generated.ts`: the same codepoints as a plain map, because a
// card face drawn on canvas has no `::before` to read them from.
// Usage:
//   bun scripts/gen-mana-oracle.mjs          # write
//   bun scripts/gen-mana-oracle.mjs --check  # fail if stale
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const srcCss = join(root, "node_modules/mana-font/css/mana.css");
const outCss = join(root, "styles/mana-oracle.css");
const outTs = join(root, "app/domain/mana-glyphs.generated.ts");

const HEADER = `\
/* Subset of mana-font for oracle/approximates pips + mana-tray symbols (duo, color indicators).
 * Regenerate: \`just client-mana-oracle\` (or \`bun scripts/gen-mana-oracle.mjs\`).
 * Check stale: \`just client-mana-oracle-check\`.
 */
`;

/** Multicolor duo (any-color credit) + color indicators (of_colors) — no Strixhaven school duos. */
function extractTrayExtras(css) {
  const duoStart = css.indexOf(".ms-duo {");
  if (duoStart < 0) throw new Error("mana.css: missing `.ms-duo {` block");
  const ciStart = css.indexOf("\n.ms-ci {", duoStart);
  if (ciStart < 0) throw new Error("mana.css: missing `.ms-ci {` block");
  const mechanic = css.indexOf("\n.ms-mechanic {", ciStart);
  if (mechanic < 0) throw new Error("mana.css: missing `.ms-mechanic {` marker");

  const duoBlock = css.slice(duoStart, ciStart);
  // Keep only generic multicolor duo rules; drop school-specific duo colorizations.
  const duoKept = duoBlock
    .split(/(?=\.ms-duo)/)
    .filter((chunk) => {
      if (!chunk.trim()) return false;
      if (chunk.includes("ms-school-")) return false;
      return true;
    })
    .join("");

  const ciBlock = css.slice(ciStart + 1, mechanic); // drop leading newline
  return `${duoKept.trimEnd()}\n\n${ciBlock.trimEnd()}\n`;
}

function extract(css) {
  const start = css.indexOf(".ms {");
  if (start < 0) throw new Error("mana.css: missing `.ms {` block");
  const end = css.indexOf(".ms-100::before");
  if (end < 0) throw new Error("mana.css: missing `.ms-100::before` marker");
  const glyphBlock = css.slice(start, end);

  const c0 = css.indexOf(".ms-cost {");
  if (c0 < 0) throw new Error("mana.css: missing `.ms-cost {` block");
  const c1 = css.indexOf("span.ms-half");
  if (c1 < 0) throw new Error("mana.css: missing `span.ms-half` marker");
  const costBlock = css.slice(c0, c1);

  const tray = extractTrayExtras(css);

  const text = `${HEADER}${glyphBlock.trimEnd()}\n\n${costBlock.trimEnd()}\n\n${tray}`;
  if (text.includes("MPlantin") || text.includes("ability-")) {
    throw new Error("mana-oracle extract picked up unrelated rules");
  }
  return text;
}

/** `.ms-w::before, .ms-rw::after { content: "\e600"; }` → the codepoint under each selector. */
const CONTENT_RULE = /([^{}]+)\{\s*content:\s*"\\([0-9a-f]{4,6})";\s*\}/g;
const SELECTOR = /^\.ms-([a-z0-9-]+)::(before|after)$/;

/**
 * The `.ms-*` suffixes worth a codepoint: exactly the ones `manaFontClass` can return, read off its
 * own `KNOWN` set so the two cannot drift. mana-font ships ~600 glyphs — ability icons, every
 * numeral to a million, per-set variants — and a card face draws none of them.
 */
function wantedSuffixes() {
  const source = readFileSync(join(root, "app/domain/oracleText.ts"), "utf8");
  const known = /const KNOWN = new Set\(\[([\s\S]*?)\]\)/.exec(source);
  if (known == null) throw new Error("oracleText.ts: no `const KNOWN = new Set([…])` to read");
  return new Set([...known[1].matchAll(/"([^"]+)"/g)].map(([, code]) => code));
}

function extractGlyphs(css) {
  const wanted = wantedSuffixes();
  const before = {};
  const after = {};
  for (const [, selectors, hex] of css.matchAll(CONTENT_RULE)) {
    const glyph = String.fromCodePoint(Number.parseInt(hex, 16));
    for (const selector of selectors.split(",")) {
      const parsed = SELECTOR.exec(selector.trim());
      if (parsed == null) continue;
      const [, suffix, pseudo] = parsed;
      if (!wanted.has(suffix)) continue;
      (pseudo === "before" ? before : after)[suffix] = glyph;
    }
  }
  const missing = [...wanted].filter((suffix) => before[suffix] == null);
  if (missing.length > 0) throw new Error(`mana.css has no codepoint for: ${missing.join(", ")}`);
  return { before, after };
}

/** `{ w: "\ue600", … }` — escaped, so the file stays diffable text rather than invisible glyphs. */
function tsMap(entries) {
  return Object.entries(entries)
    .sort(([a], [b]) => (a < b ? -1 : 1))
    .map(([key, glyph]) => `  ${/^[a-z][a-z0-9]*$/.test(key) ? key : JSON.stringify(key)}: "\\u${glyph.codePointAt(0).toString(16)}",`)
    .join("\n");
}

function generateTs(css) {
  const { before, after } = extractGlyphs(css);
  return `\
// Generated by \`scripts/gen-mana-oracle.mjs\` from mana-font — do not edit.
// Regenerate: \`just client-mana-oracle\`. Check stale: \`just client-mana-oracle-check\`.
//
// The same codepoints \`mana-oracle.css\` puts in \`::before\`/\`::after\`, as a plain map: a card face
// drawn on canvas names them in \`ctx.fillText\`, where there is no pseudo-element to read.
// Keys are \`.ms-*\` suffixes — what \`manaFontClass\` in \`oracleText.ts\` returns.

export const MANA_GLYPH: Record<string, string> = {
${tsMap(before)}
};

/** A split (hybrid) symbol's second half, drawn beside the first — mana-font's \`::after\`. */
export const MANA_GLYPH_AFTER: Record<string, string> = {
${tsMap(after)}
};
`;
}

const css = readFileSync(srcCss, "utf8");
const generated = extract(css);
const generatedTs = generateTs(css);
const check = process.argv.includes("--check");

const outputs = [
  { path: outCss, text: generated, stale: "styles/mana-oracle.css" },
  { path: outTs, text: generatedTs, stale: "app/domain/mana-glyphs.generated.ts" },
];

if (check) {
  for (const out of outputs) {
    let existing = "";
    try {
      existing = readFileSync(out.path, "utf8");
    } catch {
      console.error(`missing ${out.path} — run: just client-mana-oracle`);
      process.exit(1);
    }
    if (existing !== out.text) {
      console.error(`${out.stale} is stale vs mana-font — run: just client-mana-oracle`);
      process.exit(1);
    }
  }
  process.exit(0);
}

for (const out of outputs) {
  writeFileSync(out.path, out.text);
  console.log(`wrote ${out.path} (${out.text.length} bytes)`);
}
