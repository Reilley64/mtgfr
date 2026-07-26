# Heavenly Inferno — engine-capability increments

Sole engine backlog for this deck. Numbering local to this file. Effort S/M/L/XL.
Land an increment only when a real deck card becomes faithful (or measurably closer) by it.

---

## 1. Kaalia: put an Angel/Demon/Dragon from hand tapped & attacking — M — **LANDED**

Depends on: nothing (extends `put_creature_from_hand`, §6, + `attacks`-an-opponent trigger).
Cards: **Kaalia of the Vast** (commander, faithful).
Landed: 2026-07-24 — `ChoiceEffect::PutCreatureFromHand` promoted from a unit variant to a struct
variant with three defaulted fields (Cauldron Dance's TOML unchanged): `subtypes` (eligibility
filter at raise time), `keep` (suppresses the haste grant + next-end-step sacrifice), and a
`serde(skip)` `defender` the `attacks` trigger threads so the put-in creature enters tapped and
attacking that opponent via the existing `Event::TokenEnteredAttacking` path (CR 508.4).
Cost/trigger corrected against Scryfall to `{1}{R}{W}{B}` / "attacks an opponent". A `ponytail:`
note in the TOML records that the reused put-in path carries only the defending *player*, so
attacking a planeswalker enters the creature attacking that planeswalker's controller (rare edge).

Sketch: `put_creature_from_hand` gains optional params `{ subtypes = ["Angel","Demon","Dragon"],
enters_tapped_attacking = true, keep = true }` — no end-step sacrifice (unlike Cauldron Dance),
placed onto the battlefield **tapped and attacking the attacked opponent**. Fires on an
"attacks an opponent" trigger that threads the defending player into the effect so the new
attacker joins that combat against that player. Reuse `Game::declare_attackers` bookkeeping to
register the surprise attacker legally (already tapped, skips attack-cost/again-triggers as a
put-in effect).

## 2. `cast_from_hand` ETB intervening-if + controller/color mass filters — M — **LANDED**

Depends on: #6 (color/controller filter) for the mass-destroy target set.
Cards: **Dread Cacodemon** (faithful), **Reiver Demon** (faithful).
Landed: 2026-07-24 — new `Condition::CastFromHand` (source-object-based, same shape as
`ColorWasSpentToCastThis`/`SourceUntapped`): `Permanent` gained a `cast_from_hand: bool` field,
copied from the existing `Spell::cast_from_hand` at the one `Event::PermanentEntered` apply site
(the "read it before the spell is gone" idiom `spent_colors` already uses); every other
permanent-entry path (`fresh_permanent`'s default) correctly defaults it `false`. Read via
`Game::ability_condition_holds`, same wrapper `SourceUntapped`/`ThisPermanentEnteredUntapped` use.
Also added `ControlEffect::TapAll { filter }` (the tap-side mirror of the existing `UntapAll`) for
Dread's second ETB step. Reiver Demon's mass-destroy filter needed no changes — #6's `exclude`/
`not_color` axes already expressed "nonartifact, nonblack" — it only needed the new condition
wired onto its existing `[[abilities]]` block. Corrected against Scryfall: Dread Cacodemon taps
"all **other** creatures you control" (not "all creatures," as this sketch originally assumed) —
`tap_all`'s filter uses the existing `other = true` axis, so Dread never taps itself.

Sketch: `[abilities.condition]` (§5 intervening-if) gains `cast_from_hand = true` — true only when
this permanent's last cast was from the hand zone (engine already tracks cast source for
`uncounterable`/impulse). Dread Cacodemon then = destroy_all `{ types="creature",
controller="opponents" }` + `tap_all { controller="you", except_self=true }`. Reiver Demon =
destroy_all `{ types="creature", color_not=["artifact-ish"], ... }` — folds into #6.

## 3. Resolution-scoped "for each … this way" count formulas — M — **LANDED**

Depends on: nothing.
Cards: **Congregate**, **Syphon Mind**, **Syphon Flesh**, **Malfegor** (all faithful).
Landed: 2026-07-24 — `ChoiceEffect::EachOpponentDiscards` (APNAP fan-out, empty hands skipped)
and choiceless `ChoiceEffect::DiscardYourHand` both tally
`ResolutionFrame::cards_discarded_this_way`; `EachPlayerSacrifices` gained a `count: Amount`;
`ResolutionFrame::creatures_sacrificed_this_way` + its `Amount` arm feed Syphon Flesh's Zombie
mint; Congregate uses the existing `per_creature_on_battlefield` amount with `times = 2`. A
`ponytail:` note on `DiscardYourHand` marks the future `Effect::Discard` family promotion trigger
(promote once a 2nd/3rd choiceless discard card arrives).

Sketch: add amount/count formulas read from the current resolution's running tally:
`per_creature_on_battlefield` already exists → let `gain_life`/`target_player_gains_life` accept a
`{ formula = "per_creature_on_battlefield", times = 2 }` (Congregate). Add
`per_card_discarded_this_way` (Syphon Mind: each opp discards 1, then `draw_cards` that many) and
`per_creature_sacrificed_this_way` (Syphon Flesh: `each_player_sacrifices` then `create_token`
that many; Malfegor: `discard` hand then `each_player_sacrifices` scope=each_opponent, count =
cards discarded this way). Thread a per-resolution counter on the effect-eval context.

## 4. Protection from a chosen color — M — **LANDED (Voice of All, Mother of Runes, Bathe in Light)**

