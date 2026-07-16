// Cross-reference the mtgfr card pool with Scryfall oracle tags (otags) and emit an HTML report.
// Run from repo root: node tooling/analyze-otags.mjs

import { readFileSync, writeFileSync, readdirSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = join(ROOT, "crates/cards/data");
const ID_MAP = join(ROOT, "client/src/lib/card-ids.json");
const OUT = join(ROOT, "docs/reports/scryfall-otags-analysis.html");

const UA = { "User-Agent": "mtgfr/0.1 (otag analysis)", Accept: "application/json" };

// --- Scryfall bulk fetch helpers ---

async function fetchBulkMeta(type) {
  const res = await fetch(`https://api.scryfall.com/bulk-data/${type}`, { headers: UA });
  if (!res.ok) throw new Error(`bulk meta ${type}: ${res.status}`);
  return res.json();
}

async function downloadJson(url) {
  const res = await fetch(url, { headers: UA });
  if (!res.ok) throw new Error(`download ${url}: ${res.status}`);
  return res.json();
}

// --- Card pool parsing ---

function parseTomlCard(text, filename) {
  const name = text.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
  const approximates = text.match(/^approximates\s*=\s*"([^"]+)"/m)?.[1] ?? null;
  const subtypes = text.match(/^subtypes\s*=\s*\[([^\]]*)\]/m)?.[1];
  const subtypeList = subtypes
    ? [...subtypes.matchAll(/"([^"]+)"/g)].map((m) => m[1])
    : [];
  const hasAbilities = /\[\[abilities\]\]/.test(text);
  const isLand = /\[kind\]\s*\n[^\[]*type\s*=\s*"land"/m.test(text);
  return { filename, name, approximates, subtypes: subtypeList, hasAbilities, isLand };
}

function loadPool() {
  const nameToId = JSON.parse(readFileSync(ID_MAP, "utf8"));
  const cards = [];
  for (const f of readdirSync(DATA_DIR).filter((x) => x.endsWith(".toml"))) {
    const text = readFileSync(join(DATA_DIR, f), "utf8");
    const card = parseTomlCard(text, f);
    card.scryfallId = nameToId[card.name] ?? null;
    cards.push(card);
  }
  return { cards, nameToId };
}

// --- Engine-relevant otag taxonomy (hand-curated mapping to backlog increments) ---

