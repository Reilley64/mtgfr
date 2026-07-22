// Rewrite fidelity-grind precon fixtures from Archidekt print UUIDs.
// Unlike rewrite-precon-fixtures.mjs (SoC — prefers set:soc), this stamps each
// line's `print` from Archidekt `card.uid` so Commander 2011 / MO theme decks
// keep their precon art (cmd / td0). Pool TOMLs keep Scryfall's preferred
// default_print (`/cards/named`); this script does not touch TOMLs.
//
//   node tooling/rewrite-grind-precon-fixtures.mjs
//
// Requires network access to archidekt.com.

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const FIX_DIR = join(ROOT, "crates/server/fixtures/decks");

/** Archidekt face/name quirks → pool TOML `name`. */
const NAME_ALIASES = {
  "Jötun Grunt": "Jotun Grunt",
  "Nezumi Graverobber // Nighteyes the Desecrator": "Nezumi Graverobber",
};

const DECKS = [
  { file: "political_puppets.json", archidekt: 2209176 },
  { file: "enchantress_rubinia.json", archidekt: 2209180 },
  { file: "deathdancer_xira.json", archidekt: 2209179 },
  { file: "mirror_mastery.json", archidekt: 2209174 },
];

function loadPool() {
  const byName = new Map();
  for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
    const text = readFileSync(join(DATA_DIR, file), "utf8");
    const name = text.match(/^\s*name\s*=\s*"((?:[^"\\]|\\.)*)"/m)?.[1];
    const id = text.match(/^\s*id\s*=\s*"([^"]+)"/m)?.[1];
    if (name && id) byName.set(name, id);
  }
  return byName;
}

function isCommander(categories) {
  return (categories ?? []).some(
    (c) => c === "Commander" || (typeof c === "object" && c?.name === "Commander"),
  );
}

async function fetchDeck(id) {
  const res = await fetch(`https://archidekt.com/api/decks/${id}/`, {
    headers: { Accept: "application/json", "User-Agent": "mtgfr/0.1" },
  });
  if (!res.ok) throw new Error(`Archidekt ${id}: ${res.status}`);
  return res.json();
}

const pool = loadPool();

for (const { file, archidekt } of DECKS) {
  const deck = await fetchDeck(archidekt);
  let commander = null;
  let commanderPrint = null;
  const cards = [];
  const missing = [];

  for (const entry of deck.cards) {
    const card = entry.card;
    const archName = card.oracleCard?.name;
    if (!archName) continue;
    const poolName = NAME_ALIASES[archName] ?? archName;
    const id = pool.get(poolName);
    if (!id) {
      missing.push(archName);
      continue;
    }
    const print = card.uid;
    if (!print) throw new Error(`${file}: ${archName} missing card.uid`);
    if (isCommander(entry.categories)) {
      commander = id;
      commanderPrint = print;
      continue;
    }
    cards.push({ id, count: entry.quantity ?? 1, print });
  }

  if (missing.length) throw new Error(`${file}: missing pool cards: ${missing.join(", ")}`);
  if (!commander) throw new Error(`${file}: no commander category found`);

  const path = join(FIX_DIR, file);
  const old = JSON.parse(readFileSync(path, "utf8"));
  const rank = new Map(old.cards.map((c, i) => [c.id, i]));
  cards.sort(
    (a, b) => (rank.get(a.id) ?? 10_000) - (rank.get(b.id) ?? 10_000) || a.id.localeCompare(b.id),
  );

  writeFileSync(
    path,
    JSON.stringify({ commander, commander_print: commanderPrint, cards }, null, 1) + "\n",
  );
  console.log(`rewrote ${file} (${cards.length} lines, archidekt ${archidekt})`);
}
