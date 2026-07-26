# Prismari Artistry deck increments (2026-07-26)

Deck report: [prismari-artistry.md](prismari-artistry.md). This file is the sole
engine-capability backlog for this deck (ranked increments plus the concrete Prismari cards they
unblock).

From `docs/decklists/prismari_artistry.md` (official Wizards `soc` precon; commander Rootha,
Mastering the Moment). After the documentation re-audit, 10 of the deck's 85 nonbasic cards need
engine work. The live gaps are narrow but real: Prismari's second-generation copy effects drop
copy-granted copiable riders like haste or myriad, and `Goldspan Dragon`'s "two mana of any one
color" shortcut is now observable against the deck's blue-red costs.

### 1. `copy-effect-exception-riders-must-be-copiable` — 9 cards, XL — LANDED 2026-07-26

**Depends on:** none.
**Cards:** `brudiclad_telchor_engineer.toml`, `cursed_mirror.toml`,
`determined_iteration.toml`, `muddle_the_ever_changing.toml`,
`redoubled_stormsinger.toml`, `replication_technique.toml`, `rite_of_replication.toml`,
`rionya_fire_dancer.toml`, `twinflame.toml`
**Sketch:** Prismari's copy shell is not blocked on "copy a creature" itself — those first
generation paths are already landed. The blocker is that the engine still treats several
copy-effect exceptions as transient boosts instead of part of the copied object's next
generation of copiable values. `Twinflame`, `Rionya, Fire Dancer`, `Determined Iteration`, and
temporary `Cursed Mirror` copies should stay "that creature, except it has haste" when copied
again; `Muddle, the Ever-Changing` should stay "that creature, except it has myriad" when copied
again. Today the follow-on copy paths mostly read `def_id_of(...)`, which preserves the copied
identity but drops those copy-granted riders. The fix is one shared "current copiable snapshot"
read that every permanent-copy and token-copy path consults.

**Slices:**
1. **Copiable-snapshot infrastructure (M).** _LANDED 2026-07-26._ Added `Permanent::copy_rider_keywords`
   (set by the new `Event::CopyRiderKeywordsGranted`), unioned onto effective keywords by
   `runtime_continuous_effects` and read back by the `Game::copiable_keywords` accessor — the
   keyword half of the copiable snapshot (`def_id_of` is the def half). Twinflame/Rionya/Determined
   Iteration token haste, Cursed Mirror's copy haste, and Muddle's myriad now ride this rider
   instead of a transient `TempBoost`; an until-end-of-turn copy clears it when its `def` reverts.
   Regressions: `twinflame_token_copiable_snapshot_carries_haste`,
   `muddle_copied_form_copiable_snapshot_carries_myriad` (plus the existing haste/myriad behavior
   tests, unchanged).
2. **Token-copy readers (M).** _LANDED 2026-07-26._ `TokenEffect::CreateCopy` and
   `TokenEffect::CopyEachEnteredThisTurnTokenTappedAttacking` now read `copiable_keywords` of the
   copied object and emit the rider on each minted token, so `Determined Iteration`,
   `Replication Technique`, `Rite of Replication`, `Rionya`, `Twinflame`, and `Redoubled
   Stormsinger` preserve the copied rider when they copy a first-generation copy (CR 707.2).
   Regression: `rite_of_replication_copying_a_twinflame_token_preserves_haste`.
3. **Permanent-copy readers (L).** _LANDED 2026-07-26._ Routed `answer_enter_as_copy`
   (`Cursed Mirror`), `answer_each_other_token_becomes_copy` (`Brudiclad`), and
   `TokenEffect::BecomeCopyOfTargetCreatureGainingMyriad` (`Muddle`) through `copiable_keywords`,
   so each carries the copied object's own copy-effect rider onto the new copy (CR 707.2), unioned
   with any rider it adds itself. `answer_choose_copy_card_from_list` (Spirit of Resilience, not in
   this pool) copies an artifact/creature *card* — never a battlefield permanent — so it can carry
   no copiable rider and was left unrouted rather than adding a dead read. Regressions:
   `muddle_copying_a_twinflame_haste_token_keeps_both_haste_and_myriad`,
   `brudiclad_copying_a_twinflame_token_carries_its_haste_rider` (the `Cursed Mirror` reader shares
   the same pattern and stays covered by its existing behavior test).

### 2. `linked-any-one-color-mana-credits` — 1 card, M — LANDED 2026-07-26

**Depends on:** none.
**Cards:** `goldspan_dragon.toml`
**Landed:** `StaticEffect::GrantManaAbility` gained a `single_color` flag (threaded through
`granted_mana_abilities` / `ability_at` and skipped by the auto-tap mana estimate), so Goldspan's
granted Treasure ability reuses the existing "add N mana of any one color" path (CR 106.4): it
pauses on `ChooseManaColor` and produces two mana of the one chosen color, never two independent
wildcards. Regressions: `goldspan_dragon_grants_treasures_two_mana` (updated to the color-choice
behavior) and `goldspan_treasure_cannot_split_across_two_colors`.
**Sketch:** `Goldspan Dragon` currently grants Treasure
`"{T}, Sacrifice this artifact: Add two mana of any one color."` as `mana = ["any", "any"]`,
which produces two independent wildcard credits. Prismari makes that over-permissiveness
observable immediately: a single Goldspan Treasure can split across blue and red pips on the same
spell or ability, even though both mana must be the same chosen color. Add a mana-credit form that
represents "N mana of one chosen color" (or an equivalent grouped payment plan) and use it for
Goldspan's granted ability. Regression: one Goldspan Treasure can fund `{U}{U}` or `{R}{R}`, but
it cannot by itself pay one blue and one red pip of the same cost.
