import { expect, test } from "vitest";
import { buildAnswerFromDraft, choiceDraftKey, choiceIntent, type PromptDraft } from "~/choice";

test("choiceIntent maps discard answer", () => {
  const pc = { kind: "discard" as const, count: 2, items: [], player: 0 };
  expect(choiceIntent(pc, { kind: "discard", cards: [3, 7] })).toEqual({
    kind: "discard",
    player: 0,
    cards: [3, 7],
  });
});

test("choiceIntent maps search_library decline", () => {
  const pc = { kind: "search_library" as const, items: [], player: 1 };
  expect(choiceIntent(pc, { kind: "search", choice: null })).toEqual({
    kind: "search_library",
    player: 1,
    choice: null,
  });
});

test("choiceIntent maps scry arrange", () => {
  const pc = { kind: "scry" as const, items: [{ id: 1, label: "A" }], player: 0 };
  expect(choiceIntent(pc, { kind: "arrange", top: [1], bottom: [2] })).toEqual({
    kind: "arrange_top",
    player: 0,
    top: [1],
    bottom: [2],
  });
});

test("choiceIntent maps order_triggers", () => {
  const pc = { kind: "order_triggers" as const, count: 2, labels: ["A", "B"], player: 0, source: 5 };
  expect(choiceIntent(pc, { kind: "order", order: [1, 0] })).toEqual({
    kind: "choose_order",
    player: 0,
    order: [1, 0],
  });
});

test("choiceIntent maps assign combat damage", () => {
  const pc = { kind: "assign_combat_damage" as const, items: [], player: 0, source: 9 };
  expect(choiceIntent(pc, { kind: "assign", assignment: [{ blocker: 4, amount: 3 }] })).toEqual({
    kind: "assign_damage",
    player: 0,
    assignment: [{ blocker: 4, amount: 3 }],
  });
});

test("choiceDraftKey changes when scry items change", () => {
  const a = { kind: "scry" as const, items: [{ id: 1, label: "A" }], player: 0 };
  const b = { kind: "scry" as const, items: [{ id: 2, label: "B" }], player: 0 };
  expect(choiceDraftKey(a)).not.toBe(choiceDraftKey(b));
});

test("buildAnswerFromDraft builds discard from card-pick draft", () => {
  const pc = { kind: "discard" as const, count: 2, items: [], player: 0 };
  const draft: PromptDraft = { kind: "card-pick", picked: [1, 2] };
  expect(buildAnswerFromDraft(pc, draft)).toEqual({ kind: "discard", cards: [1, 2] });
});

test("buildAnswerFromDraft builds proliferate from empty card-pick", () => {
  const pc = { kind: "proliferate" as const, items: [], player: 0, source: 1 };
  const draft: PromptDraft = { kind: "card-pick", picked: [] };
  expect(buildAnswerFromDraft(pc, draft)).toEqual({ kind: "sacrifice", ids: [] });
});
