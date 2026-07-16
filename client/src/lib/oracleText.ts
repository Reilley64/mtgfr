// Split oracle / approximates prose into plain text and mana-font symbol parts.

export type OraclePart = { kind: "text"; text: string } | { kind: "symbol"; code: string; ms: string };

const KNOWN = new Set([
  "0",
  "1",
  "2",
  "3",
  "4",
  "5",
  "6",
  "7",
  "8",
  "9",
  "10",
  "11",
  "12",
  "13",
  "14",
  "15",
  "16",
  "17",
  "18",
  "19",
  "20",
  "100",
  "1000000",
  "w",
  "u",
  "b",
  "r",
  "g",
  "c",
  "x",
  "y",
  "z",
  "s",
  "e",
  "p",
  "h",
  "tap",
  "untap",
  "wu",
  "wb",
  "ub",
  "ur",
  "br",
  "bg",
  "rw",
  "rg",
  "gw",
  "gu",
  "2w",
  "2u",
  "2b",
  "2r",
  "2g",
  "cw",
  "cu",
  "cb",
  "cr",
  "cg",
  "wp",
  "up",
  "bp",
  "rp",
  "gp",
  "wup",
  "wbp",
  "ubp",
  "urp",
  "brp",
  "bgp",
  "rwp",
  "rgp",
  "gwp",
  "gup",
  "1-2",
]);

/** Mana-font `.ms-*` suffix for a brace code (`G`, `T`, `U/R`), or null if unknown. */
export function manaFontClass(code: string): string | null {
  if (code === "") return null;
  const upper = code.toUpperCase();
  if (upper === "T") return "tap";
  if (upper === "Q") return "untap";
  // `{1/2}` half-mana uses a hyphen in mana-font.
  if (upper === "1/2") return "1-2";
  const ms = upper.toLowerCase().replaceAll("/", "");
  if (KNOWN.has(ms)) return ms;
  // Hybrid pairs: wire/COLOR_PAIRS use WUBRG order (W/R); mana-font uses a fixed class (rw).
  if (ms.length === 2) {
    const rev = `${ms[1]}${ms[0]}`;
    if (KNOWN.has(rev)) return rev;
  }
  // Phyrexian hybrids: same letter-order issue before the trailing `p` (W/R/P → rwp).
  if (ms.length === 3 && ms.endsWith("p")) {
    const rev = `${ms[1]}${ms[0]}p`;
    if (KNOWN.has(rev)) return rev;
  }
  return null;
}

function pushText(parts: OraclePart[], text: string) {
  if (text === "") return;
  const prev = parts[parts.length - 1];
  if (prev?.kind === "text") {
    prev.text += text;
    return;
  }
  parts.push({ kind: "text", text });
}

/** Split prose on `{…}` mana/tap symbols; unknown braces stay in text runs. */
export function splitOracleText(text: string): OraclePart[] {
  const parts: OraclePart[] = [];
  const re = /\{([^}]+)\}/g;
  let last = 0;
  for (const match of text.matchAll(re)) {
    const start = match.index ?? 0;
    if (start > last) pushText(parts, text.slice(last, start));
    const code = match[1] ?? "";
    const ms = manaFontClass(code);
    if (ms) parts.push({ kind: "symbol", code, ms });
    else pushText(parts, match[0]);
    last = start + match[0].length;
  }
  if (last < text.length) pushText(parts, text.slice(last));
  return parts;
}
