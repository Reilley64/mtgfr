// Backfill `set` and `subtypes` onto the pool's card TOMLs from Scryfall.
//
// Two of the deck-builder's five search dimensions (set, creature subtype) exist nowhere in the
// card model, and set codes appear only in unreliable TOML comments. Scryfall is authoritative and
// we already hold every card's Scryfall id (client/src/lib/card-ids.json, from ADR 0015), so this
// resolves each card's `set` and `type_line`, then writes them as top-level TOML keys:
//   - `set   = "soc"`            for every matched card
//   - `subtypes = ["Goblin"]`   for non-lands (a land's types already live under [kind])
//
// Idempotent + re-runnable: it strips any top-level `set`/`subtypes` it previously wrote before
// re-inserting, and leaves land `[kind].subtypes` untouched. Run from the repo root:
//   node tooling/backfill-card-meta.mjs
//
// Supersedes the throwaway id-map resolver ADR 0015 flagged; commit this as real tooling.

import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const ID_MAP = join(ROOT, "client/src/lib/card-ids.json");

const nameToId = JSON.parse(readFileSync(ID_MAP, "utf8"));

// Batch every id through Scryfall's collection endpoint (75/request, needs a User-Agent), then
// key the result by our card-ids.json *name* — the name the TOMLs were authored against — rather
// than Scryfall's returned name, which for a DFC/adventure is "Front // Back" and wouldn't match.
async function fetchMeta() {
  const ids = [...new Set(Object.values(nameToId))];
  const byId = new Map(); // scryfall id -> { set, typeLine }
  for (let i = 0; i < ids.length; i += 75) {
    const identifiers = ids.slice(i, i + 75).map((id) => ({ id }));
    const res = await fetch("https://api.scryfall.com/cards/collection", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json", "User-Agent": "edh.reilley.dev/0.1" },
      body: JSON.stringify({ identifiers }),
    });
    if (!res.ok) throw new Error(`Scryfall ${res.status}: ${await res.text()}`);
    const { data, not_found } = await res.json();
    for (const c of data) byId.set(c.id, { set: c.set, typeLine: c.type_line ?? "" });
    if (not_found?.length) console.warn(`  ${not_found.length} ids not found in this batch`);
    await new Promise((r) => setTimeout(r, 100)); // be polite to Scryfall.
  }
  const byName = new Map();
  for (const [name, id] of Object.entries(nameToId)) {
    const m = byId.get(id);
    if (m) byName.set(name, m);
  }
  return byName;
}

// Subtypes are the segment after the "—" of the front face's type line.
// "Legendary Creature — Human Wizard" -> ["Human","Wizard"]; "Instant" -> [].
function subtypesOf(typeLine) {
  const front = typeLine.split("//")[0];
  const dash = front.indexOf("—");
  if (dash < 0) return [];
  return front.slice(dash + 1).trim().split(/\s+/).filter(Boolean);
}

const isLand = (typeLine) => /\bLand\b/.test(typeLine.split("//")[0]);

const tomlStr = (s) => `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
const tomlList = (xs) => `[${xs.map(tomlStr).join(", ")}]`;

function backfillFile(path, meta) {
  const text = readFileSync(path, "utf8");
  const lines = text.split("\n");
  const firstTable = lines.findIndex((l) => /^\s*\[/.test(l));
  const cut = firstTable < 0 ? lines.length : firstTable;
  const nameIdx = lines.findIndex((l) => /^\s*name\s*=/.test(l));
  if (nameIdx < 0) throw new Error(`${path}: no top-level name key`);

  // Drop any top-level set/subtypes we wrote on a previous run (before the first table only,
  // so a land's [kind].subtypes is never touched).
  const kept = lines.filter((l, i) => !(i < cut && /^\s*(set|subtypes)\s*=/.test(l)));
  const nameLine = kept.findIndex((l) => /^\s*name\s*=/.test(l));

  const inserts = [`set = ${tomlStr(meta.set)}`];
  const subs = subtypesOf(meta.typeLine);
  if (!isLand(meta.typeLine) && subs.length) inserts.push(`subtypes = ${tomlList(subs)}`);

  kept.splice(nameLine + 1, 0, ...inserts);
  writeFileSync(path, kept.join("\n"));
}

const nameOf = (text) => text.match(/^\s*name\s*=\s*"((?:[^"\\]|\\.)*)"/m)?.[1];

// A name absent from the (occasionally stale) id map: resolve it live via Scryfall's fuzzy
// `named` endpoint, and hand back its id so we can also patch the image map. Returns null on miss.
async function resolveByName(name) {
  const res = await fetch(`https://api.scryfall.com/cards/named?fuzzy=${encodeURIComponent(name)}`, {
    headers: { Accept: "application/json", "User-Agent": "edh.reilley.dev/0.1" },
  });
  await new Promise((r) => setTimeout(r, 100));
  if (!res.ok) return null;
  const c = await res.json();
  return { id: c.id, set: c.set, typeLine: c.type_line ?? "" };
}

const meta = await fetchMeta();
console.log(`Resolved ${meta.size} cards from the id map.`);

let done = 0;
const missed = [];
const newIds = {}; // name -> id, for stragglers resolved live; merged back into card-ids.json.
for (const file of readdirSync(DATA_DIR).filter((f) => f.endsWith(".toml"))) {
  const path = join(DATA_DIR, file);
  const name = nameOf(readFileSync(path, "utf8"));
  let m = name && meta.get(name);
  if (!m && name) {
    m = await resolveByName(name);
    if (m) newIds[name] = m.id;
  }
  if (!m) {
    missed.push(name ?? file);
    continue;
  }
  backfillFile(path, m);
  done++;
}
console.log(`Backfilled ${done} files.`);

// Keep the image map in sync: fold any live-resolved ids into card-ids.json (sorted, stable).
if (Object.keys(newIds).length) {
  const merged = { ...nameToId, ...newIds };
  const sorted = Object.fromEntries(Object.keys(merged).sort().map((k) => [k, merged[k]]));
  writeFileSync(ID_MAP, JSON.stringify(sorted, null, 2) + "\n");
  console.log(`Patched ${Object.keys(newIds).length} ids into card-ids.json: ${Object.keys(newIds).join(", ")}`);
}
if (missed.length) console.log(`Skipped ${missed.length} (no Scryfall match): ${missed.join(", ")}`);