Depends on: nothing (extends `choose_color` + the protection keyword).
Cards: **Voice of All** (faithful), **Mother of Runes** (faithful), **Bathe in Light** (faithful as of #5's radiance batch).

Sketch: protection value gains a `"chosen_color"` sentinel resolved against a color the object
chose. Voice of All: `choose_color` as-enters, store on the permanent, protection reads it
(static, permanent). Mother of Runes: `{T}:` grant target-you-control protection-from-chosen
until EOT (a pump-style until-EOT protection grant that takes a color choice at activation).
Bathe in Light's batch grant landed with #5.

## 5. Radiance targeting — M — **LANDED**

Depends on: #4 (Bathe grants chosen-color protection).
Cards: **Bathe in Light** (faithful), **Cleansing Beam** (faithful).
Landed: 2026-07-25 — one shared `Game::radiance_batch(target)` helper (`characteristics.rs`): the
target plus every other battlefield creature sharing ≥1 color with it (`Game::colors_of`); a
colorless target's batch is itself alone. `TargetSpec` is untouched — radiance still targets one
creature; the batch expansion happens only at resolution. Cleansing Beam: new
`DamageEffect::Radiance` (mint arm mirrors `EachCreature`'s per-creature protection/Tajic/Phantom
Centaur checks, minus the axes this card doesn't need). Bathe in Light: new
`PumpEffect::RadianceChosenColorProtectionUntilEndOfTurn`, the batch-capable twin of #4's
`GrantChosenColorProtectionUntilEndOfTurn`. Along the way, fixed a real gap the first
instant/sorcery `choose_color` card exposed: `Event::ColorChosen` assumed its `source` was always
a permanent (true for every existing `choose_color` card — Flickering Ward, Wild Mongrel, Mother
of Runes, Voice of All — since all are self/permanent-sourced), but Bathe in Light's choice is
made *by the spell itself* mid-resolution, which isn't a permanent. Added
`Spell::chosen_color` (the spell-side twin of `Permanent::chosen_color`) and
`Game::chosen_color_of` (reads whichever the object is), and pointed `Event::ColorChosen`'s apply
arm and the new pump arm's read at it.

Sketch: a target mode `radiance = { by = "color" }` — one chosen target creature plus every other
creature sharing a color with it, resolved at resolution. Cleansing Beam = `damage_each` over that
batch (2 damage). Bathe in Light = until-EOT protection-from-chosen grant over that batch (#4).

## 6. Color-/controller-filtered mass damage & destroy — S — **LANDED**

Depends on: nothing.
Cards: **Oros, the Avenger** (faithful), **Reiver Demon** (faithful as of #2's cast-from-hand
intervening-if).
Landed: 2026-07-24 — the `not_color` / `controller` filter axes this sketch called for were
already implemented (`PermanentFilter::color: ColorFilter::NotColor`, `FilterController::Opponent`
in `filter.rs`/`de.rs`/`query.rs`), just unused by any card; no list-form negation was needed
since Reiver's "nonartifact, nonblack" is `exclude = "artifact"` + `not_color = "black"`, two
separate existing axes. The real gap was `Effect::DestroyAll` never checking regeneration shields
at all (CR 701.15) — added a `cant_be_regenerated: bool` field (mirroring `DestroyTarget`'s) plus
shield-consulting logic in `resolution/destroy.rs`, which also makes Winds of Rath's "can't be
regenerated" clause mechanically real for the first time.

Sketch: `damage_each_creature` / `destroy_all` filters gain `color_not = "white"` and
`color_not = ["artifact","black"]`-style negation plus `controller = "opponents"`. Oros: combat-
damage-to-player trigger, `may_pay {2}{W}` reflexive, then `damage_each_creature { amount=3,
color_not="white" }`.

## 7. Multikicker — M — **LANDED**

Depends on: nothing (extends the kicker cost machinery, §2 `[cost.additional.kicker]`).
Cards: **Lightkeeper of Emeria**, **Comet Storm** (faithful).
Landed: 2026-07-25 — both cards, faithfully, on new general Multikicker (CR 702.33c) machinery
mirroring Replicate's own shape: `[cost.additional.multikicker]` (a `[cost]`-shaped table),
`Intent::Cast.multikicker_count` (how many times the caster paid it — each payment a full extra
instance of the cost, like Replicate, not Strive's "beyond the first"), and
`Game::spell_multikicker_count` for effects to read the declared count back. Lightkeeper of
Emeria's ETB ("gain 2 life for each time it was kicked") uses the existing `Amount::Scaled { times,
per }` with a new base keyword `"spell_multikicker_count"`. Comet Storm's "choose any target, then
choose another target for each time this spell was kicked" needed a new `TargetCount` flag,
`multikicker_scaled`, structurally identical to `strive_scaled`/`sacrifice_scaled` except its
cast-time substitution is "1 + N" rather than "exactly N" — the actual "harder half" turned out to
be no harder than adding that one arithmetic difference, not a variable-target subsystem. One real
gap surfaced and was fixed rather than approximated: `Game::spell_multikicker_count` (like its
`spell_sacrifice_count`/`spell_was_kicked`/`spell_strive_count`/`spell_replicate_count` siblings)
only read `Object::Spell`, so Lightkeeper's ETB trigger — which resolves *after* the spell has
become the permanent — always saw 0. Fixed with the same "read it before the spell is gone" idiom
`Permanent::entered_with_x` already uses for `{X}`: a new `Permanent::entered_multikicker_count`
field, copied over at `Event::PermanentEntered` alongside `entered_with_x`, with
`spell_multikicker_count` falling back to it. Verified against Scryfall (the `cmd` print for both):
oracle text, oracle id, and print id all matched with no discrepancies. 7 new engine tests (the
declared count's mana fold for each card, the ETB life gain kicked and unkicked, Comet Storm's
1 + N targets each taking the full undivided X, its rejection when the declared count outruns the
legal targets, and the "no Multikicker to pay" guard); no regression.

Sketch: `[cost.additional.multikicker]` payable any number of times; expose a `kicked_count` the
effects read. Lightkeeper: `gain_life { amount = { per_kick = 2 } }`. Comet Storm: `{X}{R}{R}`,
choose a target then one more per kick, X damage to each — needs a variable target count driven by
`kicked_count` (the harder half; stage Comet Storm after Lightkeeper).

## 8. Kicked / main-phase conditional extra target & amount — M — **LANDED**

Depends on: nothing.
Cards: **Orim's Thunder**, **Return to Dust**, **Sulfurous Blast** (faithful).
Landed: 2026-07-25 — all three cards, faithfully, on two new independent `TargetCount` flags and
one new `Amount` variant, all siblings of Multikicker's own shape (§7). `kicked_scaled` (Orim's
Thunder, CR 702.33g) is a second, wholly independent target clause that only exists when kicked —
unlike `multikicker_scaled`'s single clause that grows, an unkicked cast forces the clause to
`(0, 0)`, not down to a shared minimum, since the two clauses target unrelated things (an
artifact/enchantment, then a creature). `main_phase_scaled` (Return to Dust; CR 601.2c governs the
mechanism, no card-specific subrule exists for "cast during your main phase" conditionality) is the
opposite false-case shape on a *single* clause: cast outside the caster's main phase, `max` caps
down to `min` instead of zeroing, since the optional second target is the *same* clause as the
mandatory first — CR 601.2c's same-clause distinctness gives the oracle's "other" for free.
Sulfurous Blast's "cast during your main phase... instead" bonus is a new `Amount::
IfSpellCastDuringMainPhase { then, else_ }`, deserialized from `{ if_main_phase = <Amount>, else =
<Amount> }`, mirroring `IfSpellKicked`/`if_kicked` exactly, including the same pre-substitution
requirement in `DamageEffect::EachCreature` (a real gap this surfaced: without pre-resolving the
spell-level flag against the true source before per-creature substitution swaps `source` for each
damaged creature, the `each_creature` clause silently always picked the "else" branch regardless of
actual cast timing — `each_player` was unaffected since its `source` is never swapped). All three
read new ambient `Spell` state computed identically at every real-cast site (mirroring
`cast_from_hand`): `Spell.cast_during_main_phase`, set from `active_player == controller &&
matches!(step, Main1 | Main2)`. Verified against Scryfall (the `cmd` print for all three): oracle
text, oracle id, and print id all matched the increments-doc sketch with no discrepancies. 6 new
engine tests (Orim's Thunder kicked/unkicked; Return to Dust in/outside main phase; Sulfurous Blast
in/outside main phase); no regression.

Sketch: two small pieces. (a) `cast_during_main_phase` amount/rider gate — Sulfurous Blast:
`damage_each { amount = { if_main_phase = 3, else = 2 } }`; Return to Dust: optional second target
enabled only when cast in a main phase. (b) kicked extra target — Orim's Thunder: base destroys
artifact/enchantment; a second (creature) target present only when kicked, `deal_damage
{ amount = "target_mana_value" }` to it.

## 9. Vivid land + storage land — S — **LANDED**

Depends on: nothing.
Cards: **Vivid Meadow**, **Molten Slagheap**.
Landed: 2026-07-25 — both cards, faithfully, with **zero engine changes**. The DSL surface this
sketch anticipated needing (`remove_counters`/`remove_counters_kind = "charge"` as an activation
cost, an `"any"` any-color `add_mana` credit, `remove_counters_x` + `repeat = "x"` for the
storage-land X-counters-cost → X-mana shape) already existed, built for the `mirror-mastery` deck's
cycle-mates **Vivid Crag**/**Vivid Creek**/**Vivid Grove** and **Fungal Reaches**.
Vivid Meadow (`crates/cards/data/vivid_meadow.toml`) is Vivid Crag's exact template with {W}
instead of {R}; Molten Slagheap (`crates/cards/data/molten_slagheap.toml`) is Fungal Reaches'
exact template with {B}/{R} instead of {R}/{G} — so the anticipated "ship Vivid Meadow, hold
Molten Slagheap" fallback wasn't needed; the X-counters-cost carve-out was already small and
already shipped. Verified against Scryfall (the precon's own Commander 2011 `cmd` print for both):
oracle text, oracle id, and print id all matched with no discrepancies. New regression
tests in `crates/engine/tests/game.rs`: `vivid_meadow_enters_tapped_with_two_charge_counters_and_taps_for_white`,
`vivid_meadow_removes_a_charge_counter_for_a_color_it_cannot_otherwise_produce` (the any-color
credit pays a {U} cost — a color the land's plain tap mode can never make),
`molten_slagheap_taps_for_colorless_and_stores_a_counter`,
`molten_slagheap_removes_x_storage_counters_for_x_mana_in_black_or_red`.

Sketch: Vivid Meadow — enters tapped with 2 charge counters (`enters_with_counters`), `{T}: W`,
plus an activated mana ability whose cost is `{ tap = true, remove_counters = { kind="charge",
n=1 } }` producing one mana of any color. Add `remove_counters` as an activation-cost term and an
`any_color` add_mana mode gated on it. Molten Slagheap — `{T}: C`; `{1},{T}:` add a storage
counter; `{1}, remove X storage counters:` add X mana in B/R. The X-counter-cost → X-mana output
is the storage-land shape (an `{X}`-in-activation-cost carve-out scoped to counter removal). If
the X carve-out proves large, ship Vivid Meadow (S) and hold Molten Slagheap with an
`approximates` note (no colored fixing missed if left as `{T}: C` + fixed 1-counter mode).

## 10. Activation-count sacrifice trigger — S — **LANDED**

Depends on: nothing.
Cards: **Dragon Whelp** (faithful).
Landed: 2026-07-25 — no new subsystem needed. `once_per_turn.activated`
(`Event::AbilityActivatedThisTurn`) already tallied per-turn activations, gated behind
`once_each_turn`-capped abilities; generalized to record every activation. `schedule_at_next_upkeep`'s
`fire_at` field already supported `Step::End` generically despite its name and sparse docs — it
just had no exerciser. Added one new `Condition::SourceActivatedThisTurnAtLeast { at_least }`,
special-cased at the `Effect::Conditional` resolve site alongside the other source-scoped
conditions, reading that same per-turn tally.

Sketch: track per-turn activation count of a named ability; a delayed
`schedule_sacrifice_at_next_end_step` fires once the count reaches 4. Reuse `nth_each_turn`
counting + the next-end-step scheduler analogous to `schedule_at_next_upkeep`.

## 11. Global combat-static anthems — L — **LANDED**

Depends on: nothing.
Cards: **Avatar of Slaughter**, **Razorjaw Oni**, **Basandra, Battle Seraph**.

Landed: 2026-07-25 — three disjoint riders, each its own smallest addition. Avatar: added
`all_players` to `StaticEffect::KeywordAnthem` (mirroring `Anthem`'s existing field) for "All
creatures have double strike", plus a new unit `StaticEffect::MustAttackEachCombat` with a plain
existence scan and a dedicated loop in `Game::declare_attackers` for "... and attack each combat
if able" — architecturally distinct from the existing `must_attack` (Ruhan-style specific-opponent)
machinery since Avatar names no required defender. Razorjaw Oni: new
`StaticEffect::CantBlockFilter { filter }`, scanned battlefield-wide (any controller) and consulted
in `Game::can_block` for "Black creatures can't block" — `filter = { color = "black" }` needed no
`PermanentFilter` changes. Basandra, Battle Seraph: new unit `StaticEffect::CantCastDuringCombat`
(global existence check consulted at the top of `Game::cast_timing_ok`, ahead of the instant-speed
bypass) for "Players can't cast spells during combat", plus a previously-undocumented third ability
this sketch omitted — `{R}: Target creature attacks this turn if able.` — landed as a new unit
`MiscEffect::MustAttackTarget` (`TargetSpec::Creature`) that reuses the existing
`Event::MustAttackDeclared`/`declare_attackers` `must_attack` loop via a sentinel: recording the
target's own controller as `defender` reads as "must attack, any legal defender" (the loop's
`required_legal` gate already short-circuits when `required == player`). 9 new engine tests (3 per
rider); no regression.

Sketch: extend `anthem_static` scope to **all creatures** (not just you/opponents) and add
restriction statics: `must_attack_all` + `keywords=["double_strike"]` (Avatar); a
`cant_block { color="black" }` global (Razorjaw, checked in `Game::can_block`); a
`cant_cast_during_combat { who="all" }` static (Basandra) checked in `Game::cast`. Three disjoint
riders; land as riders across waves.

## 12. Per-player war/peace choice anthem — M — **LANDED**

Depends on: #11 (all-creatures anthem scope).
Cards: **Archangel of Strife**.

Landed: 2026-07-25 — new sticky `Player::chose_war: Option<bool>` (modeled on
`has_citys_blessing`) plus a `war_choice: Option<bool>` field on `StaticEffect::Anthem`, checked in
`matching_anthems` against the candidate's own seat. The as-enters choice reuses the
council's-dilemma fan-out wholesale: a new unit `ChoiceEffect::EachPlayerChoosesWarOrPeace` raises
the existing `PendingChoice::CastVote`/`Intent::ChooseMode` machinery (APNAP order, a `["war",
"peace"]` ballot) and `answer_vote` writes each answer straight to the voter's own `chose_war`
instead of tallying — no new wire types. Found and fixed a real staleness bug along the way: that
direct `Player` mutation isn't accompanied by an `Event`, so the centrally-invalidated
`characteristics_cache` never noticed the change; fixed with an explicit `invalidate_owner` call in
`answer_vote`, following `spawn.rs`'s existing precedent for cache invalidation outside the
`Event`-driven path. 2 new engine tests; no regression. Carries an `approximates`: the choice is an
as-enters replacement effect (CR 614.1c) but is modeled as an ordinary ETB trigger, so it goes on
the stack instead of being made as the Archangel enters — the engine has no as-enters *choice*
machinery yet. `chose_war` is one sticky value per seat, so a second Archangel would overwrite the
first's choices.

Sketch: as-enters, each player records a `war|peace` choice on themselves; two `anthem_static`s
keyed by that per-player flag (`+3/+0` to war players' creatures, `+0/+3` to peace players').

## 13. Conditional per-opponent attack/cast lockout — L — **LANDED**

Depends on: nothing.
Cards: **Angelic Arbiter**.

Landed: 2026-07-25 — Scryfall's oracle text is a blanket per-player restriction, not the
defender-scoped "can't attack you" this sketch guessed at: "Each opponent who cast a spell this
turn can't attack **with creatures**" / "Each opponent who attacked **with a creature** this turn
can't cast spells" — simpler than sketched, so no `PermanentFilter`/`CantBeAttackedBy` reuse was
needed. Landed as two bespoke unit statics mirroring #11's `CantCastDuringCombat`/
`MustAttackEachCombat` shape: `StaticEffect::CantAttackIfCastThisTurn` (checked in a new
`Game::cant_attack_if_cast_this_turn` against the existing `Player::spells_cast_this_turn` tally,
consulted in `Game::can_attack`) and `StaticEffect::CantCastIfAttackedThisTurn` (checked
against a new turn-scoped `Player::attacked_this_turn` flag, set by `Event::AttackerDeclared` and
reset at untap alongside the engine's other this-turn tallies, consulted at the top of
`Game::cast_timing_ok`). Both existence-scan helpers gate on "some other player controls the
static" for the CR "opponent" restriction. 6 new engine tests (both directions, the per-turn
reset, and that Arbiter's own controller is exempt); no regression.

The can't-attack ban lives in `Game::can_attack` rather than at the top of `declare_attackers`
because a restriction beats a requirement (CR 509.1a): `can_attack` is the single choke every
must-attack loop, the declaration legality check, and the `query.rs` affordance already read, so a
creature under the ban stops being "able" and #11's Avatar of Slaughter (or goad) stops demanding
it attack. Enforcing it only at the declaration deadlocked the step in both directions — attacking
banned, not attacking illegal — covered by a regression test pairing the two cards.

Sketch: two per-opponent, per-turn watched flags — "cast a spell this turn" and "attacked this
turn" — each gating the other action for that opponent (can't-attack static via
`Game::declare_attackers`, can't-cast via `Game::cast`), reset each turn.

## 14. Choose others' attackers/blockers — XL — **LANDED**

Depends on: nothing. Cards: **Master Warcraft** (faithful).
Landed: 2026-07-26 — landed as one wave rather than the sketch's slice-per-wave staging: the engine
test harness has no synthetic-instant builder, so an override can only be exercised through a real
card, and a Master Warcraft with half its text printed would have shipped unfaithful. Both halves
are the same shape — a turn-scoped seat override in `CombatExtras` (`attack_declarer` /
`block_declarer`), cleared at the next untap step like the combat-damage shields, and read through
one accessor each (`Game::attack_declarer`, `Game::block_declarer`) so the single choke
(`Game::declare_attackers` / `Game::declare_blockers`), the affordance list and the priority
auto-seal all route through it. The attack half rebinds to the active player immediately past the
gate, so every legality check below reads the creatures actually on offer. The block half needed
one new helper, `Game::block_seats_for` — with several declarations to displace (one per attacked
player), an overridden declaration is a single submission covering every attacked seat at once,
while each blocker is still checked against its own controller via the unchanged `Game::can_block`.
An override falls back to the ordinary declarer once the chosen seat has lost (CR 104.3a). The
timing gate is a new `cast_only_before_attackers` card flag mirroring `cast_only_during_combat`
(`Step` gained `Ord` for the window comparison — declaration order is turn order). 6 new engine
tests; no regression; no `approximates` needed.
Sketch: (a) "you choose which creatures attack this turn" — a controller override for the attack
declaration step; (b) "you choose which creatures block and how" — override the block declaration.
Cast-before-attackers timing gate. Each slice is a distinct priority-flow override; land one slice
per wave.

## 15. Search-denial + extra-turn skip static — M — **LANDED** (search-denial half)

Depends on: nothing. Cards: **Stranglehold**.
Landed: 2026-07-25 — `opponents_cant_search_libraries` (`StaticEffect`, no fields), checked in
`Game::opponent_search_denied` (`characteristics.rs`) against a live opposing `Timing::Static`
instance. The single choke every library search raises through is
`pending::raise::library::search_library`: a denied search returns `None` outright, so it neither
finds a card nor shuffles (CR 701.19f — the shuffle is tied to the search that didn't happen).
Fixed a latent regression the guard would otherwise introduce: a denied seat inside an
`AllPlayers` fan-out (Veteran Explorer) previously would have silently dropped the rest of the
fan-out, since its raise no longer pauses; `Game::continue_search_fanout` is now `pub(crate)` and
self-recursive so a denied seat walks straight to the next queued player instead. Checked the
whole card pool for an extra-turn source before falling back — none exists (no card grants an
extra turn and the engine has no extra-turn machinery), so "If an opponent would begin an extra
turn, that player skips that turn instead" is dropped and named in `approximates` per the
sketch's own fallback.
Sketch: a `opponents_cant_search` static (checked at every `search_library` entry for an opponent)
+ an `opponents_skip_extra_turns` static (checked where extra turns are granted). No extra-turn
source in this deck, so the second half is unobservable here — land the search-denial half and
name the extra-turn half in `approximates` unless an extra-turn source exists in pool.

## 16. Random reanimate from an opponent's graveyard — M — **LANDED**

Depends on: nothing. Cards: **Tariel, Reckoner of Souls** (faithful).
Landed: 2026-07-25 — new `ZoneEffect::ReanimateRandomFromTargetOpponentGraveyard { target }`
(`target = "opponent"`, a real `TargetSpec::OpponentPlayer` choice, not the random half). The
opponent's graveyard is filtered to creature cards and the pick made with the same injected
derive-per-op RNG `exile_random_from_graveyard_may_play` (Advanced Reconstruction) already uses —
resolves via `Game::run_misc_choreo` (needs `&mut self` for the RNG), reusing `reanimate_event`
(the same mint `reanimate_to_battlefield` calls) under the ability's controller. No-op on a
graveyard with no creature card. No new randomness source — reused the existing seeded
`with_op_rng`/`OpRng` path, keeping intent-replay determinism.

Sketch: `{T}:` choose a random creature card from **target opponent's** graveyard and
`reanimate_to_battlefield` it **under your control** — a random-pick variant of reanimate targeting
another player's graveyard.

## 17. Join forces — L — **LANDED**

Depends on: nothing. Cards: **Mana-Charged Dragon** (faithful).
Landed: 2026-07-25 — the sketch's "L" / "genuinely new interaction" rating was wrong: join-forces
machinery already existed end to end from Collective Voyage (`ChoiceEffect::JoinForcesPayMana` →
`PendingChoice::JoinForcesPayment` fan-out in turn order → `Amount::ManaPaidThisWay`) and needed no
engine changes to reuse from a triggered ability instead of a spell. The only genuinely new piece
was the trigger timing itself: a new `Trigger::AttacksOrBlocks` (`de.rs` `TriggerTag`, TOML
`"attacks_or_blocks"`), since the existing `blocks_or_becomes_blocked` also fires on the *attacker*
side of a block (wrong here — "attacks or blocks" only cares about the *blocker* half). The attack
half rides alongside `Trigger::Attacks` off `Event::AttackerDeclared`; the block half is a new
batch-scan (`Game::queue_attacks_or_blocks_block_triggers`) called from `Game::declare_blockers`,
scoped to the blocker side only and deduped like the existing blocks-or-becomes-blocked scan. The
TOML itself is a two-step `[[abilities.effects]]` sequence — `join_forces_pay_mana` then
`pump_self_until_end_of_turn` reading `"mana_paid_this_way"` — already-existing DSL surface,
reused verbatim from Collective Voyage's shape. 5 new engine tests cover: the trigger firing on
attack, firing on block, *not* double-firing when the Dragon itself is blocked (it "becomes
blocked", not "blocks"), the +X/+0 total across three players' payments, and the no-payment case.
No regression; no `approximates` needed.
Sketch: a "join forces" prompt — each player in turn order, starting with you, may pay any amount
of mana; sum it into X. Attacks/blocks trigger → this creature gets +X/+0 EOT. New multi-player
mana-contribution pending-choice; a genuinely new interaction. Low priority (one card).

## 18. `intimidate` keyword — S — **LANDED**

Depends on: nothing. Cards: **Vow of Malice** (faithful).
Landed: 2026-07-25 — `Keyword::Intimidate` added beside `Keyword::Fear`, with the color-sharing
block check right after fear's in `Game::can_block` (`combat.rs`): an artifact blocker is always
legal, otherwise the blocker must share ≥1 color with the attacker (`colors_of` on both). Vow of
Malice is the existing vow-aura shape (`grant_to_attached` +2/+2 + `cant_attack_controller`) with
`intimidate` in the granted keywords. Wire projection (`schema/catalog.rs`) gained the
`intimidate` badge id + label.
Sketch: add `intimidate` (CR 702.13 — blockable only by artifact creatures and/or creatures
sharing a color) to the keyword enum, checked in `Game::can_block` (mirror `fear`). Vow of Malice
= existing vow aura (+2/+2, can't-attack-you vow counter) with `intimidate` in the granted
keywords.

## 19. Land with morph — S — **LANDED**

Depends on: nothing (morph exists). Cards: **Zoetic Cavern** (faithful).
Landed: 2026-07-25 — the face-down/land duality needed plumbing after all: `resolve_spell`
(`effects.rs`) panicked resolving a face-down spell whose hidden real `kind` is `Land` (only
`bestowed` had a kind override; added a matching `face_down` override to the flat 2/2 creature
CR 708.2 shape). The turn-face-up creature-only gate (`cast.rs::turn_face_up`,
`query.rs::turn_face_up_listable`) turned out to be manifest's restriction (CR 701.34e) wrongly
applied to morph too, which has no such restriction (CR 702.37c) — split the guard on
`perm.def.morph.is_none()`. Mana-tap paths reading `CardKind::Land { produces }` off `def.kind`
directly (`taps_for_mana`, `tap_for_mana`, both `available_mana` loops, `auto_tap_candidates` in
`priority.rs`) didn't check `face_down`, which would've let a disguised 2/2 tap for {C} while
hidden (CR 708.2: no abilities while face down) — added a `face_down` guard to each.
Sketch: confirm a face-down 2/2 can be a card whose face-up side is a Land (`{T}: C`). Morph cast
path already mints a 2/2; the face-up turn reveals the land. If the face-down/land duality needs
plumbing, it's small; else Zoetic Cavern is nearly section-C.

## 20. "Whenever a creature attacks" watch-others trigger — S — **LANDED**

Depends on: nothing. Cards: **Righteous Cause** (faithful).
Landed: 2026-07-25 — new `Trigger::CreatureAttacks` (`timing = "creature_attacks"`), fired once per
attacker declared this combat, any controller, any defender. `Game::queue_batch_attack_triggers`
gained a fourth pass after its existing per-watcher count-trigger loop: an outer loop over the
committed `attackers` set, inner loop over the battlefield, enqueuing one `TriggerGroup` per
attacker for each watcher's controller — reuses the same batch-scan entry point
(`Game::declare_attackers`) the count triggers already ride, no new dispatch subsystem. Righteous
Cause pairs it with the existing untargeted `gain_life` amount-1 effect.
Sketch: a new `timing = "creature_attacks"` trigger — fires once per attacker declared this combat,
any controller, any defender (contrast the existing `"attacks"` = this creature, `"player_attacks_
your_opponent"`, `"you_attack_with_creatures"`). Batch-scan the committed attacker set from
`Game::declare_attackers` (like `place_vow_counters`/goad read it), enqueueing one trigger per
attacker for the ability's controller. Righteous Cause pairs it with an untargeted `gain_life`
amount 1. Reclassified out of section C: no attacker-declaration watch reaches "a creature" today.

## 21. ETB reanimate-a-Dragon + all-controllers subtype pump until EOT — L — **LANDED**

Depends on: nothing. Cards: **Bladewing the Risen** (faithful).
Landed: 2026-07-25 — confirmed the sketch's gap: `PumpEffect::PumpCreaturesYouControlUntilEndOfTurn`
hardcodes `controller_of(id) == controller` on top of its own `filter`, so no existing pump mode
could go board-wide. Added `PumpEffect::PumpEachCreatureUntilEndOfTurn` (mode
`pump_each_creature_until_end_of_turn`), the board-wide twin that drops that gate — every creature
on the battlefield matching `filter`, any controller. Also added `CardFilter::PermanentWithSubtype`
(`permanent_with_subtype`) for the ETB's graveyard target. Scryfall's Oracle text reads "target
Dragon **permanent** card" from "**your** graveyard", not "creature card" from "a graveyard" as
sketched — the TOML targets `card_in_graveyard = { whose = "yours", filter = { permanent_with_subtype = ["Dragon"] } }`.
Sketch: two abilities. (a) ETB `reanimate_to_battlefield` **target Dragon creature card** from a
graveyard (existing reanimate, add a subtype filter on the target). (b) `{B}{R}:` "Dragon
creatures get +1/+1 until end of turn" — a *board-wide, every-controller* subtype pump EOT (not
"you control"), which the temp-boost path can't express yet: today's until-EOT pumps target one
object or one controller's creatures, not "each Dragon on the battlefield regardless of
controller." Add an all-controllers subtype-filtered temp-boost.

## 22. End-step no-creatures self-sac + any-player "beginning of the end step" timing — L — **LANDED**

Depends on: nothing.
Cards: **Pyrohemia** (faithful).
Landed: 2026-07-25 — gap (a) needed no work: `Trigger::EachEndStep` already existed
(`timing = "each_end_step"`, landed for Relic Retriever, fidelity increment elsewhere) and already
fires on every player's end step, not just the controller's. Gap (b) reused
`Game::creatures_on_battlefield` (already backing `Amount::PerCreatureOnBattlefield`) behind a new
`Condition::NoCreaturesOnBattlefield`, paired both as `[abilities.condition]` (CR 603.4's
placement-time check) and nested in a `{ type = "conditional", … }` step (the resolution-time
recheck), mirroring `source_untapped`/Howling Mine's existing double-check shape. Scryfall's
current Oracle text reads "At the beginning of the end step" (pre-templating wording, not "each
end step") — CR-equivalent since a turn has exactly one end step and the ability isn't restricted
to "your" end step, so it maps onto `each_end_step` like Relic Retriever.

Sketch: Pyrohemia is `{R}:` "deals 1 damage to each creature and each player" (expressible —
`damage_each_creature` + `damage_each_player`) plus "At the beginning of each end step, if there
are no creatures on the battlefield, sacrifice Pyrohemia." Two engine gaps: (a) a
`beginning_of_each_end_step` trigger timing that fires on **every** player's end step, not just the
controller's; (b) an intervening-if condition `no_creatures_on_battlefield`. The activated damage
half is fine — the trigger timing + board-empty condition is the engine work.

## 23. `put_counters` -1/-1 misses the P/T cache invalidation — S — **LANDED**

Depends on: nothing. Cards: **Gwyllion Hedge-Mage** (now faithful in section C).
Landed: 2026-07-24 — added `Event::KindCountersPlaced { object, .. }` to the first arm of
`Game::invalidate_characteristics_cache` (`characteristics_cache.rs`), alongside
`Event::CountersPlaced`.
Sketch: `put_counters kind = "minus_one_minus_one"` on an already-on-battlefield creature shrinks
power but not toughness (a 2/2 → 1/2). Root cause: `Event::KindCountersPlaced` is missing from
`Game::invalidate_characteristics_cache` (`characteristics_cache.rs`), so the target's cached
toughness stays stale — power only looks right because nothing had cached it. `pt_layers` already
subtracts a `-1/-1` counter from both axes symmetrically; the fix is one line — add
`Event::KindCountersPlaced { object, .. }` to the first invalidation arm alongside
`Event::CountersPlaced`. (Wickerbough Elder's enters-with-counters path escapes the bug only
because its ETB fires `PermanentEntered` → board-wide invalidation.) Gwyllion's other half — 2+
Plains → mint a 1/1 Kithkin Soldier — is fully expressible (a Kithkin Soldier token profile was
drafted, re-add it with the card); the whole card lands the moment the cache line does.

## 24. A post-cast-clause spell advertises a cast-time target it can't accept — S — **LANDED**

Depends on: nothing. Cards: **Return to Dust**, **Orim's Thunder** (both already faithful, #8).
Landed: 2026-07-25 — `Game::target_spec_of` now returns `TargetSpec::None` when
`spell_multi_target` is `Some`, the guard `Game::split_half_cast_targets` already carried.
Sketch: found by the Phase 6 live drive — 2879 `reject.illegal_target` acks on two cards. A spell
whose targets are chosen after it is on the stack (CR 601.2c, the `ChooseSpellTargets` pause) was
projected into `ActionView.needs_target: true`, because `target_spec_of` read the effect's
`TargetSpec` without asking whether the clause is deferred. `validate_cast` rejects any target on
such a cast intent, so the board staged a target click that could only bounce — every
`main_phase_scaled` / `kicked_scaled` / `strive_scaled` / multi-clause spell in the pool.
`Game::legal_targets` keeps its pre-cast enumeration (which creatures Twinflame could copy at all);
only the cast-time *requirement* changed.

## 25. Equip lists opponents' creatures as legal targets — S — **LANDED**

Depends on: nothing. Cards: **Lightning Greaves** (already faithful, section A), every Equipment.
Landed: 2026-07-26 — `Effect::Control(ControlEffect::Equip)` now reports
`TargetSpec::CreatureYouControl` instead of `TargetSpec::Creature`.
Sketch: found by the same Phase 6 drive as #24 — after that fix, the *whole* remaining reject
tally (2836 `reject.illegal_target`) was one card: Lightning Greaves' equip, 12 bounced targets per
activation. Equip's activation gate already enforced "target creature you control" (CR 702.6e,
`cast.rs`), but the effect's `TargetSpec` said any creature, so `Game::legal_targets` — the single
enumeration the client highlights from — offered the three opponents' creatures for a click the
gate could only reject. Same defect class as #24: the advertisement was wider than the gate.

## 26. A zero-count target clause pauses on a choice nothing can answer — S — **LANDED**

Depends on: nothing. Cards: **Orim's Thunder** cast unkicked (#8); every `kicked_scaled` /
`x_scaled` / `sacrifice_scaled` clause that settles to zero (Immoral Bargain and Silkguard at
X=0, Run the Play at X=0).
Landed: 2026-07-26 — the forced-auto-fill branch in `Game::advance_spell_target_clauses`
(`cast.rs`) now also covers `hi == 0`, filling the clause with `&legal[..hi]` (none) instead of
raising the pause.
Sketch: found by the Phase 6 drive after #24/#25 — the whole remaining reject tally was
8 `reject.illegal_choice` on one spell, a `ChooseSpellTargets { min: 0, max: 0, legal: [7
creatures] }`. "Choose zero targets" has exactly one answer, so it is a forced clause, not a
choice; pausing on it prompted the player for a pick the handler could only reject. Same defect
class as #24/#25 a third time — the advertisement was wider than the gate.

## 27. A `PayCost` prompt offers a payment the player cannot make — M — **DONE**

Depends on: nothing. Cards: **Oros, the Avenger** ("you may pay {2}{W}"), every optional paid
trigger.
Sketch: the last reject standing in the Phase 6 drive (1 `reject.cannot_pay_cost`). The prompt is
raised whether or not the cost is payable, and `PendingChoiceView::PayCost` carried no
affordability flag, so the client showed a live "Pay {2}{W}" button that did nothing when clicked
(`pay_optional_cost` deliberately leaves the choice pending so the player can still decline).
Same class as #24–#26 — the advertisement was wider than the gate.
Done: `Game::can_pay_cost` wraps `plan_auto_taps`, the same planner `settle_payment` runs, so the
flag and the gate cannot disagree; `PendingChoiceView::PayCost` carries `can_pay` through to the
client, which disables Pay when it is false. The sketch's premise that "`settle_payment` auto-taps
**lands** only" was wrong — `plan_auto_taps` already plans paid tap-for-mana abilities (signets,
filter lands, karoos). The real ceiling is sacrifice-cost sources (cracking a Treasure), which the
planner does not model; the flag re-evaluates live, so it flips true the moment the player floats
that mana by hand.

## 28. Archangel of Strife's war/peace answers are one value per seat — S — **DONE**

Depends on: nothing. Cards: **Archangel of Strife**.
Sketch: `Player::chose_war` was a single `Option<bool>` per seat, so a second Archangel of Strife
overwrote every player's answer to the first. CR 614.12 makes the choice once **per permanent** —
each copy asks again, and each copy's own anthems read that copy's answers.
Done: `Player::war_choices: Vec<(ObjectId, bool)>` — one entry per (asking Archangel, ballot).
`answer_vote` pushes against the ballot's own `source`; the anthem scan in `characteristics.rs`
reads with the anthem's own `source`, so a seat that chose war for one copy and peace for another
collects both buffs. Regression test:
`two_archangels_of_strife_each_track_their_own_war_peace_answers`.

## 29. "As ~ enters, choose …" is modeled as an ETB trigger — M — **DONE**

Depends on: nothing. Cards: **Archangel of Strife**, **Flickering Ward**, **Patchwork Banner**,
**Voice of All**.
Sketch: all four expressed a CR 614.12 replacement effect as `timing = "etb"`, so the choice went
on the stack: there was an observable priority window during which the permanent was on the
battlefield with no color / no creature type / neither anthem chosen yet. Only Archangel declared
it in `approximates`; the other three were silent, which the "silence = faithful" rule forbids.
Done: a new `Trigger::AsEnters` (TOML `timing = "as_enters"`), watched off the same entry events as
`Etb` and queued ahead of it. `Game::place_pending_triggers` runs such a group **inline** via
`Game::run` instead of placing it — a choice it raises returns, and the post-intent pipeline
re-enters placement once answered. All four scripts retargeted; Archangel's `approximates` is gone
(#28 retired its other half). Regression test:
`voice_of_all_chooses_its_color_as_it_enters_rather_than_on_the_stack` asserts an empty stack and
an already-pending `ChooseColor` the instant it enters.
Ceiling (`ponytail:` in `triggers.rs`): state-based actions run one pipeline phase ahead of trigger
placement, so the choice lands a sweep later than CR 704.3 strictly wants. Unobservable for every
as-enters card in the pool — their payoffs only add stats or grant protection, and nothing between
the two sweeps gets priority.
