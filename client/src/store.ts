// The client's view of the game. Each stream delta now carries the viewer's full render
// state *and* the events that produced it (ADR 0006): we replace the rendered `state` and
// narrate the `events` into a scrollable game log. No snapshot refetch mid-stream.

import * as Match from "effect/Match";
import { createStore } from "solid-js/store";
import { playerLabel } from "~/lib/players";
import type { StreamFrame, VisibleEvent, VisibleState, WireTarget } from "~/wire/types";

/** A delta's payload (`seq`, `events`, `state`) — the non-snapshot arm of `StreamFrame` minus its
 * `frame` tag. The generator inlines it rather than exporting a named `DeltaEnvelope`. */
type DeltaEnvelope = Omit<Extract<StreamFrame, { frame: "delta" }>, "frame">;

/** `VisibleState.viewer` for a spectator — a watcher with no seat (server: `schema::SPECTATOR_VIEWER`).
 * The board renders read-only: no hand, no action affordances. */
export const SPECTATOR_VIEWER = 255;

export interface LogLine {
  seq: number;
  text: string;
  /** Server auto-submit or the viewer's own draw — shown with an AUTO chip in the log. */
  auto?: boolean;
}

export interface GameStore {
  state: VisibleState | null;
  seq: number;
  reject: string | null;
  log: LogLine[];
}

export const [game, setGame] = createStore<GameStore>({
  state: null,
  seq: 0,
  reject: null,
  log: [],
});

/** Reset to a blank game: called on Board mount so a new table doesn't render the last one's state. */
export function resetGame(): void {
  setGame({ state: null, seq: 0, reject: null, log: [] });
}

/** Replace the view with a snapshot, ignoring any that's older than what we already show. */
export function applySnapshot(seq: number, state: VisibleState): void {
  if (state && seq >= game.seq) {
    moveMap = new Map(); // a snapshot carries no events → no zone-move glides
    stackResolved = new Set();
    setGame({ state, seq });
  }
}

/**
 * Fold a delta: the delta is self-sufficient (full render `state` + `events` to narrate).
 * Same-`seq` empty-event frames are hold ticks (dwell) and only refresh the countdown.
 * Server auto-submitted actions (`auto_actions`) and the viewer's own draws append as `auto`
 * log lines (AUTO chip in the panel) — no toast.
 */
export function applyDelta(delta: DeltaEnvelope): void {
  if (delta.seq < game.seq) return;
  if (delta.seq === game.seq) {
    if (delta.events.length === 0 && game.state) {
      setGame("state", "stack_hold_remaining_ms", delta.state.stack_hold_remaining_ms ?? 0);
    }
    return;
  }
  const viewer = delta.state.viewer;
  const eventLines: LogLine[] = [];
  for (const e of delta.events) {
    // Viewer's draws: one AUTO log line (name the card when known). Skip the generic `describe`
    // line so we don't get "P0 draws Shock" and "Drew Shock" back-to-back.
    if (e.kind === "card_drawn" && e.player === viewer) {
      eventLines.push({
        seq: delta.seq,
        text: e.card ? `Drew ${e.card}` : "Drew a card",
        auto: true,
      });
      continue;
    }
    const text = describe(e, delta.state);
    if (text != null) eventLines.push({ seq: delta.seq, text });
  }
  const autoLines: LogLine[] = (delta.auto_actions ?? []).map((text) => ({
    seq: delta.seq,
    text,
    auto: true,
  }));
  const lines = [...eventLines, ...autoLines];
  // Provenance for the canvas glide, rebuilt before the board re-lays out.
  ({ moves: moveMap, fromStack: stackResolved } = extractProvenance(delta.events));
  setGame({ state: delta.state, seq: delta.seq });
  if (lines.length) setGame("log", (log) => [...log, ...lines].slice(-200));
}

export function setReject(reason: string | null): void {
  setGame("reject", reason);
}

// Zone-change provenance from the last delta: new object id → the id it came `from`. A zone change
// mints a fresh object id (a hand card and its battlefield permanent are different ids), so this is
// how the canvas tween knows a card *moved* rather than appeared — it seeds the new card's glide at
// the old one's position (see Board.tsx). Rebuilt per delta; a snapshot carries no events, so empty.
let moveMap = new Map<number, number>();

/** New-object-id → source-object-id for zone moves in the most recent delta. */
export function zoneMoves(): Map<number, number> {
  return moveMap;
}

