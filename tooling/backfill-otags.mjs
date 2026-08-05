// Backfill Scryfall Tagger oracle-tag slugs onto the pool's card TOMLs.
//
// Deck-builder search matches thematic queries ("spirit", "tokens", "enchantment engine") via
// `search_blob`, which indexes these slugs. Pure catalog metadata — the engine never reads them.
//
// Join path: the TOML's own `id` (a Scryfall oracle_id) → oracle-tags bulk taggings.
//
// Idempotent + re-runnable: strips any top-level `otags = [...]` before re-inserting.
// Run from the repo root:  node tooling/backfill-otags.mjs
// Pass --only-missing to leave already-tagged cards alone (a freshly authored wave, without
// re-churning the rest of the pool against whatever Tagger has changed since).

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { gunzipSync } from "node:zlib";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");

const UA = { "User-Agent": "edh.reilley.dev/0.1", Accept: "application/json" };
const MAX_TAGS = 12;
const WEIGHT_RANK = { high: 0, median: 1, low: 2 };
const ONLY_MISSING = process.argv.includes("--only-missing");

async function fetchBulk(type) {
  const meta = await fetch(`https://api.scryfall.com/bulk-data/${type}`, { headers: UA });
  if (!meta.ok) throw new Error(`bulk meta ${type}: ${meta.status}`);
  // The tag bulks ship as JSONL only; the card bulks still offer a JSON array.
  const { download_uri, jsonl_download_uri } = await meta.json();
  const uri = download_uri ?? jsonl_download_uri;
  if (!uri) throw new Error(`bulk ${type}: no download uri`);
  const res = await fetch(uri, { headers: UA });
  if (!res.ok) throw new Error(`bulk download ${type}: ${res.status}`);
  if (download_uri) return res.json();
  // `.jsonl.gz` is served as a plain object, so fetch does not decompress it for us.
  const raw = Buffer.from(await res.arrayBuffer());
  const body = (uri.endsWith(".gz") ? gunzipSync(raw) : raw).toString("utf8");
  return body.split("\n").filter(Boolean).map((line) => JSON.parse(line));
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
const oracleIdOf = (text) => text.match(/^\s*id\s*=\s*"([^"]+)"/m)?.[1];

console.log("Fetching Scryfall oracle-tags bulk…");
const oracleTags = await fetchBulk("oracle-tags");

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

console.log(`Resolved otags for ${oracleToTags.size} oracle ids.`);

let done = 0;
let empty = 0;
let skipped = 0;
const missed = [];
for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
  const path = join(DATA_DIR, file);
  const text = readFileSync(path, "utf8");
  const oid = oracleIdOf(text);
  if (!oid) {
    missed.push(nameOf(text) ?? file);
    continue;
  }
  if (ONLY_MISSING && /^\s*otags\s*=/m.test(text)) {
    skipped++;
    continue;
  }
  const entries = oracleToTags.get(oid);
  if (!entries?.length) {
    empty++;
    backfillFile(path, []);
    continue;
  }
  backfillFile(path, topSlugs(entries));
  done++;
}
console.log(`Backfilled ${done} files with otags (${empty} cards had no tags).`);
if (skipped) console.log(`Left ${skipped} already-tagged files alone (--only-missing).`);
if (missed.length) console.log(`Skipped ${missed.length} (no oracle id): ${missed.join(", ")}`);