const ENGINE_OTAG_CLUSTERS = {
  historic: {
    increment: "#67 spell-filter-extensions",
    engineHook: "SpellFilter::Historic",
    description: "Artifacts, legendaries, and Sagas — cast triggers and cost reducers",
    slugHints: [
      "historic", "synergy-historic", "cost-reducer-historic", "graveyard-fuel-historic",
      "impulse-historic", "cast-historic",
    ],
  },
  enchantment: {
    increment: "#67, #81",
    engineHook: "SpellFilter::Enchantment, watch-trigger enchantment-dies",
    description: "Enchantment spells, enchantment ETB/dies payoffs",
    slugHints: [
      "enchantment", "enchantment-matters", "enchantment-engine", "aura", "aura-matters",
      "enchant-creature", "cost-reducer-enchantment", "synergy-aura", "synergy-enchantment",
      "death-trigger",
    ],
  },
  tribal: {
    increment: "#64, #90",
    engineHook: "PermanentFilter.subtypes, AnthemStatic.subtypes",
    description: "Creature-type payoffs (Spirit, Pest, Goblin, etc.)",
    slugHints: [
      "typal-", "noncreature-typal", "spirit", "pest", "goblin", "elf", "zombie", "dragon",
      "angel", "beast", "squirrel", "cat", "dog", "bird", "wizard", "warrior", "cleric",
      "soldier", "elemental", "human", "merfolk", "vampire", "dinosaur", "artifact-creature",
    ],
  },
  token: {
    increment: "#64, #70, #88",
    engineHook: "Token profiles, PermanentFilter.token",
    description: "Token creation and token payoffs",
    slugHints: ["token", "create-token", "token-matters", "populate"],
  },
  sacrifice: {
    increment: "#78",
    engineHook: "SacrificeCost + PermanentFilter",
    description: "Sacrifice outlets and sacrifice payoffs",
    slugHints: ["sacrifice", "sacrifice-outlet", "sacrifice-payoff", "sacrifice-a-creature"],
  },
  graveyard: {
    increment: "#69",
    engineHook: "CardFilter graveyard, reanimate",
    description: "Graveyard recursion and graveyard-as-resource",
    slugHints: ["graveyard", "reanimate", "recursion", "mill", "self-mill", "flashback", "escape"],
  },
  counter: {
    increment: "#75",
    engineHook: "Counter kinds map, put_counters",
    description: "+1/+1 counters and named counter kinds",
    slugHints: ["counter", "plus-one-plus-one", "+1/+1", "proliferate", "charge-counter"],
  },
  combat: {
    increment: "#80, #88",
    engineHook: "combat.rs restrictions, attack triggers",
    description: "Combat restrictions, attack/block payoffs",
    slugHints: ["combat", "attack", "blocking", "menace", "flying", "trample", "double-strike"],
  },
  mana: {
    increment: "existing AddMana",
    engineHook: "AddMana, GrantManaAbility",
    description: "Ramp, mana dorks, treasure/food/clue",
    slugHints: ["ramp", "mana", "treasure", "food", "clue", "mana-dork"],
  },
  removal: {
    increment: "#66",
    engineHook: "destroy, exile, bounce TargetSpec filters",
    description: "Targeted and mass removal",
    slugHints: ["removal", "destroy", "exile", "bounce", "board-wipe"],
  },
  draw: {
    increment: "existing draw effects",
    engineHook: "draw, impulse",
    description: "Card draw and impulse",
    slugHints: ["draw", "card-draw", "impulse", "loot", "rummage"],
  },
  copy: {
    increment: "#83",
    engineHook: "SpellCopied, storm",
    description: "Spell/permanent copying",
    slugHints: ["copy", "storm", "replicate", "twincast"],
  },
  modified: {
    increment: "#90",
    engineHook: "PermanentFilter modified predicate",
    description: "Modified creature payoffs",
    slugHints: ["modified", "counter-on-creature"],
  },
  color: {
    increment: "#63",
    engineHook: "CardDef color, color filters",
    description: "Monocolored/multicolored payoffs",
    slugHints: ["monocolored", "multicolored", "color-matters"],
  },
  equipment: {
    increment: "#67",
    engineHook: "SpellFilter artifact subtypes",
    description: "Equipment and Vehicle matters",
    slugHints: ["equipment", "vehicle", "equip", "attach", "synergy-equipment", "synergy-vehicle"],
  },
};

const CASE_STUDIES = [
  {
    name: "Teshar, Ancestor's Apostle",
    file: "teshar_ancestors_apostle.toml",
    increment: "#67",
    otags: ["synergy-historic", "reanimate-creature", "cast-trigger-you"],
    gap: "Historic cast trigger dropped — no SpellFilter::Historic",
  },
  {
    name: "Starfield Mystic",
    file: "starfield_mystic.toml",
    increment: "#67, #81",
    otags: ["cost-reducer-enchantment", "death-trigger", "gains-pp-counters"],
    gap: "Reducer broadened to noncreature; enchantment-dies trigger dropped",
  },
  {
    name: "Sram, Senior Edificer",
    file: "sram_senior_edificer.toml",
    increment: "#67",
    otags: ["synergy-aura", "synergy-equipment", "synergy-vehicle", "cast-trigger-you"],
    gap: "Aura/Equipment/Vehicle cast trigger dropped entirely",
  },
  {
    name: "Vanguard of the Restless",
    file: "vanguard_of_the_restless.toml",
    increment: "#64, #90",
    otags: ["typal-spirit", "anthem", "creaturefall", "trigger-from-graveyard"],
    gap: "Spirit anthem + Spirit-ETB recursion both dropped/approximated",
  },
  {
    name: "Quintorius, Field Historian",
    file: "quintorius_field_historian.toml",
    increment: "#64",
    otags: ["typal-spirit", "anthem", "repeatable-creature-tokens"],
    gap: "Spirit anthem works; minted Spirit token lacks subtype/color",
  },
  {
    name: "Feral Appetite",
    file: "feral_appetite.toml",
    increment: "#64, #90",
    otags: ["typal-pest", "anthem", "attacking-matters"],
    gap: "Pest + attacking + deathtouch anthem inexpressible",
  },
];

function clusterForTag(slug, label) {
  const key = (slug + " " + label).toLowerCase();
  for (const [cluster, meta] of Object.entries(ENGINE_OTAG_CLUSTERS)) {
    if (meta.slugHints.some((h) => key.includes(h))) return cluster;
  }
  return null;
}

