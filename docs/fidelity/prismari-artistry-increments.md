# Prismari Artistry deck increments (2026-07-26)

Deck report: [prismari-artistry.md](prismari-artistry.md). This file is the sole
engine-capability backlog for this deck (ranked increments plus the concrete Prismari cards they
unblock).

From `docs/decklists/prismari_artistry.md` (official Wizards `soc` precon; commander Rootha,
Mastering the Moment). After the documentation re-audit, 10 of the deck's 85 nonbasic cards need
engine work. The live gaps are narrow but real: Prismari's second-generation copy effects drop
copy-granted copiable riders like haste or myriad, and `Goldspan Dragon`'s "two mana of any one
color" shortcut is now observable against the deck's blue-red costs.

### 1. `copy-effect-exception-riders-must-be-copiable` — 9 cards, XL

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
2. **Token-copy readers (M).** Route `TokenEffect::CreateCopy` and
   `TokenEffect::CopyEachEnteredThisTurnTokenTappedAttacking` through that accessor so
   `Determined Iteration`, `Replication Technique`, `Rite of Replication`, `Rionya`, `Twinflame`,
   and `Redoubled Stormsinger` preserve the copied rider when they copy a first-generation copy.
3. **Permanent-copy readers (L).** Route `answer_enter_as_copy`,
   `answer_each_other_token_becomes_copy`, `answer_choose_copy_card_from_list`, and
   `TokenEffect::BecomeCopyOfTargetCreatureGainingMyriad` through the same accessor so
   `Cursed Mirror`, `Brudiclad`, and `Muddle` preserve copiable exception riders when the thing
   they copy is already a copy.

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
