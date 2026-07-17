import { describe, expect, it } from "vitest";
import type { PendingChoiceView } from "~/api/generated";
import { type AnswerInput, choiceIntent } from "~/lib/choice";

// A pending choice carrying just the `player` the mapping reads (the rest is form input).
const pc = (kind: PendingChoiceView["kind"]): PendingChoiceView =>
  ({
    kind,
    player: 2,
    source: 0,
    items: [],
    labels: [],
    count: 0,
    cost: { generic: 0, colored: [] },
  }) as PendingChoiceView;

describe("choiceIntent", () => {
  const cases: [AnswerInput, unknown][] = [
    [
      { kind: "order", order: [1, 0] },
      { kind: "choose_order", player: 2, order: [1, 0] },
    ],
    [
      { kind: "target", id: 7 },
      { kind: "choose_targets", player: 2, targets: [{ kind: "object", id: 7 }] },
    ],
    [
      { kind: "target", id: 0, player: 1 },
      { kind: "choose_targets", player: 2, targets: [{ kind: "player", player: 1 }] },
    ],
    [
      { kind: "targets", ids: [7, 8] },
      {
        kind: "choose_targets",
        player: 2,
        targets: [
          { kind: "object", id: 7 },
          { kind: "object", id: 8 },
        ],
      },
    ],
    [
      { kind: "may", yes: true },
      { kind: "answer_may", player: 2, yes: true },
    ],
    [
      { kind: "pay", pay: false },
      { kind: "pay_optional_cost", player: 2, pay: false },
    ],
    [
      { kind: "assign", assignment: [{ blocker: 5, amount: 3 }] },
      { kind: "assign_damage", player: 2, assignment: [{ blocker: 5, amount: 3 }] },
    ],
    [
      { kind: "arrange", top: [1], bottom: [2, 3] },
      { kind: "arrange_top", player: 2, top: [1], bottom: [2, 3] },
    ],
    [
      { kind: "search", choice: 9 },
      { kind: "search_library", player: 2, choice: 9 },
    ],
    [
      { kind: "search", choice: null },
      { kind: "search_library", player: 2, choice: null },
    ],
    [
      { kind: "sacrifice", ids: [4] },
      { kind: "choose_sacrifices", player: 2, sacrifices: [4] },
    ],
    [
      { kind: "discard", cards: [6, 8] },
      { kind: "discard", player: 2, cards: [6, 8] },
    ],
    [
      { kind: "put_land", choice: 3 },
      { kind: "put_land_from_hand", player: 2, choice: 3 },
    ],
    [
      { kind: "choose_exiled", choice: 5 },
      { kind: "choose_exiled_with_card", player: 2, choice: 5 },
    ],
    [
      { kind: "choose_exiled", choice: null },
      { kind: "choose_exiled_with_card", player: 2, choice: null },
    ],
    [
      { kind: "select_top", cards: [1, 3] },
      { kind: "select_from_top", player: 2, cards: [1, 3] },
    ],
    [
      { kind: "mode", mode: 1 },
      { kind: "choose_mode", player: 2, mode: 1 },
    ],
    [
      { kind: "target_players", players: [0, 3] },
      { kind: "choose_target_players", player: 2, players: [0, 3] },
    ],
    [
      { kind: "distribute", to_hand: [1], to_bottom: [2, 3], to_exile_may_play: [] },
      { kind: "distribute_top", player: 2, to_hand: [1], to_bottom: [2, 3], to_exile_may_play: [] },
    ],
    [
      { kind: "shuffle_gy", cards: [4, 5] },
      { kind: "shuffle_from_graveyard", player: 2, cards: [4, 5] },
    ],
    [
      { kind: "choose_exiled_cast", choice: 6 },
      { kind: "choose_exiled_with_card_to_cast", player: 2, choice: 6 },
    ],
    [
      { kind: "choose_exiled_cast", choice: null },
      { kind: "choose_exiled_with_card_to_cast", player: 2, choice: null },
    ],
    [
      { kind: "choose_exiled_dig", choice: 7 },
      { kind: "choose_exiled_dig_to_cast_free", player: 2, choice: 7 },
    ],
    [
      { kind: "trigger_modes", modes: [{ index: 0, target: { kind: "player", player: 1 } }] },
      { kind: "choose_trigger_modes", player: 2, modes: [{ index: 0, target: { kind: "player", player: 1 } }] },
    ],
    [
      { kind: "mana_color", color: 3 },
      { kind: "choose_mana_color", player: 2, color: 3 },
    ],
    [
      { kind: "creature_type", subtype: "Zombie" },
      { kind: "choose_creature_type", player: 2, subtype: "Zombie" },
    ],
    [
      { kind: "color", color: 1 },
      { kind: "choose_color", player: 2, color: 1 },
    ],
    [
      { kind: "opponent_pile", pile: 0 },
      { kind: "choose_opponent_pile", player: 2, pile: 0 },
    ],
    [
      { kind: "revealed", choice: 9 },
      { kind: "revealed_card_to_battlefield_or_hand", player: 2, choice: 9 },
    ],
    [
      { kind: "revealed", choice: null },
      { kind: "revealed_card_to_battlefield_or_hand", player: 2, choice: null },
    ],
    [
      { kind: "attach_host", host: 4 },
      { kind: "choose_attach_host", player: 2, host: 4 },
    ],
    [
      { kind: "keep_tapped", ids: [3, 4] },
      { kind: "decline_untap", player: 2, keep_tapped: [3, 4] },
    ],
    [
      { kind: "top_or_bottom", top: true },
      { kind: "choose_top_or_bottom", player: 2, top: true },
    ],
    [
      { kind: "return_land", land: null },
      { kind: "return_land_or_sacrifice", player: 2, land: null },
    ],
    [
      { kind: "cast_face_down_choice", choice: 6 },
      { kind: "cast_creature_face_down", player: 2, choice: 6 },
    ],
  ];

  it.each(cases)("maps %o", (answer, intent) => {
    expect(choiceIntent(pc("may_yes_no"), answer)).toEqual(intent);
  });

  it("takes the answering player from the pending choice", () => {
    const intent = choiceIntent(pc("may_yes_no"), { kind: "may", yes: false });
    expect(intent).toMatchObject({ player: 2 });
  });
});

// FORMS (`client/src/components/molecules/prompt-forms.tsx`) maps every PendingChoiceView["kind"] to a form
// component via `Record<PendingChoiceView["kind"], Component<FormProps>>` — that Record is
// itself the exhaustiveness check (bun run build fails if a kind's form is missing). A runtime
// re-check here isn't possible: this project's vitest config has no DOM (jsdom/happy-dom), and
// importing *any* .tsx file crashes on solid-js/web's SSR guard the moment its module loads
// (reproducible with the pre-existing hand molecule, unrelated to this change) — so prompt-forms.tsx
// can't be imported from a plain .test.ts here.
