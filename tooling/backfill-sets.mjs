// Backfill Scryfall printing set codes onto the pool's card TOMLs.
//
// Coverage credits every set a card appears in (not just the oracle-cards representative
// `set`). Source: Scryfall `default_cards` bulk JSONL keyed by oracle `id` on each TOML.
//
// Idempotent + re-runnable: strips any top-level `set =` / `sets = [...]` before re-inserting.
// Inserts `sets = ["…"]` after `name =` (sorted unique lowercase). Run from repo root:
//   node tooling/backfill-sets.mjs

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");

const UA = { "User-Agent": "edh.reilley.dev/0.1", Accept: "application/json" };

const tomlStr = (s) => `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
const tomlList = (xs) => `[${xs.map(tomlStr).join(", ")}]`;

/** Lowercase, dedupe, and lexicographically sort set codes. */
export function uniqueSortedSets(codes) {
  return [...new Set(codes.map((c) => c.toLowerCase()))].sort();
}

/**
 * Rewrite top-level set metadata in a card TOML string.
 * Drops `set =` / `sets =` before the first table; inserts `sets = […]` after `name =`.
 */
export function rewriteTomlSets(text, sets) {
  const lines = text.split("\n");
  const firstTable = lines.findIndex((l) => /^\s*\[/.test(l));
  const cut = firstTable < 0 ? lines.length : firstTable;
  const nameIdx = lines.findIndex((l) => /^\s*name\s*=/.test(l));
  if (nameIdx < 0) throw new Error("no top-level name key");

  const kept = lines.filter(
    (l, i) => !(i < cut && (/^\s*set\s*=/.test(l) || /^\s*sets\s*=/.test(l))),
  );
  const nameLine = kept.findIndex((l) => /^\s*name\s*=/.test(l));

  const normalized = uniqueSortedSets(sets);
  if (normalized.length) kept.splice(nameLine + 1, 0, `sets = ${tomlList(normalized)}`);
  return kept.join("\n");
}

async function fetchOracleToSets() {
  const meta = await fetch("https://api.scryfall.com/bulk-data/default-cards", { headers: UA });
  if (!meta.ok) throw new Error(`bulk meta default-cards: ${meta.status}`);
  const { jsonl_download_uri } = await meta.json();
  if (!jsonl_download_uri) throw new Error("bulk meta default-cards: missing jsonl_download_uri");

  const res = await fetch(jsonl_download_uri, { headers: UA });
  if (!res.ok) throw new Error(`bulk download default-cards: ${res.status}`);

  const text = gunzipSync(Buffer.from(await res.arrayBuffer())).toString("utf8");
  const oracleToSets = new Map();

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    if (!line) continue;
    const card = JSON.parse(line);
    if (!card.oracle_id || typeof card.set !== "string" || !card.set) continue;
    if (!oracleToSets.has(card.oracle_id)) oracleToSets.set(card.oracle_id, new Set());
    oracleToSets.get(card.oracle_id).add(card.set.toLowerCase());
  }

  return oracleToSets;
}

const oracleIdOf = (text) => text.match(/^\s*id\s*=\s*"([^"]+)"/m)?.[1];

function backfillFile(path, sets) {
  const text = readFileSync(path, "utf8");
  writeFileSync(path, rewriteTomlSets(text, sets));
}

async function main() {
  console.log("Fetching Scryfall default-cards bulk…");
  const oracleToSets = await fetchOracleToSets();
  console.log(`Indexed ${oracleToSets.size} oracles from printings.`);

  let updated = 0;
  let empty = 0;
  const missing = [];

  for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
    const path = join(DATA_DIR, file);
    const text = readFileSync(path, "utf8");
    const oracleId = oracleIdOf(text);
    if (!oracleId) {
      missing.push(file);
      continue;
    }

    const setCodes = oracleToSets.get(oracleId);
    if (!setCodes) {
      missing.push(file);
      backfillFile(path, []);
      continue;
    }

    const sets = uniqueSortedSets([...setCodes]);
    backfillFile(path, sets);
    if (sets.length) updated++;
    else empty++;
  }

  console.log(`Backfilled ${updated} files with sets (${empty} oracles had no set codes).`);
  if (missing.length) console.log(`Missing oracle for ${missing.length} files: ${missing.join(", ")}`);
}

const isMain = import.meta.url === pathToFileURL(process.argv[1] ?? "").href;
if (isMain) {
  await main();
}
