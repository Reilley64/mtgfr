// Backfill Scryfall Tagger oracle-tag slugs onto the pool's card TOMLs.
//
// Deck-builder search matches thematic queries ("spirit", "tokens", "enchantment engine") via
// `search_blob`, which indexes these slugs. Pure catalog metadata — the engine never reads them.
//
// Join path: TOML name → card-ids.json → scryfall id → oracle-cards bulk oracle_id →
// oracle-tags bulk taggings.
//
// Idempotent + re-runnable: strips any top-level `otags = [...]` before re-inserting.
// Run from the repo root:  node tooling/backfill-otags.mjs

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const ID_MAP = join(ROOT, "client/src/lib/card-ids.json");

const UA = { "User-Agent": "mtgfr/0.1", Accept: "application/json" };
const MAX_TAGS = 12;
const WEIGHT_RANK = { high: 0, median: 1, low: 2 };

const nameToId = JSON.parse(readFileSync(ID_MAP, "utf8"));

async function fetchBulk(type) {
  const meta = await fetch(`https://api.scryfall.com/bulk-data/${type}`, { headers: UA });
  if (!meta.ok) throw new Error(`bulk meta ${type}: ${meta.status}`);
  const { download_uri } = await meta.json();
  const res = await fetch(download_uri, { headers: UA });
  if (!res.ok) throw new Error(`bulk download ${type}: ${res.status}`);
  return res.json();
}

function topSlugs(tagEntries) {
  return [...tagEntries]
    .sort((a, b) => (WEIGHT_RANK[a.weight] ?? 3) - (WEIGHT_RANK[b.weight] ?? 3) || a.slug.localeCompare(b.slug))
    .slice(0, MAX_TAGS)
    .map((t) => t.slug);
}

const tomlStr = (s) => `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
const tomlList = (xs) => `[${xs.map(tomlStr).join(", ")}]`;

function backfillFile(path, slugs) {
  const text = readFileSync(path, "utf8");
  const lines = text.split("\n");
  const firstTable = lines.findIndex((l) => /^\s*\[/.test(l));
  const cut = firstTable < 0 ? lines.length : firstTable;
  const nameIdx = lines.findIndex((l) => /^\s*name\s*=/.test(l));
  if (nameIdx < 0) throw new Error(`${path}: no top-level name key`);

  const kept = lines.filter((l, i) => !(i < cut && /^\s*otags\s*=/.test(l)));
  const nameLine = kept.findIndex((l) => /^\s*name\s*=/.test(l));

  if (slugs.length) kept.splice(nameLine + 1, 0, `otags = ${tomlList(slugs)}`);
  writeFileSync(path, kept.join("\n"));
}

const nameOf = (text) => text.match(/^\s*name\s*=\s*"((?:[^"\\]|\\.)*)"/m)?.[1];

console.log("Fetching Scryfall oracle-cards bulk…");
const oracleCards = await fetchBulk("oracle-cards");
console.log("Fetching Scryfall oracle-tags bulk…");
const oracleTags = await fetchBulk("oracle-tags");

const idToOracle = new Map(oracleCards.map((c) => [c.id, c.oracle_id]));

const oracleToTags = new Map();
for (const tag of oracleTags) {
  if (tag.type && tag.type !== "oracle") continue;
  const slug = tag.slug;
  if (!slug) continue;
  for (const t of tag.taggings ?? []) {
    if (!t.oracle_id) continue;
    if (!oracleToTags.has(t.oracle_id)) oracleToTags.set(t.oracle_id, []);
    oracleToTags.get(t.oracle_id).push({ slug, weight: t.weight ?? "low" });
  }
}

const tagsByName = new Map();
for (const [name, id] of Object.entries(nameToId)) {
  const oid = idToOracle.get(id);
  if (!oid) continue;
  const entries = oracleToTags.get(oid);
  if (entries?.length) tagsByName.set(name, topSlugs(entries));
}

console.log(`Resolved otags for ${tagsByName.size} cards.`);

let done = 0;
let empty = 0;
const missed = [];
for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
  const path = join(DATA_DIR, file);
  const name = nameOf(readFileSync(path, "utf8"));
  if (!name) {
    missed.push(file);
    continue;
  }
  const slugs = tagsByName.get(name);
  if (!slugs) {
    if (nameToId[name]) empty++;
    else missed.push(name);
    backfillFile(path, []);
    continue;
  }
  backfillFile(path, slugs);
  done++;
}
console.log(`Backfilled ${done} files with otags (${empty} cards had no tags).`);
if (missed.length) console.log(`Skipped ${missed.length} (no name/id match): ${missed.join(", ")}`);
