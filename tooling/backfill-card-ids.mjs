// Backfill `id` (Scryfall oracle id) and `default_print` (Scryfall preferred print UUID)
// onto pool TOMLs. Preferred print = Scryfall `/cards/named` (or the card-ids.json seed).
// Precon fixtures stamp their own prints (SoC / Archidekt) separately — do not copy those
// into CardDef.default_print.
//
// Uses client/src/lib/card-ids.json (name → printing UUID) as the seed, then reads each card's
// `oracle_id` from Scryfall's collection endpoint. Idempotent: strips prior top-level id /
// default_print before re-inserting. Run from repo root:
//   node tooling/backfill-card-ids.mjs

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const ID_MAP = join(ROOT, "client/src/lib/card-ids.json");

const nameToPrint = JSON.parse(readFileSync(ID_MAP, "utf8"));

async function fetchOracleIds() {
  const prints = [...new Set(Object.values(nameToPrint))];
  const byPrint = new Map(); // print uuid -> { oracleId, printId }
  for (let i = 0; i < prints.length; i += 75) {
    const identifiers = prints.slice(i, i + 75).map((id) => ({ id }));
    const res = await fetch("https://api.scryfall.com/cards/collection", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        "User-Agent": "edh.reilley.dev/0.1",
      },
      body: JSON.stringify({ identifiers }),
    });
    if (!res.ok) throw new Error(`Scryfall ${res.status}: ${await res.text()}`);
    const { data, not_found } = await res.json();
    for (const c of data) {
      byPrint.set(c.id, { oracleId: c.oracle_id, printId: c.id });
    }
    if (not_found?.length) console.warn(`  ${not_found.length} ids not found in this batch`);
    await new Promise((r) => setTimeout(r, 100));
  }
  const byName = new Map();
  for (const [name, printId] of Object.entries(nameToPrint)) {
    const m = byPrint.get(printId);
    if (m) byName.set(name, m);
  }
  return byName;
}

async function resolveByName(name) {
  const res = await fetch(
    `https://api.scryfall.com/cards/named?fuzzy=${encodeURIComponent(name)}`,
    { headers: { Accept: "application/json", "User-Agent": "edh.reilley.dev/0.1" } },
  );
  await new Promise((r) => setTimeout(r, 100));
  if (!res.ok) return null;
  const c = await res.json();
  return { oracleId: c.oracle_id, printId: c.id };
}

const tomlStr = (s) => `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;

function backfillFile(path, meta) {
  const text = readFileSync(path, "utf8");
  const lines = text.split("\n");
  const firstTable = lines.findIndex((l) => /^\s*\[/.test(l));
  const cut = firstTable < 0 ? lines.length : firstTable;

  const kept = lines.filter(
    (l, i) => !(i < cut && /^\s*(id|default_print)\s*=/.test(l)),
  );
  const nameLine = kept.findIndex((l) => /^\s*name\s*=/.test(l));
  if (nameLine < 0) throw new Error(`${path}: no top-level name key`);

  kept.splice(
    nameLine + 1,
    0,
    `id = ${tomlStr(meta.oracleId)}`,
    `default_print = ${tomlStr(meta.printId)}`,
  );
  writeFileSync(path, kept.join("\n"));
}

const nameOf = (text) => text.match(/^\s*name\s*=\s*"((?:[^"\\]|\\.)*)"/m)?.[1];

const meta = await fetchOracleIds();
console.log(`Resolved ${meta.size} cards from the id map.`);

let done = 0;
const missed = [];
const newPrints = {};
for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
  const path = join(DATA_DIR, file);
  const name = nameOf(readFileSync(path, "utf8"));
  let m = name && meta.get(name);
  if (!m && name) {
    m = await resolveByName(name);
    if (m) {
      newPrints[name] = m.printId;
      meta.set(name, m);
    }
  }
  if (!m) {
    missed.push(name ?? file);
    continue;
  }
  backfillFile(path, m);
  done++;
}

if (Object.keys(newPrints).length) {
  const merged = { ...nameToPrint, ...newPrints };
  writeFileSync(ID_MAP, JSON.stringify(merged, Object.keys(merged).sort(), 2) + "\n");
  console.log(`Patched ${Object.keys(newPrints).length} new prints into card-ids.json.`);
}

console.log(`Backfilled id + default_print on ${done} TOMLs.`);
if (missed.length) {
  console.error(`Missed ${missed.length}:`, missed.slice(0, 20));
  process.exit(1);
}
