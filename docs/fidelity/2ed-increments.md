# Unlimited Edition (`2ed`) increments (2026-07-27)

Set report: [2ed.md](2ed.md). This file is the sole engine-capability backlog for `2ed`
(ranked increments + per-card exotics). Numbering is local to this file.

This is a **set** grind, not a deck grind — intake is Scryfall `set:2ed unique:cards`, not an
Archidekt link, and there is no precon to ship at the end. 292 unique cards: 28 already in the
pool, 134 authorable today, 127 blocked here, 4 out of scope.

Ranked S-first within dependency order. The centre of gravity is three clusters that between
them gate 51 cards:

- **Damage prevention** (#4, #5, #6) — 1993 white and artifact are built almost entirely out of
  prevention shields, and the engine has none. 18 cards.
- **Continuous characteristic-defining values** (#1, #2) — `*/*` creatures and "damage equal to
  the number of Swamps" both want a permanent count as a live amount. 11 cards.
- **Untap-step manipulation** (#7, #18) — Stasis, Winter Orb, Mana Vault, Time Vault. 10 cards.

Below those, 1993 leans hard on combat rules the engine skips entirely: banding (#14), landwalk
(#3), and block restriction/requirement (#11).

### Observability re-audit

Six pool-absence claims in `crates/engine/src` are falsified by 2ed. Each is folded into the
increment that clears it:

| Claim | Where | Falsified by | Increment |
| --- | --- | --- | --- |
| "the pool's own creature types … widen this when a card needs a type not yet printed" | `types/stack.rs` `CREATURE_TYPES` | 21 types (Wall, Unicorn, Pegasus, Specter, Wraith, Shade, Nightmare, …) | #27 |
| "a color filter … grows from a real card that wants one" | `types/filter.rs` `SpellFilter` | Blue/Red Elemental Blast, Deathgrip, Lifeforce, Gloom | #9 |
| "no card in the pool changes a permanent's name (CR 707.9), so a copiable-name-vs-current-name distinction doesn't arise" | `types/filter.rs` `PermanentFilter::name` | Clone, Vesuvan Doppelganger, Copy Artifact | #12 |
| "a fixed slot array sized to exactly what the pool's cards consume (charge, story, …)" | `types/effect/shared.rs` | +1/+0 (Clockwork Beast), corpse (Scavenging Ghoul), mire (Cyclopean Tomb), vitality (Living Artifact) | #28 |
| "a single bool covers the pool's one keyword-exclusion need … generalize to a `without_keyword` if a second turns up" | `types/filter.rs` `PermanentFilter::without_flying` | "non-Wall creature" ×4 (Cockatrice, Thicket Basilisk, Keldon Warlord, Siren's Call, Nettling Imp) — subtype exclusion, same shape | #21 |
| "no pool card needs a *fixed* range yet" | `de.rs` | not falsified — left as is | — |

No `approximates` field in `crates/cards/data` rests on a 2ed absence: every 2ed card already in
the pool is faithful, so section B is empty.

---

### 1. `permanent-count-amount` — 7 cards, M
Depends on: nothing.
`Amount` can count creatures you control, creatures that died, cards in a hand — but it cannot
count *permanents matching a filter that a given player controls*. Every "equal to the number of
Swamps" card in 2ed wants exactly that, and #2 is built on top of it. *Sketch:* an
`Amount::PermanentsControlled { filter: PermanentFilter, who: Who }` variant resolved in
`amount.rs` through the existing `Game::permanent_matches`, with `who` reusing the
`This`/`TargetPlayer`/`EachPlayer` axis the damage effects already thread. Karma and Power Surge
resolve it per-player inside an each-upkeep trigger, so it must be evaluated at resolution
against the *triggering* player, not the source's controller. Power Surge additionally needs the
count snapshotted at the beginning of the turn rather than read live — a turn-scoped
`untapped_lands_at_turn_start` per player, set in the Untap step.
*Cards:* gaea_s_liege, karma, keldon_warlord, nightmare, plague_rats, power_surge,
volcanic_eruption.

### 2. `characteristic-defining-power-toughness` — 6 cards, M
Depends on: #1.
`*/*` creatures. The pool has `set_own_base_pt_from_amount` (Trench Gorger), but that is a
one-shot resolution effect that writes a fixed number; a CDA (CR 604.3) is continuous — it
re-reads its count every time characteristics are computed, in layer 7a, before any other P/T
effect. Nightmare gaining a Swamp mid-combat must grow immediately. *Sketch:* a
`StaticEffect::BasePowerToughnessFromAmount { amount }` read by `characteristics.rs`'s existing
base-P/T resolution *before* the layer-7b pump pass, with `characteristics_cache.rs` invalidating
on any battlefield change (the amounts are all permanent counts, so the existing
permanent-entered/left invalidation hooks cover it). Gaea's Liege switches its count on whether
it is attacking, so the CDA amount is evaluated in combat context. Aspect of Wolf and Animate
Artifact are the Aura form — the same static, scoped to `enchanted_host` rather than self, which
`set_attached_base_pt` (Darksteel Mutation) already has a scope for.
*Cards:* animate_artifact, aspect_of_wolf, gaea_s_liege, keldon_warlord, nightmare, plague_rats.

### 3. `landwalk` — 5 cards, S — **done**
Depends on: nothing.
Plains-/island-/swamp-/mountain-/forestwalk (CR 702.14) — an evasion keyword parameterised by a
basic land type. The `Keyword` enum has no parameterised variant except `Protection(Color)`, so
this follows that shape exactly. *Sketch:* `Keyword::Landwalk(BasicLandType)`; the blocking
legality check in combat gains one arm — a creature with landwalk can't be blocked if the
defending player controls a land with that type (read from `CardKind::Land::subtypes`, the
rules-relevant list, not `CardDef::subtypes`). Grants come free: Goblin King, Lord of Atlantis,
and Zombie Master use the existing `keyword_anthem` static, Burrowing uses `grant_to_attached`.
*Landed:* `Keyword::Landwalk(BasicLandType)`, one arm in `Game::can_block` reading the
*defending* player's printed land subtypes through the existing
`Game::lands_with_subtype_controlled`. Printed, Aura-granted, and lord-granted landwalk all fell
out of shapes that already existed. Two of the seven cards did not fit after all and moved to
their own increments: Island Sanctuary (#65) and Zombie Master (#66).
*Cards:* bog_wraith, burrowing, goblin_king, lord_of_atlantis, shanodin_dryads.

### 4. `damage-prevention-shields` — 13 cards, M
Depends on: nothing.
"Prevent the next N damage that would be dealt to any target this turn" (CR 615). The engine has
prevention *statics* — `prevent_all_combat_damage_this_turn` (Fog), `prevent_combat_damage`,
`prevent_damage_to_self_removing_counter` (Phantom Centaur) — but no consumable shield: a
turn-scoped counter attached to a permanent or player that damage decrements as it is dealt.
Nine white/artifact cards in 2ed are this one shape. *Sketch:* a turn-scoped
`Vec<PreventionShield { target: Target, amount: u32, source_filter: Option<…> }>` on `Game`,
cleared at cleanup alongside the other until-end-of-turn state, consulted in the single
`deal_damage` path (all damage already funnels through it, so this is one guard, not one per
caller) and decremented in place. Forcefield's "prevent all but 1" and Rock Hydra's
per-1-damage counter removal are the same consumption hook with different arithmetic. Guardian
Angel's repeatable top-up ("you may pay {1} … prevent the next 1") adds to an existing shield.
*Cards:* circle_of_protection_black, circle_of_protection_blue, circle_of_protection_green,
circle_of_protection_red, circle_of_protection_white, conservator, forcefield, guardian_angel,
healing_salve, power_leak, reverse_damage, rock_hydra, samite_healer.

### 5. `source-of-your-choice-prevention` — 6 cards, M
Depends on: #4, #9.
The Circle of Protection cycle. Beyond #4 this needs the shield to be keyed to a *source chosen
on activation* rather than the damaged object — "the next time a black source of your choice
would deal damage to you." *Sketch:* `PendingChoice::ChooseDamageSource` offering every object
matching a `ColorFilter` (from #9) that could deal damage — battlefield permanents plus objects
on the stack, since a source need not be a permanent (CR 609.7) — recorded as the shield's
`source_filter`. Reverse Damage is the same shield with a life-gain rider on consumption, so the
shield needs to report how much it prevented.
*Cards:* circle_of_protection_black, circle_of_protection_blue, circle_of_protection_green,
circle_of_protection_red, circle_of_protection_white, reverse_damage.

### 6. `damage-redirection` — 3 cards, M
Depends on: #4.
"That source deals that damage to you instead" (CR 615.10). A redirection is a replacement, not a
prevention, but it hangs off the same hook in `deal_damage` and must not loop. *Sketch:* the
shield record gains a `redirect_to: Option<Target>`; on consumption the damage event is re-issued
against the new target with a recursion guard (one redirect per damage event, CR 616.1). Veteran
Bodyguard is the static form — "all damage that would be dealt to you by unblocked creatures is
dealt to this creature instead", conditioned on the bodyguard being untapped, so it is a
`StaticEffect` scanned at damage time rather than an activated shield.
*Cards:* jade_monolith, personal_incarnation, veteran_bodyguard.

### 7. `untap-step-restrictions` — 9 cards, M
Depends on: nothing.
"Doesn't untap during your untap step" / "players skip their untap steps" / "can't untap more
than one." The untap step currently untaps everything a player controls unconditionally.
*Sketch:* the untap step consults three new scanners before building its untap set — a per-
permanent `doesnt_untap` (printed static: Mana Vault, Basalt Monolith, Time Vault; granted:
Paralyze; conditional: Meekstone's power ≥ 3), a per-player `skips_untap_step` (Stasis), and a
per-player cap on how many permanents of a filter may untap (Smoke: one creature; Winter Orb: one
land, and only while Winter Orb is untapped). The cap needs a `PendingChoice::ChooseUntapSet`
because the player picks which one. `DeclineUntap` already exists as a pending choice, so the
step already knows how to pause. Instill Energy is the inverse — an *extra* untap outside the
step, which is an ordinary `untap_target` gated to once each turn.
*Cards:* basalt_monolith, instill_energy, mana_vault, meekstone, paralyze, smoke, stasis,
time_vault, winter_orb.

### 8. `basic-land-type-changing` — 7 cards, M
Depends on: nothing.
"Enchanted land is a Swamp" / "All Mountains are Plains" / "All Forests are 1/1 creatures that
are still lands." Layer 4 (type-changing) with a layer-7b rider in two cases. `set_attached_types`
exists (Darksteel Mutation) but overwrites the whole type line and doesn't touch land subtypes or
the mana abilities they imply. *Sketch:* a `StaticEffect::SetLandSubtypes { scope, subtypes,
replace: bool }` applied in `characteristics.rs` before mana-ability derivation, so a land that
becomes a Swamp taps for {B} and *loses* its old intrinsic ability (CR 305.7 — changing a land's
subtype replaces its mana abilities). Kormus Bell and Living Lands add the creature type and a
base P/T on top, which is #2's static in its non-CDA fixed form. Cyclopean Tomb's mire counters
key the change to a counter rather than an Aura, and its leaves-the-battlefield clause schedules
an unbounded series of upkeep triggers — model that as a delayed trigger that re-registers itself
until no mire counters it placed remain.
*Cards:* conversion, cyclopean_tomb, evil_presence, gaea_s_liege, kormus_bell, living_lands,
phantasmal_terrain.

### 9. `color-filters-on-spells` — 4 cards, S — **done**
Depends on: nothing.
*Landed:* no engine change. The premise was stale — the colour axis was already there and already
read. `SpellFilter::Color(Color)` (TOML `filter = { color = "red" }`) is matched by
`Game::spell_matches_filter` against `color_identity`, and that is exactly what
`TargetSpec::SpellOnStack` consults when it checks counter-target legality; `PermanentFilter.color`
has been there since Northern Paladin. All four cards were pure authoring. The Blasts are
cast-time modal (`modal = true`, one `spell`-timed ability per mode), not a resolution-time
`choose_one` — a spell's "Choose one —" is picked as it is cast (CR 601.2b).
Six cards left over the split. `Trigger::CastSpell` carries a `SpellFilter` too, so nothing here
blocked the five artifact "rods" either; they moved wholesale to #10, the shape they actually
need. Gloom's cost-increase work is now #67.
*Cards:* blue_elemental_blast, deathgrip, lifeforce, red_elemental_blast.

### 10. `optional-mana-payment-in-trigger` — 6 cards, S — **done**
Depends on: nothing.
*Landed:* no engine change — the third stale premise in a row. "A trigger whose resolution offers
a mana payment" already has a shape and it is not a resolution pause at all: an `optional = true`
triggered ability with a non-free `[abilities.cost]` raises `PendingChoice::PayCost` *before* the
ability goes on the stack (`Game::place_pending_triggers`), answered by
`Intent::PayOptionalCost` — Trudge Garden's "you may pay {2}" is the same card shape.
`Trigger::CastSpell` already carries both the `SpellFilter` (so `{ color = "white" }` works — #9)
and `CasterScope::AnyPlayer` for "whenever *a player* casts", so the five artifact rods needed
nothing either. All six are pure authoring.
*Cards:* crystal_rod, iron_star, ivory_cup, soul_net, throne_of_bone, wooden_sphere.

### 11. `block-restrictions-and-requirements` — 7 cards, L
Depends on: nothing.
1993 combat is full of "can't be blocked except by X", "all creatures able to block it do so",
and "can block an additional creature." The engine's blocking legality check knows keywords
(flying, menace, fear, `can_block_only_flyers`, `lesser_power_cant_block`) but has no general
restriction/requirement pass and no per-creature block count above one. *Sketch:* split the
declare-blockers validation into two passes matching CR 509.1b/509.1c — collect restrictions
(Invisibility's "except by Walls", Juggernaut's "not by Walls", Ironclaw Orcs' "can't block power
≥ 2") and requirements (Lure's "all able to block it do so", Blaze of Glory's "blocks each
attacking creature if able"), then verify the declared set satisfies the maximum possible number
of requirements without violating any restriction. A per-creature `max_blocks` (Two-Headed Giant
of Foriys, Blaze of Glory's "any number") rides along. False Orders is the odd one — removing a
blocker mid-combat and re-declaring it — and `remove_from_combat` already exists for the first
half. This is the prerequisite most likely to grow: #14 (banding) sits on the same pass.
*Cards:* blaze_of_glory, false_orders, invisibility, ironclaw_orcs, juggernaut, lure,
two_headed_giant_of_foriys.

### 12. `copy-a-permanent` — 3 cards, L
Depends on: nothing.
"You may have this creature enter as a copy of any creature on the battlefield" (CR 707).
Falsifies the `PermanentFilter::name` ponytail — a copy changes the permanent's name, so
copiable-values-vs-current-values becomes a real distinction. The engine copies *spells*
(`copy_triggering_spell`, `create_copy`) and has one bespoke creature-copy
(`become_copy_of_target_creature_gaining_myriad`), but no general as-enters copy. *Sketch:* a
`CopyableValues` snapshot on `Permanent` — the copiable characteristics (name, mana cost, color,
types, subtypes, abilities, base P/T) read from the copied permanent's own copiable values, not
its current ones (CR 707.2), so copying a Clone copies what that Clone copied. `characteristics.rs`
reads from that snapshot when present instead of the `CardDef`. Vesuvan Doppelganger layers two
exceptions on top (doesn't copy color; keeps its own upkeep ability) plus an upkeep re-copy;
Copy Artifact adds "it's an enchantment in addition to its other types."
*Cards:* clone, copy_artifact, vesuvan_doppelganger.

### 13. `copy-target-spell` — 1 card, M
Depends on: nothing.
Fork. `copy_triggering_spell` copies the spell that *triggered* the ability; Fork copies a
targeted spell on the stack, may choose new targets, and the copy is red regardless of the
original. *Sketch:* an `Effect::CopyTargetSpell { new_targets: bool, set_color: Option<Color> }`
reusing the existing stack-copy machinery with the target chosen at cast time
(`instant_or_sorcery_spell_on_stack` already exists as a target spec) and a
`PendingChoice::ChooseTarget` raised at resolution for the copy's targets.
*Cards:* fork.

### 14. `banding` — 4 cards, L
Depends on: #11.
Banding (CR 702.22). Attacking as a band, being blocked as a group, and the defining ugly part:
when a banding creature blocks or is blocked, *its controller* — not the attacking creature's
controller — assigns that creature's combat damage. *Sketch:* an attack-declaration grouping
(bands are declared with the attackers and can't change), block legality treating a band as one
object, and a damage-assignment ownership flip in the existing `AssignCombatDamage` pending
choice, which already knows how to ask a player to divide damage among blockers — it needs to ask
a *different* player. Helm of Chatzuk grants it until end of turn, so it must be a real
`Keyword`, not a card flag.
*Cards:* benalish_hero, helm_of_chatzuk, mesa_pegasus, timber_wolves.

### 15. `color-changing-effects` — 5 cards, M
Depends on: nothing.
The lace cycle. `set_own_color_until_end_of_turn` exists but is self-scoped and turn-scoped;
these target a spell *or* permanent and last indefinitely (layer 5, no duration). *Sketch:* an
`Effect::SetColor { target, colors, replace: true }` written into a permanent-level or
stack-object-level color override that `characteristics.rs` applies in layer 5, with no cleanup
hook. The target spec needs a "spell or permanent" variant — the pool has
`single_target_spell_on_stack` and permanent targets, but nothing that accepts either.
*Cards:* chaoslace, deathlace, lifelace, purelace, thoughtlace.

### 16. `text-changing-effects` — 2 cards, L
Depends on: #3, #15.
Magical Hack and Sleight of Mind — layer 3, rewriting a word in the printed text. This is only
tractable because the engine's card model is structured, not textual: the two cards each rewrite
exactly one *enumerated* thing. *Sketch:* a permanent-level substitution map
(`BasicLandType → BasicLandType` for Magical Hack, `Color → Color` for Sleight of Mind) applied
in `characteristics.rs` at layer 3 to every enumerated land-type and color field on the object's
abilities before later layers read them. Nothing textual is parsed. The scope is deliberately
narrow and its ceiling is the printed cards: it will not rewrite a word the card model doesn't
already store as an enum.
*Cards:* magical_hack, sleight_of_mind.

### 17. `random-discard` — 2 cards, S
Depends on: nothing.
"Discards a card at random" / "discards X cards at random." No `mode = "random"` or `at_random`
exists anywhere in the pool. The engine's determinism rule means this must draw from the injected
RNG the engine already threads for shuffling, never a wall-clock or thread-local source.
*Sketch:* a `random: bool` on the existing discard effect; when set, the controller does not
choose — the discarded cards are picked by the injected RNG. Card identity stays hidden from the
opponent's projection through the existing visibility filter (the discard *event* is public, the
hand it was drawn from is not).
*Cards:* hypnotic_specter, mind_twist.

### 18. `extra-turns` — 2 cards, M
Depends on: #7 (Time Vault only).
"Take an extra turn after this one" (CR 505.6a). The turn structure advances through a fixed
player rotation with no concept of an inserted turn. *Sketch:* a `Vec<PlayerId>` extra-turn queue
on `Game`, consumed by the turn-advance path before consulting the normal rotation, so multiple
extra turns stack in the right order (last created, first taken). Time Vault additionally needs
its skip-your-turn replacement, which is the other half of #7's untap-step work.
*Cards:* time_vault, time_walk.

### 19. `land-tap-triggers-and-bonuses` — 5 cards, M
Depends on: nothing.
"Whenever a player taps a land for mana" (Mana Flare, Manabarbs, Gauntlet of Might) and "whenever
enchanted land becomes tapped" (Psychic Venom, Lifetap). `tapped_for_mana_bonus` (Fertile Ground,
Wild Growth) exists but is a *static* on the enchanted host that adds mana; there is no trigger,
and no way to scope it to every land of a type across all players. *Sketch:* two things — (a) a
`Timing::LandTappedForMana` trigger carrying the land and its controller, fired from the mana-
ability path; (b) widening `tapped_for_mana_bonus`'s `scope` beyond `enchanted_host` to a
permanent filter, so Gauntlet of Might's "whenever a Mountain is tapped for mana, its controller
adds an additional {R}" is the existing static over a wider scope. Psychic Venom triggers on *any*
tap, not just for mana, so it hangs off the tap event rather than the mana path.
*Cards:* gauntlet_of_might, lifetap, mana_flare, manabarbs, psychic_venom.

### 20. `pay-or-consequence-upkeep` — 3 cards, S
Depends on: nothing.
"At the beginning of your upkeep, this creature deals 8 damage to you unless you pay {G}{G}{G}{G}."
`sacrifice_self_unless_pay` and `PayEchoOrSacrifice` cover pay-or-sacrifice; there is no
pay-or-damage, and Lord of the Pit's is pay-a-*sacrifice*-or-damage. *Sketch:* generalize the
existing `SacrificeUnlessPay` pending choice into `PayOrElse { cost, otherwise: Vec<Effect> }` so
the penalty is an effect list rather than a hardcoded sacrifice — the three cards then differ only
in their `otherwise`. Demonic Hordes' penalty also needs #41.
*Cards:* demonic_hordes, force_of_nature, lord_of_the_pit.

### 21. `blocks-or-blocked-by-trigger` — 2 cards, M
Depends on: nothing.
"Whenever this creature blocks or becomes blocked by a non-Wall creature, destroy that creature at
end of combat." Two gaps meet here. (a) No trigger fires on the blocking relationship itself —
the pool has attack triggers and damage triggers, not block triggers. (b) "non-Wall" is a subtype
*exclusion*, which falsifies the `without_flying` ponytail's "generalize if a second keyword
exclusion turns up" — 2ed has five non-Wall clauses. *Sketch:* a `Timing::BlocksOrBlocked` firing
once per blocking pair at declare-blockers with the other creature as `triggering_creature`, plus
`PermanentFilter::subtypes_exclude: &[&str]` replacing the ad-hoc bools. The delayed destroy is an
end-of-combat scheduled effect, which `schedule_*` effects already have a shape for.
*Cards:* cockatrice, thicket_basilisk.

### 22. `damage-taken-history` — 3 cards, M
Depends on: nothing.
"You gain life equal to the damage dealt to you this turn" / "whenever you're dealt damage, put
that many vitality counters." `triggering_damage_dealt` exists as an amount but only inside the
triggering ability; nothing accumulates per-player damage across a turn, and there is no
"whenever you're dealt damage" trigger at all. *Sketch:* a turn-scoped `damage_taken_this_turn`
per player on `Game`, incremented in the single `deal_damage` path and cleared at cleanup, exposed
as an `Amount`; plus a `Timing::PlayerDealtDamage` firing with the amount. Both are cheap once
`deal_damage` is already being touched for #4 — sequence this after that increment.
*Cards:* lich, living_artifact, simulacrum.

### 23. `mana-emptying` — 3 cards, M
Depends on: nothing.
"That player loses all unspent mana" — a 1993 artefact of mana burn that survives in the Oracle
text. The engine's mana pool empties at step boundaries; nothing empties it on demand, and Power
Sink's "they tap all lands with mana abilities they control" is a filtered mass tap the existing
`tap_all` doesn't filter by ability. *Sketch:* an `Effect::EmptyManaPool { who }` plus a filter on
`tap_all`. Power Sink is otherwise the standard counter-unless-pays shape.
*Cards:* drain_power, mana_short, power_sink.

### 24. `attack-restrictions-by-defender` — 3 cards, S
Depends on: nothing.
"This creature can't attack unless defending player controls an Island" and Animate Wall's "can
attack as though it didn't have defender." Attack legality checks keywords and tapped state but
takes no per-attacker predicate against the *defending player's* board. *Sketch:* a
`StaticEffect::CantAttackUnless { condition }` evaluated per candidate defender in the declare-
attackers legality check (a creature may be legal against one opponent and not another — this is
the multiplayer wrinkle the printed card never had to consider), and a
`StaticEffect::IgnoresDefender` consulted beside the `Defender` keyword check. Pirate Ship and Sea
Serpent's "when you control no Islands, sacrifice this" is a state trigger the engine has a shape
for (`no_creatures_on_battlefield` is the same idea).
*Cards:* animate_wall, pirate_ship, sea_serpent.

### 25. `amount-arithmetic` — 2 cards, S
Depends on: #1 (Aspect of Wolf only).
"X damage … where X is the number of cards in their hand minus 4" and "half the number of Forests
you control, rounded down / rounded up." `Amount` has `half_x` and `half_x_rounded_down` for the
X-cost case only; there is no way to halve or offset any other amount. *Sketch:* wrap rather than
multiply variants — `Amount::Offset { inner: Box<Amount>, delta: i32, floor_zero: bool }` and
`Amount::Half { inner: Box<Amount>, round_up: bool }`, resolved recursively in `amount.rs`. Black
Vise clamps at zero (a 3-card hand deals no damage, not negative damage).
*Cards:* aspect_of_wolf, black_vise.

### 26. `forced-attack-with-delayed-punishment` — 2 cards, M
Depends on: #21 (subtype exclusion).
"That creature attacks this turn if able. Destroy it at the beginning of the next end step if it
didn't attack." `must_attack_target` (Basandra) and `must_attack_each_combat` exist; the
punishment half does not, and neither does the "has controlled continuously since the beginning of
the turn" qualifier that both cards use to exempt freshly-arrived creatures. *Sketch:* a
`controlled_since_turn_start` flag on `Permanent` (set at untap, cleared on control change or
entry) as a filter axis, plus a delayed end-step trigger that checks the existing
`attacked_this_turn` flag — which #1's neighbourhood already maintains.
*Cards:* nettling_imp, siren_s_call.

### 27. `widen-creature-types` — 0 cards, S
Depends on: nothing.
Re-audit fallout, not a card blocker. `CREATURE_TYPES` in `types/stack.rs` is the candidate list
for "choose a creature type" prompts and its own ponytail says to widen it when a card needs a
type not printed on anything in the pool. 2ed prints 21 such types: Archer, Assassin, Barbarian,
Basilisk, Cockatrice, Gargoyle, Illusion, Juggernaut, Minotaur, Nightmare, Nymph, Ogre, Pegasus,
Pirate, Serpent, Shade, Specter, Spider, Unicorn, Wall, Wraith. *Sketch:* add them to the list.
Land it in the same wave as the first batch of 2ed creatures so the two stay consistent.
*Cards:* none directly — every "choose a creature type" card in the pool gains the options.

### 28. `counter-kinds` — 5 cards, S — **corpse landed, 3 kinds left**
Depends on: nothing.
Falsifies the fixed counter-slot array in `types/effect/shared.rs`. 2ed needs four kinds it
doesn't have: +1/+0 (Clockwork Beast), corpse (Scavenging Ghoul), mire (Cyclopean Tomb), vitality
(Living Artifact). Three are inert bookkeeping counters; +1/+0 is a real P/T counter that
`characteristics.rs` must apply in layer 7d beside +1/+1.

`CounterKind::Corpse` landed (`COUNT` 10 → 11, one `ALL` entry, one `message.rs` name) and
Scavenging Ghoul ships. The only other engine gap it needed was game-wide death counting:
`Amount::CreaturesDiedThisTurn` is per-controller, and the Ghoul's "for each creature that died
this turn" names no controller, so `Amount::CreaturesDiedThisTurnAnyController` sums every
player's tally (they all clear at the same Untap step, so the sum is exact — no new field).
Everything else the card needs already existed: `"each_end_step"`, `put_counters` with a named
`kind`, and `remove_counters` / `remove_counters_kind` as an activation cost paying
`regenerate_shield { target = "this" }`.

The other four cards each need something *besides* a counter kind, which is why the slot-array
work alone doesn't finish this increment:
- **Cyclopean Tomb** — mire counters are the easy half; it also needs #8 (changing a land's type)
  and a rest-of-game delayed trigger that unwinds them when the Tomb leaves.
- **Living Artifact** — vitality counters need a "whenever you're dealt damage" watcher.
- **Clockwork Beast** — +1/+0 is a real P/T counter (layer 7d, beside +1/+1); it also caps its own
  activation ("can't cause the total to be greater than seven", a bound on the effect's amount)
  and needs an end-of-combat conditional removal.
- **Rock Hydra** — blocked on #4 (damage prevention).
*Cards:* clockwork_beast, cyclopean_tomb, living_artifact, rock_hydra, scavenging_ghoul.

### 29. `extra-land-plays-and-land-play-trigger` — 1 card, S
Depends on: nothing.
Fastbond. "You may play any number of lands on each of your turns" plus "whenever you play a land,
if it wasn't the first land you played this turn." The engine enforces one land per turn with a
counter; the counter exists, nothing lifts the cap and nothing triggers on the play. *Sketch:* a
`StaticEffect::AdditionalLandPlays { count: Option<u32> }` (`None` = unlimited) consulted by the
land-play legality check, and a `Timing::LandPlayed` trigger carrying the per-turn ordinal so the
intervening-if reads it directly.
*Cards:* fastbond.

### 30. `counter-spell-with-mana-value-x` — 1 card, S — **done**
Depends on: nothing.
Spell Blast. Shipped as `SpellFilter::ManaValueEqualsX`, matched inline in
`Game::legal_targets_for`'s `SpellOnStack` arm. That function already threaded the filtering
spell's chosen `x` (for `PermanentFilter::mv_eq_x`) and is the single choke both cast-time
legality and the CR 608.2b resolution re-check route through, so the change was one early-return
there plus two exhaustiveness arms — no new field, and no signature change to
`spell_matches_filter`'s call sites, which have no X of their own and so answer `false`.
*Cards:* spell_blast.

### 31. `look-at-target-players-hand` — 1 card, S
Depends on: nothing.
Glasses of Urza. The engine reveals cards and looks at library tops but has no "look at a hand"
— it is purely a visibility grant to one player, with the server-side per-player filter being the
thing that has to change. *Sketch:* a one-shot `Effect::LookAtHand { target_player }` that widens
the activating player's projection of that hand for the duration of the resolution, threaded
through the same visibility filter that already special-cases revealed cards. No game state
changes; this is a projection-layer effect.
*Cards:* glasses_of_urza.

### 32. `spend-mana-as-another-color` — 1 card, S
Depends on: nothing.
Sunglasses of Urza. The mana payment path matches colors exactly. *Sketch:* a
`StaticEffect::SpendManaAsThoughAnotherColor { from: Color, to: Color }` consulted by the payment
matcher as a fallback when an exact match fails. Cost *reduction* already hooks the payment path,
so the seam exists.
*Cards:* sunglasses_of_urza.

### 33. `discard-to-library-top-replacement` — 1 card, S
Depends on: nothing.
Library of Leng. "If an effect causes you to discard a card, discard it, but you may put it on
top of your library instead." `no_maximum_hand_size` (the card's other half) already exists.
*Sketch:* a replacement consulted in the discard path offering
`PendingChoice::ChooseDiscardDestination`. Note the Oracle wording — the card *is* discarded
(discard triggers still fire), it just lands elsewhere.
*Cards:* library_of_leng.

### 34. `exile-instead-of-dying-replacement` — 1 card, S — **done**
Depends on: nothing.
Disintegrate. The sketch's premise was wrong on one point: the existing `cant_be_regenerated` is a
field on the *destroy* effect, not a mark on the permanent, so a damage rider had nothing to sit
alongside — Disintegrate needed both halves, not just the exile one. Shipped as two turn-scoped
`Permanent` flags (`cant_be_regenerated_this_turn`, `exile_instead_of_dying_this_turn`) set by two
new rider fields on `Effect::Damage(DamageEffect::Target)`, carried there on `Event::DamageMarked`.
The exile flag ORs into the `finality_counter` guard at `Game::graveyard_or_command`, the one dies
choke, which already implemented CR 614.12; the regeneration flag went into a new
`Game::regeneration_shield_available`, which all four shield-consuming sites (the lethal-damage
SBA and the three `destroy` paths) now route through instead of reading `regeneration_shields`
directly. Nothing reached the wire — `project_event` drops the two rider fields, since what the
client sees is the consequence (a creature that doesn't regenerate, a `MovedToExile` where a
`MovedToGraveyard` would have been) as events it already renders.
*Cards:* disintegrate.

### 35. `aura-attachment-restriction` — 1 card, S
Depends on: nothing.
Consecrate Land's "can't be enchanted by other Auras." *Sketch:* a
`StaticEffect::CantBeEnchanted` consulted by the Aura target-legality check and by the
state-based Aura-attachment sweep. The indestructible half is an ordinary keyword grant.
*Cards:* consecrate_land.

### 36. `grant-triggered-ability-to-attached` — 1 card, M
Depends on: nothing.
Farmstead grants the enchanted *land* a triggered ability ("At the beginning of your upkeep, you
may pay {W}{W}…"). `grant_to_attached` grants keywords and `grant_source_abilities_until_end_of_turn`
grants a whole ability set; neither grants a single authored triggered ability to a host.
*Sketch:* let `grant_to_attached` carry an `abilities` list the trigger scanner picks up on the
host — `triggers.rs` already has a granted-triggered-abilities scanner (its ponytail notes it has
one consumer today), so this widens that path rather than adding one. The payload is #10's
optional-mana-payment shape.
*Cards:* farmstead.

### 37. `aura-reattachment-on-trigger` — 1 card, M
Depends on: nothing.
Kudzu — when the enchanted land becomes tapped, destroy it, and *that land's controller* attaches
Kudzu to a land of their choice. Two novelties: an Aura surviving its host's destruction (rather
than being swept as an orphan) and a re-attachment choice made by an opponent. *Sketch:* an
`Effect::ReattachSelf { chooser, filter }` raising a `PendingChoice::ChooseAttachTarget` for the
named player, with the state-based orphan sweep exempting the Aura for the window between host
destruction and the choice resolving — the same exemption `enchant_graveyard` already carves out.
*Cards:* kudzu.

### 38. `shuffle-hand-and-graveyard-then-draw` — 1 card, S
Depends on: nothing.
Timetwister. `each_player_discards_hand_then_draws` (Wheel of Fortune, already in the pool) is the
neighbour; Timetwister shuffles hand *and* graveyard into the library instead of discarding, and
Timetwister itself goes to the graveyard after (so it isn't shuffled in). *Sketch:* a sibling mode
on that effect with the zones to shuffle and no discard step.
*Cards:* timetwister.

### 39. `graveyard-position-recursion` — 1 card, M
Depends on: nothing.
Nether Shadow — "if this card is in your graveyard with three or more creature cards above it."
The graveyard is ordered in the model but nothing reads position. *Sketch:* a
`Condition::CardsAboveThisInGraveyard { at_least, filter }` reading the existing ordering, plus
the upkeep trigger firing from the graveyard (triggers from a non-battlefield zone — check whether
the scanner already sweeps graveyards for `may_return_from_graveyard`; if it does, this is only
the condition).
*Cards:* nether_shadow.

### 40. `untapped-conditioned-anthem` — 1 card, S
Depends on: nothing.
Castle's "untapped creatures you control get +0/+2." `anthem` filters by color, subtype, and
controller but not by tapped state — and this one must re-evaluate the instant a creature taps.
*Sketch:* a `tapped: Option<bool>` axis on `PermanentFilter`, with `characteristics_cache.rs`
invalidating on tap/untap (confirm it already does — the cache invalidates on
`CombatCleared` and battlefield changes; a bare tap may not be one).
*Cards:* castle.

### 41. `opponent-chosen-sacrifice` — 1 card, S
Depends on: nothing.
Demonic Hordes' "sacrifice a land of an opponent's choice." Sacrifice effects always let the
sacrificing player choose. *Sketch:* a `chooser: Who` field on the sacrifice effect routing the
existing `PendingChoice` to a different player. In a 4-player game "an opponent's choice" is
underspecified by the printed card — pick the one whose upkeep-trigger controller is being
punished, i.e. raise the choice to the next opponent in turn order, and record that as an
`approximates` on the card.
*Cards:* demonic_hordes.

### 42. `filter-comparing-to-source` — 1 card, S
Depends on: nothing.
Stone Giant's "target creature you control with toughness less than this creature's power."
Filters compare against constants, never against the source's own live characteristics.
*Sketch:* a `toughness_less_than: Option<Amount>` on `PermanentFilter` resolved at target-legality
time with the source in scope — `Amount::SourcePower` already exists, so this is a filter axis, not
a new amount.
*Cards:* stone_giant.

### 43. `mass-symmetrical-rebalancing` — 1 card, L
Depends on: nothing.
Balance. Three sequential symmetrical operations, each finding the minimum across players and
making everyone else match: sacrifice lands down to the fewest, discard down to the fewest, and
sacrifice creatures down to the fewest — with each affected player choosing which of their own.
*Sketch:* a `BalanceZone { zone, filter }` effect that computes the minimum, then fans out one
`PendingChoice` per player over-threshold in APNAP order (CR 101.4). `each_player_sacrifices`
exists but takes a fixed count; the novelty is the derived per-player count and the three-phase
sequencing.
*Cards:* balance.

### 44. `aura-etb-conditional-self-grant` — 1 card, S
Depends on: nothing.
Earthbind — "When this Aura enters, **if** enchanted creature has flying, this Aura deals 2 damage
to that creature **and this Aura gains** 'Enchanted creature loses flying.'" The intervening-if
exists; the self-granted static does not — the Aura only strips flying if the check passed on
entry, so it can't be a plain printed static. *Sketch:* a
`Effect::GrantStaticToSelf { effect }` writing a granted-static onto the permanent that
`characteristics.rs`'s static scanners read alongside printed ones.
*Cards:* earthbind.

### 45. `pump-by-own-power-with-delayed-destroy` — 1 card, S
Depends on: nothing.
Berserk. `Amount::TargetPower` exists, so "+X/+0 where X is its power" is close — but it must
snapshot at resolution, not track live. The rider is a delayed end-step destroy conditioned on
whether the creature attacked, which is `attacked_this_turn` (#1's neighbourhood) plus a scheduled
effect. Also needs the cast-timing restriction "only before the combat damage step."
*Cards:* berserk.

### 46. `mana-from-variable-amount` — 1 card, S
Depends on: nothing.
Sacrifice — "Add an amount of {B} equal to the sacrificed creature's mana value." Mana effects add
fixed quantities; `Amount::SacrificedCreaturePower` exists but not mana value, and the mana effect
takes no amount. *Sketch:* an `amount: Amount` on the mana-add effect, plus
`Amount::SacrificedCreatureManaValue`. The additional cost (`[cost.additional]` sacrifice) already
exists.
*Cards:* sacrifice.

### 47. `lich-life-replacement` — 1 card, L
Depends on: #22.
Lich. Four interlocking replacements: you don't lose at 0 or less life, life gain becomes card
draw, damage taken becomes a sacrifice of that many permanents, and losing the enchantment loses
the game. `life_gain_replacement` exists (Pest Rescuer); the loss-prevention does not, and the
state-based-action check for 0 life is unconditional. *Sketch:* a per-player
`ignores_zero_life_loss` flag consulted by the SBA check, plus the damage-taken trigger from #22.
Worth landing last — it touches the loss condition, which every other test in the suite implicitly
depends on.
*Cards:* lich.

### 48. `pile-based-block-assignment` — 2 cards, XL
Depends on: #11.
Camouflage and Raging River — both replace declare-blockers with a pile-division ritual, and
Camouflage assigns piles to attackers *at random*. This is the least valuable work in the file
(two cards, no reuse, and the mechanic was abandoned in 1994) and the most invasive (it replaces
the declare-blockers step rather than constraining it). *Sketch:* deferred — do not start this
until every other increment has landed, and reconsider whether an `approximates` is the honest
answer instead.
*Cards:* camouflage, raging_river.

### 49. `controlling-another-players-actions` — 2 cards, XL
Depends on: nothing.
Word of Command ("you control that player until this finishes resolving") and Drain Power ("target
player activates a mana ability of each land they control"). One player making decisions on
another's behalf, mid-resolution, with a restricted legal-action set. The engine's submit path is
built around a single acting player per pending choice, so this is a structural change to the
choice model, not an effect. *Sketch:* a `PendingChoice` variant carrying both an `acting_player`
(who answers) and a `subject_player` (whose resources are spent), with the visibility filter
widened for the duration. Drain Power is the tractable half — its action set is exactly "tap each
land for mana", so it can be modelled as a direct effect without a real control handoff. Word of
Command is the hard half and should be the last thing attempted in this set.
*Cards:* drain_power, word_of_command.

### 50. `deny-unknown-fields-on-effect-tables` — 0 cards, S — **done**
Depends on: nothing.
Found while authoring this set, not by reading a card. The top-level card table and
`[[abilities]]` blocks already reject unknown keys; **effect** tables did not, so appending
`bogus_field_check = 7` to a card TOML loaded clean. That is the worst place for the hole to be:
an `[[abilities.effects]]` block is the last table in most card files, so a key written one line
too far lands in the effect rather than at top level, and effects are the highest-churn surface
in the DSL. A typo'd `toughnes` on an anthem shipped a +1/+0 lord with a green suite. This is a
fidelity hazard for every future grind, not a card gap.
*Landed:* `deny_unknown_fields` on all nineteen `mode`-tagged effect family enums and on the
`type`-tagged `Effect` itself (which covers the structural composers, whose arms carry no `mode`
leaf). Nothing in the existing pool turned red — no card was relying on an ignored key.
*Cards:* none — a guard against silently unfaithful cards.

### 51. `land-subtype-permanent-filter` — 2 cards, S
Depends on: nothing.
A land's printed types live under `[kind].subtypes` (CR 305), but `PermanentFilter::subtypes`
matches against `Game::effective_subtypes`, which reads only the card's **top-level** `subtypes`.
So `filter = { types = "land", subtypes = ["Plains"] }` matches no Plains at all, and the two
mass land-hate spells have no faithful shape. *Sketch:* fold a land's `[kind].subtypes` into
`effective_subtypes` for battlefield lands, the way the catalog already unions the two — then the
existing `subtypes` axis covers both halves and `CardFilter::LandWithSubtype` gets a battlefield
twin for free.
*Cards:* flashfires, tsunami.

### 52. `blocking-creature-filter` — 1 card, S
Depends on: nothing.
`PermanentFilter` has an `attacking` axis but no `blocking` one, so "target blocking creature"
can't be expressed. `anthem_static`'s own `blocking_only` already reads `CombatState::blocks` for
Crescendo of War — this is the same read, hoisted onto the shared filter. *Sketch:* add
`blocking: bool` to `PermanentFilter`, checked against `Game::blockers_of` in
`permanent_matches`, and drop `anthem_static::blocking_only` in favour of it.
*Cards:* righteousness.

### 53. `evenly-divided-damage-and-per-target-cost` — 1 card, M
Depends on: nothing.
Fireball needs two things the DSL lacks. `DamageEffect::Target`'s `divided` splits an amount
**as the caster chooses**; Fireball divides it *evenly, rounded down*, which is a computed split
with no choice at all. And "this spell costs {1} more to cast for each target beyond the first"
is a cost modification keyed to a target count chosen during casting — the cost pipeline has no
hook that late. *Sketch:* a `divided = "evenly"` arm on the damage effect (an enum where the bool
is now), plus a `cost_per_extra_target` field on `CardDef` consulted after targets are declared
in the cast path.
*Cards:* fireball.

### 54. `damage-then-gain-that-much-life` — 1 card, M
Depends on: nothing.
Drain Life deals X damage to any target and gains life "equal to the damage dealt" — capped by
the victim's life total / loyalty / toughness before the damage. The existing
`each_opponent_drain` bundles its own loss+gain and can't target; there is no way to write "gain
life equal to what the previous step actually dealt". Its "Spend only black mana on X" rider is a
second, independent gap: `Cost` has no per-symbol colour restriction on `{X}`. *Sketch:* a
`gain_life_equal_to_damage_dealt` effect reading the resolving ability's own damage-dealt tally
(the cap falls out of it, since the tally is already the *actual* damage), plus an
`x_colors = ["black"]` field on `[cost]` gating payment.
*Cards:* drain_life.

### 55. `rearrange-target-players-library-top` — 1 card, S
Depends on: 31 (`look-at-target-players-hand`) shares its "look at another player's hidden zone"
visibility work.
`look_at_top` always digs the resolving controller's own library. Natural Selection looks at the
top three of **target player's** library, reorders them, and may then have that player shuffle.
*Sketch:* a `whose = "target_player"` axis on `look_at_top` plus an ordering choice (the pool
already has "put back in any order" for scry/surveil — reuse that pending-choice shape) and an
optional shuffle step.
*Cards:* natural_selection.

### 56. `activate-only-during-your-turn` — 1 card, S
Depends on: nothing.
`sorcery_speed` is the only activation-timing gate the DSL has, and it is stricter than what
Disrupting Scepter prints: "Activate only during your turn" allows activation in combat, in
either end step, and with the stack non-empty, all of which `sorcery_speed` forbids (CR 602.5b vs
a plain turn check). Authoring it as `sorcery_speed` would quietly narrow a card, so the Scepter
waits. *Sketch:* a `your_turn_only` bool on the activated-ability cost fields, checked in
`ability_activation_gate` next to `sorcery_speed` — an independent axis, not a widening of it.
*Cards:* disrupting_scepter.

### 57. `until-end-of-combat-animation` — 1 card, M
Depends on: 56 (`activate-only-during-your-turn`) — both are activation/duration gates on the
same ability shape, and the combat-window check is the same kind of predicate.
Jade Statue needs two things at once: `animate_self_until_end_of_turn` only knows the
until-end-of-turn duration, and there is no "activate only during combat" gate for an activated
ability (`cast_only_during_combat` is a *spell*-level field). An until-EOT animation is strictly
better than the printed one — the Statue would survive past combat as a 3/6 — so this can't be
approximated down. *Sketch:* a `duration` field on the animation effect (`end_of_turn` /
`end_of_combat`, cleared at the end-of-combat step alongside the existing cleanup) plus a
`combat_only` activation gate reusing `cast_only_during_combat`'s own window predicate.
*Cards:* jade_statue.

### 58. `damage-the-entering-permanents-controller` — 1 card, S
Depends on: nothing.
Ankh of Mishra watches lands enter (`Trigger::PermanentEnters` with `EnterController::AnyPlayer`
already covers the watch) and then damages *that land's controller*. The two damage effects that
read a triggering object aim elsewhere: `DamageEffect::ToEnteringPermanent` hits the permanent
itself, and `DamageEffect::ToTargetController` reads an enclosing `Sequence`'s shared target, which
a trigger with no target never sets. *Sketch:* a `DamageEffect::ToEnteringPermanentController {
amount }` reading the same `TriggerContext` slot `ToEnteringPermanent` already threads.
*Cards:* ankh_of_mishra.

### 59. `land-put-into-graveyard-watch` — 1 card, M
Depends on: 58 (`damage-the-entering-permanents-controller`) — Dingus Egg's payoff is the exit-side
twin of Ankh's and wants the same "that land's controller" addressing.
There is no trigger for a *land* leaving the battlefield for a graveyard. The death watches are
creature-, enchantment-, and nonland-permanent-scoped (`CreatureDies`, `EnchantmentYouControlDies`,
`NonlandPermanentYouControlDiesIncludingThis`) — lands are the one permanent type deliberately
outside all of them, and every arm is controller-scoped besides. *Sketch:* a
`Trigger::PermanentPutIntoGraveyard { filter: PermanentFilter, controller: EnterController }`
mirroring `PermanentEnters`'s filter+scope shape, with the dying permanent's controller on the
context. *Cards:* dingus_egg.

### 60. `each-upkeep-payoff-addresses-that-player` — 1 card, S
Depends on: nothing.
`Trigger::EachUpkeep` fires on every player's upkeep but, per its own `ponytail:` note on
`queue_each_upkeep_triggers`, does not thread `TriggerContext::active_player` the way
`EachDrawStep` does. Copper Tablet's "deals 1 damage to **that player**" therefore has no way to
name whose upkeep it is; `DamageEffect::EachPlayer` would hit the whole table once per upkeep,
which is four times the printed damage in a four-player game. *Sketch:* thread `active_player` in
`queue_each_upkeep_triggers` (the note already sketches it) plus a
`DamageEffect::ToTriggeringPlayer { amount }`. *Cards:* copper_tablet.

### 61. `upkeep-of-the-enchanted-permanents-controller` — 4 cards, M
Depends on: 60 (`each-upkeep-payoff-addresses-that-player`) — same "damage the player this upkeep
belongs to" payoff, reached from an Aura rather than from a free-standing permanent.
The 2ed upkeep-tax Aura cycle reads "At the beginning of the upkeep of enchanted <permanent>'s
controller, this Aura deals 1 damage to that player." Both halves are missing: `Trigger::Upkeep`
is scoped to the *Aura's* controller, not the host's (an Aura you cast on an opponent's land
would tax you), and the payoff needs the same "that player" addressing as increment 60. Note the
cycle is deliberately type-agnostic — land, enchantment, creature, and artifact hosts — so the
trigger wants the enchant restriction it already carries rather than a per-type variant.
*Sketch:* a `Trigger::UpkeepOfEnchantedPermanentsController` queued off the same upkeep event as
`EachUpkeep`, gated on the Aura's attachment and firing with the host's controller in the
context's player slot. *Cards:* cursed_land, feedback, wanderlust, warp_artifact.

### 62. `damage-equal-to-the-dying-creatures-toughness` — 1 card, M
Depends on: 61 (`upkeep-of-the-enchanted-permanents-controller`) — both are Aura payoffs aimed at
the host's controller, so they want the same context plumbing.
Creature Bond's `Trigger::EnchantedCreatureDies` watch already exists, but the payoff needs two
things it can't get: an amount read from the dying creature's *last-known* toughness (CR
603.6c/603.10 — the creature is gone by resolution, so no live characteristic read works), and the
dying creature's controller as the damage recipient. `Amount` has no last-known-information arm.
*Sketch:* an `Amount::DyingPermanentToughness` fed from the death snapshot `Game::apply` already
captures for the `*IncludingThis` arms, plus the increment-61 "damage the host's controller"
recipient. *Cards:* creature_bond.

### 63. `whenever-this-is-dealt-damage` — 1 card, M
Depends on: nothing.
Fungusaur grows every time it is dealt damage, from any source — combat, a burn spell, a ping.
The damage-shaped triggers in the pool all watch damage *this permanent deals*
(`DealsCombatDamageToCreature`, `DealsDamageToOpponent`, `CreatureDealtDamageByThisDies`); nothing
watches damage *received*. This is not a rename of one of those — the event is a different one,
and it fires once per damage event rather than once per combat. *Sketch:* a
`Trigger::ThisIsDealtDamage` queued off the damage-marking path with the amount on the context (a
"dealt damage" watcher that scales with the amount is the obvious next consumer, so thread it
even though Fungusaur ignores it). *Cards:* fungusaur.

### 64. `fixed-color-tapped-for-mana-bonus` — 1 card, S
Depends on: nothing.
`StaticEffect::TappedForManaBonus` already has the right watch and the right scope
(`LandTapScope::EnchantedHost`), but `LandTapBonusColor` offers only `AnyColor` (Fertile Ground's
"one mana of any color") and `Produced` (Mirari's Wake's "any type that land produced"). Wild
Growth adds an additional **{G}** specifically — strictly narrower than `AnyColor` and unrelated
to `Produced`, so neither approximation is faithful. *Sketch:* a
`LandTapBonusColor::Fixed(Color)` arm, credited without the `ChooseManaColor` pause `AnyColor`
raises. *Cards:* wild_growth.

### 65. `skip-your-draw-step-for-an-attack-shield` — 1 card, M
Depends on: #3 (landwalk), which supplies half the exemption.
Island Sanctuary — "If you would draw a card during your draw step, instead you may skip that
draw. If you do, until your next turn, you can't be attacked except by creatures with flying
and/or islandwalk." Three things the engine lacks at once. First, an *optional replacement* on
the draw-step draw (`may skip`) — the existing draw-replacement shapes are mandatory. Second, a
`CantBeAttackedBy { filter }` whose filter is the *exempt* set rather than the banned set, and one
that reads keywords rather than types/colors — today's filter has no keyword axis. Third, an
"until your next turn" duration, which is longer than the until-end-of-turn temporaries the
engine keeps and shorter than a permanent static.
*Sketch:* an as-would-draw replacement raising a yes/no `PendingChoice`; on yes, record a
per-player shield with an expiry of the controller's next upkeep and read it in
`declare_attackers` as "this attacker must have flying or islandwalk." The keyword axis on
`PermanentFilter` is the reusable part — Island Sanctuary is the first card to need one, but
"creatures with flying can't …" is a common template.
*Cards:* island_sanctuary.

### 66. `grant-an-activated-ability-to-a-filter` — 1 card, M
Depends on: nothing.
Zombie Master's second clause — "Other Zombies have '{B}: Regenerate this permanent.'" The engine
can grant an activated ability, but only from an *attachment*: `GrantToAttached`'s
`granted_ability` is read by `Game::granted_attachment_abilities`, which walks
`Game::attachments(host)`. A lord grants to everything matching a filter, which that scan can't
see. (Its first clause, swampwalk, is plain #3 work and lands the moment this one does.)
*Sketch:* give `Anthem` the same optional `granted_ability` sub-table `GrantToAttached` already
has, and widen the granted-ability lookup from the attachment scan to the same
`matching_anthems`-style filter scan the keyword grants already use, so `ability_at` addresses
both sources through one accessor. Zombie Master then becomes two anthem blocks.
*Cards:* zombie_master.

### 67. `spells-and-abilities-cost-more` — 1 card, M
Depends on: nothing.
Split out of #9, which turned out to be pure authoring. Gloom is the one 2ed card that taxes
rather than discounts: "White spells cost {3} more to cast" and "Activated abilities of white
enchantments cost {3} more to activate." The engine only ever moves a cost *down* —
`StaticEffect::ReduceSpellCost { amount, filter, .. }` is subtracted at the cast choke — and it
has no hook at all on the *activation* choke (`AttackTax` is the nearest tax in shape, but it
gates attacking, not activating).
*Sketch:* let the cost delta be signed rather than adding an `Increase` twin — one `Amount` that
can go either way keeps the single subtraction at the cast choke and the mana-planning path
untouched. The second clause needs a genuinely new hook: an activation-cost tax keyed to a
`PermanentFilter`, read where an activated ability's cost is assembled, the same place
`ActivationCost::mana` is spent from. The spell half alone is worth landing first; the ability
half is what makes this M rather than S.
*Cards:* gloom.
