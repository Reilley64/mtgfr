import { test } from "node:test";
import assert from "node:assert/strict";
import { rewriteTomlMeta, subtypesOf } from "./backfill-card-meta.mjs";

test("subtypesOf reads the front-face creature subtypes", () => {
  assert.deepEqual(subtypesOf("Legendary Creature — Human Wizard"), ["Human", "Wizard"]);
  assert.deepEqual(subtypesOf("Instant"), []);
  assert.deepEqual(subtypesOf("Creature — Faerie Rogue // Instant"), ["Faerie", "Rogue"]);
});

test("rewriteTomlMeta removes singular set and leaves sets alone", () => {
  const input = `name = "Goblin Test"
sets = ["cmd", "soc"]
id = "00000000-0000-0000-0000-000000000001"
set = "cmd"
subtypes = ["Goblin"]

[kind]
type = "creature"
power = 1
toughness = 1
`;
  const out = rewriteTomlMeta(input, {
    typeLine: "Creature — Goblin Wizard",
  });
  assert.match(out, /^sets = \["cmd", "soc"\]$/m);
  assert.doesNotMatch(out, /^\s*set\s*=/m);
  assert.match(out, /^subtypes = \["Goblin", "Wizard"\]$/m);
  assert.match(out, /\[kind\]/);
});
