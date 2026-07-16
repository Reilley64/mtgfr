// Rewrite precon fixtures from {name,count} to {id,count,print} with SoC/precon prints.
// Prefer set:soc Printing when Scryfall has one; else CardDef.default_print.
//   node tooling/rewrite-precon-fixtures.mjs

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const FIX_DIR = join(ROOT, "crates/server/fixtures/decks");

function loadPool() {
  const byName = new Map();
  for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
    const text = readFileSync(join(DATA_DIR, file), "utf8");
    const name = text.match(/^\s*name\s*=\s*"((?:[^"\\]|\\.)*)"/m)?.[1];
    const id = text.match(/^\s*id\s*=\s*"([^"]+)"/m)?.[1];
    const print = text.match(/^\s*default_print\s*=\s*"([^"]+)"/m)?.[1];
    const set = text.match(/^\s*set\s*=\s*"([^"]+)"/m)?.[1] ?? "";
    if (name && id && print) byName.set(name, { id, print, set });
  }
  return byName;
}

async function socPrint(oracleId) {
  const q = encodeURIComponent(`oracleid:${oracleId} set:soc`);
  const res = await fetch(`https://api.scryfall.com/cards/search?q=${q}&unique=prints`, {
    headers: { Accept: "application/json", "User-Agent": "mtgfr/0.1" },
  });
  await new Promise((r) => setTimeout(r, 100));
  if (!res.ok) return null;
  const body = await res.json();
  return body.data?.[0]?.id ?? null;
}

const pool = loadPool();
const socCache = new Map(); // oracle id -> print uuid | null

async function printFor(name) {
  const meta = pool.get(name);
  if (!meta) throw new Error(`unknown card ${name}`);
  if (meta.set === "soc") return meta.print;
  if (!socCache.has(meta.id)) {
    socCache.set(meta.id, await socPrint(meta.id));
  }
  return socCache.get(meta.id) ?? meta.print;
}

for (const file of readdirSync(FIX_DIR).filter((f) => f.endsWith(".json"))) {
  const path = join(FIX_DIR, file);
  const deck = JSON.parse(readFileSync(path, "utf8"));
  const commanderMeta = pool.get(deck.commander);
  if (!commanderMeta) throw new Error(`${file}: unknown commander ${deck.commander}`);
  const commanderPrint = await printFor(deck.commander);
  const cards = [];
  for (const entry of deck.cards) {
    const meta = pool.get(entry.name);
    if (!meta) throw new Error(`${file}: unknown ${entry.name}`);
    cards.push({
      id: meta.id,
      count: entry.count,
      print: await printFor(entry.name),
    });
  }
  const out = {
    commander: commanderMeta.id,
    commander_print: commanderPrint,
    cards,
  };
  // Keep wire shape: commander is id string; print for commander stored how?
  // DeckDetail has commander: String (card id) and cards with print.
  // Commander print: need a field on deck OR include commander as a card line.
  // Plan: commander is Card id; commander print must live somewhere.
  // Options: (a) add commander_print to DeckDetail (b) store in a side map when resolving.
  // For fixtures/DeckDetail, add optional commander_print — or required.
  writeFileSync(
    path,
    JSON.stringify(
      {
        commander: commanderMeta.id,
        commander_print: commanderPrint,
        cards,
      },
      null,
      1,
    ) + "\n",
  );
  console.log(`rewrote ${file} (${cards.length} lines)`);
}
