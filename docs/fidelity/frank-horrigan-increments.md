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

### 2. `counter-filter-axis-and-class-level-gating` — 2 cards, S — LANDED 2026-07-26
_Landed 2026-07-26: (a) `PermanentFilter::with_counter: Option<CounterAxis>` (`Any` /
`PlusOnePlusOne`), matched in `Game::permanent_matches` beside the `modified` guard. (b) the
falsified sentence below is corrected in place — the real remaining gap was
`characteristics.rs`'s `keyword_anthem_static_grants` (the `KeywordAnthem` static scanner), which
never read `ability.min_level`; it now guards the same way `matching_anthems`
(`characteristics.rs:1298`) and `cost_reduction` (`characteristics.rs:1737`) already did. Swept
every other `Timing::Static` scanner in `characteristics.rs` for the same gap: the attachment-scan
family (`ControlAttached`, `SetAttachedBasePt`, goad/cant-attack/cant-block grants, etc.) is
structurally exempt — an Aura can't be a Class, so it never carries `min_level`. Left deliberately
ungated, with no driving card yet: `granted_mana_abilities` (`GrantManaAbility`),
`has_no_max_hand_size` (`NoMaximumHandSize`), `grants_graveyard_recursion`
(`PlayFromGraveyardOncePerTurn`), the token/life-gain/cast-X replacement scanners
(`TokenReplacement`, `LifeGainReplacement`, `CastXReplacement`), and the four combat/noncombat
damage-prevention self-shield queries (`noncombat_damage_prevented_to_creature`,
`phantom_shield_active`, `combat_damage_prevented_to_creature`,
`combat_damage_prevented_by_source`) — each is a real latent instance of the same bug shape if a
future Class ever carries one of those statics at a level, but none does today; gate them when a
card actually needs it. inspiring_call is fully faithful; innkeepers_talent keeps a trimmed
`approximates`. Still blocked: (i) innkeepers_talent's Level 3 counter-doubling replacement, on
#19; (ii) innkeepers_talent's Level 2 ward doesn't cover the Class itself — the engine tracks a
Class's level as a plain `Permanent::level` scalar, not the level counters CR 717.2 puts on it,
so `has_any_counter` returns false for a leveled Class and it fails its own `with_counter = "any"`
filter. Fixing that means modeling level as real counters (a Class-model change, out of this
increment); noted on the card and on the `with_counter` row in DSL_REFERENCE._
Depends on: nothing.
Two small gaps that meet on Innkeeper's Talent.
(a) `PermanentFilter` has no counter axis. `modified` (CR 701.29) is strictly broader — it also
matches Auras and Equipment — so it can't express "creature you control with a +1/+1 counter on
it" (Inspiring Call, for both its `per_permanent` draw count and its indestructible grant) or
"permanents you control with counters on them" (Innkeeper's Talent Level 2). *Sketch:* a
`with_counter: Option<CounterAxis>` field distinguishing "any counter" from "+1/+1 specifically".
(b) `min_level` is consulted at `triggers.rs:2981`, `cast.rs:2035` (activated abilities),
`characteristics.rs:1298` (the `Anthem` scanner), and `characteristics.rs:1737` (the
`ReduceSpellCost` scanner) — but *not* by `characteristics.rs`'s `keyword_anthem_static_grants`
(the `KeywordAnthem` static scanner), so a Class's level-gated keyword-anthem statics function at
level 1 — Innkeeper's Talent's Level 2 ward would be live the moment it hits the battlefield.
*Sketch:* copy the `min_level` guard `matching_anthems` already uses into
`keyword_anthem_static_grants`. *Cards:* inspiring_call, innkeepers_talent (Level 3's doubler
static waits on #19).

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

### 5. `opponent-count-condition` — 1 card, S — LANDED 2026-07-26
_Landed 2026-07-26: `Condition::YouHaveOpponents { at_least }` (TOML `"you_have_opponents"`) —
`condition_holds`'s existing `living_players().filter(|&p| p != ctx.controller).count()` shape,
matching `AnOpponentControlsLands`'s neighbouring arm. Every other seat is an opponent (CR 102.3);
an eliminated seat drops out (CR 800.4a), so the count is live seats, not the table's starting
size. undergrowth_stadium authored fresh (absent from the pool) and is fully faithful — a two-line
land, no residual. Still blocked: nothing._
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

### 7. `one-way-damage-equal-to-power` — 1 card, S — LANDED 2026-07-26
_Landed 2026-07-26: `MiscEffect::Fight` grew a `one_way: bool` (TOML `one_way`, default `false`)
alongside `ally_is_shared_target`; `Game::fight` guard-returns before the enemy→ally damage event
when set. Correction to this section's premise: the cast/resolution target split isn't "both
targets chosen up front" — Infectious Bite reuses the same cast-time-enemy /
resolution-time-pause-for-ally split every other `fight`-shaped effect here already uses
(Decisive Denial, Primal Might), just with the back-half damage event skipped; this is not a
fight at all (CR 701.12/701.12c never apply — the oracle text never says "fights"), so nothing
that cares about fighting is told one happened. infectious_bite authored, fully faithful — its
poison rider (`put_counters_on_player`, `each_opponent` scope) landed in #20 slice 1 as noted.
Still blocked: nothing._

### 8. `phyrexian-mana` — 1 card, S/M
Depends on: nothing.
`{B/P}` is unsupported in `[cost]`. *Sketch:* a hybrid-with-life pip that the cost payer may
settle with either the color or 2 life, recording *which* was chosen so #16's compleated rider
can read it. *Cards:* vraska_betrayals_sting (with #16).

**Pre-written brief** at `/private/tmp/claude-501/-Users-reilley-Repositories-mtgfr/a6559256-1122-41b4-8623-2c96d64f687b/scratchpad/brief-8.md`.
#8 stalled 6/6 in wave 7 — the agent could not locate the mana-payment surface and burned its
whole context searching. The brief names every site (`types/mana.rs:5/24/38/92/114/142/801/1219`
plus the shockland pay-life-choice precedent). Use it as written; do not re-derive.

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

### 10. `per-mode-targets-on-activated-abilities` — 1 card, M — LANDED 2026-07-26
_Landed 2026-07-26: `ChoiceRequest::ChooseMode`/`PendingChoice::ChooseMode` gained an
`activated: bool` (existing resolution-time triggered-ability raises pass `false`, unchanged
behavior). `Game::activate_ability` now branches on `Effect::ChooseOne` once costs are paid and
the ability would otherwise hit the stack, raising `ChooseMode { activated: true, target: None,
… }` instead of placing. `answer_choose_mode` splits on the flag: `false` is the original
run-immediately triggered-ability path; `true` takes the chosen mode and either places it
straight onto the stack (`place_ability_second_clause`, mirroring the `ChooseTarget` handler's
own "up to N, declined" placement call) when the mode's own `effect.target()` is
`TargetSpec::None` (Cankerbloom's proliferate mode), or raises a fresh `ChoiceRequest::ChooseTarget
{ activated: true, … }` scoped to that mode's own legal set (the destroy-artifact/
destroy-enchantment modes) — reusing the existing `ChooseTarget` answer handler and its
`place_ability_second_clause` call verbatim, no new pending machinery. An empty legal set for the
chosen mode is `Reject::IllegalChoice`, leaving the mode pause standing (CR 601.2c: a mode with no
legal target can't be chosen) rather than stranding the activator on an unpayable pick. `//
ponytail:` noted on the variant: CR 601.2b orders mode choice ahead of CR 601.2h cost payment;
this engine pays costs first and raises the pause once the ability would otherwise hit the stack,
unobservable because no player gets priority between the two. `cankerbloom` authored fresh
(absent from the pool), fully faithful apart from the pool's standard proliferate residual
(increment #17, not in this wave). Still blocked: nothing._
Depends on: nothing.
`Effect::ChooseOne` raises a `ChoiceRequest::ChooseMode` carrying the *ability's* single
pre-chosen target (`resolution/pause_choose.rs`), so an activated modal ability whose modes
target different things ("destroy target artifact" / "destroy target enchantment" / "proliferate")
can't be expressed. The card-level `modal`/`choose` flags are spell-timing only. *Sketch:* choose
the mode first, then raise that mode's own target request — the same two-step the spell path
already does, lifted onto activated abilities. *Cards:* cankerbloom.

### 11. `multikicker` — 1 card, M — LANDED 2026-07-26
_Landed 2026-07-26: `AdditionalCost::multikicker: Option<&'static Cost>` (TOML
`[cost.additional.multikicker]`, the same `[cost]`-shaped sub-table as `kicker`/`replicate`) —
copied end to end from Replicate, not folded into the existing binary `kicker`/`kicked` flag.
`Intent::Cast` gained `multikicker_count: u8` (proto `intent.proto` field 15, next free after
`alternative_cost = 14`), threaded through `Game::cast_cost`'s ×N pip fold beside Replicate's,
`Event::SpellCast`, and `Object::Spell::multikicker_count`; `playable.rs` rejects a nonzero count
declared against a spell with no Multikicker cost, mirroring Replicate's own gate.
`Permanent::entered_times_kicked: u8` is locked in from `Spell::multikicker_count` at
`Event::PermanentEntered` (`from` is still the resolving Spell at that point, the same "read it
before the spell is gone" idiom as `evoked`), and the new `Amount::TimesKicked` (TOML
`"times_kicked"`) reads a new `Game::times_kicked` helper that checks *both* `Object::Spell` and
`Object::Permanent` — the enters-with-counters site's `source` is already the fresh permanent by
resolution time, so a spell-only read would have silently returned 0 (caught by a dedicated TDD
test). Not surfaced on the wire to the client (`VisibleEvent::SpellCast` drops it with a
`ponytail:` note, same as `replicate_count`); client codegen not required (gitignored, no client
reader). everflowing_chalice authored fresh and is fully faithful (`{0}` cost needed an explicit
`[cost]\ngeneric = 0` table for the frame audit to match Scryfall's printed `{0}`, since an absent
`[cost]` also parses to zero but audits as "no cost text"). The existing single-kicker `kicked: bool`
and `Amount::IfSpellKicked` are untouched; `types/mana.rs`'s single-kicker ponytail is trimmed to
name only its one remaining residual (a card printing both a single Kicker and a Multikicker cost
isn't modeled — none in the pool does). Still blocked: nothing._
Depends on: nothing. **Falsifies `types/mana.rs:223`** ("single-kicker only … grow those from a
real card that needs one" — the deferral condition is now met).
*Sketch:* a `multikicker` cost that may be paid any number of times, recording the *count* paid on
the spell, plus `Amount::TimesKicked` so "enters with a charge counter for each time it was
kicked" can read it. The existing binary `if_kicked` stays as sugar for count ≥ 1. *Cards:*
everflowing_chalice.

### 12. `monstrosity` — 1 card, M — LANDED 2026-07-26
_Landed 2026-07-26: `Permanent::monstrous: bool` (never cleared at Untap; a fresh object always
starts `false`), `CountersEffect::Monstrosity { count }` (TOML `type = "counters"`,
`mode = "monstrosity"`) — a no-op (CR 701.28c, not even the counters) if the source is already
monstrous, otherwise its +1/+1 counters route through the same replacement pipeline `PutCounters`
uses before `Event::BecameMonstrous` sets the flag, so Doubling Season/Hardened Scales apply and the
flag sets even if a replacement drives the count to zero. `Trigger::BecomesMonstrous` (TOML
`"becomes_monstrous"`) fires off that event via the same self-scan idiom as `TurnedFaceUp`, never on
a later no-op activation. alpha_deathclaw authored fresh (absent from the pool) and is fully
faithful — all three printed abilities (menace, trample; the "enters or becomes monstrous" destroy
authored as two `[[abilities]]` blocks sharing one oracle sentence, like the commander's "enters or
attacks"; the `{5}{B}{G}: Monstrosity 4` activated ability) with no residual. Still blocked:
nothing._
Depends on: nothing.
Monstrosity (CR 701.28) is absent entirely. *Sketch:* a `monstrous: bool` on `Permanent`, an
activated `monstrosity N` effect that is a no-op when already monstrous and otherwise places N
+1/+1 counters (through the existing replacement pipeline) and sets the flag, plus a
`Trigger::BecomesMonstrous`. Alpha Deathclaw's "enters **or** becomes monstrous" is one ability
with two trigger conditions, like the commander's "enters or attacks". *Cards:* alpha_deathclaw.

### 13a. `garruk-cursed-huntsman-wolves` — 1 card, M — LANDED 2026-07-27
_Landed 2026-07-27: `garruk_cursed_huntsman.toml` authored fresh (absent from the pool) with its
`0` and `−3` abilities; the `−6` emblem mode is omitted and named in `approximates` (CR 114
emblems don't exist in the engine — see #13b, now unblocked). Its Wolf token
(`crates/cards/data/tokens/wolf.toml`) carries its own `dies` trigger, a new
`CountersEffect::PutLoyaltyCounterEach { filter }` — "put a loyalty counter on each Garruk you
control" walks the battlefield by `PermanentFilter` (same shape as `PutCountersEach`) and emits
`Event::LoyaltyChanged { amount: 1 }` per match, since loyalty is the scalar `Permanent::loyalty`,
not a `CounterKind`; no new counter kind added. The filter names a permanent type/subtype, not the
creating walker, so a Wolf's death still bumps an unrelated Garruk you control after the creating
Garruk has died (tested), and never an opponent's Garruk (tested). Also cleared the stale note at
`promise_of_loyalty.toml:3` ("planeswalker defenders unmodeled" — already false;
`combat.rs`'s attack-declaration `resolved` list records every attack against its defending player
regardless of whether the declared target was that player or their planeswalker, so the "or
planeswalkers you control" clause was already covered) and the same false claim on
`counter_scaled_attack_tax`'s and `cant_be_attacked_by`'s `DSL_REFERENCE.md` rows. Still blocked:
the `−6` emblem residual, cleared by #13b (LANDED 2026-07-27).
Depends on: nothing. **Clears the stale note at `promise_of_loyalty.toml:3`** ("planeswalker
defenders unmodeled" — already false; `Defender::Planeswalker` exists and `combat.rs:430` says so).
Garruk, Cursed Huntsman's `0` and `−3` abilities plus its Wolf token, whose own death trigger
puts a loyalty counter on each Garruk its controller controls — a token ability pointing at a
permanent type, not at its creator, so it survives the creating walker's death. Loyalty is the
scalar `Permanent::loyalty`, mutated by the existing `Event::LoyaltyChanged`; no new
`CounterKind`. The `−6` emblem mode is omitted and flagged in `approximates`. Fix the Promise
of Loyalty note in the same change. *Cards:* garruk_cursed_huntsman.

### 13b. `emblems` — clears 13a's residual, L — LANDED 2026-07-27
_Landed 2026-07-27: emblems (CR 114) exist. Took the **command-zone-object** shape, not a
dedicated `Game::emblems` store: an emblem is an `Object::Card` in `Zone::Command` with
`commander = false`, which needs no new struct, gives the emblem a real `ObjectId` (so
`Game::matching_anthems` takes it as a third source chain beside battlefield and graveyard sources
with no signature change), and is an unambiguous discriminator — the engine's only other two ways
into `Zone::Command` (`Game::designate_commander`, `Event::MovedToCommandZone`) both hardcode
`commander = true`, and both castability gates require it, so an emblem is never castable.
`Game::emblems(player)` reads them back; there is no remover, by CR 114.5. New
`MiscEffect::GetEmblem { emblem }` names the emblem by Scryfall oracle id through the existing
`data/tokens/` `CardDef` registry (`crates/cards/data/tokens/emblem_garruk_cursed_huntsman.toml`,
`[kind] type = "sorcery"` so its `TypeSet` is empty — CR 114.5's "no characteristics other than
its abilities"); its single static ability is a plain `StaticEffect::Anthem { power = 3,
toughness = 3, keywords = ["trample"] }`. New public `Event::EmblemCreated` (never redacted —
CR 114.2) with its `VisibleEvent`/proto arms. `garruk_cursed_huntsman.toml` is fully faithful —
`approximates` deleted. Still blocked: nothing._
Depends on: 13a (LANDED 2026-07-27, unblocked). Emblems are an ownerless, unremovable, non-permanent object in a per-player
store carrying static abilities only (CR 114.1–114.5). Garruk's is a `StaticEffect::Anthem`
("Creatures you control get +3/+3 and have trample"), which already exists — the store is the
only new machinery. Wire the `−6` and delete 13a's `approximates` line. No emblem removal,
copying, or targeting: CR 114 says none exist.

**Both 13a and 13b have pre-written briefs** at `/private/tmp/claude-501/-Users-reilley-Repositories-mtgfr/a6559256-1122-41b4-8623-2c96d64f687b/scratchpad/brief-13.md` (verbatim oracle text
and `file:line` anchors). #13 stalled 6/6 in wave 6 as a single increment; the original sketch
also misquoted the card (`0:`, not `+1:`). Use the briefs as written.

### 14. `double-counters-or-cull-and-gain` — 1 card, M — LANDED 2026-07-26
_Landed 2026-07-26: the premise above overstated the gap — `CountersEffect::DoubleCounters`
already existed and already routed through the counter-replacement pipeline
(`doubled_counters_event` → `counters_after_replacements`); only the source-power-at-most
condition and the cull-and-gain-life effect were actually missing. Added
`Condition::SourcePowerAtMost { at_most }` (source-object-based, same shape as
`SourceHasCounters`/`SourceEnteredWithXAtLeast` — only reachable inside `{ type = "conditional",
… }`, special-cased at the `Game::run` resolve site) and
`CountersEffect::RemoveAllButOnePlusOneCounterThenGainLife { target }` (keeps exactly one +1/+1
counter — a no-op at zero or one already present — and gains 1 life per counter *actually
removed*). Also fixed a real bug the card's own "if … Otherwise …" shape exposed:
`Effect::Conditional` gained an `otherwise: &'static [Effect]` field (default empty, backward
compatible) because the established "two independently-conditioned `conditional` steps sharing one
condition" pattern (Whirlpool Whelm/Court Hussar) mis-fires when the first step's own effect
changes what the shared condition reads — Lily's "double" mutates her own power, so at any power in
[9,16] doubling would cross back over the 16 threshold and the second (negated) step would
spuriously *also* cull right after doubling. `otherwise` evaluates `condition` exactly once and
branches, closing that gap; Lily is authored as a single `conditional` step with `then`/`otherwise`
instead of two. lily_bowen_raging_grandma is fully faithful — no `approximates`. Still blocked:
nothing._
Depends on: nothing.
Lily Bowen's upkeep needs three things the DSL lacks: a source-power-**at-most** condition
(`Condition` has `TargetPowerAtLeast` and `SourceHasCounters { at_least }` only), "double the
number of +1/+1 counters on this" (which must route through the counter-replacement pipeline —
with Branching Evolution out it is 2×, with it 4×), and "remove all but one +1/+1 counter, then
gain 1 life for each removed **this way**" (`remove_all_counters_then_draw` draws instead, and the
count must be the number actually removed). *Cards:* lily_bowen_raging_grandma.

### 15. `grant-triggered-ability-to-attached` — 1 card, M — LANDED 2026-07-26
Depends on: nothing.
`grant_to_attached`'s `granted_ability` is a `GrantedAbility { cost, effects }` — activated only.
Power Fist (`{1}{G}` Equipment, Equip {2}) grants the equipped creature **trample** *and* a
*triggered* ability ("whenever this creature deals combat damage to a player, put that many
+1/+1 counters on it") — the trample half already lands via `GrantToAttached { keywords }`, the
gap is the triggered half. *Sketch:* let the granted ability carry a `Trigger` instead of a cost,
and have the trigger scanners see granted abilities as well as printed ones. Note "that many" is
the damage dealt, so the grant must thread the trigger's own damage amount into the counter count.
*Cards:* power_fist.

**LANDED 2026-07-26:** `GrantedAbility` gained `trigger: Option<Trigger>`, mutually exclusive with
the activated-only `cost` path (`Game::granted_attachment_abilities` now excludes any grant with
`trigger.is_some()`). `Game::queue_combat_damage_triggers` chains a new
`Game::granted_attachment_triggers(host)` accessor alongside `functional_abilities` (ponytail-scoped
to that one scanner — the pool's one consumer). `Amount::CombatDamageDealt` already threaded the
damage amount via `fill_combat_damage`/`contextualize_effect`, so no new amount surface was needed.
`power_fist.toml` is faithful: trample via `GrantToAttached { keywords }`, the counters trigger via
the new `trigger` field, Equip {2} — no `approximates`. Still blocked: nothing.

### 16. `compleated` — 1 card, M
Depends on: #8 phyrexian-mana.
"If life was paid, this planeswalker enters with two fewer loyalty counters" — an as-enters
replacement reading how the `{B/P}` pip was actually settled. *Sketch:* the cast records the
life-paid choice on the spell; the planeswalker's enters-with-loyalty read subtracts 2 per pip
paid with life. Also needs Vraska's −2 (#25). *Cards:* vraska_betrayals_sting.

### 17. `proliferate-full-scope` — 9 cards + observers, L — LANDED 2026-07-27
Depends on: #20 slice 1 (for the player half).
`proliferate()` (`pending/raise/optional.rs:14`) enumerates only `game.battlefield()` filtered on
`plus_counters > 0 || kind_counters.any(...)`. It therefore silently omits **two** things:
players (poison/rad) and **loyalty** counters (a bare `Permanent::loyalty` i32, not a
`CounterKind`). The pool's one existing proliferate card already prints "any number of
permanents **and/or players**" in its own reminder text, so this is a live wrong-behavior bug, not
just a missing feature. *Sketch:* widen the choice's candidate enumeration and its answer
validation to a `ProliferateTarget` sum (permanent | player), and make the "give another counter
of each kind already there" step address loyalty alongside `kind_counters`. *Cards:* every
proliferate source in the deck — agent_frank_horrigan, atomize,
contagion_clasp, contagion_engine, karns_bastion, evolution_sage, thirsting_roots,
unnatural_restoration, drown_in_ichor, glistening_sphere, contaminant_grafter, cankerbloom,
blightbelly_rat, bloated_contaminator, vraska_betrayals_sting — plus the existing pool's
expansion_algorithm.

_Corrected 2026-07-27: two of the four originally-listed gaps are not gaps at all. A **Class's
level is not a counter** (confirmed by design; proliferate does nothing to a Class), so
`Permanent::level` is correctly out of scope. A **suspended card in exile is neither a permanent
nor a player**, so CR 701.27 can never reach `Game::exile_time_counters` — the
`CounterKind::Time` ponytail note claiming otherwise was wrong and has been deleted rather than
trimmed._

_Landed 2026-07-27: `ProliferateTarget { Permanent | Player }` is the choice's option and answer
type; `Intent::ChooseProliferate { permanents, players }` (proto tag 58, `Answer::Proliferate`)
replaces the borrowed `ChooseSacrifices` wire shape, since a seat can't ride a `Vec<ObjectId>`.
`PlayerCounterKind::ALL` landed with its first consumer. Player counters are placed unreplaced —
`counters_after_replacements` stays +1/+1-and-permanent-only until #19. All 13 proliferate
cards in the deck are faithful with no `approximates`. Still blocked: nothing in the engine —
but the **client has not caught up**: `client/app/board/action/targeting.ts` still lists
`proliferate` in `ONBOARD_CARD_PICK_KINDS` and answers it through the sacrifice multi-select
path, which now returns `Reject::IllegalChoice`. It must send `Answer::Proliferate { permanents,
players }` and let a seat be picked._

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

### 20. `player-counters-poison-infect-toxic` — 17 cards, XL — LANDED 2026-07-27
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
*2026-07-26 — slice 2 built (the XL is not LANDED; slices 3–5 remain).* `Keyword::Infect` plus two
shared chokes in `resolution/damage.rs` — `Game::creature_damage_events` (`Event::DamageMarked` →
`Event::KindCountersPlaced { kind: MinusOneMinusOne }`) and `Game::player_damage_events`
(`Event::LifeChanged` → `Event::PlayerCountersPlaced { kind: Poison }`) — with every damage mint
routed through them: all eight `mint_damage` arms (single target, each-creature, each-player,
each-other-opponent, to-entering-permanent, to-self, to-target-controller) and `combat.rs`'s
blocker-assignment loop, `deal_creature_damage` (combat damage **and** fight) and `damage_player`.
Prevention/protection guards, `DamageDealtToPlayer` / `CombatDamageDealtToCreature` /
`DeathtouchMarked`, lifelink and the commander tally all still fire off the original amount;
planeswalker damage is untouched. `resolve_deal_damage_to_entering`'s "did the damage land" probe
recognises the counter form too. plague_stinger, phyresis and ichor_rats are faithful. Residual
(`ponytail:` on `creature_damage_events`): the two watchers that ride a bare `Event::DamageMarked`
— Armadillo Cloak's enchanted-host damage trigger and Vampiric Dragon's `damaged_this_turn` tally —
don't see infect damage; upgrade path is a source-carrying `Event::DamageDealtToCreature` marker.

**Slice 3 — `Keyword::Toxic(u8)`.** CR 702.164: *in addition to* its normal combat damage, a
creature with toxic N gives the player it damages N poison counters. Unlike infect this does not
replace the damage. Multiple instances add (CR 702.164b). *Cards:* bilious_skulldweller,
blightbelly_rat, bloated_contaminator, necrogen_communion (Aura grant).
*2026-07-27 — slice 3 built (the XL is not LANDED; slices 4–5 remain).* `Keyword::Toxic(u8)`
(TOML `{ toxic = N }`), `Game::toxic_amount` beside `ward_amount` — a **sum** over
`effective_keywords`, not a first-match, because CR 702.164b makes multiple instances add — and the
`Event::PlayerCountersPlaced { kind: Poison }` push at the tail of `combat.rs::damage_player`,
below every prevention guard (prevented combat damage was never dealt, so it grants no counters)
and on the combat path only (a toxic source's noncombat damage grants nothing). Toxic stacks with
infect: an infecting toxic-1 creature dealing 3 gives 4 poison and costs no life. No new wire
surface — the slice-1 `VisibleEvent::PlayerCountersPlaced` / `PlayerView.poison` already carry it;
`wire_keyword`/`keyword_label` gained `toxic:N` / `Toxic N`. bilious_skulldweller and
necrogen_communion (including its `reanimate_dying_enchanted_creature` return-under-your-control
clause) are faithful; blightbelly_rat and bloated_contaminator carry only the shared #17
proliferate-scope `approximates`.

**Slice 4 — poison readers.** `Condition::AnOpponentHasPoisonAtLeast { at_least }` (the printed
**Corrupted** ability word, CR 702.165), `Amount::OpponentsPoisonCounters`, and Vraska's −9
"top up to nine" (a *difference*, not a fixed add — it must place nothing if the target already
has nine or more). *Cards:* contaminant_grafter, glistening_sphere, phyrexian_swarmlord,
vraska_betrayals_sting, venerated_rotpriest.

Two toxic cards moved here out of slice 3 — printing toxic is not what gates them:
- **contaminant_grafter** was already listed in this slice for its Corrupted ability
  (`Condition::AnOpponentHasPoisonAtLeast`); its "whenever one or more creatures you control deal
  combat damage to one or more players" is additionally a *batch* watch that no `CombatDamageScope`
  arm expresses (`types/trigger.rs:578`).
- **venerated_rotpriest** needs "whenever a creature **you control** becomes the target of a spell"
  — `Trigger::BecomesTargeted` (`types/trigger.rs:429`) is self-referential only — plus a
  *targeted-opponent* player-counter put, which `CountersEffect::PutCountersOnPlayer`'s `EdictScope`
  cannot express (its only targeted arm, `TargetedPlayers`, is "any number of target players",
  `types/filter.rs:860-871`).

*2026-07-27 — slice 4 built (the XL is not LANDED; slice 5 remains).*
`Condition::AnOpponentHasPoisonAtLeast { at_least }` is an existential over living opponents, and
the same `Ability::condition` slot serves Contaminant Grafter's intervening-if (CR 603.4) and
Glistening Sphere's activation restriction (CR 602.5b) — `Game::ability_activation_gate` already
honored the slot, so no new field. `Amount::OpponentsPoisonCounters` sums (not maxes) across living
opponents. Vraska's −9 is its own effect, `CountersEffect::TopUpCountersOnPlayer { kind, to }`
(TOML `top_up_counters_on_player`), targeting a player and emitting **no** event at all when the
target is already at or above `to`. Venerated Rotpriest took the `EdictScope::TargetedOpponent`
route over a second targeting spelling: `PutCountersOnPlayer` reports
`TargetSpec::OpponentPlayer` under that scope and the shared targeting machinery does the rest.
Two watchers widened: `CombatDamageScope::YourCreaturesBatch` drains through
`BatchTriggerScratch::creatures_dealt_combat_damage_this_batch` for exactly one trigger per combat
damage step (CR 603.3b), and `Trigger::BecomesTargeted` gained a `who: BecomesTargetedScope` axis
(`this` / `creature_you_control`, TOML sibling `targeted`) instead of a second trigger variant.
contaminant_grafter, glistening_sphere, phyrexian_swarmlord (plus the `phyrexian_insect` token
profile) and venerated_rotpriest are faithful with no `approximates` — the proliferate-scope note
they were authored with was cleared when #17 landed in the same wave; vraska_betrayals_sting names
three residuals — the `{B/P}` pip modeled as plain `{B}` (#8), Compleated's enters-with-two-fewer that
therefore never applies (#16), and the −2 becomes-a-Treasure mode dropped rather than approximated
(#25). Follow-up noticed: `striding_shotcaller.toml` still carries a `ponytail:` note saying its
`who = "your_creatures"` approximates a batch watch — `"your_creatures_batch"` now expresses it
faithfully.

**Slice 5 — poison-scaled pump.** Phyresis Outbreak's "each creature your opponents control gets
-1/-1 until end of turn **for each poison counter its controller has**" — a per-permanent amount
resolved against *that permanent's controller*, not the spell's controller. *Cards:*
phyresis_outbreak.
*2026-07-27 — slice 5 built; with it the XL is LANDED.* `Amount::ControllersPoisonCounters` (TOML
`controllers_poison_counters`) reads the poison on the one player the amount is relative to, and
`PumpEffect::WeakenEachCreature` now resolves both amounts **inside** its per-creature loop with
that creature's own controller as `resolve_amount`'s `controller` argument — the parameter is "the
player this amount is relative to", not "the effect's controller" (CR 122.1). The three existing
weakeners (Massacre, Toxic Deluge, Doomwake Giant) pass `Fixed`/`X`, which ignore that argument, so
the hoist-removal is behaviour-preserving for them. The two clauses are two ordered
`[[abilities.effects]]` blocks, which is exactly the "Then" sequencing (CR 608.2): an opponent at
zero poison ends the resolution at one and takes `-1/-1`. phyresis_outbreak is faithful with no
`approximates`.

Landing rule: this XL is LANDED only when all five slices are built; slices get dated progress
notes until then.

### 21. `rad-counters` — 2 cards, L — LANDED 2026-07-27
Depends on: #20 slice 1 (the player counter store).
Rad counters are a player counter with their own turn-based rules action: at the beginning of each
player's precombat main phase, a player with rad counters mills that many cards, and for each
**nonland** card milled this way loses 1 life and removes one rad counter. *Sketch:* a `Rad` kind
in the player store plus the rules action in the precombat-main step alongside the existing
turn-based actions — it is a rules action, not a triggered ability, so it uses no stack and no
player may respond to it. *Cards:* feral_ghoul, bloatfly_swarm (also needs #22).

_Landed 2026-07-27: `PlayerCounterKind::Rad` sits beside `Poison` in the player counter store (no
lose-the-game threshold — `LETHAL_POISON` stays poison-indexed), and
`Game::perform_rad_counter_mill` runs it as a `Step::Main1` turn-based action in
`perform_turn_based_actions`: no stack object, no priority window, only the active player's own
counters. It reuses `Game::mill_events`, counts the non-`CardKind::Land` cards actually milled, and
emits one `LifeChanged` of `-n` plus one `PlayerCountersPlaced` of `-n` from that real count, so a
short library never removes more counters than it spent. `feral_ghoul.toml` is faithful — menace,
the "another creature you control dies" +1/+1 trigger, and a `dies` trigger reading
`Amount::SourcePower` off the existing CR 603.10a `dying_creature_stats` LKI capture. `PlayerView`
carries a public `rad` count (`stream.proto` field 14). Still blocked: no client surface shows the
`rad` count yet; the board needs a chip beside the poison indicator._

### 22. `damage-prevention-replacing-counters` — 1 card, M — LANDED 2026-07-27
Depends on: #21.
Bloatfly Swarm: "if damage would be dealt to this creature while it has a +1/+1 counter on it,
prevent that damage, remove that many +1/+1 counters from it, then give each player a rad counter
for each +1/+1 counter removed this way." A self-hosted damage replacement (CR 615) whose payload
is counter removal, capped by the counters actually present — the removal count is
`min(damage, counters)`, and the rad counters follow that *actual* number. *Cards:*
bloatfly_swarm.

_Landed 2026-07-27: a new `StaticEffect::PreventDamageToSelfRemovingCountersGivingRad` sibling of
Phantom Centaur's `PreventDamageToSelfRemovingCounter`, found by the same
`Game::phantom_shield_active` scan (both are self-only, CR 615, and cover combat and noncombat
damage alike). `Game::phantom_shield_counter_removal` now takes the incoming damage `amount` and
returns `Vec<Event>`: Phantom Centaur's variant still ignores `amount` and removes exactly one
counter (regression-tested against a 5-damage hit); Bloatfly Swarm's removes
`min(amount, counters present)`, then emits one `PlayerCountersPlaced` rad counter per living
player per counter removed (CR 102.1 — the controller included). All five call sites
(`resolution/damage.rs` ×3, `combat.rs` ×2) thread the amount through unchanged.
`bloatfly_swarm.toml` is faithful, reusing the existing `enters_with_counters` static mode for its
five +1/+1 counters._

_Reconciled 2026-07-27 (verify stage): the two shields do **not** share a CR 614.1 predicate, and
`phantom_shield_active` was originally a bare `matches!` over both variants — so a Bloatfly Swarm
with no +1/+1 counters still prevented damage outright, where its "while it has a +1/+1 counter on
it" clause means the replacement simply doesn't apply and the damage is dealt and marked normally.
The lane's own `bloatfly_swarm_takes_damage_normally_with_no_counters` could not catch it: a
counterless Bloatfly Swarm is a 0/0 that dies to the CR 704.5a state-based action whether or not
the damage landed, so the assertion passed either way. `phantom_shield_active` now branches per
variant (Phantom Centaur unconditional, Bloatfly Swarm gated on `plus_counters(target) > 0`),
which fixes all five call sites at once since every one of them gates its early return on that
predicate. Regression: `bloatfly_swarm_with_no_counters_is_dealt_damage_normally` holds it alive
under anthems so the marked damage is observable._

### 23. `final-act-player-counter-mode` — 1 existing card, S — LANDED 2026-07-27
Depends on: #20 slice 1. **Falsifies `final_act.toml:13` and `:22`.**
Final Act's fifth mode ("each opponent loses all counters") was dropped because "this pool tracks
no player-level counters" — that justification dies with #20, and the `approximates` text is
surfaced verbatim to users in the catalog. Restore the mode, raise `choose_max` from 3 to 4, and
trim the note to the genuinely-remaining "destroy all battles" residual. *Cards:* final_act
(existing pool).

_Landed 2026-07-27: new `CountersEffect::RemoveAllPlayerCounters { scope }` (mirrors
`PutCountersOnPlayer`'s `EdictScope` shape, iterating every `PlayerCounterKind` via a new `ALL`
const) restores the fifth mode as `final_act.toml` mode 3, `choose_max` raised to 4. The
`approximates` note now names only the "destroy all battles" residual. Still blocked: that
one mode — battles aren't a modeled permanent type, and no card in this deck makes one._

### 24. `plus-minus-counter-annihilation-sba` — observers, S — LANDED 2026-07-26
Depends on: nothing. **Falsifies `characteristics.rs:1100`.**
CR 704.5r: a permanent with both +1/+1 and -1/-1 counters has N of each removed as a state-based
action. The note calls this "unobservable today (no pool card puts both kinds on one creature)" —
Contagion Clasp and Contagion Engine both already place real -1/-1 counters onto a deck that stacks
+1/+1 counters on nearly every creature (infect does *not* land -1/-1 counters here — that's future
#20 slice 2 work; corrected out of this section's premise, which previously claimed it did).
*Sketch:* one sweep in `check_state_based_actions` removing `min(plus, minus)` of each; delete the
ponytail. *Cards:* contagion_clasp, contagion_engine.
_Landed 2026-07-26: a sweep at the top of `check_state_based_actions` (`apply.rs`) removes
`min(plus_counters, kind_counters[MinusOneMinusOne])` of each kind via the same negative
`CountersPlaced`/`KindCountersPlaced` idiom `remove_all_counters_events` uses, placed before the
death/toughness sweep in the same function. Ordering is provably immaterial here — both kinds
already contribute independent P/T deltas in `pt_layers`, so a creature's net toughness is
identical whether or not annihilation has run yet; the sweep matters only for counter-counting
readers (has-a-counter checks, proliferate's candidate scan), and `sweep_state_based_actions`'s
existing fixpoint loop covers any residual ordering concern regardless. Deleted the falsified
"unobservable today" ponytail paragraph in `characteristics.rs` (kept the CR 121.4/122.1 sentence
explaining the -1/-1 layer itself). Tests (`crates/engine/tests/game.rs`):
`plus_and_minus_counters_annihilate_as_a_state_based_action`,
`plus_and_minus_counters_fully_annihilate_when_counts_are_equal`,
`plus_and_minus_counter_annihilation_can_still_kill_in_the_same_sba_sweep`. `contagion_clasp` /
`contagion_engine` keep their #17 proliferate `approximates` notes untouched — this increment
doesn't clear those._

### 25. `becomes-treasure-losing-all-else` — 1 card, M
Depends on: nothing.
Vraska's −2: "target creature becomes a Treasure artifact with '{T}, Sacrifice this artifact: Add
one mana of any color' and loses all other card types and abilities." A CR 613 layer-1/4/6 type-
and-ability-setting effect. *Sketch:* an existing-permanent retype that clears printed abilities
and grants one activated ability, distinct from the token/copy path. *Cards:*
vraska_betrayals_sting.
