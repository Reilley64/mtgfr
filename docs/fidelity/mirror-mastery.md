# Mirror Mastery (Commander 2011) Fidelity Report

**Deck:** [Mirror Mastery](https://archidekt.com/decks/2209174/mirror_mastery_commander_2011)  
**Commander:** Riku of Two Reflections  
**Total unique non-basic cards:** 72  
**Date:** 2026-07-19

## Status Summary

- **A. In pool, faithful:** 22 cards (30.6%)
- **B. In pool, approximated:** 2 cards (2.8%)
- **C. New, expressible today:** 16 cards (22.2%)
- **D. New, needs engine work:** 32 cards (44.4%)

**Grind progress (as of 2026-07-22):** 72 of 72 checked (100%) — A 22/22, B 2/2, C 16/16, D 32/32.
The section letters are the original audit's buckets and don't move; the checkboxes below are the
running state. Reconciled against the backlog on 2026-07-22: every section-D card whose TOML is on
disk was re-opened, diffed against live Scryfall oracle text, and checked ability-by-ability before
being ticked (14 caught up in that pass — #167, #175, #176, #178, #180, #181, #182, #184, #186,
#189, #191, #197, #199, #200 — plus #203 clearing Vengeful Rebirth's last residual). Wave 8
(2026-07-22) added four: the exotic Vow of Wildness (authoring-only), #188 Firespout, #190
Invigorate, and #171 Conundrum Sphinx — each frame-audited against a live Scryfall
`cards/collection` fetch by oracle id, zero mismatches. Wave 9 (2026-07-22) closed Section B
entirely: #201 `planeswalker-as-attack-defender` made a planeswalker a legal attack defender
(CR 506.2/508.1a), which cleared the whole Vow cycle's "or planeswalkers you control" residual, and
#179 authored Collective Voyage as the join-forces card it actually is. Wave 10 (2026-07-22) added
three more Section-D cards — #172 Edric, Spymaster of Trest, #173 Hydra Omnivore and #177 Artisan of
Kozilek — each frame-audited against a live Scryfall `cards/collection` fetch by oracle id, zero
mismatches. Wave 11 (2026-07-22) closed the deck: #197 Nucklavee, #187 Fire // Ice (the pool's first
split card) and #168 Intet, the Dreamer — all three backlog premises were wrong and were corrected
in place; each card frame-audited against a live Scryfall `cards/collection` fetch by oracle id,
zero mismatches. **Every card in this deck is now faithful.**

**Target:** 100% faithful (all 72 cards scripted, with only deliberate residuals flagged)

---

## A. In Pool, Faithful (22 cards)

All 22 cards are in the pool and fully faithful, with no `approximates` notes.

- [x] Aethersnipe
- [x] Baloth Woodcrasher
- [x] Chain Reaction
- [x] Chartooth Cougar
- [x] Command Tower
- [x] Cultivate
- [x] Elvish Aberration
- [x] Explosive Vegetation
- [x] Faultgrinder
- [x] Garruk Wildspeaker
- [x] Gruul Signet
- [x] Gruul Turf
- [x] Kodama's Reach
- [x] Krosan Tusker
- [x] Lightning Greaves
- [x] Rupture Spire
- [x] Simic Growth Chamber
- [x] Simic Signet
- [x] Simic Sky Swallower
- [x] Sol Ring
- [x] Spitebellows
- [x] Temple of the False God

---

## B. In Pool, Approximated (2 cards)

Both entered the pool with the Political Puppets deck. Their shared "or planeswalkers you control"
residual was cleared by [#201 `planeswalker-as-attack-defender`](mirror-mastery-increments.md) on
2026-07-22 — a planeswalker is now a legal attack defender (CR 508.1a) and
`grant_to_attached`'s `cant_attack_controller` reads the defender's *controller*, so both halves of
the attack ban hold. Neither card carries an `approximates` any more.

- [x] Vow of Flight — +2/+2, flying, can't attack you or planeswalkers you control (#201, landed)
- [x] Vow of Lightning — +2/+2, first strike, can't attack you or planeswalkers you control (#201, landed)

---

## C. New, Expressible Today (16 cards)

These cards can be scripted with the current DSL and no engine changes.

### Lands (6 cards)

- [x] Evolving Wilds — fetchland (proven pattern exists)
- [x] Izzet Boilerworks — bounce land (Azorius Chancery pattern)
- [x] Kazandu Refuge — gainland (life on ETB)
- [x] Vivid Crag — charge counter land
- [x] Vivid Creek — charge counter land
- [x] Vivid Grove — charge counter land

### Artifacts (3 cards)

- [x] Armillary Sphere — tap + sacrifice to search basics
- [x] Izzet Signet — 2-mana rock (signet pattern exists)
- [x] Prophetic Prism — ETB cantrip + any-color mana

### Instants & Sorceries (6 cards)

- [x] Colossal Might — pump +4/+2 trample instant
- [x] Electrolyze — divided damage + draw
- [x] Ray of Command — temporary control until EOT (ponytail: "tap on loss" rider dropped)
- [x] Ruination — destroy all nonbasic lands
- [x] Savage Twister — {X} damage to all creatures
- [x] Tribute to the Wild — each opponent sacrifices artifact or enchantment

### Enchantments (1 card)

- [x] Vow of Wildness — Aura +3/+3, trample, `cant_attack_controller = true`; authored 2026-07-22 with no engine change. Its "or planeswalkers you control" residual (and its two Section-B siblings') was cleared the same day by #201; the `approximates` and `# ponytail:` are gone.

---

## D. New, Needs Engine Work (32 cards)

These cards require engine additions before they can be scripted. Detailed increments are in [`mirror-mastery-increments.md`](mirror-mastery-increments.md).

### Lands (2 cards)

- [x] **Fungal Reaches** — storage land scripted with a new `remove_counters_x` activation cost + `CounterKind::Storage`, and `Mana::OfColors` (already in the engine) for "any combination of {R} and/or {G}" — no new pending choice needed (#184, landed)
- [x] **Homeward Path** — "each player gains control of all creatures they own" scripted with a new `revert_all_creatures_to_owners` effect (#185, landed)

### Creatures (7 cards)

- [x] **Fierce Empath** — ETB search a creature with mana value 6+; scripted with a new `creature_with_mana_value_at_least = N` card filter (#193, landed)
- [x] **Magmatic Force** — real oracle text is "at the beginning of each upkeep, this creature deals 3 damage to any target" (no variable X — the increment's quoted "power plus toughness" text was misquoted); scripted with the existing `each_upkeep` trigger + fixed-3 `deal_damage` (#195, closed as dead variant)
- [x] **Magus of the Vineyard** — "at the beginning of each player's first main phase, that player adds {G}{G}"; scripted with a new `each_player_first_main_phase` trigger and a recipient axis on `add_mana` (#196, landed)
- [x] **Nucklavee** — two *separate* optional ETB triggers (premise correction: not one ability with two targets), scripted with new `sorcery_with_color` / `instant_with_color` card filters plus a fix to `OrderTriggers`, which was fabricating a merged ability and losing each trigger's own `optional`/`cost` (#197 — renumbered from the backlog's #197 slot; landed). Fully faithful, no residual
- [x] **Rapacious One** — combat-damage-to-a-player trigger creating that many Eldrazi Spawn, scripted with the existing `combat_damage_dealt` amount on `create_token` (#197, landed)
- [x] **Valley Rannet** — Mountaincycling {2} + forestcycling {2} scripted as two `[[hand_ability]]` entries after `hand_ability` became an array (#199, landed)
- [x] **Veteran Explorer** — dies-trigger scripted with a new `searcher = "all_players"` fan-out on `search_library` (#200, landed)

### Instants & Sorceries (7 cards)

- [x] **Brainstorm** — scripted with a new `put_from_hand_on_top` effect (#186, landed)
- [x] **Fire // Ice** — the pool's first split card (CR 709): new `[[half]]` inline card tables, `Intent::CastSplitHalf { half }`, and a `split_halves_on_stack` registry so every stack exit restores the fused card (CR 709.4) (#187, landed). Fully faithful, no residual. **Client debt:** the hand card needs two cast affordances (one per half) and must send `WireIntent::CastSplitHalf`; nothing in the catalog projection exposes per-half costs yet (same posture as `adventure`)
- [x] **Firespout** — both spent-color clauses scripted with the existing `conditional` + `color_was_spent_to_cast_this` after that condition learned to read a spell still on the stack (`Game::spell_spent_colors`), plus a new `with_flying` permanent filter (#188, landed; premise correction: `Spell::spent_colors` accounting already existed). Fully faithful, no residual
- [x] **Hull Breach** — 3rd mode's two independent target clauses scripted after `mode_target_clauses` gained multi-clause splitting (#189, landed)
- [x] **Invigorate** — scripted with a new `CardDef::alternative_cost` (`condition` + non-mana `rider`), a new `alternative_cost` flag on `Intent::Cast`, and a new `opponent_gains_life` effect (#190, landed). Residual: the rider always picks the lowest-seat-index living opponent instead of letting the caster choose (needs a `PendingChoice::ChooseOpponent`)
- [x] **Spell Crumple** — scripted with `countered_dest = "library_bottom"` plus a new `tuck_self_to_library_bottom` effect (#191, landed)
- [x] **Vengeful Rebirth** — "Exile Vengeful Rebirth" scripted with a new `exile_self_on_resolve` effect (#192, landed); the conditional "deals damage equal to that card's mana value to any target" clause landed with #203 (non-modal spells now split one ability into independent target clauses, plus a `returned_nonland_card_mana_value` amount) — fully faithful, no residual

### Copy/Cast-from-Exile Mechanics (3 cards)

- [x] **Riku of Two Reflections** — both triggers scripted with the existing `copy_triggering_spell` / `create_token_copy` (#167, landed; premise correction: Riku copies instants/sorceries, not creature spells)
- [x] **Intet, the Dreamer** — premise correction: the permission is "without paying its mana cost for as long as Intet remains on the battlefield", not "at normal cost until your next turn". Scripted with new `face_down` / `free_while_source` flags on the existing `exile_top_may_play` effect plus a `play_from_exile_free_while_source` registry read live off the source (#168, landed). Fully faithful, no residual
- [x] **Call the Skybreaker** — retrace on expensive token-generator; validated end-to-end and the discard-a-land rider fixed to only apply to graveyard casts (#169, landed)

### Complex Triggers & Conditions (4 cards)

- [x] **Animar, Soul of Elements** — "creature spells you cast cost {1} less for each +1/+1 counter on Animar"; scripted with the existing `reduce_spell_cost` + `per_counter_on_source` amount, no engine change (#170, landed; premise correction: the counter-reading cost reducer already existed)
- [x] **Conundrum Sphinx** — the pool's first "name a card" (CR 201.3): scripted with a new `each_player_names_card_then_reveals_top` effect over a `PendingChoice::ChooseCardName` APNAP fan-out (#171, landed; premise correction: every player names a card for themselves, there is no opponent guess). Residual: seats name sequentially, so a later seat sees an earlier seat's reveal first. **Client debt:** no prompt form for `PendingChoiceView::ChooseCardName` yet
- [x] **Edric, Spymaster of Trest** — scripted with a new `deals_combat_damage_to_player` scope `who = "any_creature_damaging_your_opponent"` (any creature, but only damage landing on one of the watcher's opponents) plus a new `damaging_creature_controller_may_draw` payoff whose drawer answers the may-pause itself (#172, landed). Residual: "its controller" is locked in at trigger placement (CR 603.10a), not re-read at resolution
- [x] **Hydra Omnivore** — scripted with a new `damage_each_other_opponent` effect over the existing `combat_damage_dealt` amount (#173, landed; premise correction: "an opponent"/"each other opponent", not "a player"/"each other player")

### Missing Keywords/Mechanics (3 cards)

- [x] **Deadwood Treefolk** — vanishing 3 scripted with a new `CardDef::vanishing` keyword (enters with time counters, upkeep tick, real sacrifice trigger) plus an enters-*or-leaves* return of *another* target creature card, via a new `other` flag on the `card_in_graveyard` target (#174, landed)
- [x] **Death by Dragons** — scripted with `create_token` + `controller = "each_other_player"`; the briefed forced-attack clause does not exist on the real card (#175, closed as dead variant)
- [x] **Hunting Pack** — Storm scripted with `timing = "when_you_cast_this"` + `copy_triggering_spell` with `last_known_information = true` (#176, landed)

### Other Gaps (6 cards)

- [x] **Artisan of Kozilek** — authoring-only: annihilator 2 is `timing = "attacks"` + the existing `defending_player_sacrifices`, and the cast trigger is `timing = "when_you_cast_this"`, `optional = true` + `reanimate_to_battlefield` (#177, landed; premise correction: no new keyword field was needed, and annihilator already routes through `Game::defender_controller` when the attack is aimed at a planeswalker)
- [x] **Avatar of Fury** — scripted with `reduce_own_generic` + the `an_opponent_controls_lands` condition (#178, landed; premise correction: own-cost reduction, not a free cast)
- [x] **Collective Voyage** — scripted with a new `join_forces_pay_mana` effect + `mana_paid_this_way` amount feeding `search_library`'s new `count_amount` cap (#179, landed; premise correction: join forces, not a caster-chosen `cost.x`)
- [x] **Disaster Radius** — scripted with a new `reveal_creature_from_hand` additional cost + `revealed_creature_mana_value` amount (#180, landed; premise correction: reveal a creature, not sacrifice a land)
- [x] **Prophetic Bolt** — scripted with the existing `deal_damage` + `look_at_top` (#181, landed; premise correction: look-at-top, no exile/filter)
- [x] **Trench Gorger** — scripted with a new `SearchDest::Exile` + `count = "any"` search and a new `set_own_base_pt_from_amount` effect (#182, landed)

---

## Observability Re-Audit Findings

The Mirror Mastery deck falsifies **two stale ponytail claims** in the current pool. Both were
re-verified against the code: the *claims* are stale, but the *gaps* are real and neither is the
one the note states.

### 1. Planeswalker Claims Now Falsified

**Garruk Wildspeaker** is a planeswalker permanent in this deck. The pool already had one
(Quintorius, History Chaser), so "no planeswalker permanent exists" has been false for a while —
that reason, not the residual, is what was stale:

- `crates/cards/data/nils_discipline_enforcer.toml`:  
  `# ponytail: "or planeswalkers you control" is unreachable — the pool has no planeswalker permanent`  
  **Verified:** planeswalkers exist; the real blocker is that `Intent::DeclareAttackers` carries
  `(ObjectId, PlayerId)`, so no attack can be aimed at a planeswalker and
  `Game::attacker_tax_owed` keys on `PlayerId`. Re-scoped to increment
  [#201 `planeswalker-as-attack-defender`](mirror-mastery-increments.md); the note's reason is
  corrected in place, the residual stays.

- `crates/cards/data/volcanic_torrent.toml`:  
  `# ponytail: planeswalkers are dropped — no pool card fields a planeswalker permanent to damage`  
  **Verified:** planeswalkers exist and single-target damage already reaches them
  (`TargetSpec::CreatureOrPlaneswalker`); the real blocker is `Effect::DamageEachCreature`'s sweep
  filtering `is_creature_on_battlefield`, so every *mass*-damage effect drops them. Re-scoped to
  increment [#202 `planeswalker-as-damage-and-effect-target`](mirror-mastery-increments.md) —
  **landed**: `DamageEachCreature` now takes `include_planeswalkers`, the note is deleted, and
  Volcanic Torrent's planeswalker half is faithful.

### 2. Vow Cycle Reclassified

A prior wave moved Vow of Flight / Vow of Lightning / Vow of Wildness to Section D on the claim
that `grant_to_attached` "lacks a controller-restricted attack ban". That was checked and is
false: `cant_attack_controller` exists and is read live in `declare_attackers`, and the first two
Vows already ship in the pool with it. They move to Section B (in pool, one residual) and
Vow of Wildness to Section C (expressible today, same residual).

### 3. Other Ponytail Claims

No other ponytail claims in `crates/cards/data/` or `crates/engine/src/` are falsified by Mirror Mastery cards. Claims about Vehicles, battles, and other unmodeled types remain valid.

---

## Next Steps

**Phase 3:** Author all 39 Section C cards in TDD waves (failing test → TOML), with mandatory frame audit after each wave.

**Phase 4:** Engine grind loop for the 18 Section D cards, following increments in `mirror-mastery-increments.md`.

**Phase 5:** Client catch-up after engine waves (pending-choice forms, event arms).

**Phase 5.5:** Ship Mirror Mastery as an in-game precon.

**Phase 6:** Final verify, sync-merge main, commit hygiene, open PR.

**Phase 7:** PR watch through CI and review to merge.