// --- Analysis ---

async function main() {
  console.log("Loading card pool…");
  const { cards } = loadPool();
  const withId = cards.filter((c) => c.scryfallId);
  const approxCards = cards.filter((c) => c.approximates);

  console.log("Fetching Scryfall oracle-cards bulk metadata…");
  const oracleMeta = await fetchBulkMeta("oracle-cards");
  console.log(`  Downloading ${(oracleMeta.size / 1e6).toFixed(1)} MB oracle cards…`);
  const oracleCards = await downloadJson(oracleMeta.download_uri);

  console.log("Fetching Scryfall oracle-tags bulk metadata…");
  const tagsMeta = await fetchBulkMeta("oracle-tags");
  console.log(`  Downloading ${(tagsMeta.size / 1e6).toFixed(1)} MB oracle tags…`);
  const oracleTags = await downloadJson(tagsMeta.download_uri);

  // id -> oracle_id
  const idToOracle = new Map();
  for (const c of oracleCards) idToOracle.set(c.id, c.oracle_id);

  // oracle_id -> card names in pool
  const oracleToPoolCards = new Map();
  for (const c of withId) {
    const oid = idToOracle.get(c.scryfallId);
    if (!oid) continue;
    c.oracleId = oid;
    if (!oracleToPoolCards.has(oid)) oracleToPoolCards.set(oid, []);
    oracleToPoolCards.get(oid).push(c);
  }

  // oracle_id -> tags[]
  const oracleToTags = new Map();
  const tagCatalog = [];
  for (const tag of oracleTags) {
    const cluster = clusterForTag(tag.slug ?? "", tag.label ?? "");
    const entry = {
      id: tag.id,
      slug: tag.slug,
      label: tag.label,
      description: tag.description ?? "",
      cluster,
      poolCount: 0,
      approxCount: 0,
      poolCards: [],
    };
    for (const t of tag.taggings ?? []) {
      if (!t.oracle_id) continue;
      if (!oracleToTags.has(t.oracle_id)) oracleToTags.set(t.oracle_id, []);
      oracleToTags.get(t.oracle_id).push({
        slug: tag.slug,
        label: tag.label,
        weight: t.weight,
        annotation: t.annotation ?? "",
        cluster,
      });
      const poolHits = oracleToPoolCards.get(t.oracle_id);
      if (poolHits?.length) {
        entry.poolCount += poolHits.length;
        for (const pc of poolHits) {
          if (!entry.poolCards.find((x) => x.name === pc.name)) {
            entry.poolCards.push({
              name: pc.name,
              approximates: pc.approximates,
              filename: pc.filename,
            });
            if (pc.approximates) entry.approxCount++;
          }
        }
      }
    }
    if (entry.poolCount > 0) tagCatalog.push(entry);
  }
  tagCatalog.sort((a, b) => b.poolCount - a.poolCount || a.label.localeCompare(b.label));

  // Per-card tag assignment
  for (const c of withId) {
    c.otags = oracleToTags.get(c.oracleId) ?? [];
    c.engineClusters = [...new Set(c.otags.map((t) => t.cluster).filter(Boolean))];
  }

  const tagged = withId.filter((c) => c.otags.length > 0);
  const untagged = withId.filter((c) => c.otags.length === 0);
  const approxTagged = approxCards.filter((c) => c.otags?.length > 0);

  // Cluster rollup
  const clusterRollup = {};
  for (const [name, meta] of Object.entries(ENGINE_OTAG_CLUSTERS)) {
    clusterRollup[name] = { ...meta, tags: [], poolCards: new Set(), approxCards: new Set() };
  }
  for (const tag of tagCatalog) {
    if (!tag.cluster) continue;
    const r = clusterRollup[tag.cluster];
    r.tags.push(tag);
    for (const pc of tag.poolCards) {
      r.poolCards.add(pc.name);
      if (pc.approximates) r.approxCards.add(pc.name);
    }
  }

  // High-value gaps: approx cards whose otags point at unimplemented engine clusters
  const gapCards = approxCards
    .map((c) => {
      const full = withId.find((x) => x.name === c.name);
      return {
        ...c,
        otags: full?.otags ?? [],
        clusters: full?.engineClusters ?? [],
      };
    })
    .filter((c) => c.otags.length > 0)
    .sort((a, b) => b.clusters.length - a.clusters.length || a.name.localeCompare(b.name));

  const stats = {
    poolTotal: cards.length,
    withScryfallId: withId.length,
    withOtags: tagged.length,
    withoutOtags: untagged.length,
    approxTotal: approxCards.length,
    approxWithOtags: approxTagged.length,
    uniqueTagsHittingPool: tagCatalog.length,
    totalTagAssignments: tagged.reduce((n, c) => n + c.otags.length, 0),
    oracleTagsTotal: oracleTags.length,
    generatedAt: new Date().toISOString(),
  };

  console.log("Writing HTML report…");
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, renderHtml({ stats, tagCatalog, clusterRollup, gapCards, untagged, cards: withId }));
  console.log(`Wrote ${OUT}`);
}

