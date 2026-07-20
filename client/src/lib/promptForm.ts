// Pure prompt chrome / surface policy — importable from vitest without Solid.
// Solid forms in components/molecules/prompt-forms.tsx remain the render adapters.

import type { ChoiceItem, PendingChoiceView } from "~/wire/types";

/** Corner panel vs full-screen card picker (replaces a parallel CARD_PICK_KINDS set). */
export type PromptChrome = "panel" | "fullscreen";

const FULLSCREEN_KINDS: ReadonlySet<PendingChoiceView["kind"]> = new Set([
  "discard",
  "sacrifice_edict",
  "choose_target",
  "choose_spell_targets",
  "put_land_from_hand",
  "put_creature_from_hand",
  "choose_exiled_with_card",
  "search_library",
  "select_from_top",
  "scry",
  "surveil",
  "proliferate",
  "phase_out",
  "choose_own_sacrifices",
  "devour",
  "caster_keep_permanents",
  "exile_from_graveyard",
  "may_sacrifice",
  "may_return_from_graveyard",
  "may_discard",
  "shuffle_from_graveyard",
  "choose_exiled_with_card_to_cast",
  "choose_exiled_dig_to_cast_free",
  "choose_exiled_to_cast_free",
  "opponent_chooses_exiled_nonland",
  "choose_attach_host",
  "choose_copy_target",
  "choose_counter_target_for_player",
  "choose_ability_targets",
  "choose_activation_cost_targets",
  "choose_target_players",
  "distribute_top",
  "choose_trigger_modes",
  "choose_dredge",
  "put_from_hand_on_top",
  "opponent_chooses_revealed_to_graveyard",
  "pay_cumulative_upkeep_or_sacrifice",
]);

/** Chrome for a pending Choice view kind. */
export function promptChrome(kind: PendingChoiceView["kind"]): PromptChrome {
  return FULLSCREEN_KINDS.has(kind) ? "fullscreen" : "panel";
}

/** True when PromptHost should skip panel chrome (fullscreen pickers). */
export function isFullscreenPrompt(kind: PendingChoiceView["kind"]): boolean {
  return promptChrome(kind) === "fullscreen";
}

/** Keep the first item per card name — library searches offer one representative object per face. */
export function dedupeChoiceItems(items: readonly ChoiceItem[]): ChoiceItem[] {
  const seen = new Set<string>();
  const out: ChoiceItem[] = [];
  for (const it of items) {
    if (seen.has(it.label)) continue;
    seen.add(it.label);
    out.push(it);
  }
  return out;
}

/** Case-insensitive substring filter on the card name (label). Empty query keeps all items. */
export function filterChoiceItems(items: readonly ChoiceItem[], query: string): ChoiceItem[] {
  const q = query.trim().toLowerCase();
  if (q === "") return [...items];
  return items.filter((it) => it.label.toLowerCase().includes(q));
}

/** Deduped then filtered candidates for a pick-one library search (or similar). */
export function searchableChoiceItems(items: readonly ChoiceItem[], query: string): ChoiceItem[] {
  return filterChoiceItems(dedupeChoiceItems(items), query);
}

/**
 * Card-pick kinds that show a name filter and (for pick-one) dedupe by face.
 * Keep in sync with `searchable` on the matching form in components/molecules/prompt-forms.tsx.
 */
export function cardPickIsSearchable(kind: PendingChoiceView["kind"]): boolean {
  return kind === "search_library";
}

/** Floor for the CardPickPrompt card strip (`data-testid="pick-card-scroll"`). Flex shrink must not
 * collapse the grid to 0px on a short viewport (library search then shows only Fail-to-find). */
export const PICK_CARD_SCROLL_MIN_CLASS = "min-h-[min(40vh,280px)]";
