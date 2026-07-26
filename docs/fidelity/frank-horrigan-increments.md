# Frank Horrigan deck increments (2026-07-26)

Deck report: [frank-horrigan.md](frank-horrigan.md). This file is the sole engine-capability
backlog for this deck (checklist + ranked increments + per-card exotics). Numbering is local to
this file.

From `docs/fidelity/frank-horrigan.md` (Archidekt 24351870 — commander Agent Frank Horrigan).
35 of the deck's 53 new cards need engine work; ranked S-first within dependency order. The
observability re-audit falsified nine pool-absence claims — each is folded into the increment
that clears it (#2, #11, #13, #17, #19, #20, #23, #24).

This deck's centre of gravity is **#20**, an XL that builds player-level counters (poison) plus
infect and toxic from nothing. #21 (rad counters) rides on its first slice. Everything else is
ordinary card-shaped work.

---

### 1. `attacked-this-turn-condition` — 1 card, S — LANDED 2026-07-26
_Landed 2026-07-26: `Permanent::attacked_this_turn` (set in `apply.rs`'s `Event::AttackerDeclared`
arm — event-sourced, not in `declare_attackers`; cleared for *every* battlefield permanent in the
Untap `StepBegan` block beside `entered_this_turn`, so a new turn from any seat ends "this turn").
`Condition::SourceAttackedThisTurn` is read directly by `characteristics.rs`'s
`conditional_keywords` scanner — that hardcoded `if let SourceHasCounters` became a `match` with an
arm per source-object condition, since a static keyword grant (CR 604.3) has no `TriggerContext` to
route through `condition_holds`; `triggers.rs` keeps an unreachable `false` arm for exhaustiveness.
`characteristics_cache.rs` invalidates the whole battlefield on Untap `StepBegan` (the same shape as
`CombatCleared`). A token put onto the battlefield attacking is deliberately not flagged — it was
never *declared* an attacker. agent_frank_horrigan authored; its "proliferate twice" is one
`times = 2` effect (CR 701.27b repeats the process, each repetition its own choice). Still blocked:
the card keeps one `approximates` — proliferate can't choose players, so its own reminder text's
"permanents and/or players" is unmodeled until #17 lands._
Depends on: nothing.
`Condition::SourceAttackedThisTurn` for the commander's "has indestructible as long as it
attacked this turn." Combat already records attackers; the condition reads that per-turn flag on
the source permanent and is consumed by the existing `conditional_keywords` static. *Sketch:* a
turn-scoped `attacked_this_turn` bool on `Permanent`, set at declare-attackers alongside the
existing attack bookkeeping and cleared at untap — not "is currently attacking", which lapses at
end of combat while the printed grant does not. *Cards:* agent_frank_horrigan.

### 2. `counter-filter-axis-and-class-level-gating` — 2 cards, S
Depends on: nothing.
Two small gaps that meet on Innkeeper's Talent.
(a) `PermanentFilter` has no counter axis. `modified` (CR 701.29) is strictly broader — it also
matches Auras and Equipment — so it can't express "creature you control with a +1/+1 counter on
it" (Inspiring Call, for both its `per_permanent` draw count and its indestructible grant) or
"permanents you control with counters on them" (Innkeeper's Talent Level 2). *Sketch:* a
`with_counter: Option<CounterAxis>` field distinguishing "any counter" from "+1/+1 specifically".
(b) **Falsifies `triggers.rs:2946`.** `min_level` is consulted at exactly one site
(`triggers.rs:2960`), so a Class's level-gated *static* abilities function at level 1 — Innkeeper's
Talent's Level 2 ward and Level 3 doubler would both be live the moment it hits the battlefield.
*Sketch:* read `ability.min_level` in the static scanners (`characteristics.rs`) and the bespoke
trigger scanners the note names. *Cards:* inspiring_call, innkeepers_talent (with #17, #19).

### 3. `put-counters-each-counter-kind` — 1 card, S — LANDED 2026-07-26
_Landed 2026-07-26: `CountersEffect::PutCountersEach` gained the same `kind: Option<CounterKind>`
axis `PutCounters` already had. `None` keeps the historical +1/+1 path unchanged
(`counters_after_replacements` → `Event::CountersPlaced`); `Some(kind)` mints
`Event::KindCountersPlaced` directly, bypassing the +1/+1-only replacement pipeline (pinned by a
regression test: Doubling Season doesn't amplify a -1/-1 `PutCountersEach`). `label.rs` mirrors
`PutCounters`'s own kind-name arm. contagion_engine authored — ETB half fully faithful. Still
blocked: the card's own `{4}, {T}: Proliferate twice` carries the same residual as
agent_frank_horrigan's proliferate — CR 701.27 can't yet choose players (poison/rad) or Class
level/exiled time/scream counters (increment #17, not this wave)._

### 4. `greatest-power-amount` — 1 card, S — LANDED 2026-07-26
_Landed 2026-07-26: `Amount::GreatestPowerAmongCreaturesYouControl` (TOML
`"greatest_power_among_creatures_you_control"`) — `TotalPowerYouControl`'s live-`Game::power` scan
with `.max().unwrap_or(0)`. Negative-power edge needs no clamp: `Draw(DrawEffect::Cards)` already
floors through `resolve_count`. While adding it, both this arm and the pre-existing
`TotalPowerYouControl` were fixed to test `controller_of`, not `owner_of` — "creatures you control"
is a control test (CR 109.4/CR 720), so a stolen creature counted for the wrong seat. garruk_primal_hunter
authored (all three loyalty abilities), plus the 6/6 green Wurm token. Still blocked: nothing._
Depends on: nothing.
`Amount` offers `TotalPowerYouControl` but no per-creature maximum. *Sketch:*
`Amount::GreatestPowerAmongCreaturesYouControl`, resolved off the same live-power scan
(`Game::power`), 0 with no creatures. *Cards:* garruk_primal_hunter (−3).

### 5. `opponent-count-condition` — 1 card, S
Depends on: nothing.
No condition counts *opponents* (`OpponentsControlLands` counts lands). *Sketch:*
`Condition::YouHaveOpponents { at_least }` reading live seat count minus eliminated players — it
must track eliminations, not the table's starting size, or the land reads wrong in a game that
has gone to two. *Cards:* undergrowth_stadium.

### 6. `shockland-pay-life-as-enters` — 1 card, S/M — LANDED 2026-07-26
_Landed 2026-07-26: `CardDef::enters_tapped_unless_you_pay_life: Option<u8>` (TOML root key,
`crates/engine/src/types/card.rs`). Took the "pause before the enter" shape (most faithful to CR
614.12): `Game::play_land` raises `PendingChoice::PayLifeOrEntersTapped` and returns no events
before the land exists on the battlefield at all — declining or affording nothing mints
`Event::LandPlayed` tapped immediately; paying mints `LandPlayed` (tapped, per `enters_tapped`'s
unconditional `true` for this field) followed by `Event::LifeChanged` and `Event::Untapped`.
Answered by the existing `Intent::PayOptionalCost`, the `SacrificeUnlessPay` family's land-drop
twin. Offered only when `life >= life_cost` (CR 119.4 — a player may pay life down to and
including 0); below the cost there is no prompt and the land just enters tapped. `overgrown_tomb`
authored, fully faithful, frame-audit clean._
Depends on: nothing.
No pool land pays life as it enters — `enters_tapped_unless` takes a `Condition`, and `pay_life`
exists only as a cast/activation cost. Overgrown Tomb's "as this land enters, you may pay 2 life;
if you don't, it enters tapped" is a CR 614.12 as-enters replacement with a *choice*, not a
condition. *Sketch:* an `enters_tapped_unless_you_pay_life: Option<u32>` on the land frame that
raises an optional pay-or-decline pending choice during the enters replacement, defaulting to
"don't pay" (enters tapped) when declined, and only offered at all when the player's life total is
**greater than or equal to** the cost (CR 119.4 — a player may pay life down to and including 0;
the engine already models this for cast costs, see `crates/engine/src/types/mana.rs:201-204`).
*Cards:* overgrown_tomb.

### 7. `one-way-damage-equal-to-power` — 1 card, S
Depends on: nothing.
"Target creature you control deals damage equal to its power to target creature you don't
control" — a one-way fight. The existing fight machinery (`resolution/pause_fight.rs`) is
mutual-only. *Sketch:* a `mutual: bool` (or a `one_way` mode) on the fight effect, skipping the
back-half damage event; both targets are still chosen up front. *Cards:* infectious_bite (its
poison rider needs #20 slice 1).

### 8. `phyrexian-mana` — 1 card, S/M
Depends on: nothing.
`{B/P}` is unsupported in `[cost]`. *Sketch:* a hybrid-with-life pip that the cost payer may
settle with either the color or 2 life, recording *which* was chosen so #16's compleated rider
can read it. *Cards:* vraska_betrayals_sting (with #16).

### 9. `total-mana-value-budget-targets` — 1 card, M — LANDED 2026-07-26
_Landed 2026-07-26: `TargetCount::total_mv_max: Option<Amount>` (TOML `total_mv_max` on the count
table, hand-written `Deserialize` in `de.rs`) — a set-level CR 601.2c budget checked once against
the *summed* mana value of the whole chosen set, in `Game::choose_targets`'s
`PendingChoice::ChooseTarget` arm. Over budget is `Reject::IllegalChoice` and leaves the pending
choice untouched — never a silent truncation. The sketch pointed at `ChooseAbilityTargets`/second
target clauses; that was wrong for this card, whose destroy is a first-clause multi-target ETB
trigger (Numot's "up to two target lands" shape). Not projected to the wire, following
`SelectFromTop`'s `mv_budget` precedent (server-side rejection is the whole enforcement).
rampaging_yao_guai authored; "any number" takes the `u8` field ceiling (255) with a `ponytail:`
note. Still blocked: nothing._
Depends on: nothing.
"Destroy any number of target artifacts and/or enchantments with total mana value X or less" needs a
*summed* budget across the chosen target set. `mv_budget` exists only on `dig`/`look_at_top`, and
`x_scaled` caps the *number* of targets, not their total. *Sketch:* a `total_mv_max: Amount` on
the target spec, validated when the target set is submitted (CR 601.2c — legality is checked over
the whole set, so this is a set-level predicate, not a per-target filter). *Cards:*
rampaging_yao_guai.

### 10. `per-mode-targets-on-activated-abilities` — 1 card, M
Depends on: nothing.
`Effect::ChooseOne` raises a `ChoiceRequest::ChooseMode` carrying the *ability's* single
pre-chosen target (`resolution/pause_choose.rs`), so an activated modal ability whose modes
target different things ("destroy target artifact" / "destroy target enchantment" / "proliferate")
can't be expressed. The card-level `modal`/`choose` flags are spell-timing only. *Sketch:* choose
the mode first, then raise that mode's own target request — the same two-step the spell path
already does, lifted onto activated abilities. *Cards:* cankerbloom.

### 11. `multikicker` — 1 card, M
Depends on: nothing. **Falsifies `types/mana.rs:223`** ("single-kicker only … grow those from a
real card that needs one" — the deferral condition is now met).
*Sketch:* a `multikicker` cost that may be paid any number of times, recording the *count* paid on
the spell, plus `Amount::TimesKicked` so "enters with a charge counter for each time it was
kicked" can read it. The existing binary `if_kicked` stays as sugar for count ≥ 1. *Cards:*
everflowing_chalice.

### 12. `monstrosity` — 1 card, M
Depends on: nothing.
Monstrosity (CR 701.28) is absent entirely. *Sketch:* a `monstrous: bool` on `Permanent`, an
activated `monstrosity N` effect that is a no-op when already monstrous and otherwise places N
+1/+1 counters (through the existing replacement pipeline) and sets the flag, plus a
`Trigger::BecomesMonstrous`. Alpha Deathclaw's "enters **or** becomes monstrous" is one ability
with two trigger conditions, like the commander's "enters or attacks". *Cards:* alpha_deathclaw.

### 13. `emblems` — 1 card, M
Depends on: nothing. **Clears the stale note at `promise_of_loyalty.toml:3`** ("planeswalker
defenders unmodeled" — already false; `Defender::Planeswalker` exists and `combat.rs:430` says so).
*Sketch:* emblems are an ownerless, unremovable object in a command-zone-adjacent store carrying
static effects only (CR 114). Garruk, Cursed Huntsman needs the emblem *and* a token-borne death
trigger that puts a loyalty counter on each Garruk its controller controls — a token ability
pointing at a permanent type, not at its creator, so it survives the creating walker's death.
Fix the Promise of Loyalty note in the same change. *Cards:* garruk_cursed_huntsman.

### 14. `double-counters-or-cull-and-gain` — 1 card, M
Depends on: nothing.
Lily Bowen's upkeep needs three things the DSL lacks: a source-power-**at-most** condition
(`Condition` has `TargetPowerAtLeast` and `SourceHasCounters { at_least }` only), "double the
number of +1/+1 counters on this" (which must route through the counter-replacement pipeline —
with Branching Evolution out it is 2×, with it 4×), and "remove all but one +1/+1 counter, then
gain 1 life for each removed **this way**" (`remove_all_counters_then_draw` draws instead, and the
count must be the number actually removed). *Cards:* lily_bowen_raging_grandma.

### 15. `grant-triggered-ability-to-attached` — 1 card, M
Depends on: nothing.
`grant_to_attached`'s `granted_ability` is a `GrantedAbility { cost, effects }` — activated only.
Power Fist grants the equipped creature a *triggered* ability ("whenever this creature deals
combat damage to a player, put that many +1/+1 counters on it"). *Sketch:* let the granted ability
carry a `Trigger` instead of a cost, and have the trigger scanners see granted abilities as well
as printed ones. Note "that many" is the damage dealt, so the grant must thread the trigger's own
damage amount into the counter count. *Cards:* power_fist.

### 16. `compleated` — 1 card, M
Depends on: #8 phyrexian-mana.
"If life was paid, this planeswalker enters with two fewer loyalty counters" — an as-enters
replacement reading how the `{B/P}` pip was actually settled. *Sketch:* the cast records the
life-paid choice on the spell; the planeswalker's enters-with-loyalty read subtracts 2 per pip
paid with life. Also needs Vraska's −2 (#25). *Cards:* vraska_betrayals_sting.

### 17. `proliferate-full-scope` — 9 cards + observers, L
Depends on: #20 slice 1 (for the player half). **Falsifies `types/effect/shared.rs:1035`**, whose
note understates its own gap.
`proliferate()` (`pending/raise/optional.rs:14`) enumerates only `game.battlefield()` filtered on
`plus_counters > 0 || kind_counters.any(...)`. It therefore silently omits **four** things:
players (poison/rad), **loyalty** counters (a bare `Permanent::loyalty` i32, not a `CounterKind`),
**Class level** counters (a bare `Permanent::level`), and the exiled-card time/scream counters the
note actually mentions. The pool's one existing proliferate card already prints "any number of
permanents **and/or players**" in its own reminder text, so this is a live wrong-behavior bug, not
just a missing feature. *Sketch:* widen the choice's candidate enumeration and its answer
validation to a `ProliferateTarget` sum (permanent | player), and make the "give another counter
of each kind already there" step address loyalty, level, and the exile store alongside
`kind_counters`. *Cards:* every proliferate source in the deck — agent_frank_horrigan, atomize,
contagion_clasp, contagion_engine, karns_bastion, evolution_sage, thirsting_roots,
unnatural_restoration, drown_in_ichor, glistening_sphere, contaminant_grafter, cankerbloom,
blightbelly_rat, bloated_contaminator, vraska_betrayals_sting — plus the existing pool's
expansion_algorithm.

### 18. `whenever-you-proliferate-trigger` — 1 card, S
Depends on: #17 (proliferate must emit a watchable event first).
Proliferate is a resolution-time `ChoiceEffect` that emits no event, so there is nothing for a
trigger to watch. *Sketch:* emit an `Event::Proliferated { player }` once per proliferate
instance and add `Trigger::YouProliferate`. Note the commander's "proliferate twice" is two
instances (CR 701.27 — each is a separate proliferate), so Scheming Aspirant triggers twice off
it; test that explicitly. *Cards:* scheming_aspirant.

### 19. `counter-replacement-generalization` — 3 cards + observers, L
Depends on: #20 slice 1 (for the player half). **Falsifies `characteristics.rs:1807` and
`characteristics.rs:1811`; puts `ozolith_the_shattered_spire.toml:10` on notice.**
`counters_after_replacements(object: ObjectId, base: i32)` is +1/+1-only and keyed by an object,
so three printed clauses have no call site at all:
- Winding Constrictor's "if **you** would get one or more counters" (player-side)
- Vorinclex's "counters … on a permanent **or player**", *each of those kinds* (all kinds)
- Vorinclex's opponent-facing **halving**, which the current model cannot express three ways over:
  `times: i32` (`types/effect/static.rs:76`) has no ÷2-round-down, the loop's
  `if p.owner != controller { continue }` (`characteristics.rs:1827`) means an opponent's Vorinclex
  never applies, and — the real sting — with a halving in the mix the documented fixed
  `(base + Σadd) × Πtimes` order **stops being the affected player's best order**, so CR 616.1's
  ordering choice becomes genuinely load-bearing rather than safely assumed away.

*Sketch:* re-key the function on a `CounterRecipient` (permanent | player) and a `CounterKind`
selector; give the replacement a `factor` that expresses ×N and ÷N-round-down; scope each
replacement by whose counters it affects (yours / an opponent's). Offer the CR 616.1 ordering
choice **only when the applicable set is not order-independent** — with adders and multipliers
alone the maximizing order is forced and a prompt would be noise. Widening this function makes
Ozolith's over-broad "any permanent you control" shape a live bug (it is presently harmless only
because level counters route around this path) — fix Ozolith's filter in the same change.
*Cards:* vorinclex_monstrous_raider, winding_constrictor, innkeepers_talent (L3), plus existing
pool ozolith_the_shattered_spire, hardened_scales, doubling_season.

### 20. `player-counters-poison-infect-toxic` — 17 cards, XL
Depends on: nothing (this is the foundation).
**Falsifies `types/effect/shared.rs:1070`** — and the remedy that note prescribes ("grow this slot
array, add the matching variant") **cannot work**: `kind_counters` is an array on `Permanent`, and
poison lives on *players*. This needs a new player-side store, not a wider slot array.

Nothing in the engine touches poison, infect, or toxic (`grep -ri` over `crates/engine/src/`
returns only Toxic Deluge's *name*). `Player` (`types/card.rs:1803`) has no counter field and
`CountersEffect` has no put-on-player variant. Staged:

**Slice 1 — player counter store + SBA.** A counter map on `Player` (it is not `Copy`, so this is
cheap), `CountersEffect::PutCountersOnPlayer { kind, count, scope }`, `Event::PlayerCountersPlaced`,
the CR 704.5c "10 or more poison counters → that player loses the game" state-based action wired
into the existing `check_state_based_actions` loss sweep beside the commander-damage check, and
the `VisibleEvent` + projection arms (poison totals are public — do **not** redact them).
*Cards:* infectious_inquiry, vraskas_fall, ichor_rats (ETB half).
*2026-07-26 — slice 1 built (the XL is not LANDED; slices 2–5 remain).* `PlayerCounterKind` +
`Player::kind_counters`, `CountersEffect::PutCountersOnPlayer { kind, count, scope }` (reusing
`EdictScope`), `Event::PlayerCountersPlaced`, the CR 704.5c check in `apply.rs`'s elimination
sweep, `Game::player_counters` / `place_player_counters`, and the public (unredacted)
`VisibleEvent::PlayerCountersPlaced` + `PlayerView.poison` wire surface. infectious_inquiry and
vraskas_fall are faithful; ichor_rats carries one `approximates` for the unmodeled Infect keyword.

**Slice 2 — `Keyword::Infect`.** CR 702.90: damage to creatures becomes -1/-1 counters, damage to
players becomes poison counters. This is a damage *replacement*, so it must sit at the shared
damage choke in `resolution/damage.rs` / `combat.rs::damage_player`, not only on the combat path —
Infectious Bite and any noncombat source must route through it too. Lifelink and deathtouch still
apply off the original damage amount. *Cards:* plague_stinger, ichor_rats, phyresis (Aura grant).

**Slice 3 — `Keyword::Toxic(u8)`.** CR 702.164: *in addition to* its normal combat damage, a
creature with toxic N gives the player it damages N poison counters. Unlike infect this does not
replace the damage. Multiple instances add (CR 702.164b). *Cards:* bilious_skulldweller,
blightbelly_rat, bloated_contaminator, contaminant_grafter, venerated_rotpriest,
necrogen_communion (Aura grant).

**Slice 4 — poison readers.** `Condition::AnOpponentHasPoisonAtLeast { at_least }` (the printed
**Corrupted** ability word, CR 702.165), `Amount::OpponentsPoisonCounters`, and Vraska's −9
"top up to nine" (a *difference*, not a fixed add — it must place nothing if the target already
has nine or more). *Cards:* contaminant_grafter, glistening_sphere, phyrexian_swarmlord,
vraska_betrayals_sting.

**Slice 5 — poison-scaled pump.** Phyresis Outbreak's "each creature your opponents control gets
-1/-1 until end of turn **for each poison counter its controller has**" — a per-permanent amount
resolved against *that permanent's controller*, not the spell's controller. *Cards:*
phyresis_outbreak.

Landing rule: this XL is LANDED only when all five slices are built; slices get dated progress
notes until then.

### 21. `rad-counters` — 2 cards, L
Depends on: #20 slice 1 (the player counter store).
Rad counters are a player counter with their own turn-based rules action: at the beginning of each
player's precombat main phase, a player with rad counters mills that many cards, and for each
**nonland** card milled this way loses 1 life and removes one rad counter. *Sketch:* a `Rad` kind
in the player store plus the rules action in the precombat-main step alongside the existing
turn-based actions — it is a rules action, not a triggered ability, so it uses no stack and no
player may respond to it. *Cards:* feral_ghoul, bloatfly_swarm (also needs #22).

### 22. `damage-prevention-replacing-counters` — 1 card, M
Depends on: #21.
Bloatfly Swarm: "if damage would be dealt to this creature while it has a +1/+1 counter on it,
prevent that damage, remove that many +1/+1 counters from it, then give each player a rad counter
for each +1/+1 counter removed this way." A self-hosted damage replacement (CR 615) whose payload
is counter removal, capped by the counters actually present — the removal count is
`min(damage, counters)`, and the rad counters follow that *actual* number. *Cards:*
bloatfly_swarm.

### 23. `final-act-player-counter-mode` — 1 existing card, S
Depends on: #20 slice 1. **Falsifies `final_act.toml:13` and `:22`.**
Final Act's fifth mode ("each opponent loses all counters") was dropped because "this pool tracks
no player-level counters" — that justification dies with #20, and the `approximates` text is
surfaced verbatim to users in the catalog. Restore the mode, raise `choose_max` from 3 to 4, and
trim the note to the genuinely-remaining "destroy all battles" residual. *Cards:* final_act
(existing pool).

### 24. `plus-minus-counter-annihilation-sba` — observers, S
Depends on: nothing. **Falsifies `characteristics.rs:1100`.**
CR 704.5r: a permanent with both +1/+1 and -1/-1 counters has N of each removed as a state-based
action. The note calls this "unobservable today (no pool card puts both kinds on one creature)" —
Contagion Clasp and Contagion Engine both place real -1/-1 counters, and infect damage places them
too, onto a deck that stacks +1/+1 counters on nearly every creature. *Sketch:* one sweep in
`check_state_based_actions` removing `min(plus, minus)` of each; delete the ponytail.
*Cards:* contagion_clasp, contagion_engine, plus every infect source (#20 slice 2).

### 25. `becomes-treasure-losing-all-else` — 1 card, M
Depends on: nothing.
Vraska's −2: "target creature becomes a Treasure artifact with '{T}, Sacrifice this artifact: Add
one mana of any color' and loses all other card types and abilities." A CR 613 layer-1/4/6 type-
and-ability-setting effect. *Sketch:* an existing-permanent retype that clears printed abilities
and grants one activated ability, distinct from the token/copy path. *Cards:*
vraska_betrayals_sting.