// Permanents in the most recent delta that entered by a spell resolving off the stack
// (`permanent_entered.from` is the spell's stack object — the engine emits this event only
// from spell resolution; tokens, land drops, and reanimations have their own events). The
// stack renders as a DOM overlay, not canvas cards, so these have no canvas origin to glide
// from — the board seeds their entrance at the overlay's anchor instead.
let stackResolved = new Set<number>();

/** Ids of permanents that entered from the stack in the most recent delta. */
export function resolvedFromStack(): Set<number> {
  return stackResolved;
}

/** One pass over a delta's events building both glide-provenance structures, so the move map
 * and the from-stack set can't drift apart (one scan, one reset story). */
function extractProvenance(events: VisibleEvent[]): {
  moves: Map<number, number>;
  fromStack: Set<number>;
} {
  const moves = new Map<number, number>();
  const fromStack = new Set<number>();
  for (const e of events) {
    Match.value(e).pipe(
      Match.discriminator("kind")("moved_to_graveyard", "moved_to_exile", "milled", "moved_to_command_zone", (e) =>
        moves.set(e.card, e.from),
      ),
      Match.discriminator("kind")("exiled_on_adventure", (e) => moves.set(e.card, e.from)),
      Match.discriminator("kind")("permanent_entered", (e) => {
        moves.set(e.permanent, e.from);
        fromStack.add(e.permanent);
      }),
      Match.discriminator("kind")("reanimated_to_battlefield", (e) => moves.set(e.permanent, e.from)),
      // A flicker's return mints a fresh permanent id (CR 400.7) — glide it from the exiled card.
      Match.discriminator("kind")("flickered_to_battlefield", (e) => moves.set(e.permanent, e.from)),
      // Every other kind is deliberately not a glide-provenance move. Listed (not orElse'd) so a
      // new engine event kind is a compile error here until someone decides whether it glides.
      Match.discriminator("kind")(
        "abilities_granted",
        "ability_activated_this_turn",
        "ability_countered",
        "ability_resolved",
        "added_subtypes",
        "attached_to",
        "attacker_declared",
        "base_pt_set_until_end_of_turn",
        "became_copy",
        "cast_from_exile_free_bottoms_library_on_leave",
        "channel_colorless_mana_granted",
        "combat_damage_copy_armed",
        "combat_damage_prevented",
        "granted_abilities_ended",
        "leveled_up",
        "manifested",
        "reanimated_creature_became",
        "returned_exiled_card_to_graveyard",
        "spell_counters_divided",
        "time_counters_placed",
        "time_counters_removed",
        "token_granted_return_exiled_on_leave",
        "turned_face_up",
        "types_added_until_end_of_turn",
        "blocker_declared",
        "card_drawn",
        "card_exiled_with_source_left_exile",
        "cast_from_exile_free_ended",
        "cast_from_exile_free_permission_granted",
        "citys_blessing_gained",
        "combat_cleared",
        "combat_damage_dealt_to_player",
        "combat_damage_divided",
        "combat_damage_watch_armed",
        "combat_damage_watch_consumed",
        "commander_cast_from_command_zone",
        "commander_damage_dealt",
        "conditioned_control_ended",
        "conditioned_control_gained",
        "control_ended_until_end_of_turn",
        "control_gained",
        "control_gained_until_end_of_turn",
        "counters_placed",
        "creature_type_chosen",
        "damage_cleared",
        "damage_dealt_to_player",
        "damage_marked",
        "deathtouch_marked",
        "delayed_trigger_scheduled",
        "delayed_triggers_fired",
        "discarded",
        "drew_from_empty_library",
        "exiled_from_graveyard_may_play",
        "exiled_from_library_may_play",
        "exiled_from_library_to_choose_cast_free",
        "exiled_until_source_leaves",
        "exiled_until_source_leaves_minting_illusion",
        "exiled_with_source",
        "flash_permission_granted",
        "goad_cleared",
        "goaded",
        "keywords_stripped",
        "kind_counters_placed",
        "land_played",
        "leaves_illusion_minted",
        "library_shuffled",
        "life_changed",
        "lost_summoning_sickness",
        "loyalty_activated",
        "loyalty_changed",
        "mana_added",
        "mana_emptied",
        "mana_spent",
        "must_attack_declared",
        "next_cast_trigger_armed",
        "next_cast_trigger_consumed",
        "phased_in",
        "phased_out",
        "play_from_exile_ended",
        "play_from_exile_permission_armed",
        "player_lost",
        "priority_passed",
        "put_on_bottom_of_library",
        "put_onto_battlefield_from_hand",
        "regenerated",
        "regeneration_shield_created",
        "regeneration_shields_expired",
        "returned_from_linked_exile",
        "returned_to_hand",
        "revealed_top_of_library",
        "sacrificed",
        "searched_to_battlefield",
        "searched_to_hand",
        "spell_cast",
        "spell_ceased_to_exist",
        "spell_copied",
        "spell_damage_divided",
        "spell_targets_chosen",
        "prepared_changed",
        "prepared_spell_cast",
        "adventure_spell_cast",
        "color_chosen",
        "step_began",
        "tapped",
        "temp_boost",
        "temp_boosts_ended",
        "token_ceased_to_exist",
        "token_created",
        "token_entered_attacking",
        "triggered_ability_on_stack",
        "triggered_ability_this_turn",
        "tucked_to_library",
        "untapped",
        "vow_counters_placed",
        () => {},
      ),
      Match.exhaustive,
    );
  }
  return { moves, fromStack };
}

