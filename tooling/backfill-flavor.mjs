// Backfill printed flavor text onto the pool's card TOMLs from Scryfall.
//
// A rendered card face sets its flavor text under the rules text, in italics below the divider,
// so the face needs the words as a datum. Flavor is per-printing, and each TOML already names the
// printing the client shows art for — `default_print` — so the words match the art.
//
// Join path: the TOML's own `default_print` (a Scryfall card id) → that printing's `flavor_text`.
//
// Idempotent + re-runnable: strips any top-level `flavor` before re-inserting.
// Run from the repo root:  node tooling/backfill-flavor.mjs

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const UA = { "User-Agent": "edh.reilley.dev/0.1", Accept: "application/json" };

// The front face's flavor: a card prints one block per face, and a face renders its own. Empty
// string when the printing has none — plenty of cards print no flavor at all.
function flavorOf(card) {
  if (typeof card.flavor_text === "string") return card.flavor_text;
  if (Array.isArray(card.card_faces)) return card.card_faces[0]?.flavor_text ?? "";
  return "";
}

// Batch the printings through Scryfall's collection endpoint (75/request, needs a User-Agent).
async function fetchFlavor(ids) {
  const byId = new Map(); // scryfall card id -> flavor text
  for (let i = 0; i < ids.length; i += 75) {
    const identifiers = ids.slice(i, i + 75).map((id) => ({ id }));
    const res = await fetch("https://api.scryfall.com/cards/collection", {
      method: "POST",
      headers: { "Content-Type": "application/json", ...UA },
      body: JSON.stringify({ identifiers }),
    });
    if (!res.ok) throw new Error(`Scryfall ${res.status}: ${await res.text()}`);
    const { data, not_found } = await res.json();
    for (const c of data) byId.set(c.id, flavorOf(c));
    if (not_found?.length) console.warn(`  ${not_found.length} ids not found in this batch`);
    await new Promise((r) => setTimeout(r, 100)); // be polite to Scryfall.
  }
  return byId;
}

// A single-line TOML basic string: escape backslashes, quotes, and newlines (flavor runs to
// several lines, and an attribution sits on its own). Matches what `oracle` already does.
const tomlStr = (s) =>
  `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\r?\n/g, "\\n")}"`;

const nameOf = (text) => text.match(/^\s*name\s*=\s*"((?:[^"\\]|\\.)*)"/m)?.[1];
const printOf = (text) => text.match(/^\s*default_print\s*=\s*"([^"]+)"/m)?.[1];

function backfillFile(path, flavor) {
  const lines = readFileSync(path, "utf8").split("\n");
  const firstTable = lines.findIndex((l) => /^\s*\[/.test(l));
  const cut = firstTable < 0 ? lines.length : firstTable;
  // Drop any top-level `flavor` from a previous run (before the first table only).
  const kept = lines.filter((l, i) => !(i < cut && /^\s*flavor\s*=/.test(l)));
  // Under `oracle` when the card has one, so the two printed-text keys sit together.
  const oracleLine = kept.findIndex((l) => /^\s*oracle\s*=/.test(l));
  const anchor = oracleLine < 0 ? kept.findIndex((l) => /^\s*name\s*=/.test(l)) : oracleLine;
  if (anchor < 0) throw new Error(`${path}: no top-level name key`);
  if (flavor) kept.splice(anchor + 1, 0, `flavor = ${tomlStr(flavor)}`);
  writeFileSync(path, kept.join("\n"));
}

const files = readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"));
const prints = new Map(); // file -> default_print
for (const file of files) {
  const print = printOf(readFileSync(join(DATA_DIR, file), "utf8"));
  if (print) prints.set(file, print);
}

console.log(`Fetching flavor for ${prints.size} printings…`);
const flavor = await fetchFlavor([...new Set(prints.values())]);

let done = 0;
let plain = 0;
const missed = [];
for (const file of files) {
  const path = join(DATA_DIR, file);
  const print = prints.get(file);
  const text = print && flavor.get(print);
  if (text == null) {
    missed.push(nameOf(readFileSync(path, "utf8")) ?? file);
    continue;
  }
  backfillFile(path, text);
  if (text) done++;
  else plain++;
}
console.log(`Backfilled ${done} files.`);
if (plain) console.log(`${plain} printings print no flavor.`);
if (missed.length) console.log(`Skipped ${missed.length} (no Scryfall match): ${missed.join(", ")}`);
