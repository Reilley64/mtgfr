// Backfill printed (oracle) rules text onto the pool's card TOMLs from Scryfall.
//
// The deck builder's read-the-text hover wants a card's real printed text as a datum, not the
// engine's simplified `summary`. Scryfall is authoritative and we already hold every card's
// Scryfall id (client/src/lib/card-ids.json), so this resolves each card's `oracle_text` and
// writes it as a top-level TOML key:
//   - oracle = "At the beginning of your upkeep, …"
// A DFC/adventure joins its faces' text with " // " so both halves are readable.
//
// Idempotent + re-runnable: it strips any top-level `oracle` it previously wrote before
// re-inserting. Sibling to backfill-card-meta.mjs — same id-map + fuzzy-fallback approach. Run
// from the repo root:  node tooling/backfill-oracle.mjs

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const ID_MAP = join(ROOT, "client/src/lib/card-ids.json");

const nameToId = JSON.parse(readFileSync(ID_MAP, "utf8"));

// A card's full printed text: a single-faced card's `oracle_text`, or a DFC/adventure's faces
// joined. Empty string for a vanilla (no rules text) — nothing to write.
function oracleOf(card) {
  if (typeof card.oracle_text === "string") return card.oracle_text;
  if (Array.isArray(card.card_faces)) {
    return card.card_faces.map((f) => f.oracle_text).filter(Boolean).join(" // ");
  }
  return "";
}

// Batch every id through Scryfall's collection endpoint (75/request, needs a User-Agent), then
// key the result by our card-ids.json *name* (the name the TOMLs were authored against).
async function fetchOracle() {
  const ids = [...new Set(Object.values(nameToId))];
  const byId = new Map(); // scryfall id -> oracle text
  for (let i = 0; i < ids.length; i += 75) {
    const identifiers = ids.slice(i, i + 75).map((id) => ({ id }));
    const res = await fetch("https://api.scryfall.com/cards/collection", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json", "User-Agent": "mtgfr/0.1" },
      body: JSON.stringify({ identifiers }),
    });
    if (!res.ok) throw new Error(`Scryfall ${res.status}: ${await res.text()}`);
    const { data, not_found } = await res.json();
    for (const c of data) byId.set(c.id, oracleOf(c));
    if (not_found?.length) console.warn(`  ${not_found.length} ids not found in this batch`);
    await new Promise((r) => setTimeout(r, 100)); // be polite to Scryfall.
  }
  const byName = new Map();
  for (const [name, id] of Object.entries(nameToId)) {
    if (byId.has(id)) byName.set(name, byId.get(id));
  }
  return byName;
}

// A single-line TOML basic string: escape backslashes, quotes, and newlines (oracle text is
// multi-paragraph). Matches the single-line convention `approximates`/`set` already use.
const tomlStr = (s) =>
  `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\r?\n/g, "\\n")}"`;

const nameOf = (text) => text.match(/^\s*name\s*=\s*"((?:[^"\\]|\\.)*)"/m)?.[1];

function backfillFile(path, oracle) {
  const lines = readFileSync(path, "utf8").split("\n");
  const firstTable = lines.findIndex((l) => /^\s*\[/.test(l));
  const cut = firstTable < 0 ? lines.length : firstTable;
  // Drop any top-level `oracle` from a previous run (before the first table only).
  const kept = lines.filter((l, i) => !(i < cut && /^\s*oracle\s*=/.test(l)));
  const nameLine = kept.findIndex((l) => /^\s*name\s*=/.test(l));
  if (nameLine < 0) throw new Error(`${path}: no top-level name key`);
  kept.splice(nameLine + 1, 0, `oracle = ${tomlStr(oracle)}`);
  writeFileSync(path, kept.join("\n"));
}

// A name absent from the id map: resolve it live via Scryfall's fuzzy `named` endpoint.
async function resolveByName(name) {
  const res = await fetch(`https://api.scryfall.com/cards/named?fuzzy=${encodeURIComponent(name)}`, {
    headers: { Accept: "application/json", "User-Agent": "mtgfr/0.1" },
  });
  await new Promise((r) => setTimeout(r, 100));
  if (!res.ok) return null;
  return oracleOf(await res.json());
}

const oracle = await fetchOracle();
console.log(`Resolved ${oracle.size} cards from the id map.`);

let done = 0;
const missed = [];
const vanilla = [];
for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
  const path = join(DATA_DIR, file);
  const name = nameOf(readFileSync(path, "utf8"));
  let text = name && oracle.get(name);
  if (text == null && name) text = await resolveByName(name);
  if (text == null) {
    missed.push(name ?? file);
    continue;
  }
  if (!text) {
    vanilla.push(name); // no rules text — nothing to write.
    continue;
  }
  backfillFile(path, text);
  done++;
}
console.log(`Backfilled ${done} files.`);
if (vanilla.length) console.log(`Skipped ${vanilla.length} vanilla (no text): ${vanilla.join(", ")}`);
if (missed.length) console.log(`Skipped ${missed.length} (no Scryfall match): ${missed.join(", ")}`);