// A human-readable log line for an event, joining object ids → names against the delta's
// (post-apply) state. Returns null for events with no narrative value (priority, mana).
// Wire event kinds are projected in crates/schema/src/projection/event.rs — new kinds land
// there first; the exhaustive lists below are the client-side follow-up for log/provenance.
function describe(e: VisibleEvent, state: VisibleState): string | null {
  const name = (id: number) => state.objects.find((o) => o.id === id)?.name ?? `#${id}`;
  const p = (n: number) => playerLabel(state.players, n);
  const t = (target: WireTarget) => (target.kind === "object" ? name(target.id) : p(target.player));
  return Match.value(e).pipe(
    Match.withReturnType<string | null>(),
    Match.discriminators("kind")({
      spell_cast: (e) => `${p(e.controller)} casts ${name(e.spell)}${e.target != null ? ` → ${t(e.target)}` : ""}`,
      land_played: (e) => `${p(e.player)} plays ${name(e.permanent)}`,
      permanent_entered: (e) => `${name(e.permanent)} enters`,
      triggered_ability_on_stack: (e) =>
        `${name(e.source)}'s ability triggers${e.target != null ? ` → ${t(e.target)}` : ""}`,
      damage_marked: (e) => `${name(e.object)} takes ${e.amount}${e.source != null ? ` from ${name(e.source)}` : ""}`,
      damage_dealt_to_player: (e) => `${name(e.source)} deals ${e.amount} damage to ${p(e.player)}`,
      life_changed: (e) => `${p(e.player)} ${e.amount < 0 ? "loses" : "gains"} ${Math.abs(e.amount)} life`,
      moved_to_graveyard: (e) => `${name(e.card)} dies`,
      moved_to_command_zone: (e) => `${name(e.card)} returns to the command zone`,
      counters_placed: (e) => `${name(e.object)} gets ${e.count} +1/+1 counter${e.count === 1 ? "" : "s"}`,
      attacker_declared: (e) => `${name(e.object)} attacks`,
      blocker_declared: (e) => `${name(e.blocker)} blocks ${name(e.attacker)}`,
      card_drawn: (e) => `${p(e.player)} draws${e.card ? ` ${e.card}` : " a card"}`,
      // Decking out is the one loss with no visible cause on the board — no lethal damage, no
      // commander damage, just a library that ran out. Say it, or the `player_lost` line below
      // reads as an unexplained death.
      drew_from_empty_library: (e) => `${p(e.player)} tries to draw from an empty library`,
      player_lost: (e) => `${p(e.player)} loses the game`,
      creature_type_chosen: (e) => `${name(e.object)} is chosen as ${e.subtype}`,
      color_chosen: (e) => {
        const colors = ["white", "blue", "black", "red", "green"];
        return `${name(e.object)} is chosen as ${colors[e.color] ?? `color ${e.color}`}`;
      },
      // counter_kind is a numeric engine index with no client name table, so the kind stays unnamed.
      kind_counters_placed: (e) => `${name(e.object)} gets ${e.count} counter${e.count === 1 ? "" : "s"}`,
      control_gained: (e) => `${p(e.controller)} gains control of ${name(e.object)}`,
      conditioned_control_gained: (e) => `${p(e.controller)} gains control of ${name(e.object)}`,
      conditioned_control_ended: (e) => `control of ${name(e.object)} reverts`,
      ability_countered: (e) => `${name(e.source)}'s ability is countered`,
      flickered_to_battlefield: (e) => `${name(e.permanent)} is exiled and returns`,
      token_entered_attacking: (e) => `${name(e.token)} enters attacking`,
      citys_blessing_gained: (e) => `${p(e.player)} gains the city's blessing`,
      // def is the card name string — a hidden-zone card not in state.objects, so use it directly.
      revealed_top_of_library: (e) => `${p(e.player)} reveals ${e.def}`,
      library_shuffled: (e) => `${p(e.player)} shuffles their library`,
      phased_out: (e) => `${name(e.object)} phases out`,
      phased_in: (e) => `${name(e.object)} phases in`,
      adventure_spell_cast: (e) =>
        `${p(e.controller)} casts ${name(e.spell)}${e.target != null ? ` → ${t(e.target)}` : ""}`,
      regenerated: (e) => `${name(e.object)} regenerates`,
      // Face-down transformations — don't leak a manifested card's identity (CR 708.2).
      manifested: (e) => `${p(e.controller)} manifests a card`,
      turned_face_up: (e) => `${name(e.permanent)} is turned face up`,
      became_copy: (e) => `${name(e.object)} becomes a copy`,
      leveled_up: (e) => `${name(e.object)} levels up to level ${e.level}`,
    }),
    // Events with no narrative value (priority, mana, bookkeeping) stay out of the log. Listed
    // (not orElse'd) so a new engine event kind is a compile error here until someone decides
    // whether it narrates.
    Match.discriminator("kind")(
      "abilities_granted",
      "ability_activated_this_turn",
      "ability_resolved",
      "added_subtypes",
      "attached_to",
      "base_pt_set_until_end_of_turn",
      "cast_from_exile_free_bottoms_library_on_leave",
      "channel_colorless_mana_granted",
      "combat_damage_copy_armed",
      "combat_damage_prevented",
      "granted_abilities_ended",
      "reanimated_creature_became",
      "returned_exiled_card_to_graveyard",
      "spell_counters_divided",
      "time_counters_placed",
      "time_counters_removed",
      "token_granted_return_exiled_on_leave",
      "types_added_until_end_of_turn",
      "card_exiled_with_source_left_exile",
      "cast_from_exile_free_ended",
      "cast_from_exile_free_permission_granted",
      "combat_cleared",
      "combat_damage_dealt_to_player",
      "combat_damage_divided",
      "combat_damage_watch_armed",
      "combat_damage_watch_consumed",
      "commander_cast_from_command_zone",
      "commander_damage_dealt",
      "control_ended_until_end_of_turn",
      "control_gained_until_end_of_turn",
      "damage_cleared",
      "deathtouch_marked",
      "delayed_trigger_scheduled",
      "delayed_triggers_fired",
      "discarded",
      "exiled_from_graveyard_may_play",
      "exiled_from_library_may_play",
      "exiled_from_library_to_choose_cast_free",
      "exiled_on_adventure",
      "exiled_until_source_leaves",
      "exiled_until_source_leaves_minting_illusion",
      "exiled_with_source",
      "flash_permission_granted",
      "goad_cleared",
      "goaded",
      "keywords_stripped",
      "leaves_illusion_minted",
      "lost_summoning_sickness",
      "loyalty_activated",
      "loyalty_changed",
      "mana_added",
      "mana_emptied",
      "mana_spent",
      "milled",
      "moved_to_exile",
      "must_attack_declared",
      "next_cast_trigger_armed",
      "next_cast_trigger_consumed",
      "play_from_exile_ended",
      "play_from_exile_permission_armed",
      "priority_passed",
      "put_on_bottom_of_library",
      "put_onto_battlefield_from_hand",
      "reanimated_to_battlefield",
      "regeneration_shield_created",
      "regeneration_shields_expired",
      "returned_from_linked_exile",
      "returned_to_hand",
      "sacrificed",
      "searched_to_battlefield",
      "searched_to_hand",
      "spell_ceased_to_exist",
      "spell_copied",
      "spell_damage_divided",
      "spell_targets_chosen",
      "prepared_changed",
      "prepared_spell_cast",
      "step_began",
      "tapped",
      "temp_boost",
      "temp_boosts_ended",
      "token_ceased_to_exist",
      "token_created",
      "triggered_ability_this_turn",
      "tucked_to_library",
      "untapped",
      "vow_counters_placed",
      () => null,
    ),
    Match.exhaustive,
  );
}
