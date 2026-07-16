#!/usr/bin/env node
// Regenerate `src/mana-oracle.css` from `mana-font`'s full stylesheet — pip/cost rules plus
// tray extras (multicolor duo + color indicators). Not the full ability glyph sheet.
// Usage:
//   node scripts/gen-mana-oracle.mjs          # write
//   node scripts/gen-mana-oracle.mjs --check  # fail if stale
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const srcCss = join(root, "node_modules/mana-font/css/mana.css");
const outCss = join(root, "src/mana-oracle.css");

const HEADER = `\
/* Subset of mana-font for oracle/approximates pips + mana-tray symbols (duo, color indicators).
 * Regenerate: \`just client-mana-oracle\` (or \`node scripts/gen-mana-oracle.mjs\`).
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

const generated = extract(readFileSync(srcCss, "utf8"));
const check = process.argv.includes("--check");

if (check) {
  let existing = "";
  try {
    existing = readFileSync(outCss, "utf8");
  } catch {
    console.error(`missing ${outCss} — run: just client-mana-oracle`);
    process.exit(1);
  }
  if (existing !== generated) {
    console.error("src/mana-oracle.css is stale vs mana-font — run: just client-mana-oracle");
    process.exit(1);
  }
  process.exit(0);
}

writeFileSync(outCss, generated);
console.log(`wrote ${outCss} (${generated.length} bytes)`);