function esc(s) {
  return String(s ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderHtml({ stats, tagCatalog, clusterRollup, gapCards, untagged, cards }) {
  const topTags = tagCatalog.slice(0, 40);
  const clusterRows = Object.entries(clusterRollup)
    .map(([k, v]) => ({
      key: k,
      ...v,
      poolSize: v.poolCards.size,
      approxSize: v.approxCards.size,
      tagCount: v.tags.length,
    }))
    .filter((r) => r.poolSize > 0)
    .sort((a, b) => b.approxSize - a.approxSize || b.poolSize - a.poolSize);

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Scryfall Oracle Tags × mtgfr — Analysis Report</title>
<style>
  :root {
    --bg: #0f1419;
    --surface: #1a2332;
    --surface2: #243044;
    --text: #e8edf4;
    --muted: #8b9cb3;
    --accent: #5b9fd4;
    --accent2: #c9a227;
    --danger: #e07a6a;
    --ok: #6bc98a;
    --border: #2d3a4f;
    --mono: "JetBrains Mono", "Fira Code", ui-monospace, monospace;
    --sans: "IBM Plex Sans", system-ui, sans-serif;
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; padding: 2rem clamp(1rem, 4vw, 3rem) 4rem;
    font-family: var(--sans); background: var(--bg); color: var(--text);
    line-height: 1.55; font-size: 15px;
  }
  h1 { font-size: 1.75rem; font-weight: 600; margin: 0 0 0.25rem; letter-spacing: -0.02em; }
  h2 { font-size: 1.25rem; margin: 2.5rem 0 0.75rem; color: var(--accent); font-weight: 600; }
  h3 { font-size: 1rem; margin: 1.5rem 0 0.5rem; color: var(--muted); font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; font-size: 0.8rem; }
  p, li { color: var(--text); }
  .subtitle { color: var(--muted); margin-bottom: 2rem; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 1rem; margin: 1.5rem 0; }
  .stat {
    background: var(--surface); border: 1px solid var(--border); border-radius: 10px;
    padding: 1rem 1.1rem;
  }
  .stat .n { font-size: 1.75rem; font-weight: 700; color: var(--accent2); font-variant-numeric: tabular-nums; }
  .stat .l { font-size: 0.8rem; color: var(--muted); margin-top: 0.2rem; }
  .callout {
    background: var(--surface); border-left: 3px solid var(--accent);
    padding: 1rem 1.25rem; margin: 1.25rem 0; border-radius: 0 8px 8px 0;
  }
  .callout.warn { border-left-color: var(--danger); }
  .callout.ok { border-left-color: var(--ok); }
  table { width: 100%; border-collapse: collapse; font-size: 0.88rem; margin: 0.75rem 0 1.5rem; }
  th, td { text-align: left; padding: 0.55rem 0.65rem; border-bottom: 1px solid var(--border); vertical-align: top; }
  th { color: var(--muted); font-weight: 600; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.03em; }
  tr:hover td { background: var(--surface2); }
  code, .mono { font-family: var(--mono); font-size: 0.85em; background: var(--surface2); padding: 0.1em 0.35em; border-radius: 4px; }
  .tag { display: inline-block; background: var(--surface2); border: 1px solid var(--border);
    border-radius: 999px; padding: 0.15em 0.55em; margin: 0.1em 0.15em 0.1em 0; font-size: 0.78rem; }
  .tag.cluster { border-color: var(--accent); color: var(--accent); }
  .tag.gap { border-color: var(--danger); color: var(--danger); }
  .bar-wrap { background: var(--surface2); border-radius: 4px; height: 6px; overflow: hidden; min-width: 60px; }
  .bar { height: 100%; background: var(--accent); border-radius: 4px; }
  .bar.warn { background: var(--danger); }
  details { margin: 0.5rem 0; background: var(--surface); border: 1px solid var(--border); border-radius: 8px; }
  summary { cursor: pointer; padding: 0.65rem 1rem; font-weight: 500; }
  details > div { padding: 0 1rem 1rem; }
  .rec { counter-reset: rec; list-style: none; padding: 0; }
  .rec li { counter-increment: rec; padding: 0.75rem 0 0.75rem 2.5rem; position: relative; border-bottom: 1px solid var(--border); }
  .rec li::before {
    content: counter(rec); position: absolute; left: 0; top: 0.75rem;
    width: 1.6rem; height: 1.6rem; background: var(--accent); color: var(--bg);
    border-radius: 50%; display: flex; align-items: center; justify-content: center;
    font-size: 0.75rem; font-weight: 700;
  }
  a { color: var(--accent); }
  .mermaid { background: var(--surface); padding: 1rem; border-radius: 8px; border: 1px solid var(--border); overflow-x: auto; }
  footer { margin-top: 3rem; padding-top: 1rem; border-top: 1px solid var(--border); color: var(--muted); font-size: 0.8rem; }
</style>
</head>
<body>

<h1>Scryfall Oracle Tags × mtgfr</h1>
<p class="subtitle">How community-maintained <code>otag:</code> labels from <a href="https://scryfall.com/docs/tagger-tags">Scryfall Tagger</a> map to the card pool and engine backlog. Generated ${esc(stats.generatedAt)}.</p>

<div class="grid">
  <div class="stat"><div class="n">${stats.poolTotal}</div><div class="l">Cards in pool</div></div>
  <div class="stat"><div class="n">${stats.withOtags}</div><div class="l">Pool cards with ≥1 otag (${Math.round(100 * stats.withOtags / stats.withScryfallId)}%)</div></div>
  <div class="stat"><div class="n">${stats.uniqueTagsHittingPool}</div><div class="l">Distinct otags touching pool</div></div>
  <div class="stat"><div class="n">${stats.approxTotal}</div><div class="l">Cards with <code>approximates</code></div></div>
  <div class="stat"><div class="n">${stats.approxWithOtags}</div><div class="l">Approx cards also tagged (${Math.round(100 * stats.approxWithOtags / stats.approxTotal)}%)</div></div>
  <div class="stat"><div class="n">${stats.oracleTagsTotal.toLocaleString()}</div><div class="l">Total Scryfall oracle tags</div></div>
</div>

<h2>Executive summary</h2>

<div class="callout ok">
  <strong>Oracle tags are a useful <em>authoring and prioritization</em> layer, not a runtime dependency.</strong>
  They cluster cards by mechanical theme (Historic, enchantment-matters, tribal, sacrifice) in ways that align with
  <code>FIDELITY_BACKLOG.md</code> increments #63–#90 — but they do <em>not</em> replace the explicit card DSL.
  The project's ADR 0002 posture (grow filters from real cards) still applies: import otags as <strong>catalog hints</strong>
  and <strong>gap detectors</strong>, not as executable rules.
</div>

<div class="callout warn">
  <strong>${stats.withoutOtags} pool cards have no oracle tags</strong>${stats.withoutOtags > 0 ? " (mostly basics, vanilla creatures, and simple lands)." : " — full Scryfall ID coverage in this pool, but tag <em>precision</em> varies: e.g. only 1 card carries <code>synergy-historic</code> while Sram's historic-adjacent payoff is tagged <code>synergy-aura</code>."}
  Tag coverage is community-driven and thematically inconsistent — never gate engine work on tag presence alone.
</div>

<h2>Recommended uses</h2>
<ol class="rec">
  <li><strong>Backlog triage.</strong> When landing <code>SpellFilter::Historic</code> (#67), query otag <code>historic</code> to find all pool cards that care — not just the 13 listed in the increment table.</li>
  <li><strong>Approximate-card clustering.</strong> ${stats.approxWithOtags} of ${stats.approxTotal} <code>approximates</code> cards carry otags; group them by shared tag to spot batch authoring wins (e.g. all Spirit-tribal anthems).</li>
  <li><strong>Deck-builder search (ADR 0017 extension).</strong> Store top-weight otags on <code>CatalogCard</code> for "enchantment matters" / "tokens" browse filters — orthogonal to <code>subtypes</code>.</li>
  <li><strong>CI validation tooling.</strong> A card tagged <code>historic-matters</code> without a <code>cast_spell</code> trigger using <code>historic</code> filter → lint warning.</li>
  <li><strong>Do not:</strong> auto-generate <code>[[abilities]]</code> from otags; import the full ${stats.oracleTagsTotal.toLocaleString()}-tag ontology; or treat tag weights as rules priority.</li>
</ol>

<h2>Case studies: otags → engine gaps</h2>
<p>Six high-signal cards where Scryfall otags name the exact mechanical themes blocked by current engine limits.</p>
<table>
  <thead><tr><th>Card</th><th>Key otags</th><th>Backlog</th><th>Gap</th></tr></thead>
  <tbody>
${CASE_STUDIES.map((c) => `    <tr>
      <td><strong>${esc(c.name)}</strong><br><code>${esc(c.file)}</code></td>
      <td>${c.otags.map((t) => `<span class="tag">${esc(t)}</span>`).join(" ")}</td>
      <td><code>${esc(c.increment)}</code></td>
      <td style="font-size:0.85rem">${esc(c.gap)}</td>
    </tr>`).join("\n")}
  </tbody>
</table>

<h2>Engine cluster rollup</h2>
<p>Hand-mapped clusters linking otag themes → engine hooks → backlog increments. Sorted by approx-card count (highest leverage first).</p>

<table>
  <thead><tr>
    <th>Cluster</th><th>Backlog</th><th>Engine hook</th><th>Pool cards</th><th>Approx</th><th>Otags</th>
  </tr></thead>
  <tbody>
${clusterRows.map((r) => {
  const pct = r.poolSize ? Math.round(100 * r.approxSize / r.poolSize) : 0;
  return `    <tr>
      <td><strong>${esc(r.key)}</strong><br><span style="color:var(--muted);font-size:0.82rem">${esc(r.description)}</span></td>
      <td><code>${esc(r.increment)}</code></td>
      <td><code>${esc(r.engineHook)}</code></td>
      <td>${r.poolSize}</td>
      <td>${r.approxSize} <div class="bar-wrap"><div class="bar${pct > 40 ? " warn" : ""}" style="width:${pct}%"></div></div></td>
      <td>${r.tagCount}</td>
    </tr>`;
}).join("\n")}
  </tbody>
</table>

<h2>Top oracle tags in the pool</h2>
<p>Most frequent otags among the ${stats.withScryfallId} Scryfall-mapped cards (by pool card hits).</p>

<table>
  <thead><tr><th>Tag</th><th>Cluster</th><th>Pool</th><th>Approx</th><th>Example cards</th></tr></thead>
  <tbody>
${topTags.map((t) => `    <tr>
      <td><a href="https://scryfall.com/search?q=otag:${esc(t.slug)}">${esc(t.label)}</a><br><code>${esc(t.slug)}</code></td>
      <td>${t.cluster ? `<span class="tag cluster">${esc(t.cluster)}</span>` : "—"}</td>
      <td>${t.poolCount}</td>
      <td>${t.approxCount}</td>
      <td>${t.poolCards.slice(0, 4).map((c) => esc(c.name)).join(", ")}${t.poolCards.length > 4 ? "…" : ""}</td>
    </tr>`).join("\n")}
  </tbody>
</table>

<h2>Approximate cards × otags (gap radar)</h2>
<p>Cards with fidelity gaps whose Scryfall otags point at missing engine capabilities — prime batch-unblock candidates.</p>

<table>
  <thead><tr><th>Card</th><th>Engine clusters</th><th>Top otags</th><th>Approximates</th></tr></thead>
  <tbody>
${gapCards.slice(0, 60).map((c) => `    <tr>
      <td><strong>${esc(c.name)}</strong><br><code>${esc(c.filename)}</code></td>
      <td>${c.clusters.map((x) => `<span class="tag cluster">${esc(x)}</span>`).join(" ") || "—"}</td>
      <td>${c.otags.slice(0, 5).map((t) => `<span class="tag">${esc(t.label)}</span>`).join(" ")}</td>
      <td style="font-size:0.82rem;color:var(--muted)">${esc(c.approximates?.slice(0, 120))}${(c.approximates?.length ?? 0) > 120 ? "…" : ""}</td>
    </tr>`).join("\n")}
  </tbody>
</table>
${gapCards.length > 60 ? `<p><em>Showing 60 of ${gapCards.length} tagged approximate cards.</em></p>` : ""}

<h2>Architecture: where otags fit</h2>
<div class="mermaid">
flowchart LR
  subgraph scryfall [Scryfall Tagger]
    OT[Oracle Tags bulk]
    OC[Oracle Cards bulk]
  end
  subgraph tooling [mtgfr tooling]
    BF[backfill-card-meta.mjs]
    OTAG[analyze-otags.mjs]
    LINT[future: otag-lint.mjs]
  end
  subgraph data [Card data]
    TOML["crates/cards/data/*.toml"]
    IDS[card-ids.json]
  end
  subgraph engine [Engine - unchanged runtime]
    DSL[CardDef + abilities]
    FILT[SpellFilter / PermanentFilter]
  end
  subgraph catalog [Catalog / deck builder]
    CAT[CatalogCard]
    PG[(Postgres catalog_cards)]
  end
  OT --> OTAG
  OC --> OTAG
  IDS --> OTAG
  IDS --> BF
  BF --> TOML
  OTAG -->|hints + report| TOML
  OTAG -->|proposed otags field| CAT
  TOML --> DSL
  TOML --> CAT
  CAT --> PG
  LINT --> TOML
  OTAG -.->|cluster → increment map| LINT
</div>

<h2>Proposed schema extension</h2>
<div class="callout">
  Add optional catalog-only field to <code>CardDef</code> / <code>CatalogCard</code>:
  <pre style="margin:0.5rem 0 0;overflow-x:auto"><code># tooling/backfill-otags.mjs — catalog only, engine ignores
otags = ["historic-matters", "recursion", "flying"]

# Store weight for deck-builder ranking (optional)
# otag_weights = { "historic-matters" = "high", "flying" = "medium" }</code></pre>
  Join path: <code>name → card-ids.json → scryfall id → oracle_id → oracle-tags bulk taggings</code>.
  Re-run nightly or on pool changes; same idempotency pattern as <code>backfill-oracle.mjs</code>.
</div>

<h2>Untagged pool cards</h2>
<details>
  <summary>${untagged.length} cards with Scryfall IDs but no oracle tags</summary>
  <div><p>${untagged.map((c) => esc(c.name)).join(", ")}</p></div>
</details>

<h2>Concrete next steps</h2>
<table>
  <thead><tr><th>Step</th><th>Effort</th><th>Impact</th></tr></thead>
  <tbody>
    <tr><td><code>tooling/backfill-otags.mjs</code> — write top-N weighted otags to TOML</td><td>S</td><td>Deck-builder thematic search; authoring reference</td></tr>
    <tr><td>Otag-aware fidelity linter — flag tag/engine mismatches</td><td>M</td><td>Catch dropped triggers before playtesting</td></tr>
    <tr><td>Map otag clusters → backlog increment IDs in tooling</td><td>S</td><td>Auto-generate increment card lists for validation</td></tr>
    <tr><td>Land <code>SpellFilter::Historic</code> (#67) using otag-derived card set as test matrix</td><td>S (engine) + S (tooling)</td><td>Unblocks Teshar, Sram, Starfield Mystic batch</td></tr>
    <tr><td>Do <em>not</em> add otags to engine <code>Game</code> state</td><td>—</td><td>Preserves determinism and ADR 0002</td></tr>
  </tbody>
</table>

<footer>
  Data sources: <a href="https://api.scryfall.com/bulk-data/oracle-tags">Scryfall oracle-tags bulk</a>,
  <a href="https://api.scryfall.com/bulk-data/oracle-cards">oracle-cards bulk</a>.
  Pool: <code>crates/cards/data/</code> (${stats.poolTotal} TOMLs).
  Script: <code>tooling/analyze-otags.mjs</code>.
</footer>

<script type="module">
  import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs";
  mermaid.initialize({ startOnLoad: true, theme: "dark", themeVariables: { primaryColor: "#5b9fd4", lineColor: "#8b9cb3", textColor: "#e8edf4" } });
</script>
</body>
</html>`;
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
