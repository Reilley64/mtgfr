import { test } from "node:test";
import assert from "node:assert/strict";
import { uniqueSortedSets, rewriteTomlSets } from "./backfill-sets.mjs";

test("uniqueSortedSets lowercases, dedupes, sorts", () => {
  assert.deepEqual(uniqueSortedSets(["C16", "cmd", "cmd", "soc"]), ["c16", "cmd", "soc"]);
});

test("rewriteTomlSets replaces set/sets in the header block", () => {
  const input = `name = "Animar, Soul of Elements"
id = "725880b2-1675-414f-b61b-cf6533797dbf"
default_print = "cb073d5b-9515-492d-9b2d-0f64e85f1da8"
set = "cmd"
otags = ["cast-trigger-you"]

[cost]
green = 1
`;
  const out = rewriteTomlSets(input, ["c16", "cmd"]);
  assert.match(out, /sets = \["c16", "cmd"\]/);
  assert.doesNotMatch(out, /^\s*set\s*=/m);
  assert.match(out, /\[cost\]/);
});
