# Unlimited Edition (`2ed`) increments (2026-07-27)

Set report: [2ed.md](2ed.md). This file is the sole engine-capability backlog for `2ed`
(ranked increments + per-card exotics). Numbering is local to this file.

This is a **set** grind, not a deck grind — intake is Scryfall `set:2ed unique:cards`, not an
Archidekt link, and there is no precon to ship at the end. 292 unique cards: 28 already in the
pool, 134 authorable today, 127 blocked here, 4 out of scope.

Ranked S-first within dependency order. The centre of gravity is three clusters that between
them gate 51 cards:

- **Damage prevention** (#4, #5, #6) — 1993 white and artifact are built almost entirely out of
  prevention shields, and the engine has none. 18 cards. (#4 landed the consumable shield the
  rest of the cluster is built on; its four odd-arithmetic residuals are #68–#71.)
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

### 1. `permanent-count-amount` — 2 cards, M — **done**
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
*Landed:* the sketch was wrong about what was missing. `Amount::PerPermanentMatching { filter,
zone }` already counts permanents matching a filter, and `filter.controller = "you"` already picks
a side; what no card could say was *whose* "you" it is. Karma's count belongs to the player being
billed, not to Karma's controller — so `DamageEffect::ToTriggeringPlayer` now resolves its amount
against the recipient rather than the source's controller. That is the whole `who` axis: no new
`Amount` variant, no new field, and every other `to_triggering_player` card in the pool is a flat
`amount = 1` that cannot tell the difference. Power Surge needed one genuinely new amount,
`"untapped_lands_at_turn_start"`, because its count is a snapshot — tapping out with the trigger on
the stack must not shrink it. The snapshot is taken in `Game::apply`'s `Event::StepBegan` arm when
the *upkeep* begins, not at untap: the Untap arm runs *before* the untap turn-based action, and no
player receives priority between untapping and the upkeep (CR 502.3), so the later moment holds the
same count and needs no new event or wire message. The five other cards listed here never needed
this increment: the four `*/*` creatures are pure #2 (the existing filtered count already says what
they count), and volcanic_eruption turned out to be unrelated — see #73.
*Cards:* karma, power_surge.

### 2. `characteristic-defining-power-toughness` — 3 cards, M — **done**
Depends on: nothing (#1 turned out not to be a prerequisite — see its *Landed:* note).
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
*Landed:* the CDA needed no new continuous-effect machinery and no cache work.
`StaticEffect::BasePowerToughnessFromAmount { power, toughness }` is read by `pt_base` — the
function that hands `apply_pt_layers` its starting numbers — so the defining count *replaces the
printed box* rather than joining the layer list. That is exactly CR 604.3/613.3's layer 7a for
free: a later base-set Aura (Darksteel Mutation) still clobbers it in 7b, and counters and anthems
still sum on top in 7c/7d, with no timestamp to invent. Both amounts resolve on every uncached
recompute, so the count is live; the battlefield invalidation hooks that already existed cover it
(the test proves a Swamp entering grows Nightmare immediately). The `[kind]` box is authored 0/0 —
`*` is not valid TOML, and `tooling/scaffold_card_frame.py` used to emit a literal `power = *`, so
it now scaffolds 0/0 with a comment instead. Keldon Warlord's "non-Wall" needed one new filter
axis, `exclude_subtypes`, which is the general form the `nonlair` field's own ponytail note asked
for on a second subtype exclusion; `nonlair` stays as it is because it deliberately reads a land's
*printed* type line, not the layered view. Three of the six cards listed here turned out to want
different things and were split out — Gaea's Liege into #74, Aspect of Wolf into #75, Animate
Artifact into #76.
*Cards:* keldon_warlord, nightmare, plague_rats.

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
*defending* player's land subtypes through the existing `Game::lands_with_subtype_controlled`
(which reads *effective* subtypes, so a land whose type was changed counts — see #16). Printed, Aura-granted, and lord-granted landwalk all fell
out of shapes that already existed. Two of the seven cards did not fit after all and moved to
their own increments: Island Sanctuary (#65) and Zombie Master (#66).
*Cards:* bog_wraith, burrowing, goblin_king, lord_of_atlantis, shanodin_dryads.

### 4. `damage-prevention-shields` — 3 cards, M — **done**
Depends on: nothing.
"Prevent the next N damage that would be dealt to any target this turn" (CR 615). The engine had
prevention *statics* — `prevent_all_combat_damage_this_turn` (Fog), `prevent_combat_damage`,
`prevent_damage_to_self_removing_counter` (Phantom Centaur) — but no consumable shield: a
turn-scoped counter attached to a permanent or player that damage decrements as it is dealt.
*Landed:* `MiscEffect::PreventNextDamage { amount, target }` pushes a `(Target, i32)` entry onto a
turn-scoped `Game::damage_prevention_shields`, armed by direct mutation in `resolve_misc.rs` (the
Inkshield / Moment's Peace precedent) and cleared at the next untap step. Spending goes through
the *two* damage chokes rather than each of their dozen callers: `Game::creature_damage_events`
and `Game::player_damage_events` now return `(Vec<Event>, i32)` — the events and *what actually
landed* — so every caller's marker, lifelink, deathtouch and commander tally resize by shadowing
one binding, and the compiler found them all. Damage a shield covers entirely emits no
`DamageMarked` at all (CR 615.1), so a fully-prevented hit feeds no damage watch. The spend rides
a new `Event::DamagePrevented`, projected to the wire as `VisibleEventDamagePrevented` beside the
all-or-nothing `CombatDamagePrevented` it complements. A latent bug fell out on the way:
`DamageEffect::EachOpponent` hand-rolled its own `LifeChanged` and so bypassed infect — it routes
through the choke now.

Ten of the thirteen cards turned out not to be this shape and moved to their own increments:
the Circles and Reverse Damage were already #5, and Guardian Angel (#68), Forcefield (#69),
Rock Hydra (#70) and Power Leak (#71) each need arithmetic or a hook the plain shield doesn't
have.
*Cards:* conservator, healing_salve, samite_healer.

### 5a. `color-keyed-prevention` — 6 cards, M — **done**
Depends on: #4, #9.
The Circle of Protection cycle plus Reverse Damage, on a shield keyed to the *color* of the
source rather than to one source picked by name. All six cards ship; the source pick is #5b.

*Landed:* the shield stopped being a `(Target, i32)` pair and became `state::PreventionShield`,
because three of its four fields are riders one card prints and the plain Healing Salve shield
prints none of them. `amount` went `Option`: `None` is "prevent *that* damage", which is not
"prevent a very large amount" — it is spent outright by whatever hit it stops, so a {1} Circle
eats a four-point Drain Life and a one-point one for the same price, and there is no leftover to
carry to a second hit. `Event::DamagePrevented` grew a `source` because `apply` has to re-find
the exact shields the mint already spent, and with a color gate in play "the shields against this
target" is no longer the same list for every source; the schema projection ignores the field, so
the wire shape did not move. Both walks share `Game::shields_against` for that reason. Reverse
Damage's life rider is minted inside `spend_prevention_shields`, next to the `DamagePrevented`
and reading the actual bite, since only the spend knows how much the shield really ate.
`ColorFilter::matched_by` was pulled out as a pure predicate over the `[bool; Color::COUNT]` that
`colors_of` returns — `query.rs` now routes its own color axis through it too, and `apply` can
precompute the source's colors once instead of fighting `retain_mut` for a `&self` borrow.
*Cards:* circle_of_protection_black, circle_of_protection_blue, circle_of_protection_green,
circle_of_protection_red, circle_of_protection_white, reverse_damage.

### 5b. `source-of-your-choice-prevention` — 0 cards, L
Depends on: #5a.
Turns #5a's color gate into the printed "a black source **of your choice**". Nothing new enters
the pool; the six cards of #5a drop their `approximates` and the `ponytail:` on
`MiscEffect::PreventNextDamage::from_color` goes away. *Sketch:* `PendingChoice::ChooseDamageSource`
raised as the ability resolves, offering every object that could deal damage and matches the
`ColorFilter` — battlefield permanents *plus* objects on the stack, since a source need not be a
permanent (CR 609.7) — with the answer recorded as the shield's source `ObjectId` and checked
instead of the color at the damage choke. Sized L rather than M because a new `PendingChoice`
variant is 14 non-generated files plus proto regen, i18n, and a client prompt Scene test; the
approximation it removes only bites when two sources of the named color would hit the same player
in one turn, which the ordinary line (activate the Circle in response to the damage) never
reaches.

### 6a. `shield-redirection` — 1 card, M — **done**
Depends on: #4, #5a.
Jade Monolith. "That source deals that damage to you instead" (CR 615.10) on the same one-shot
shield record #5a built: a redirection is a replacement, not a prevention, but it arms and is
consumed identically.
*Cards:* jade_monolith.

*Landed:* `PreventionShield` gained `redirect_to: Option<Target>` and both damage chokes grew a
private `_inner` twin taking `allow_redirect`. That flag is the recursion guard: the moved damage
is dealt for real at its new home — the recipient's own shields still take their bite, which is
what CR 615.10 asks for — but a redirect shield standing there is passed over rather than
bouncing the hit again (CR 616.1). The public `creature_damage_events_with_riders` /
`player_damage_events` signatures did not move, so no caller outside this module noticed.
The DSL field is a bare `redirect_to_controller` bool rather than a named recipient, because both
pool cards that redirect send the damage to the same player who armed the shield. What the
redirect proves that prevention could not: a 2/2 walks away from a four-point Earthquake, since
the damage was never dealt to it and no state-based action has anything to collect.

### 6b. `static-damage-redirection` — 1 card, M — **done**
Depends on: #6a.
Veteran Bodyguard — "as long as this creature is untapped, all damage that would be dealt to you
by unblocked creatures is dealt to this creature instead." Not a shield: a permanent static with
two live conditions (the bodyguard untapped, the source an *unblocked attacking creature*), so
`Game::player_damage_events` has to scan the damaged player's battlefield rather than a shield
list, and combat has to be able to answer "was this attacker blocked" at damage time.
*Cards:* veteran_bodyguard.

*Landed:* nothing of #6a was reusable, which is why this was its own increment — the bodyguard
arms no shield and spends none, so `StaticEffect::RedirectUnblockedDamageToSelf` is read live off
the permanent every time damage is dealt. Both halves of "as long as this creature is untapped"
are therefore evaluated at damage time rather than cached: a bodyguard tapped after blockers were
declared protects nothing. The redirect sits in `Game::combat_damage_substep`'s unblocked arm
rather than in `Game::damage_player`, because that arm is the only place that still knows the
attacker went unblocked — a blocked attacker's trample leftover reaches the same player through
`damage_defender` and is correctly left alone.

### 6c. `personal-incarnation` — 1 card, M — **done**
Depends on: #6a.
Personal Incarnation is #6a's shield pointed the other way — "{0}: The next 1 damage that would
be dealt to this creature this turn is dealt to its owner instead" — plus two things nothing in
the pool has yet: an activation gated to the permanent's *owner* rather than its controller, and
a dies trigger for "its owner loses half their life, rounded up", which needs an `Amount` that
reads a player's life total (there is `HalfX` but nothing that halves a life total).
*Cards:* personal_incarnation.

*Landed:* three mechanisms, each the smallest thing that carried the card. The shield reuses
#6a's `PreventionShield` whole — `shield_source` on `MiscEffect::PreventNextDamage` sits it on the
permanent that armed it, which is what "dealt to **this creature**" needs and what no `TargetSpec`
can say, since the ability targets nothing. The redirect rider from #6a already sends the bite to
the player who armed the shield, and the owner-only gate below makes that player the owner, so
"is dealt to its owner instead" cost nothing new. `ActivationCost::only_owner_may_activate` is
checked in `Game::activation_ability_and_cost` after CR 602.2's controller check; the dies trigger
is a fieldless `LifeEffect::SourceOwnerLosesHalfTheirLife` rather than an `Amount` variant plus a
player selector, because nothing else in the pool reads a life total or bills the source's owner —
`Amount` gains nothing from a shape one card uses. ponytail: the owner gate only narrows. The
printed line also widens — an owner who has lost control may still activate — but the controller
check runs first, so a stolen Incarnation is activatable by nobody rather than by its thief, which
is the closer wrong answer.

### 7. `untap-step-restrictions` — 9 cards, M — **done**
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
*Landed (7a — basalt_monolith, mana_vault, meekstone):* the nine cards are four unrelated
mechanisms wearing one heading, so only the printed-static family shipped. `StaticEffect::DoesntUntap
{ self_only, filter }` and `Game::doesnt_untap` are the whole of it: the untap step's turn-based
action consults the scanner and nothing else does, which is exactly why Basalt Monolith's
"{3}: Untap this artifact" still frees it — an untap *effect* never reads the static. The scanner
copies `Game::cant_block_filter`'s battlefield-wide idiom rather than scoping to the source's
controller, and that is what makes Meekstone reach across the table for free: `FilterController::Any`
is already the filter default, so "their controllers' untap steps" needs no `all_players` flag of
the sort `Anthem` carries. `self_only` is the printed-on-itself form and ignores the filter outright.
The only genuinely new filter axis was `power_min`, the mirror of the existing `power_max`.
Mana Vault cost two small trigger additions and no new machinery: `Trigger::DrawStep`, the
controller-scoped twin of `each_draw_step` riding the same watch table and the same
`TriggerWatchEvent::for_active_player`, and `Condition::SourceTapped`, spelled as its own variant
because `[abilities.condition]` has no negation axis — only a nested `conditional` step carries
`negate`. Its "you may pay {4}" is #10's landed optional-trigger-with-`[abilities.cost]` shape
unchanged. Two corrections to the sketch above, both from the printed oracle: the Vault's ping is at
the **draw** step, not the upkeep its pay-{4} clause lives in, and Time Vault's untap clause is
tangled with #18's extra turn, not with this static at all.
*Deferred, with the blocker each hit:* **7b Paralyze** — its "enchanted creature's controller may
pay {4}" is billed to the *host's* controller, and neither `ChoiceEffect::PayOrElse` nor the
optional-trigger `PendingChoice::PayCost` has a payer axis; it would also want
`GrantToAttached { doesnt_untap: true }`, since `GrantedAbility` carries no static timing.
**7c Stasis / Smoke / Winter Orb** — a per-player skipped untap step and a per-player cap on how
many permanents may untap, the latter needing the `PendingChoice::ChooseUntapSet` the sketch names.
(Split when it was reached: Stasis shares nothing with the other two and shipped as **7c-i**.)
**7d Instill Energy / Time Vault** — "can attack as though it had haste" plus a once-each-turn
untap, and an extra turn that belongs to #18.

*Landed (7d — instill_energy):* two bools and no new machinery, once the pairing was seen.
`GrantToAttached { may_attack_ignoring_summoning_sickness }` is `may_attack_ignoring_defender` with
a different check waived — `Game::can_attack` skips `is_sick_without_haste` for the host, off the
same attachment scan, and nothing else reads it. That is the whole difference between this and
granting `Keyword::Haste`: the host's own `{T}` abilities stay as locked as they were, which is what
"as though it had haste" for *attacking* means.

The `{0}` untap needed nothing new at all. `GrantedAbility` already carries a full `ActivationCost`,
and `once_each_turn` already lives there and is already gated per (source, ability index) — so the
granted ability's ration came free. Only "during your turn" was missing, and it is one more flag
beside `only_during_opponents_turn`, checked the same way. It is *not* `sorcery_speed`: Instill
Energy's untap is meant to straighten an attacker back up mid-combat, which a sorcery-speed gate
would forbid.

Time Vault stays with #18 — its untap clause is tangled with the extra turn, not with this static.

*Landed (7b — paralyze):* the payer axis the deferral wanted turned out to be already built. Power
Leak — the other 2ed Aura that bills its own upkeep offer to the host's controller — already carries
a `#[serde(skip)] player: Option<PlayerId>` that trigger placement fills from the enchanted
permanent's controller, and `PendingChoice::PayCost` is already the pay-to-*get*-the-effect pause an
optional trigger raises. So `ChoiceEffect::TriggeringPlayerMayPay` is those two joined: Power Leak's
payer with `PayOrElse`'s fixed cost, paying to buy `then` rather than to dodge a penalty. The one
thing an ability-level `[abilities.cost]` can never express is exactly this — Mana Vault's "you may
pay {4}" always bills the *ability's* controller — and the test says so directly: the Aura's
controller submitting the answer is rejected outright, and the {4} comes out of the host
controller's pool.

`GrantToAttached { doesnt_untap }` was the genuinely new half, and it is one bool. The
battlefield-wide `DoesntUntap`'s filter names a *class* of permanents and so can't say "the one this
Aura is on", so the attachment scan is folded into `Game::doesnt_untap` rather than given its own
scanner — the untap step keeps reading exactly one thing, and "doesn't untap" means the same whether
the source is an Aura on the permanent or a Meekstone across the table. Like the battlefield-wide
form it is read *only* by the untap step, which is what lets Paralyze's own pay-{4} untap the host
it is still sitting on.

*Landed (7c-ii — smoke, winter_orb):* the sketch wanted a `PendingChoice::ChooseUntapSet`, and the
pause it describes is the one the untap step already raises. "Which of these come up?" and Rubinia's
"which of these stay down?" are the same question asked from opposite ends, so `DeclineUntap` grew a
`at_most_one: Vec<Vec<ObjectId>>` — each group a set of the offered permanents from which at most one
may untap — and the whole intent, dispatch and answer path came free. Validation is one line and
deliberately a *ceiling, not a quota*: letting two of a group up is rejected, keeping every one of
them tapped is legal. `StaticEffect::UntapAtMostOne { filter }` is the rest of it, unscoped like
`PlayersSkipUntapSteps` because both cards say "players".

The one thing that needed care is *when* each cap is read. The groups are resolved into concrete ids
before anything untaps, off the state the step started in, and a group of one is dropped outright — a
lone candidate untaps as it always would, so no existing test acquired a pause. That ordering is also
exactly the famous Winter Orb ruling: an Orb tapped down in response untaps in the same turn-based
action as your lands, and because its "as long as this artifact is untapped" was read while it was
still tapped, it stops none of them. That ruling is a test, not a comment. The gate itself rides on
the ability's own `Condition::SourceUntapped` rather than on the filter — the first static scanner to
honor `ability_condition_holds`, which is why Smoke, with no such clause, must carry no condition at
all.

ponytail: the groups aren't projected to the client, so the board offers the pause as a free yes/no
and a two-of-a-group answer bounces off the server instead of being blocked in the UI. It needs a
wire field on `PendingChoiceViewDeclineUntap` and a client-side group check; billed to the client
catch-up phase rather than to this increment. **Closed** — see *Client catch-up* at the end.

*Landed (7c-i — stasis):* the sketch bundled Stasis with Smoke and Winter Orb because all three
sound like untap-step restrictions, but they share no machinery: those two cap *how many* permanents
untap and need a subset choice, while Stasis deletes the step. Split, and Stasis is one fieldless
`StaticEffect::PlayersSkipUntapSteps` plus its upkeep tax, which is Conversion's landed
`pay_or_else` verbatim. Unscoped on purpose — "players" is everyone including the Stasis controller,
so there is no `all_players` flag and no filter.

The choice worth recording is *where* the skip is read. Writing it as a table-wide `DoesntUntap`
would have been one line less and wrong twice over: that static holds individual permanents down
*inside* a step that still runs, so phased-out permanents would phase back in (CR 502.1) and a
Pollen Lullaby "skip your next untap step" mark would be burned on a step that never happened.
`Game::players_skip_untap_steps` instead gates the untap step's two real turn-based actions —
phasing in and untapping — and leaves the rest of that arm alone. Losing summoning sickness (CR
302.6), goad expiry, and a planeswalker's loyalty refresh are per-*turn* durations this engine
merely bookkeeps in the same place; skipping them would stop creatures cast under Stasis from ever
attacking, which is not what the card does. Both halves are pinned by tests.

The upkeep tax means Stasis holds its own key, so the freeze test can prove the thaw as well: pay
the {U} once to watch both seats stay tapped, decline it the next turn, and the sacrifice lands in
the upkeep — after that turn's untap step has already gone by, so the board waits one more.

### 8a. `basic-land-type-changing` — 1 card, S — **done**
Depends on: nothing.
*Landed:* CR 305.7 for the price of one accessor, and no new effect at all. The premise was half
stale: `set_attached_types`'s `set_subtypes` *does* already reach a land's types, because
`Game::effective_subtypes` unions `CardKind::Land`'s own `subtypes` into the printed line before
the CR 613.4 layer replaces it. So Evil Presence is `set_subtypes = ["Swamp"]` and nothing else.
What was actually missing is the consequence: mana does not derive from subtypes here, it is read
off `CardKind::Land { produces }` at five sites — the tap intent, three arms of the auto-tap
planner, and `land_producible_colors`. Each repeated the same three-arm credit match, so the fix
is one `Game::land_mana_credit(land, player)` they all call: it compares the land's *effective*
basic land types against its printed ones and, when they differ, derives the credit from the new
types instead of the printed `produces` — which also drops the old mana ability, as CR 305.7 wants.
`commander_identity_credit`'s colors→cheapest-`Mana`-shape collapse was already exactly the
derivation needed, so it came out as `mana_credit_for_colors` and both callers share it. A dual
loses the half that went with the type it no longer has, which is what the Badlands test pins.
*Cards:* evil_presence.

### 8b. `chosen-basic-land-type` — 1 card, M — **done**
Depends on: 8a.
*Landed:* no new picker and no allocation. `PendingChoice::ChooseCreatureType` already carries its
own `options` slice, so `ChoiceEffect::ChooseBasicLandType` raises that same pause narrowed to a
new `BASIC_LAND_TYPES` constant — naming a creature type is `Reject::IllegalChoice` for free,
because the handler already validates the answer against the offered list. The answer lands on the
Aura's own `Permanent::chosen_subtype` exactly as Patchwork Banner's does.
Reading it back was the one real question: `ContinuousEffectKind::SetTypes` wants a
`&'static [&'static str]` and a chosen type is a single `&'static str`, which would mean leaking a
one-element slice on every read of the subtype layer. `BASIC_LAND_TYPES` is that static storage —
`&BASIC_LAND_TYPES[i..=i]` is the slice, so `set_attached_types { set_chosen_land_type = true }`
substitutes it for the printed `set_subtypes` in `attachment_type_continuous_effects` and 8a's
CR 305.7 mana derivation carries the rest unchanged. An unanswered choice contributes no
continuous effect at all, so the land is untouched until its controller names something.
`BASIC_LAND_TYPES` is stored in WUBRG order, which lets `Game::basic_land_types` drop its
string→color match for a zip over the two lists.
ponytail: `effect.static_set_attached_types` still reads "Attached creature is a …" and pulls a
`subtypes` param the engine never sends — wrong for a land Aura and empty for every host. It was
already wrong before either of these cards; folded into the client catch-up pass rather than
fixed here. **Closed** — see *Client catch-up* at the end.
*Cards:* phantasmal_terrain.

### 8c. `all-lands-of-a-type-become-another` — 3 cards, M — **done**
Depends on: 8a.
"All Mountains are Plains" (Conversion) / "All Swamps are 1/1 black Shade creatures that are still
lands" (Kormus Bell) / the same for Forests (Living Lands). 8a's derivation carries the mana half
once the subtype answer is right; what's new is the *scope* — a filter-matched global static rather
than an attached Aura, so it wants a `StaticEffect` read by `effective_subtypes` for every
permanent matching a `PermanentFilter`, not just for the host of an attachment. Kormus Bell and
Living Lands then add the creature type and a base P/T on top, which is #2's static in its
non-CDA fixed form.
*Cards:* conversion, kormus_bell, living_lands.

*Landed:* one `StaticEffect::AllLandsOfTypeBecome` for all three — they are literally the same
sentence with different riders. It is scoped by `land_types` rather than the `PermanentFilter` the
sketch above wanted, and that turned out to be the whole design: a filter has to ask for the
candidate's subtypes, which is the answer this effect changes, so a filter-scoped version recurses
through `effective_subtypes` forever. Matching names against the subtype line *as it is being
built* is both non-recursive and closer to CR 613.4 — a second conversion sees what the first one
left. `Game::land_type_statics` is the battlefield sweep (modelled on `matching_anthems`, and like
it unscoped by controller, because "All Mountains" means everyone's); `land_type_statics_on` is the
per-permanent handle the other three reads use. Four read sites in all — `effective_subtypes` for
the type swap, `effective_types` for "still lands", `pt_layers` for the 1/1, `colors_of` for Kormus
Bell's black — and 8a's `land_mana_credit` carried the mana consequence with no extra work, so a
Badlands under Conversion is a Plains that taps only for `{W}`.

Kormus Bell prints no Shade subtype (the sketch above said it did); the oracle text is "1/1 black
creatures," full stop. Conversion's upkeep half needed nothing new — the existing `pay_or_else`
choice already says "sacrifice this unless you pay {W}{W}."

ponytail: the sweep runs per characteristic read rather than off a cached count, so
`effective_subtypes` skips it entirely unless the line already carries a basic land type — nothing
but a land can be caught. Upgrade path if a board with one of these ever gets slow is a
`Game`-level count maintained as permanents enter and leave.

### 8d. `counter-keyed-type-change-and-an-unbounded-upkeep-series` — 1 card, L — **done** (split)
Depends on: 8c.
Cyclopean Tomb's mire counters key the change to a counter rather than to an Aura or a filter, and
its leaves-the-battlefield clause schedules an unbounded series of upkeep triggers — model that as
a delayed trigger that re-registers itself until no mire counters it placed remain.
*Cards:* cyclopean_tomb.

*Landed:* the counter-keyed half only; the leaves-the-battlefield series is now **#8f**, since it
is a different engine capability (a delayed trigger that re-registers itself) rather than more of
this one. Cyclopean Tomb ships with an `approximates` line saying so — a mired land stays a Swamp
for the rest of the game.

`CounterKind::Mire` is a functional reminder counter on the `Vow` model: the counter *is* the
effect, so `Game::effective_subtypes` reads the slot directly rather than looking for a continuous
effect to read it off. That is not a shortcut — the Tomb can be in a graveyard while the land is
still a Swamp, so there is nothing else left holding it. `ActivationCost::only_during_your_upkeep`
is new (28 struct literals, all mechanical) and enforces both halves of "your upkeep."

Two things the sketch got wrong. The oracle is "{2}, {T}", not a free tap, and the mire counter
goes on a **non-Swamp** land — which `PermanentFilter::exclude_subtypes` already expressed, and
which reads `effective_subtypes`, so the Tomb's own earlier work takes a land off its list for
free. No new filter axis was needed.

ponytail: the mire read applies last, after any global land-type change, instead of by CR 613.4
timestamp — a counter carries no timestamp to sort by. Give `Permanent` a mire timestamp if a board
ever holds both a mired land and a Conversion. Separately, this engine validates an activated
ability's target at resolution rather than at activation (CR 608.2b instead of CR 602.2b, a
documented posture — see `cast.rs`'s `activate_ability`), so pointing the Tomb at a Swamp costs the
{2} and fizzles instead of being refused. Left alone: changing it touches every targeted activated
ability in the pool.

### 8f. `leaves-the-battlefield-unbounded-upkeep-series` — 1 card, L
Depends on: 8d.
Cyclopean Tomb's second clause: "When this artifact is put into a graveyard from the battlefield,
at the beginning of each of your upkeeps for the rest of the game, remove all mire counters from a
land that a mire counter was put onto with this artifact but that a mire counter has not been
removed from with this artifact." Two new pieces. A `repeat` rider on
`MiscEffect::ScheduleAtNextUpkeep` so `fire_delayed_triggers` re-arms instead of draining the entry
(and scopes the upkeep to the delayed trigger's own controller, which today only `Main1` does), and
a counter-removal effect targeting a land with a mire counter. The per-Tomb bookkeeping ("put onto
with **this** artifact") is worth skipping while one Tomb per game is the realistic case — flag it
as an `approximates` rather than tracking placement provenance.
*Cards:* cyclopean_tomb.

### 8e. `land-type-change-until-its-source-leaves` — 1 card, M — **done**
Depends on: 8a.
Gaea's Liege — a *temporary* type change from an activated ability ("{T}: Target land becomes a
Forest until end of turn"), so it needs the runtime `ContinuousEffectKind::SetTypes` path with a
cleanup expiry rather than a static read off an attachment, plus the timing restriction ("only
during your turn, before attackers are declared") and a defining power/toughness counting Forests.
*Cards:* gaeas_liege.

*Landed:* the sketch had the card wrong twice. The oracle duration is "until **this creature leaves
the battlefield**", not until end of turn, and there is no timing restriction at all — `{T}` and
nothing else. Both corrections came from `tooling/scaffold_card_frame.py`, which reads Scryfall.

That real duration is cheaper than the sketched one, because it needs no expiry. The target
permanent stores `subtypes_set_while_source_remains: Option<(&[&str], ObjectId, u64)>` and
`runtime_continuous_effects` emits the `ContinuousEffectKind::SetTypes` entry only while
`self.as_permanent(source).is_some()`. Nothing schedules a cleanup and nothing clears the field —
when the Liege dies the entry simply stops being produced and the land is its printed self again.
A Liege that returns is a new `ObjectId` and correctly does not revive the old change (CR 400.7).

`PumpEffect::TargetBecomesSubtypesWhileSourceRemains` sets `set_subtypes`, replacing the whole
land-type line per CR 305.7 — so a converted Mountain taps for `{G}` and not `{R}`, which the test
asserts as a mana delta.

ponytail: `Event::SubtypesSetWhileSourceRemains` projects onto the existing
`VisibleEvent::AddedSubtypes { object }` rather than minting a wire event, a proto message and a
codegen round-trip. Both mean "re-read this object's subtype line", which is all the client does
with either, and the sustaining source is already an ordinary visible permanent. Give it its own
`VisibleEvent` if the log ever needs to name the source.

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

### 11. `block-restrictions-and-requirements` — 6 cards, L — **done**
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
*Cards:* blaze_of_glory, invisibility, ironclaw_orcs, juggernaut, lure,
two_headed_giant_of_foriys. False Orders split out — see #78.

*Landed:* the sketch's two passes, and one latent gap it didn't name. `declare_blockers` now runs
`block_restrictions_ok` (the per-pair legality loop, menace, and a new per-blocker ceiling) and
then a requirement pass. The ceiling is the gap: before this, a creature could block *every*
attacker, because CR 509.1b's default of one was nowhere in the engine. `Game::max_blocks` is where
it lives now — `1 + can_block_additional`, or no ceiling at all for a creature Blaze of Glory has
touched.

Three restrictions, one filter axis. "Can't be blocked by Walls" (Juggernaut, printed) and "can't
be blocked except by Walls" (Invisibility, granted) are the same clause from opposite sides, and
the second is authored inverted — `exclude_subtypes = ["Wall"]` — the way `cant_be_attacked_by`
already is. That took `exclude_subtypes` and `power_min`, both of which `PermanentFilter` already
had, so no new filter axis and no `de.rs` work. Ironclaw Orcs is the mirror axis rather than the
same one: `cant_block_attackers` is printed on the blocker but reads the *attacker*, where
`cant_block_filter` matches the would-be blocker and reaches the whole battlefield.

The requirement pass is the sketch's "maximum possible number of requirements" with a `ponytail:`
on it. Rather than searching for a set-wide maximum, each unmet requirement is tested on its own:
rebuild the declaration that *would* have met it — that blocker's unrequired blocks dropped — and
if that candidate passes the restriction pass, the submitted one was illegal. That is what stops a
creature dodging Lure by blocking something else. Every card in the pool forces a single creature
or a single attacker, so the per-requirement answer and CR 509.1c's maximum agree; a card forcing
two interacting groups would want the real search.

Blaze of Glory needed the rest of its text more than its effect. `cast_only_before_blockers` joins
the `cast_only_before_attackers` window family (the card pairs it with `cast_only_during_combat` —
alone it would leave the pre-combat main phase open), and `FilterController::DefendingPlayer`
learned a fallback for a source that isn't an attacking creature, since a *spell* has no declared
defender to read. Its two halves are one `MiscEffect` writing two `CombatExtras` lists straight,
no `Event` — the `prevent_all_combat_damage_this_turn` precedent. And `MustAttackEachCombat` grew
`self_only` for Juggernaut, which meant teaching `required_attacks` about it too, or every
roll-the-turn test helper would have submitted an illegal declaration.

The missing ceiling had already been used. `gomazoa_tucks_itself_and_each_creature_it_is_blocking`
had a plain Wall blocking two attackers to exercise "each creature it's blocking" — illegal Magic
that only passed because nothing enforced CR 509.1b. It now casts Blaze of Glory on the Gomazoa to
earn the plural legitimately, which is the same card doing the lifting in the engine and in the
test.

### 78. `remove-from-combat-and-re-block` — 1 card, XL — **done**
Depends on: #11 (done).
Split out of #11. False Orders: "Cast this spell only during the declare blockers step. Remove
target creature defending player controls from combat. Creatures it was blocking that had become
blocked by only that creature this combat become unblocked. You may have it block an attacking
creature of your choice." Three things #11 does not have. Removing a blocker mid-combat is the
easy half — `remove_from_combat` exists — but "had become blocked by only that creature this
combat" is a *history* question the engine doesn't record: CR 509.1h's blocked-ness is sticky, so
un-blocking an attacker means knowing it was never blocked by anyone else, not merely that it
isn't now. And the third sentence is a new `PendingChoice`: the spell's controller — the
*attacking* player — picks which attacker the removed creature re-blocks, which is a declaration
made by the wrong seat, outside the declare-blockers step, after blocks are sealed. *Sketch:* a
per-combat `blocked_by_ever` list beside `CombatState::blocks` answers the history question for a
few bytes; the choice is a `ChooseBlockTarget { player, blocker, candidates }` resolved through
the same `declare_blockers` restriction pass so the re-block is still legal. The `PendingChoice`
is what makes this XL — schema, proto, server, and client all spell every variant.
*Cards:* false_orders.

*Landed:* the history question was a real pre-existing bug, not just False Orders' price. The
engine read blocked-ness off the live block list, so **killing an attacker's only blocker after
blocks were declared let its damage through to the player** — CR 509.1h says it stays blocked.
`CombatState::blocked_ever` (appended on every `Event::BlockerDeclared`, never pruned by ordinary
removal, cleared with the rest of combat) plus `Game::is_blocked` fixes that at the root, and both
the combat-damage branch and `filter.unblocked` (Forcefield) now read it.
`an_attacker_stays_blocked_after_its_only_blocker_dies` is the regression test. A companion leak
went with it: a stored multi-block damage division could still name a blocker that had left
combat, so the division is now filtered to the creatures still blocking (CR 510.1a) — a removed
blocker's share is simply not assigned rather than sliding onto someone else.

False Orders' own three clauses cost less than the sketch feared. The cast window is a real
`cast_only_during_declare_blockers` flag rather than an approximation with the existing pair —
`cast_only_during_combat` + `cast_only_before_blockers` would wrongly allow a *pre*-blockers cast,
and this card's whole job is to rearrange a declaration that already happened. The un-blocking is
`release_solely_blocked` on `remove_from_combat`, which drops just this blocker's pairs from
`blocked_ever`: that is exactly "blocked by only that creature", since an attacker a second
creature also blocked still has that second pair on the list. And the re-aim needed no new wire
family — `PendingChoice::ChooseBlockTarget` reuses `Intent::ChooseCopyTarget` and projects onto
the `ChooseCopyTarget` view behind a `choose_block_target` discriminator, the same trick
`MayPutCounterOnCreature` already plays, so the XL price was one additive proto bool. The
candidate list is filtered through `can_block` judged against the blocker's *own* controller, so
the re-aim can't launder a block that was never legal (a ground creature still can't be pointed
at a flyer). No new `FilterController` was needed either: `DefendingPlayer` already falls back to
`sole_defending_player()` for non-creature sources — Blaze of Glory's precedent.

### 12. `copy-a-permanent` — 3 cards, L — **done**
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

*Landed:* the sketch priced this as if the engine had no as-enters copy. It has had one since
#127: `CardDef::enter_as_copy` + `PendingChoice::ChooseCopyTarget` + `Event::BecameCopy`, built
for Altered Ego and Cursed Mirror. **Clone is `enter_as_copy = {}` and needed no engine change at
all** — and `a_clone_copying_a_clone_takes_what_that_one_copied` passes, which retires the
`edict.rs` ponytail claiming the copy reads the chosen permanent's *printed* values: `def_id_of`
already returns whatever def the permanent is currently wearing, which is exactly CR 707.2's
copiable values. No `CopyableValues` snapshot was needed; the def swap already is one.

Copy Artifact is one new `CopyTargetKind::Artifact` (a `Game::is_artifact_on_battlefield` twin of
the creature/enchantment candidate scanners, reading the CR 613.4 type layer) plus an
`also_enchantment` exception carried through `Event::BecameCopy { also_types }` into the
indefinite `Permanent::added_types` slot `Game::effective_types` already unions in. The event
field is a `TypeSet` while the DSL surface is a bool: the durable log stays general, the card
surface stays honest to the one card in the pool that adds a type.

Vesuvan Doppelganger's two exceptions both turned out to be *def* facts rather than object facts,
which is what kept it small. `copy_with_exceptions` synthesizes the def the shapeshifter wears —
the copied creature's, with the copier's resolved colours written into the explicit `colors` slot
("doesn't copy that creature's color") and its own re-copy ability appended ("and it has this
ability") — and interns it, the same runtime-synthesis path Vraska's `becomes_treasure` already
uses. Baking the exceptions into the def rather than layering them on the permanent is what makes
them *copiable*, per CR 707.2, and what makes the upkeep repeat: the re-grant matches the ability
by its own effect shape, so it re-grants exactly itself and never accumulates the abilities of the
creature it was wearing. The upkeep half is `PumpEffect::BecomesCopyOfTarget` with `optional =
true` on the ability — the same two flags, the same synthesizer, so entering as a copy and
re-copying at upkeep land on identical defs.

### 13. `copy-target-spell` — 1 card, S — **done**
Depends on: nothing.
Fork. `copy_triggering_spell` copies the spell that *triggered* the ability; Fork copies a
targeted spell on the stack, may choose new targets, and the copy is red regardless of the
original. *Sketch:* an `Effect::CopyTargetSpell { new_targets: bool, set_color: Option<Color> }`
reusing the existing stack-copy machinery with the target chosen at cast time
(`instant_or_sorcery_spell_on_stack` already exists as a target spec) and a
`PendingChoice::ChooseTarget` raised at resolution for the copy's targets.
*Cards:* fork.

*Landed:* no new effect. `CopyEffect::TargetSpell` (Twincast) already is the sketch minus the
recolor: it mints the copy, and its resolution arm already runs CR 707.10c's retarget through the
same `choose_spell_targets` a fresh cast uses, so `new_targets` was never a knob worth having — the
sketch assumed a copy that keeps its targets, but the only pool card that wants that is
`copy_triggering_spell` with `may_choose_new_targets = false`. The variant went from a unit to a
struct with one optional `set_color`, and Twincast/Wild Ricochet/Rootha's TOML didn't move.

"Except that the copy is red" is a CR 613.3c layer-5 color SET, and the battlefield twin was
already built: `Permanent::set_color_eot` (Wild Mongrel), which `Game::colors_of` honors ahead of
the derived pips and *replaces* them with. `Spell::set_color` is that field on the stack side —
a spell isn't a permanent, so it needs its own slot, the same split `chosen_color` already has —
plus one arm in `colors_of`. The recolor rides on `Event::SpellCopied` so it lands inside `apply`
with the rest of the copy; the field is unprojected (`..` in the schema event projection), since
what a client renders is the copy object's color, not this event's payload.

Fork is the first card whose fidelity is only observable through another card: nothing about the
copy's *behavior* changes, only its color. The in-set payoffs that read it are Blue/Red Elemental
Blast and Circle of Protection: Red, so the regression tests assert `colors_of` directly on the
minted copy — red, not green — and a matching Twincast test pins the copy's color to the original
when no effect recolors it.

### 14. `banding` — 4 cards, L — **done**
Depends on: #11 (done).
Banding (CR 702.22). Attacking as a band, being blocked as a group, and the defining ugly part:
when a banding creature blocks or is blocked, *its controller* — not the attacking creature's
controller — assigns that creature's combat damage. *Sketch:* an attack-declaration grouping
(bands are declared with the attackers and can't change), block legality treating a band as one
object, and a damage-assignment ownership flip in the existing `AssignCombatDamage` pending
choice, which already knows how to ask a player to divide damage among blockers — it needs to ask
a *different* player. Helm of Chatzuk grants it until end of turn, so it must be a real
`Keyword`, not a card flag.
*Cards:* benalish_hero, helm_of_chatzuk, mesa_pegasus, timber_wolves. Attacking in a band split
out — see #79.

*Landed:* the sketch's three pieces are not one increment. The ownership flip is nine lines; the
attack-declaration grouping is a wire change. Only the flip landed, and all four cards carry an
`approximates` saying so.

`Keyword::Banding` is a real keyword because Helm of Chatzuk lends it, and lending goes through
`pump_until_end_of_turn`'s existing `keywords` bag — so the Helm needed no new DSL at all, just an
activated ability with a 0/0 pump. That is also the thing worth pinning in the cards crate: a
printed banding and a lent one have to land in the same bag or `has_keyword` would see two
different things.

`Game::damage_assigner` is the whole engine change. Both places that raise
`PendingChoice::AssignCombatDamage` used to hardcode `self.active_player`; they now ask, and the
answer is the first banding blocker's controller (CR 702.22e) or the attacker's controller
(CR 510.1a) otherwise. All of an attacker's blockers belong to the one defending player, so the
first banding blocker names the answer — no tie to break. Nothing else moved: the choice already
carries a `player`, the wire already routes an answer by it, and the client already renders it.

"Pegasus" was missing from `CREATURE_TYPES`, which the pool-wide chooseable-type test catches.

### 79. `attacking-bands` — 0 cards, XL
Depends on: #14 (done).
Split out of #14. The half of CR 702.22 that isn't damage assignment: "any creatures with banding,
and up to one without, can attack in a band", bands are blocked as a group (CR 702.22c — blocking
one member blocks them all), and a creature attacking in a band that is blocked has *its*
controller divide the blocker's damage among the creatures that blocker is blocking (the other
direction of the flip #14 landed). *Sketch:* `Intent::DeclareAttackers` carries a flat
`Vec<ObjectId>`, so bands have to travel through the intent, `schema`'s projection, the proto, and
the client's attack UI before the engine can even hear about them; then `Game::declare_blockers`
needs a band to count as one object for legality, and a *blocker*-side `AssignCombatDamage` has to
exist at all — today a blocker's damage goes straight to the one attacker it blocks, with no
division step, so the second flip has nothing to flip. No card is blocked on this beyond the
`approximates` on #14's four; it is listed so the ceiling is named rather than implied.

### 15. `color-changing-effects` — 5 cards, M — **done**
Depends on: nothing.
The lace cycle. `set_own_color_until_end_of_turn` exists but is self-scoped and turn-scoped;
these target a spell *or* permanent and last indefinitely (layer 5, no duration). *Sketch:* an
`Effect::SetColor { target, colors, replace: true }` written into a permanent-level or
stack-object-level color override that `characteristics.rs` applies in layer 5, with no cleanup
hook. The target spec needs a "spell or permanent" variant — the pool has
`single_target_spell_on_stack` and permanent targets, but nothing that accepts either.
*Cards:* chaoslace, deathlace, lifelace, purelace, thoughtlace.

*Landed:* the sketch called for a new permanent-level and stack-object-level override. Both already
existed — `Permanent::set_color_eot` (Wild Mongrel) and `Spell::set_color` (Fork's "except that the
copy is red"), both already read by `colors_of`, both already replacing rather than unioning. The
only thing missing was a duration that isn't end of turn, so `set_color_eot` became
`set_color: Option<(Color, bool)>`: one slot carrying its own duration, cleared at cleanup only when
the flag says so. One slot, not two, is also the CR 613.7 answer — each set replaces, so a later one
clobbering an earlier one is exactly right.

`Event::ColorSetUntilEndOfTurn` became `Event::ColorSet { object, color, until_end_of_turn }`, and
its apply dispatches on the object: a permanent gets the pair, a spell still on the stack gets
`Spell::set_color`. `PumpEffect::TargetBecomesColor` passes `false`; Wild Mongrel's answered choice
passes `true`.

`TargetSpec::SpellOrPermanent` is new and unfiltered — the one spec spanning the stack and the
battlefield. The five cards that print the wording restrict nothing, so neither does it.

The bug this turned up is the interesting part. `SpellFilter::Color` was matched against
`color_identity(&def)` — the printed pips — so Red Elemental Blast still saw a Thoughtlaced Bolt as
red and refused to counter it, which is the entire point of the cycle. Fixed at the call site that
has the stack object's id: `legal_targets_for`'s `SpellOnStack` arm now reads `colors_of(id)`, the
same carve-out `SpellFilter::ManaValueEqualsX` already takes there. `spell_matches_filter` still
reads the pips for its cast-trigger callers, which is right for them — a cast trigger fires before a
lace could respond.

ponytail: both durations project onto the one existing `VisibleEvent::ColorSetUntilEndOfTurn`. The
client only re-reads the object's colours from it; nothing renders the duration. Split it if the log
ever spells the duration out.

### 16. `text-changing-effects` — 2 cards, L — **done**
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

*Landed:* the substitution rides the object rather than a side table — `Permanent::text_swap` /
`Spell::text_swap`, one `Option<TextSwap>` slot each, which is what gives "this effect lasts
indefinitely" its right duration for nothing: a card that changes zones is a new object (CR 400.7)
and a new object was never hacked. `TextSwap` is read back at CR 613.4 layer 3 in four places —
`effective_subtypes` (and so `land_mana_credit`, which is why a Hacked Swamp taps for {W}),
`compute_effective_keywords_uncached` (landwalk's land type, protection's color),
`functional_abilities` and `ability_at` (the printed ability an activation reads, which is how a
Sleighted Circle of Protection arms against its new color). Both words are picked through the
existing `PendingChoice::ChooseCreatureType` picker, raised twice with a `TextSwapPick` tail
carrying the target, the vocabulary and the first word — so the two-question prompt cost no new
wire message and no client work beyond the `text_changed` log line.

*Ceilings.* `TextSwap::ability` rewrites `MiscEffect::PreventNextDamage`'s `from_color` and
recurses through `Effect::Sequence`; every other enumerated color or land type buried in an
`Effect` passes through unchanged. `Effect` is a wide tree whose leaves are mostly `&'static`
slices, so rewriting it wholesale would mean leaking a fresh slice on every uncached read of a
swapped object's abilities — `StaticEffect::GrantToAttached { keywords }` (Black Ward's granted
protection) is the shape that most wants it. Grow the match from a card that asks. One swap per
object, too: a second text change replaces the first rather than composing, because `Permanent`
has to stay `Copy`. `Game::lands_with_subtype_controlled` — the landwalk block check — was reading
*printed* land subtypes, which would have left a Hacked land failing to turn a swampwalk on; it
now reads `effective_subtypes`, which fixes the same pre-existing gap for Evil Presence and
Phantasmal Terrain (#3, #8a, #8b) as well.

### 17. `random-discard` — 2 cards, S — **done**
Depends on: nothing.
"Discards a card at random" / "discards X cards at random." No `mode = "random"` or `at_random`
existed anywhere in the pool. The engine's determinism rule means this must draw from the injected
RNG the engine already threads for shuffling, never a wall-clock or thread-local source.
*Landed:* `random = true` on the existing discard effect. It is the first discard that raises **no
pause** — nobody chooses, so instead of `run_hand_pause` it routes to `run_misc_choreo` and picks
from the discarder's hand with `Game::with_op_rng`, the same derive-per-op RNG the shuffle draws
from (the `ReanimateRandomFromTargetOpponentGraveyard` precedent). The pitch still goes through the
shared `discard_ids`, so discard watchers see a random discard exactly as they see a chosen one.
Two smaller widenings rode along: `count` grew from a bare `u32` to an `Amount` for Mind Twist's
`count = "x"` (`Amount` deserializes from a bare integer, so every existing discard TOML is
unchanged), and a `damaged_player` flag names the discarder as "**that player**" — the player the
source just damaged — for Hypnotic Specter, filled at trigger placement out of what is now
`TriggerContext::damage_recipient` (renamed from `combat_damage_recipient`, since
`deals_damage_to_opponent` fires on noncombat damage too). That flag is opt-in precisely because
Looter il-Kor shares the same trigger and *its* discard is still the controller's.
*Cards:* hypnotic_specter, mind_twist.

### 18. `extra-turns` — 2 cards, M — **done**
Depends on: #7 (Time Vault only).
"Take an extra turn after this one" (CR 505.6a). The turn structure advances through a fixed
player rotation with no concept of an inserted turn. *Sketch:* a `Vec<PlayerId>` extra-turn queue
on `Game`, consumed by the turn-advance path before consulting the normal rotation, so multiple
extra turns stack in the right order (last created, first taken). Time Vault additionally needs
its skip-your-turn replacement, which is the other half of #7's untap-step work.
*Cards:* time_vault, time_walk.
*Landed (18a — time_walk):* the sketch's `Vec<PlayerId>` queue is the whole mechanism, and it is
smaller than billed. `Game::advance_step` already computed the next active player in one place, so
extra turns are a two-line `match self.extra_turns.pop()` there: a queued turn belonging to a
player who has since lost is skipped rather than handed out, and an empty queue falls through to
the ordinary rotation. Popping the *last* entry gives CR 500.7's most-recent-first order for free.
Only one new event, `ExtraTurnQueued` — the pop deliberately isn't event-sourced, because the
`StepBegan` that opens the extra turn already names its `active_player`, so a replay from events
lands on the same turn order either way (the same bookkeeping shape as
`skip_starting_players_first_draw`). The effect side is a bare `MiscEffect::TakeExtraTurn` with no
target and no fields, which makes Time Walk a card with nothing else on it.
*Landed (18b — time_vault):* the deferral said `advance_step` had nowhere to raise a pause. It
does: between the `StepBegan` that names the new turn's active player and the
`perform_turn_based_actions` call that would run the untap step. Standing there is what makes the
whole card work — the offer is made after the engine knows whose turn is beginning and before a
single turn-based action has run, so a skipped turn untaps nothing, draws nothing and never opens a
priority window. `StaticEffect::MaySkipTurnWhileTapped` is fieldless (both "your turn" and "this
artifact" resolve off the source) and `MayYesNoResume::SkipTurnWhileSourceTapped` is the whole
answer path: "yes" emits the `Untapped` and sets the step marker back to `Cleanup`, so the loop's
very next pass reads `leaving_cleanup` and hands the turn on — no `EndTheTurn` machinery, no second
skip flag, and an owed extra turn is still popped ahead of the rotation because that pass is the
ordinary one. "No" runs the untap step the pause stood in front of and carries on, which is what
`a_declined_time_vault_takes_the_turn_and_stays_tapped` pins by watching a Forest come up while the
vault does not.

The two clauses stay two abilities, and that is the card: `doesnt_untap` with `self_only` is the
only thing keeping it down, and the skip is the only thing in the pool that undoes a `doesnt_untap`
on its own card. `time_vault_taps_for_an_extra_turn_and_buys_the_untap_back_with_a_later_one` walks
the actual loop — tap for the extra turn, decline the offer that stands in front of the turn you
just bought, then spend a later turn to get the vault back up.

ponytail: the skipped turn's `StepBegan` is emitted before the offer is answered, so an event replay
sees an untap step for a turn CR 614 says never happened. Nothing observes it — the per-turn tallies
it resets are reset again by the turn that does happen, and no trigger or priority window lives in
an untap step — so raising the pause *before* `StepBegan` and re-pushing the step preamble in the
"no" handler would buy nothing but a longer diff.

### 19. `land-tap-triggers-and-bonuses` — 5 cards, M — **done**
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

*Landed:* the sketch's two halves held, and the split turned out to be the load-bearing part of
the card text rather than an implementation convenience. A mana ability never uses the stack (CR
605.3), so Mana Flare and Gauntlet of Might are *not* triggered abilities at all — they are the
existing inline `tapped_for_mana_bonus` watch read at `Game::land_tapped_for_mana`, and all they
needed was a scope that isn't "you" or "the land I enchant". `LandTapScope::AnyLand(PermanentFilter)`
covers both: Mana Flare passes an unrestricted land filter, Gauntlet of Might passes
`subtypes = ["Mountain"]`, and because the filter matches against the effective type line a dual
Mountain counts (CR 305.6). The bonus keeps landing in the *tapping* player's pool, which is what
both cards print. Gauntlet's anthem half cost nothing — it is Crusade's shape with
`colors = ["red"]` and `all_players = true`.

Manabarbs, Lifetap, and Psychic Venom really are triggered abilities and really do use the stack,
so they needed `Trigger`s the pool had never asked for: no tap-related variant existed.
`PermanentBecomesTapped { filter, for_mana }` and the fieldless
`EnchantedPermanentBecomesTapped` (the tap twin of `EnchantedCreatureAttacks`, fieldless for the
same reason — no `PermanentFilter` can say "the permanent I am attached to"). Neither is
controller-scoped: both cards watch the whole table.

The `for_mana` bool is the interesting bit. "Becomes tapped" and "taps a land for mana" are
different events, and only one choke in the engine knows the difference. So the two fire from two
places — `for_mana: false` off `Event::Tapped` in `enqueue_triggers` (every tap there is: an
attack, an Icy Manipulator, a mana ability), `for_mana: true` off `land_tapped_for_mana`, which is
already the choke the inline bonus watches read and the only code that knows the tap produced mana
(CR 106.11). A land tapped for mana runs *both* chokes, so the watches partition on the flag and
the attachment pass runs only on the every-tap side; the Manabarbs test asserts 19 life, not 18,
to keep that honest.

For "that player" / "that land's controller" nothing new was needed either.
`DamageEffect::ToTriggeringPlayer` already exists, and `TriggerContext` already had a slot for the
controller of the permanent a trigger is about — it was just named `dying_permanent_controller`
for its one caller (Dingus Egg). Renamed to `triggering_permanent_controller` and reused. That was
a 14-line diff; adding a 34th context field would have meant touching all 33 explicit
`TriggerContext { … }` literals in `triggers.rs`.

ponytail: Gauntlet of Might's log line renders through the shared
`effect.static_tapped_for_mana_bonus` template, which now takes the scope's filter as a param and
reads "Whenever Mountains are tapped for mana, their controller adds an additional one red mana" —
plural where the card says "a Mountain". Give the template a singular form if a log ever needs to
quote the card.

### 20. `pay-or-consequence-upkeep` — 1 card, S — **done**
Depends on: nothing.
"At the beginning of your upkeep, this creature deals 8 damage to you unless you pay {G}{G}{G}{G}."
`sacrifice_self_unless_pay` and `PayEchoOrSacrifice` cover pay-or-sacrifice; there was no
pay-or-damage.

*Landed:* `ChoiceEffect::SacrificeSelfUnlessPay { cost }` became
`PayOrElse { cost, otherwise: &'static [Effect] }` — the penalty is now the card's own effect list
rather than a hardcoded sacrifice, carried through `ChoiceRequest`/`PendingChoice::PayOrElse` into
`Game::pay_sacrifice_unless`, whose decline branch runs it in order with the source as source and
the controller as controller. `otherwise` is required in the DSL, so the two existing consumers
(Phantasmal Forces, Rupture Spire) now spell their sacrifice out; the effect's message renders
`otherwise` as children ("Pay {G}{G}{G}{G} or: …"), which is why the i18n key moved to
`effect.choice_pay_or_else`. The wire's `PendingChoiceViewSacrificeUnlessPay` keeps its old name
and field number — `docs/WIRE_COMPAT.md` is expand-only, and the client never read the penalty.

The increment's other two cards each want a *second* mechanism and moved out: Demonic Hordes'
"tap this creature and sacrifice a land of an opponent's choice" to #41, Lord of the Pit's
"sacrifice a creature other than this creature. If you can't, …" to #72 — that one is not a
pay-or-else at all, since nothing is optional.
*Cards:* force_of_nature.

### 21. `blocks-or-blocked-by-trigger` — 2 cards, M — **done**
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

*Landed:* gap (b) was already closed and nobody had noticed. `PermanentFilter::exclude_subtypes`
has been in the engine since Keldon Warlord's "non-Wall creatures you control", so "non-Wall" is a
plain `filter = { types = "creature", exclude_subtypes = ["Wall"] }` sibling on the trigger tag —
the same `filter` key `you_sacrifice` reads. Gap (a) was half-closed too:
`Trigger::BlocksOrBecomesBlocked` already exists for Goblin Cadets, but it's deduped *once per
creature* and records no partner, which is precisely wrong for a payoff that names the other
creature. So the new variant is `BlocksOrBecomesBlockedBy { filter }`, walked per (blocker,
attacker) pair with no dedup — two creatures blocking one Basilisk both turn to stone, and each
fire carries its own partner in `TriggerContext::blocking_partner`.

"Destroy that creature at end of combat" cost one field rather than a new scheduled effect:
`DestroyEffect::ThatCreature` grew the same `at: Option<Step>` knob `DestroyEffect::Target` already
carries, so on resolution it re-schedules its own already-filled payload as a CR 603.7 delayed
ability. `Step::EndCombat` was already a real `fire_at` (decayed sacrifices there), and the partner
reaches the payload through the existing funnel — Stinkweed Imp's `fill_damaged_creature`, renamed
`fill_that_creature` and given a second caller. The `at` matters for more than flavor: a blocker
destroyed as the trigger resolved would never deal its combat damage.

### 22. `damage-taken-history` — 2 cards, M — **done**
Depends on: nothing.
"You gain life equal to the damage dealt to you this turn" / "whenever you're dealt damage, put
that many vitality counters." `triggering_damage_dealt` exists as an amount but only inside the
triggering ability; nothing accumulates per-player damage across a turn, and there is no
"whenever you're dealt damage" trigger at all. *Sketch:* a turn-scoped `damage_taken_this_turn`
per player on `Game`, incremented in the single `deal_damage` path and cleared at cleanup, exposed
as an `Amount`; plus a `Timing::PlayerDealtDamage` firing with the amount. Both are cheap once
`deal_damage` is already being touched for #4 — sequence this after that increment.
*Cards:* living_artifact, simulacrum. Lich needs the life-gain replacement first, so it ships with
#47 — this increment only hands it the damage-taken trigger it also wants.

*Landed:* no new damage path. `Event::CombatDamageDealtToPlayer` and `Event::DamageDealtToPlayer`
are the only two arms that reach a player and both were pure markers, so each grew one line adding
to a turn-scoped `Player::damage_taken_this_turn`, cleared in the same Untap loop as
`life_gained_this_turn` and `spells_cast_this_turn`. `Amount::DamageTakenThisTurn` reads it.
Damage, not life loss — a drain or a paid life cost only ever emits `Event::LifeChanged`, so
neither of those two arms sees it, which is what CR 120.1 says.

`Trigger::YouAreDealtDamage` needed no new context field: `TriggerContext::triggering_damage_dealt`
already existed for Armadillo Cloak, and `TriggerWatchScope::ControlledPlayer` already means
"permanents controlled by the event's player", so the victim-scoped watch is the existing scope
plus a new `TriggerWatchContextKind::DamageAmount`. The watch sits on *both* damage-to-player
tables (combat and noncombat alike is damage dealt to you), and `TriggerWatchEvent`'s
`combat_damage` field was renamed `damage_amount` since it now carries both.

Living Artifact's "If you do, you gain 1 life" is an intervening-if, not a conditional step: with
no counter on it the upkeep trigger never reaches the stack (CR 603.4), and once a counter is there
the removal can't fail. `Condition::SourceHasNoCountersOfKind`'s doc comment already prescribed
`SourceHasCountersOfKind { kind, at_least }` as its growth path, so that is exactly what it got.
`CountersEffect::RemoveCounterFromSelf` was +1/+1-only and fieldless; it took a
`kind: Option<CounterKind>` (`None` = +1/+1, so Ingenious Prodigy's TOML is untouched), the
`Some(kind)` arm going down the `Event::KindCountersPlaced` path a named kind lives on.

`CounterKind::Vitality` is inert bookkeeping like `Corpse` — banked damage that changes nothing
about the permanent by itself. Simulacrum reads the one tally twice, as the life it gains and as
the damage it hands its own creature, which is the whole reason the tally is an `Amount` rather
than a trigger payload.

### 23. `mana-emptying` — 3 cards, M — **done**
Depends on: nothing.
"That player loses all unspent mana" — a 1993 artefact of mana burn that survives in the Oracle
text. The engine's mana pool empties at step boundaries; nothing empties it on demand, and Power
Sink's "they tap all lands with mana abilities they control" is a filtered mass tap the existing
`tap_all` doesn't filter by ability. *Sketch:* an `Effect::EmptyManaPool { who }` plus a filter on
`tap_all`. Power Sink is otherwise the standard counter-unless-pays shape.
*Cards:* drain_power, mana_short, power_sink.

*Landed:* the sketch's `Effect::EmptyManaPool { who }` was one rung too high. `Event::ManaEmptied`
already existed with exactly the right clearing semantics for the step/phase boundary, so "loses all
unspent mana" is that same event with one new field — `to: Option<PlayerId>`, Drain Power's "you add
the mana lost this way" — and a thin `ManaEffect::LoseAllUnspent { to_you }` mint arm. The transfer
is the existing `ManaPool::merge`, which carries credit kinds over whole, so a dual land's
either-credit arrives as flexible as it left. `end_of_turn: true` on the card-driven emptying: CR
500.4's persistent-mana exception is about *boundaries*, and a card that says "all" means all.

Mana Short and Drain Power are two-step `Sequence`s sharing one `TargetSpec::Player` — the first
step names the seat, the trailing `LoseAllUnspent` sits in `Effect::target()`'s `TargetSpec::None`
list and reads the enclosing sequence's target back. First pool card to lean on that for a *player*
target rather than a permanent.

Power Sink's clause cannot be a following step, because a following step runs either way and this
one only fires on the *decline*. It became `strips_mana_on_decline: bool` threaded
`CounterTargetSpell` → `ChoiceRequest` → `PendingChoice` → `pay_or_counter`, where the decline
branch does the tap and the drain inline. Nothing new on the wire: the pay/decline prompt is
identical either way and the taps and emptied pool reach the client as ordinary events on the answer.

"Lands with mana abilities they control" needed no new predicate either — `Game::taps_for_mana` is
the one the client's tap-for-mana affordance already reads, so `PermanentFilter::has_mana_ability`
is a two-line `permanent_matches` check that spares a Fabled Passage and judges a type-changed land
by what it produces *now*. Mana Short's sweep is `ControlEffect::TapAllTargetPlayerControls`, the
other-seat twin of `tap_all`; both new sweeps skip already-tapped permanents, since `Event::Tapped`
is the untapped→tapped change (CR 701.21a) and a redundant one would fire increment 19's
becomes-tapped watches twice.

Drain Power taps on the target's behalf through `Game::tap_for_mana` itself rather than minting
credits, so every land-tap watch on the board fires exactly as it would for their own click (CR
605.3 — a mana ability uses no stack). That closes increment 49's tractable half; only Word of
Command is left there.

ponytail: Drain Power takes each land's default credit rather than offering a per-land pick, so a
land with two competing mana abilities gets the first one. Every land in the 2ed pool has one, and a
dual's either-credit stays undecided in the pool anyway. Raise a per-land pending choice if a card
ever makes the pick matter.

### 24. `attack-restrictions-by-defender` — 3 cards, S — **done**
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
*Landed:* the restriction wanted a filter, not a condition. `StaticEffect::CantAttackUnlessDefenderControls { filter }` is the exact mirror of the `CantBeAttackedBy` static sitting one screen away in `declare_attackers` — that one rides the defender, this one rides the attacker — so it reuses `permanent_matches` over the defending player's battlefield and renders through the filter machinery that already existed. Trying it as a `Condition` first was the wrong rung: conditions have no message rendering, so the card text would have projected as a blank. Animate Wall needed no new static at all, only a `may_attack_ignoring_defender` bool on the existing `grant_to_attached` bag beside `cant_attack` — "as though it didn't have defender" waives `can_attack`'s one check without touching the keyword, which is what the printed "as though" says. The restriction is also folded into `can_attack`'s legal-defender clause, because leaving it out lets goad demand an attack the card forbids and no legal declaration exists (CR 509.1a) — a soft-lock, not a rules nicety. `Condition::ControlsNoLandsWithSubtype` is new because `controls_lands_with_subtype` only counts upward and `count = 0` holds vacuously. What did *not* land: "When you control no Islands, sacrifice this creature" is a state trigger (CR 603.8) and the engine has no state-trigger shape — Pyrohemia's `no_creatures_on_battlefield`, which the sketch called the same idea, is a printed end-step trigger, not a state trigger. Both ships carry an `approximates` marker and the end-step approximation; see #77.
*Cards:* animate_wall, pirate_ship, sea_serpent.

### 25. `amount-arithmetic` — 2 cards, S → M — **done**
Depends on: #1 (Aspect of Wolf only).
"X damage … where X is the number of cards in their hand minus 4" and "half the number of Forests
you control, rounded down / rounded up." `Amount` has `half_x` and `half_x_rounded_down` for the
X-cost case only; there is no way to halve or offset any other amount. *Sketch:* wrap rather than
multiply variants — `Amount::Offset { inner: Box<Amount>, delta: i32, floor_zero: bool }` and
`Amount::Half { inner: Box<Amount>, round_up: bool }`, resolved recursively in `amount.rs`. Black
Vise clamps at zero (a 3-card hand deals no damage, not negative damage).
*Cards:* aspect_of_wolf, black_vise.
*Landed:* split in two. Aspect of Wolf's half went out with increment #75 as `Amount::Half`, and
`Amount::Offset { of, delta }` here is its twin — same leaked-`&'static` wrap, same early return
ahead of the deserializer's exactly-one-of table. The clamp is unconditional rather than the
sketch's `floor_zero` flag: every offset the pool prints lands in a damage or count slot where a
negative reads as none anyway (CR 120.8), so the flag would have had one value forever.
The arithmetic was the small half. Black Vise's real cost is the clause the sketch never
mentioned — "as this artifact enters, choose an opponent," which the pool had no way to express.
No new picker was needed: `PendingChoice::ChooseSplittingOpponent` is already the shared
"an opponent ..." pause (clash rides it too, splitting nothing), so this is one more
`SplittingContinuation` — `RememberAsChosenOpponent`, whose whole body writes
`Permanent::chosen_opponent` and raises nothing. That keeps the wire untouched: no new
`PendingChoiceView`, no proto field, no client case. Writing the answer straight onto the
permanent instead of through an `Event` follows Archangel of Strife's `Player::war_choices`; only
`chosen_color` needs an event, because it feeds the characteristics cache and this doesn't.
The trigger half is increment 61's shape exactly — `each_upkeep` plus an intervening-if
(`Condition::ChosenPlayersUpkeep`) rather than a trigger variant of its own — and the payoff
addressing is increment 60's `damage/to_triggering_player`, which resolves its amount against the
*recipient*, so `"cards_in_your_hand"` under it counts the taxed player's hand with no `who` axis
added. A two-player table never sees the choice at all: the picker collapses on a single legal
opponent.

### 26a. `forced-attack-with-delayed-punishment` — 1 card, M — **done**
Depends on: #21 (subtype exclusion).
"That creature attacks this turn if able. Destroy it at the beginning of the next end step if it
didn't attack." `must_attack_target` (Basandra) and `must_attack_each_combat` exist; the
punishment half does not, and neither does the "has controlled continuously since the beginning of
the turn" qualifier that both cards use to exempt freshly-arrived creatures.
*Cards:* nettling_imp.

*Landed:* three of the four pieces were already in the engine wearing other names. The proposed new
`Permanent` flag was unnecessary — `summoning_sick` **is** "controlled continuously since the
beginning of the turn": set on entry and on a control-granting Aura, cleared at the controller's
own untap step. It only says what it means about a permanent whose controller has already untapped
this turn, but every card printing this clause is restricted to the active player's turn, so that
is always whose permanents it reads. A `controlled_since_turn_start` filter axis over that flag was
the whole qualifier. `CardDef::cast_only_before_attackers` already existed for Master Warcraft, and
its gate (`step > DeclareAttackers || combat.attackers_declared` — the declaration shuts the window
mid-step, not the step boundary) copies verbatim onto the activated-ability side as
`ActivationCost::only_before_attackers`, alongside a plain `only_during_opponents_turn`.

The punishment half was the *negation* of Berserk's existing rider, so `only_if_it_attacked: bool`
became a three-state `AttackRider` — two mutually-exclusive bools is the classic 3am hazard.
`MustAttackTarget` grew an authorable `target: TargetSpec` (defaulting to Basandra's unqualified
"target creature") the same way `ManaEffect::Add` carries its own: the spec is the card's wording,
not the effect's fixed shape. That let the card author as Berserk's exact two-step sequence, where
the destroy shares the target the must-attack chose.

The tests had to bend to the engine's posture, not the other way around: an illegal chosen target
fizzles at resolution (CR 608.2b) rather than rejecting the activation, so "this creature is out of
reach" is asserted at the end step — a creature the ability never latched onto is still standing
even though it sat the combat out. And the must-attack requirement makes "didn't attack" unreachable
for an *able* creature, so the punishment half is only exercised by tapping the victim down with an
Icy Manipulator before attackers are declared.

### 26b. `mass-forced-attack-with-delayed-punishment` — 1 card, M — **done**
Depends on: #26a.
Siren's Call is the sweeper twin of Nettling Imp and reuses none of #26a's plumbing directly.
"Creatures the active player controls attack this turn if able" is a mass `must_attack`, and
"destroy all non-Wall creatures that player controls that didn't attack this turn" needs
`DestroyEffect::All` to grow the `at:` knob `Target`/`ThatCreature` already carry plus a filter axis
for "didn't attack this turn" (the rider is per-creature here, not per-effect, because the sweep
re-reads the battlefield when it fires). Its timing restriction is a *cast* restriction, so it also
needs `CardDef::cast_only_during_opponents_turn` — the twin of `cast_only_before_attackers`, and
about twenty exhaustive `CardDef` struct literals to touch.
*Cards:* sirens_call.
*Landed:* the sketch held, with one thing it got wrong and one it didn't see. Wrong: the sweep's
"that player" is not `FilterController::Opponent`, which at a four-player table reaches three
boards — it needed a new `FilterController::ActivePlayer`, absolute rather than relative to the
effect's controller, re-read each time the filter runs so the delayed sweep still names the same
player at the end step. Unseen: `DestroyEffect::All`'s new `at` is *not* the same knob as
`Target`'s. `Target`'s `at` bakes the chosen id into a `that_creature` payload when it schedules;
`All`'s re-runs the whole filter when the delayed ability fires, which is the only way
`did_not_attack_this_turn` can read an attack that gets declared after the scheduling. The two
halves also can't share a filter: the call goes out to every creature the active player controls,
Walls included, while the sweep spares Walls and anything that changed hands mid-turn — one filter
would either exempt a Wall from attacking or bury a creature that was never the active player's to
send. `cast_only_during_opponents_turn` came in as sketched (~155 exhaustive `CardDef` literals,
not twenty). Same Icy Manipulator trick as #26a to make a creature unable to attack, since the
requirement leaves nothing for the sweep to collect otherwise.

### 27. `widen-creature-types` — 0 cards, S — **done**
Depends on: nothing.
Re-audit fallout, not a card blocker. `CREATURE_TYPES` in `types/stack.rs` is the candidate list
for "choose a creature type" prompts and its own ponytail says to widen it when a card needs a
type not printed on anything in the pool.
*Landed:* the audit was wider than the sketch — 24 types were missing, not the 21 this set prints,
because the same drift had accumulated from earlier grinds (Centaur, Crab, Drone, Hag, Kithkin,
Leviathan, Phelddagrif, Praetor, Ranger, Rebel, Sphinx were already printed and already
unchoosable). The list is now 112 entries and, more to the point, no longer maintained by
vigilance: `CREATURE_TYPES` is `pub` and a `cards` test walks the registry, so a creature authored
with a type the list lacks fails the suite. The remaining 2ed types the sketch named — Barbarian,
Basilisk, Cockatrice, Juggernaut, Nightmare, Pegasus, Pirate, Serpent — arrive with their cards,
and that test is what will demand them.
*Cards:* none directly — every "choose a creature type" card in the pool gains the options.

### 28. `counter-kinds` — 5 cards, S — **done**
Depends on: nothing.
Falsifies the fixed counter-slot array in `types/effect/shared.rs`. 2ed needs four kinds it
doesn't have: +1/+0 (Clockwork Beast), corpse (Scavenging Ghoul), mire (Cyclopean Tomb), vitality
(Living Artifact). Three are inert bookkeeping counters; +1/+0 is a real P/T counter that
`characteristics.rs` must apply in layer 7d beside +1/+1.

`CounterKind::Corpse` landed (`COUNT` 10 → 11, one `ALL` entry, one `message.rs` name) and
Scavenging Ghoul ships. `CounterKind::Vitality` landed the same way with #22, and Living Artifact
ships. The only other engine gap it needed was game-wide death counting:
`Amount::CreaturesDiedThisTurn` is per-controller, and the Ghoul's "for each creature that died
this turn" names no controller, so `Amount::CreaturesDiedThisTurnAnyController` sums every
player's tally (they all clear at the same Untap step, so the sum is exact — no new field).
Everything else the card needs already existed: `"each_end_step"`, `put_counters` with a named
`kind`, and `remove_counters` / `remove_counters_kind` as an activation cost paying
`regenerate_shield { target = "this" }`.

`CounterKind::Mire` landed with #8 and Cyclopean Tomb ships approximated — the counter *is* the
type change (`Game::effective_subtypes` reads it straight off the land, so it outlives the Tomb),
and the rest-of-game unwind when the Tomb leaves is flagged there rather than built here.
`CounterKind::Age` and Rock Hydra landed with #4, which is done; the "blocked on #4" note above was
stale by the time this increment came up.

*Landed:* Clockwork Beast, the last card and the only one that needed a *real* P/T counter.
`CounterKind::PlusOnePlusZero` is a fourteenth slot in the fixed array, but unlike corpse/mire/
vitality it is not bookkeeping: `characteristics.rs` reads it in layer 7d as its own `PtDelta`,
power only, exactly mirroring the `MinusOneMinusOne` block a few lines up. It can't ride the scalar
`plus_counters` path at all — that field emits a symmetric `{ power: n, toughness: n }`, and the
Beast is a 0/4 that stays a 0/4 on the bottom.

The cap is a `max_total: Option<u8>` field on `PutCounters` rather than a new `Amount` variant:
"can't cause the total number of +1/+0 counters on this creature to be greater than seven" is a
property of the *ability's cap clause*, not of the amount, and `de.rs`'s `Amount` deserializer is an
exhaustive tuple match that every struct-shaped variant perturbs. `mint_counters`' named-kind arm
clamps the resolved count to the room left, so a wind-up announcing four on a Beast with five
counters places two. `"up to X"` is taken as X (then clamped) rather than a resolution-time choice —
a `ponytail:` on the card says to prompt if a card ever makes a smaller pile the better one.

The trigger was the actual work. `Trigger::EndOfCombat` cannot be queued off the `StepBegan` scan,
because the same step's turn-based action pushes `Event::CombatCleared` before the post-batch trigger
scan runs, and the intervening-if would read an emptied `CombatState`. It is queued directly from
`priority.rs`'s `Step::EndCombat` arm ahead of the clear — the idiom `declare_blockers` already uses
for its own blocks triggers — and `queue_trigger_group` evaluates the condition at queue time
(CR 603.4's first check), which is exactly when the declarations are still live. The watch is
`battlefield_all`, not controller-scoped: the "or blocked" half only ever happens on an opponent's
turn.

*Cards:* clockwork_beast, cyclopean_tomb, living_artifact, rock_hydra, scavenging_ghoul.

### 29. `extra-land-plays-and-land-play-trigger` — 1 card, S — **done**
Depends on: nothing.
Fastbond. "You may play any number of lands on each of your turns" plus "whenever you play a land,
if it wasn't the first land you played this turn." The engine enforces one land per turn with a
counter; the counter exists, nothing lifts the cap and nothing triggers on the play. *Sketch:* a
`StaticEffect::AdditionalLandPlays { count: Option<u32> }` (`None` = unlimited) consulted by the
land-play legality check, and a `Timing::LandPlayed` trigger carrying the per-turn ordinal so the
intervening-if reads it directly.
*Cards:* fastbond.
*Landed:* three small pieces, none of them the sketch's. The static is `PlayAnyNumberOfLands`,
fieldless — the sketch's `count: Option<u32>` is config for a value only one printed card sets, so
it waits for an Exploration. The trigger is a plain controller-scoped `Trigger::YouPlayALand`
alongside `YouDiscard`, and the per-turn ordinal the sketch wanted the trigger to carry is already
on the board: `Player::lands_played` is bumped by the `Event::LandPlayed` apply *before* triggers
enqueue, so CR 603.4's check reads it as an ordinary intervening-if
(`Condition::LandsPlayedThisTurnAtLeast { at_least = 2 }`) — no new context plumbing.
The real work was the cap itself, which was written out twice (the `Intent::PlayLand` legality
check in `Game::play_land` and the `land_drop_unused` playability hint in `Game::land_actions`).
Both now route through one `Game::land_drop_available`, so an offered land play is a legal one;
lifting the cap in only the legality check would have left the hint still hiding lands 2..n.
Scoped by controller rather than owner, unlike the `has_no_max_hand_size` helper it is modelled on
— a stolen Fastbond gives its permission to the thief.

### 30. `counter-spell-with-mana-value-x` — 1 card, S — **done**
Depends on: nothing.
Spell Blast. Shipped as `SpellFilter::ManaValueEqualsX`, matched inline in
`Game::legal_targets_for`'s `SpellOnStack` arm. That function already threaded the filtering
spell's chosen `x` (for `PermanentFilter::mv_eq_x`) and is the single choke both cast-time
legality and the CR 608.2b resolution re-check route through, so the change was one early-return
there plus two exhaustiveness arms — no new field, and no signature change to
`spell_matches_filter`'s call sites, which have no X of their own and so answer `false`.
*Cards:* spell_blast.

### 31. `look-at-target-players-hand` — 1 card, S — **done**
Depends on: nothing.
Glasses of Urza. The engine reveals cards and looks at library tops but has no "look at a hand"
— it is purely a visibility grant to one player, with the server-side per-player filter being the
thing that has to change. *Sketch:* a one-shot `Effect::LookAtHand { target_player }` that widens
the activating player's projection of that hand for the duration of the resolution, threaded
through the same visibility filter that already special-cases revealed cards. No game state
changes; this is a projection-layer effect.
*Landed:* the sketch's "widen the projection for the duration of the resolution" was the wrong
unit. A look has no duration — you see the cards and you keep knowing them, so the state it leaves
is a set of `(looker, card)` pairs, and the hand gate in `snapshot` becomes per-card instead of
per-hand. That is both smaller and more faithful than a hand-wide grant: cards drawn after the look
were never looked at, and a card that leaves the hand and comes back is a new object (CR 400.7), so
the set never needs clearing and can't re-expose anything. The pairs are recorded by an
`Event::LookedAtHand { player, target }` that carries no card ids at all — *that* a look happened is
public at a table, what was in the hand is not, so the log stays honest for every seat with no
redaction arm to get wrong. No pending choice and no DTO: the cards simply appear in the looker's
next snapshot — which is exactly nowhere on the board until the client catch-up pass gave them a
surface.
*Cards:* glasses_of_urza.

### 32. `spend-mana-as-another-color` — 1 card, S — **done**
Depends on: nothing.
Sunglasses of Urza. The mana payment path matches colors exactly. *Sketch:* a
`StaticEffect::SpendManaAsThoughAnotherColor { from: Color, to: Color }` consulted by the payment
matcher as a fallback when an exact match fails. Cost *reduction* already hooks the payment path,
so the seam exists.
*Cards:* sunglasses_of_urza.

*Landed:* the static is the sketch's, but nothing consults it "as a fallback when an exact match
fails" — the pool already carries a credit kind that means exactly *this*. `Mana::OfColors(mask)`
is "one mana of any colour in this set", so widening each mono `{W}` credit into `of_colors{W,R}`
before planning **is** the permission, and every existing branch of `ManaPool::spend_plan`
(colored pips, hybrid pips, generic) already spends that kind correctly — no matcher change at
all. Widening rather than recolouring is what makes it a "may": the credit still pays `{W}`.
A mask rather than an `either` pair because a colour can carry several substitutions at once, and
`of_colors` keys on the whole set.
The catch the sketch doesn't mention: `Event::ManaSpent` subtracts the planned spend from the
*real* pool, so a plan made against a widened pool has to be mapped back —
`ManaPool::unsubstitute` charges whatever the plan spent beyond the real `of_colors` stock to the
mono colour it was widened from. That is the same substitute-plan-map-back shape `spend_plan`
already runs for `Mana::Restricted` credits, one function up.
Three chokes, not one: `plan_payment` (the real spend), `plan_auto_taps` (widening the starting
pool *and* every candidate credit, so a Plains gets auto-tapped for `{R}`), and the tail of
`available_mana` (so playability hints and the `{X}` ceiling agree with what the payment path will
accept).

### 33. `discard-to-library-top-replacement` — 1 card, S — **done, with the choice approximated**
Depends on: nothing.
Library of Leng. "If an effect causes you to discard a card, discard it, but you may put it on
top of your library instead." `no_maximum_hand_size` (the card's other half) already exists.
*Sketch:* a replacement consulted in the discard path offering
`PendingChoice::ChooseDiscardDestination`. Note the Oracle wording — the card *is* discarded
(discard triggers still fire), it just lands elsewhere.
*Cards:* library_of_leng.

*Landed:* `StaticEffect::DiscardToLibraryTopInstead` (fieldless — the discarding player is read off
the discard itself) plus `Game::discards_to_library_top`, a copy of `has_no_max_hand_size`'s live
permanent scan. `Game::discard_ids` is already the shared tail every discard routes through, so the
whole replacement is one `match` there swapping `Event::MovedToGraveyard` for the existing
`Event::TuckedToLibrary { to_top: true }` — no new event, no proto field, no client change. The
`Event::Discarded` marker is emitted either way, which is what keeps "whenever you discard" watchers
blind to the swap.

*The sketch's `PendingChoice::ChooseDiscardDestination` was not built, and the "you may" is
approximated as always-yes* (recorded in the card's `approximates`). `discard_ids` returns
synchronously into six callers that keep working afterwards — `answer_may_discard`'s "if you do"
rider, `answer_discard_edict`'s next-seat prompt, the cleanup step's `advance_step`, the wheel's
per-player draw — so pausing per card needs a resumable discard path first. Split that out as its
own increment when a second card wants the choice.

*Two scope findings that made the gate cheaper than the sketch expected:* CR 701.8c replaces only
*effect* discards, and neither of the two exceptions can reach `discard_ids` — discard **costs** are
paid in `cast.rs` / `pending/handlers/optional.rs` on their own zone moves, and the cleanup-step
hand-size trim can't arise for a Leng controller at all, since the card's other half gives them no
maximum hand size. So `discard_ids` needed no `is_cleanup`/`by_effect` parameter and no call-site
churn.

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

### 35. `aura-attachment-restriction` — 1 card, S — **done**
Depends on: nothing.
Consecrate Land's "can't be enchanted by other Auras." *Sketch:* a
`StaticEffect::CantBeEnchanted` consulted by the Aura target-legality check and by the
state-based Aura-attachment sweep. The indestructible half is an ordinary keyword grant.
*Cards:* consecrate_land.

*Landed:* not a new static — a `cant_be_enchanted` bool on `grant_to_attached`, beside the
`cant_attack` / `cant_block` / `cant_attack_controller` flags that already model "the Aura forbids
its host something." Those are read by little `host_cant_*` scans over `attachments(host)`, and the
new `Game::host_cant_be_enchanted_by(host, aura)` is that same scan with one extra clause: it skips
`aura` itself. That skip is the whole of "*other* Auras" — the Aura granting the restriction never
closes the door on itself, so it never sweeps itself into the graveyard. Two enforcement points,
both already-existing chokes: `attachment_host_legal` (which the CR 704.5n sweep runs over every
attachment on each state-based check, so an Aura already on the land falls off the moment Consecrate
Land enters) and the target-retain tail of `legal_targets_for`, scoped to an Aura that isn't a
permanent yet — that is this same card's cast enumeration (CR 303.4a), not a targeted ability an
already-attached Aura might have. The indestructible half is exactly the ordinary keyword grant the
sketch predicted, so both printed clauses ride one static.

### 36. `grant-triggered-ability-to-attached` — 1 card, M — **done**
Depends on: nothing.
Farmstead grants the enchanted *land* a triggered ability ("At the beginning of your upkeep, you
may pay {W}{W}…"). `grant_to_attached` grants keywords and `grant_source_abilities_until_end_of_turn`
grants a whole ability set; neither grants a single authored triggered ability to a host.
*Sketch:* let `grant_to_attached` carry an `abilities` list the trigger scanner picks up on the
host — `triggers.rs` already has a granted-triggered-abilities scanner (its ponytail notes it has
one consumer today), so this widens that path rather than adding one. The payload is #10's
optional-mana-payment shape.
*Cards:* farmstead.

*Landed:* no new effect and no new payload — `grant_to_attached`'s `granted_ability` already carried
an optional `trigger`, so the whole grant is one new field (`optional`) plus one new granted-trigger
flavor (`Trigger::Upkeep`, fieldless, `{ upkeep = {} }`). The "optional-mana-payment shape" the
sketch reached for is already what a triggered `Ability` does: setting `optional: true` and
`cost: g.cost.mana` on the synthesized ability raises the same pay-or-decline pause an authored
optional trigger raises, so `Intent::PayOptionalCost` works unchanged. Only the *mana* half of the
grant's `ActivationCost` transfers — an `Ability::cost` is a `Cost`, and tapping or sacrificing has
no meaning for something that was never activated.

The sketch's "widen the existing scanner" was half right. `granted_attachment_triggers` did exist,
but its one consumer was the bespoke combat-damage-to-a-player scanner, which does *not* route
through `queue_trigger_group` — the shared choke every other trigger flavor goes through. So the
fix was a `.chain(attached)` in `queue_trigger_group` itself, next to the `granted_source_abilities`
chain that was already there; upkeep and every other choke-routed flavor picked the grant up at
once. The combat scanner keeps its own call.

Not folded into `functional_abilities`, the other candidate: that returns a cheap `Arc<[Ability]>`
clone and is called in tight battlefield loops by the static scans, so a per-call attachment walk
plus a `Vec` allocation would have been a real regression for a grant only the trigger paths read.

### 37. `aura-reattachment-on-trigger` — 1 card, M — **done**
Depends on: nothing.
Kudzu — when the enchanted land becomes tapped, destroy it, and *that land's controller* attaches
Kudzu to a land of their choice. Two novelties: an Aura surviving its host's destruction (rather
than being swept as an orphan) and a re-attachment choice made by an opponent. *Sketch:* an
`Effect::ReattachSelf { chooser, filter }` raising a `PendingChoice::ChooseAttachTarget` for the
named player, with the state-based orphan sweep exempting the Aura for the window between host
destruction and the choice resolving — the same exemption `enchant_graveyard` already carves out.
*Cards:* kudzu.

*Landed:* one choice effect, and nothing else new. Both "novelties" turned out to already exist.
The trigger is Psychic Venom's `enchanted_permanent_becomes_tapped` verbatim, and the sketch's
`PendingChoice::ChooseAttachHost` is not hypothetical — it is what a deployed Aura (Armored
Skyhunter, Open the Armory) already raises, complete with the answer handler, the attachment event,
and the exact orphan-sweep exemption the sketch asked for (`awaiting_host` in the CR 704.5m pass).
So `ChoiceEffect::TriggeringPlayerMayAttachThisAuraToChosen` just builds the candidate list and
raises it, with `optional: true` for the card's "**may** attach" where a deployed Aura's is
mandatory.

The chooser fills from the same `fill_triggering_permanent_controller` slot Psychic Venom's damage
step uses, so "that land's controller" needed no new plumbing either. Destroying the host is plain
`destroy` / `target` over `target = "enchanted_creature"` — that spec is "whatever this is attached
to" with no creature check, so it reads a land fine.

Declining and having no land to offer both land in the same place: unattached, swept by CR 704.5m.
That is the card, not an approximation, so no `approximates` note.

### 38. `shuffle-hand-and-graveyard-then-draw` — 1 card, S — **done**
Depends on: nothing.
Timetwister. `each_player_discards_hand_then_draws` (Wheel of Fortune, already in the pool) is the
neighbour; Timetwister shuffles hand *and* graveyard into the library instead of discarding, and
Timetwister itself goes to the graveyard after (so it isn't shuffled in). *Sketch:* a sibling mode
on that effect with the zones to shuffle and no discard step.
*Cards:* timetwister.

*Landed:* a sibling `ChoiceEffect` variant (`each_player_shuffles_hand_and_graveyard_then_draws`),
not a flag on the wheel — the zones read, the zone written, and the triggers fired all differ, and a
bool that flips discard-to-shuffle *and* adds the graveyard is two axes wearing one name. The move
itself needed nothing new: `Event::TuckedToLibrary` already accepts any origin (the graveyard
shuffle-backs use it) and `Event::LibraryShuffled` already existed. Timetwister's "(Then put
Timetwister into its owner's graveyard.)" is CR 608.2m restated, so it gets no effect — the spell is
still on the stack while everyone shuffles, which the test pins. One test-shaped surprise worth
keeping: a recycled card can be *redrawn* by the same effect, so asserting "back in the library" is
wrong; the honest assertion is "never in a graveyard".

### 39. `graveyard-position-recursion` — 1 card, S — **done**
Depends on: nothing.
Nether Shadow — "if this card is in your graveyard with three or more creature cards above it."
The graveyard is ordered in the model but nothing reads position. *Sketch:* a
`Condition::CardsAboveThisInGraveyard { at_least, filter }` reading the existing ordering, plus
the upkeep trigger firing from the graveyard (triggers from a non-battlefield zone — check whether
the scanner already sweeps graveyards for `may_return_from_graveyard`; if it does, this is only
the condition).
*Cards:* nether_shadow.
*Landed:* only the condition, and it is S rather than M. The sketch's second half was already
built: `TriggerWatch::graveyard_controller(Trigger::Upkeep)` sweeps graveyards for any card marked
`functions_in_graveyard`, and `ZoneEffect::ReturnThisFromGraveyardToBattlefield` is the effect four
pool cards (Bloodghast, Nether Traitor, …) already use.
The sketch's "the graveyard is ordered in the model" is true by accident rather than by design:
nothing stores a pile position, but every arrival mints a fresh object through
`Game::create_object`, so a later object id *is* a later burial and `Game::graveyard_cards`
(an `objects`-index scan) already hands the pile back bottom to top. `creature_cards_above_in_graveyard`
is that one comparison. A card reaching a graveyard without minting a new object would break it —
none do today.
No `filter` axis on the condition, either: `CreatureCardsAboveThisInGraveyardAtLeast { count }` is
named for its one reading, like the `*CardsInYourGraveyardAtLeast` conditions beside it. Like every
other `[abilities.condition]`, it is checked once at trigger placement — the CR 603.4 second check
on an intervening-if is an engine-wide gap, not this card's.

### 40. `untapped-conditioned-anthem` — 1 card, S — **done**
Depends on: nothing.
Castle's "untapped creatures you control get +0/+2." `anthem` filters by color, subtype, and
controller but not by tapped state — and this one must re-evaluate the instant a creature taps.
*Sketch:* a `tapped: Option<bool>` axis on `PermanentFilter`, with `characteristics_cache.rs`
invalidating on tap/untap (confirm it already does — the cache invalidates on
`CombatCleared` and battlefield changes; a bare tap may not be one).
*Cards:* castle.
*Landed:* not on `PermanentFilter` — it already has a `tapped: Option<bool>`, but `anthem` takes
no filter at all (its axes are inline fields), so the axis went on `anthem` itself as
`untapped_only: bool`, beside `attacking_only`/`blocking_only`. The sketch's cache doubt was
right: `invalidate_characteristics_cache` had no tap arm at all. Three events cover every write
to `Permanent::tapped` — `Tapped`, `Untapped`, and `Regenerated` (CR 701.15b taps inside its own
replacement rather than emitting a `Tapped`); the ETB-tapped rider is already covered by
`PermanentEntered`'s board-wide drop. Each one invalidates just the permanent that turned, since
the axis reads the candidate's own `tapped`.

### 41. `opponent-chosen-sacrifice` — 1 card, S — **done**
Depends on: #20 (landed).
Demonic Hordes' "At the beginning of your upkeep, unless you pay {B}{B}{B}, tap this creature and
sacrifice a land of an opponent's choice." The pay-or-else half is #20's landed `pay_or_else`; what
is left is the penalty. Sacrifice effects always let the sacrificing player choose. *Sketch:* a
`chooser: Who` field on the sacrifice effect routing the existing `PendingChoice` to a different
player. In a 4-player game "an opponent's choice" is underspecified by the printed card — pick the
one whose upkeep-trigger controller is being punished, i.e. raise the choice to the next opponent
in turn order, and record that as an `approximates` on the card. The penalty's other half, "tap
this creature", has no effect either — the pool taps *targets*, not the source.
*Landed:* `PendingChoice::ChooseOwnSacrifices` already carried the whole shape — it just used one `player` field for two jobs, "whose permanents" and "who answers". Splitting them cost one field on the *raise* request (`owner`) and nothing on the pending choice, the DTO, or the wire: `player` now means the chooser, which is what every routing, visibility and answer-protocol site was already asking it. `sacrifice_ids` lost its `by` parameter instead of gaining one — it reads each permanent's own controller live, which every existing caller was already passing and which is the only correct answer once a set can span seats (CR 701.16a credits the sacrifice to the controller no matter who pointed at it). "Tap this creature" is a new one-shot `tap_source`, not a filter trick: `tap_all` is scoped to the controller's whole board and would have rendered the wrong card text. In a pod "an opponent's choice" names nobody, so the next living seat in turn order answers and the card carries an `approximates` saying so.
*Cards:* demonic_hordes.

### 42. `filter-comparing-to-source` — 1 card, S — **done**
Depends on: nothing.
Stone Giant's "target creature you control with toughness less than this creature's power."
Filters compare against constants, never against the source's own live characteristics.
*Landed:* two gaps, not one — the increment named only the filter, but the card's second sentence
("Destroy that creature at the beginning of the next end step") had no shape either.
The filter half is a `toughness_less_than_source_power: bool` on `PermanentFilter`, not the
sketch's `toughness_less_than: Option<Amount>`: `power_less_than_source` sits right above it doing
the same comparison one characteristic over, and a bool mirroring it is smaller than an amount axis
no second card wants. Target enumeration already threads `source`, so nothing else moved.
The delayed half reuses flicker's schedule-or-do-it-now shape: `destroy/target` gained an
`at: Option<Step>` that, when set, emits a CR 603.7 `DelayedTriggerScheduled` carrying the id the
activation *already* chose, so the landing re-targets nothing. Its payload is the variant Stinkweed
Imp's look-back destroy already used — renamed `TriggeringDamagedCreature` → `ThatCreature`
(DSL `destroy/that_creature`), since both users print exactly "destroy that creature" and its
resolution never read anything about damage. No new effect variant, no second destroy arm.
An illegal target isn't rejected at activation (this engine's posture — CR 608.2b fizzles it at
resolution instead), so the lift gate is asserted where a player meets it: `legal_targets` offers
the 2/2 and withholds the 3/3 and the opponent's creature.
*Cards:* stone_giant.

### 43. `mass-symmetrical-rebalancing` — 1 card, L — **done**
Depends on: nothing.
Balance. Three sequential symmetrical operations, each finding the minimum across players and
making everyone else match: sacrifice lands down to the fewest, discard down to the fewest, and
sacrifice creatures down to the fewest — with each affected player choosing which of their own.
*Sketch:* a `BalanceZone { zone, filter }` effect that computes the minimum, then fans out one
`PendingChoice` per player over-threshold in APNAP order (CR 101.4). `each_player_sacrifices`
exists but takes a fixed count; the novelty is the derived per-player count and the three-phase
sequencing.
*Landed:* no `BalanceZone` — the sketch would have meant a fourth pending-choice family and its own
proto messages for what is, in the end, a different *count* on two fan-outs the pool already has.
Both `each_player_sacrifices` and the discard fan-out took a `down_to_fewest` flag instead, and the
derived per-player count rides a `floor: Option<u32>` carried on the choice request *and* the
`PendingChoice`: the smallest matching battlefield (or hand) among the scoped seats is measured
once as the effect starts — before anybody sacrifices, so every seat is judged against the same
number, CR 701.16 — and each seat's `count` is then `options.len() - floor` at raise time. A seat
already at the floor is skipped outright rather than asked for zero, the way an empty hand is
already skipped. Carrying `floor` beside `count` is what lets an answered seat hand the *next* one
its own derived count. The three-phase sequencing is a plain `Sequence` of three steps.
`EachOpponentDiscards` became `EachPlayerDiscards { scope, down_to_fewest }` on the way through —
Balance's middle clause is all-players and Syphon Mind's is opponents-only, which is the axis
`each_player_sacrifices` already had — so syphon_mind.toml now names `scope = "each_opponent"`.
Two pre-existing wire gaps closed with it: `PendingChoiceView::SacrificeEdict` never carried
`count`, so the client's `choice.ts` guessed 1 and asked for one creature under Malfegor's "for
each card discarded this way"; and the `DiscardEdict` projection hardcoded `count: 1`. Both now
send the real number.
*Cards:* balance.

### 44. `aura-etb-conditional-self-grant` — 1 card, S — **done**
Depends on: nothing.
Earthbind — "When this Aura enters, **if** enchanted creature has flying, this Aura deals 2 damage
to that creature **and this Aura gains** 'Enchanted creature loses flying.'" The intervening-if
exists; the self-granted static does not — the Aura only strips flying if the check passed on
entry, so it can't be a plain printed static. *Sketch:* a
`Effect::GrantStaticToSelf { effect }` writing a granted-static onto the permanent that
`characteristics.rs`'s static scanners read alongside printed ones.
*Cards:* earthbind.

*Landed:* the sketch's granted-static machinery turned out to be unnecessary. Nothing needs to
record that the Aura *gained an ability*; the ability's only observable effect is the keyword loss,
so the trigger's second step applies that loss directly and the intervening-if
(`Condition::EnchantedCreatureHasKeyword`, source-object-based like the upkeep-tax cycle's gate)
decides whether it ever happens. The loss is recorded on the **Aura**
(`Permanent::attachment_lost_keywords`, written by `Event::AttachedKeywordsLost`) rather than on the
host, and read at the end of `compute_effective_keywords_uncached` through the Aura's live
attachment — so it ends by itself when the Aura leaves, follows the Aura if it is re-attached, and
needs no cleanup arm. That also let the wire stay put: an Aura grounding its host is the same
"re-read this object's keywords" cue as the until-end-of-turn strip, so the projection reuses
`VisibleEvent::KeywordsStripped` instead of growing a new event and a proto message.

### 45. `pump-by-own-power-with-delayed-destroy` — 1 card, S — **done**
Depends on: nothing.
Berserk. `Amount::TargetPower` exists, so "+X/+0 where X is its power" is close — but it must
snapshot at resolution, not track live. The rider is a delayed end-step destroy conditioned on
whether the creature attacked, which is `attacked_this_turn` (#1's neighbourhood) plus a scheduled
effect. Also needs the cast-timing restriction "only before the combat damage step."
*Cards:* berserk.

*Landed:* two of the three pieces were already in the engine, and the backlog's worry about the
third was misplaced. `PumpUntilEndOfTurn` already takes an `Amount` per axis and already grants
keywords, so "gains trample and gets +X/+0 where X is its power" is `power = "target_power"`,
`toughness = 0`, `keywords = ["trample"]` — and it snapshots for free, because the amount resolves
once into a fixed temp boost rather than tracking anything. (The backlog also had the pump as
+X/+X; the printed Oracle is +X/+0.) The delayed destroy is Stone Giant's: `destroy/target` with
`at = "end"` already bakes the chosen id into a `destroy/that_creature` payload and schedules it,
so nothing is re-targeted when it fires.

What was actually missing was the *conditional* on that rider. It cannot be an
`Effect::Conditional` around the schedule, because "if it attacked this turn" has to be read when
the delayed ability fires, not when Berserk resolves — a first-main-phase Berserk on a creature
that then attacks still kills it. And it cannot be `Condition::SourceAttackedThisTurn`, because a
delayed trigger's source is Berserk itself, not the creature. So the flag rides the effect the
same way `at` does: `only_if_it_attacked` on `destroy/target`, carried into the
`destroy/that_creature` payload, checked against that creature's `Permanent::attacked_this_turn`
at fire time.

The cast restriction is the third member of an existing family: `cast_only_before_combat_damage`
next to `cast_only_during_combat` (Cauldron Dance) and `cast_only_before_attackers` (Master
Warcraft), one guard in `cast_timing_ok`. The boundary is `Step::FirstStrikeCombatDamage` rather
than `CombatDamage`, since that step is the first combat damage step whenever it exists and the
engine only creates it when a first striker is in combat (CR 510.5).

### 46. `mana-from-variable-amount` — 1 card, S — **done**
Depends on: nothing.
Sacrifice — "Add an amount of {B} equal to the sacrificed creature's mana value." Mana effects add
fixed quantities; `Amount::SacrificedCreaturePower` exists but not mana value, and the mana effect
takes no amount. *Sketch:* an `amount: Amount` on the mana-add effect, plus
`Amount::SacrificedCreatureManaValue`. The additional cost (`[cost.additional]` sacrifice) already
exists.
*Cards:* sacrifice.

*Landed:* half the sketch was already there. `ManaEffect::Add` carries a `repeat: Amount` that
multiplies the whole mana batch, so no `amount` field was needed — `mana = ["black"]` with
`repeat = "spell_sacrificed_mana_value"` is the card. The real work was the *channel*: Sacrifice is
a spell, not an activated ability, so `contextualize_sacrifice_effect` (which fills
`Amount::SacrificedCreaturePower` at `activate_ability`) doesn't reach it. A spell's effect is read
off its def at resolution, by which time the fodder is a graveyard card. The cast context is the
seam — `Spell::sacrifice_count` and `Spell::revealed_creature_mana_value` already ride
`Event::SpellCast` for exactly this reason, so `sacrificed_mana_value` joins them and
`Amount::SpellSacrificedManaValue` reads it off the resolving spell. Naming matters here: it is a
`spell_*` sibling, *not* a `sacrificed_creature_*` one, because the two families use different
mechanisms and mixing them up is a silent wrong answer.

### 47. `lich-life-replacement` — 1 card, L — **done**
Depends on: #22 (landed — the damage-taken trigger is already there).
Lich. Four interlocking replacements: you don't lose at 0 or less life, life gain becomes card
draw, damage taken becomes a sacrifice of that many permanents, and losing the enchantment loses
the game. `life_gain_replacement` exists (Pest Rescuer); the loss-prevention does not, and the
state-based-action check for 0 life is unconditional. *Sketch:* a per-player
`ignores_zero_life_loss` flag consulted by the SBA check, plus the damage-taken trigger from #22.
Worth landing last — it touches the loss condition, which every other test in the suite implicitly
depends on.
*Cards:* lich.

*Landed:* four replacements turned out to be two statics, one flag and one plain effect — no
per-player flags, no new choice family. `you_dont_lose_at_zero_life` and `life_gain_becomes_draw`
are fieldless `StaticEffect`s read off the battlefield by a shared `controls_static` scan, so they
last exactly as long as the permanent printing them and need no cleanup when it leaves. The
zero-life clause is one `&& !self.ignores_zero_life(...)` on the SBA's life arm — the other three
elimination conditions in that same `if` are untouched, which is what the card says. The life-gain
clause went in at the head of `push_apply_effect_event`, the designated replacement hook every
`Effect::Life` already routes through: it swaps the `LifeChanged` out entirely for a
`draw_with_dredge`, so nothing that watches life gain sees anything. Combat lifelink was the one
caller reaching `push_apply` directly and now routes through the hook too, which is what makes the
replacement cover lifelink and drains rather than only a printed "you gain N life".

The damage tax reused the edict machinery rather than growing a mode of its own: a new
`EdictScope::You` makes it a one-seat fan-out, and `lose_game_if_short` on `EachPlayerSacrifices`
turns a board shorter than the bill into an elimination (CR 104.3b) instead of the discount every
other edict gives. The prompt, the derived count and the client's `sacrifice_edict` view — which
carries `count` since #43 — all came free. `Amount::TriggeringDamageDealt` needed an
`EachPlayerSacrifices` arm in `map_effect_amounts` to reach the edict's `count`; without it the
bill resolved to 0. ponytail: CR 608.2 would have an unpayable seat sacrifice what they can on the
way out — skipped, since they lose either way and every permanent they own leaves with them (CR
800.4a); run the fan-out first if a card ever cares about those deaths.

Two fidelity bugs fell out on the way. `Trigger::Dies` was gated behind `CardKind::Creature`, but
CR 700.4's "dies" is any permanent put into a graveyard from the battlefield — Lich's own dies
trigger sits on an enchantment. Hoisted above the gate; the watch-*others* triggers stayed inside
it, because their printed wordings all say "creature", and no pool card had a dies trigger on a
noncreature before this one. And `Amount::YourLifeTotal` is deliberately unclamped: a controller
already below 0 resolves to a negative, which on a `lose_life` gains the difference back. No pool
card reaches that, and clamping would be inventing a rule none of them print.

### 48. `pile-based-block-assignment` — 2 cards, XL — **done**
Depends on: #11.
Camouflage and Raging River — both replace declare-blockers with a pile-division ritual, and
Camouflage assigns piles to attackers *at random*. This is the least valuable work in the file
(two cards, no reuse, and the mechanic was abandoned in 1994) and the most invasive (it replaces
the declare-blockers step rather than constraining it). *Sketch:* deferred — do not start this
until every other increment has landed, and reconsider whether an `approximates` is the honest
answer instead.
*Cards:* camouflage, raging_river.

*Landed:* both, and the sketch's fear turned out to be misplaced — the declare-blockers step never
had to be replaced, only pre-empted. `Game::can_block` is a single choke point every legality read
already goes through, so Raging River is a list of the `(blocker, attacker)` pairs its labeling
made illegal: `CombatState::cant_block_this_combat`, stored as the *excluded* pairs rather than the
allowed ones so the printed flying exemption falls out for free — a flyer is never divided into a
pile, so it is never on the list. `combat.blocked_by` turned out to be the other half: it is
already the "this seat's declaration is final" set that both the auto-seal and `block_seats_for`
read, so Camouflage's "instead of declaring blockers" is just writing the blocks down and adding
the seat to it. The declaration tail of `Game::declare_blockers` — the events, the three
"blocks or becomes blocked" trigger scans, the seal — came out as `Game::seal_blocks` so both
paths produce the same thing.

The two rituals share the subset-answer idiom (`Intent::ChooseSacrifices` over a pause carrying
its own continuation) but not their shape. Raging River asks each defending player once
(`PendingChoice::SplitBlockersIntoPiles`, left pile named, right pile inferred) and then asks the
attacker's controller once per attacking creature (`PendingChoice::ChoosePileForAttacker`,
`Intent::ChooseOpponentPile` for the bare left/right). Camouflage asks each defending player once
*per attacker aimed at them* (`PendingChoice::DivideBlockersIntoPiles`, each pile drawn from what
the last one left), then deals the piles out over `Game::with_op_rng` — the same derive-per-op
stream shuffles and Amulet of Quoz already use, so the deal replays identically. Both views ride
`PendingChoiceView::PartitionRevealed` / `ChoosePileForHand` behind two additive discriminators
(`into_piles`, `attacker`) rather than new prompt shapes.

Camouflage needed one new cast window, `cast_only_during_declare_attackers` — the attack-side twin
of False Orders' flag, plus the "your" that one doesn't print. Two clauses are approximated and say
so on the card: "creatures … that can block additional creatures may likewise be put into
additional piles" is dropped (each creature goes in at most one pile, so a Two-Headed Giant of
Foriys is dealt to one attacker), and "this turn" is read as this combat, since the piles are
divided as the spell resolves and a second combat phase gets an ordinary declare-blockers step.

### 49. `controlling-another-players-actions` — 1 card, XL — **done**
Depends on: nothing.
Word of Command ("you control that player until this finishes resolving") and Drain Power ("target
player activates a mana ability of each land they control"). One player making decisions on
another's behalf, mid-resolution, with a restricted legal-action set. The engine's submit path is
built around a single acting player per pending choice, so this is a structural change to the
choice model, not an effect. *Sketch:* a `PendingChoice` variant carrying both an `acting_player`
(who answers) and a `subject_player` (whose resources are spent), with the visibility filter
widened for the duration. Drain Power is the tractable half — its action set is exactly "tap each
land for mana", so it can be modelled as a direct effect without a real control handoff. Word of
Command is the hard half and should be the last thing attempted in this set. Drain Power landed in
increment 23 as that direct effect, so only Word of Command is left here.
*Cards:* word_of_command.

*Landed:* the sketch's structural worry didn't survive contact. Nothing about the choice model
needed to change: `PendingChoice::ChooseCardInHandToPlay { player, source, subject, options }`
just names two seats instead of one — `player` answers, `subject` plays — and the existing
`player()` accessor (which is what routes and validates a submit) still returns the answering
seat, so the submit path never learns there are two. The play itself is one ordinary internal
`Game::cast(subject, …)`, which means the printed mana restriction ("only lands that player
controls, only spent on that card") came for free: `settle_payment` auto-taps through
`auto_tap_candidates`, which only ever considers sources the paying player owns. Two gates did
have to move, both mid-resolution reads the engine already has a shape for — `validate_cast`'s
`player != self.priority` and `cast_timing_ok`'s instant-speed bypass — now both consult a new
`PlayPermissions::compelled_play: Option<(ObjectId, PlayerId)>` that is set for exactly the length
of that one `cast` call and cleared right after. It is deliberately *not* a cost waiver: the
existing free-cast-from-exile permission would have been the smaller diff and the wrong card, since
it zeroes the cost. "Plays that card **if able**" is the `Err` being swallowed: an unaffordable pick
is a legal answer that does nothing.

Reused rather than minted: the answer is `Intent::ChooseExiledDigToCastFree` (same "one object plus
a cast-time target, or decline" shape) and the view is `PendingChoiceView::ChooseExiledDigToCastFree`
behind an additive `from_opponent_hand` bool — same precedent as #78's `choose_block_target`. The
one thing the view could *not* reuse was `label_items`: these candidates are cards in someone
else's hand, so they go through `private_items(player, viewer, …)` and are redacted for every seat
but the looker. `Event::LookedAtHand` is pushed before the pause, so you keep knowing the whole hand
afterwards, not just the card you took.

Two gaps are annotated on the card rather than papered over. The compelled play asks its cast-time
questions of the controlled player, not of you — a modal card's mode, and any target past the
first, pause on their seat. And "you control the player while that spell is resolving" isn't
modelled at all: a resolution-time choice on the chosen spell goes back to its own controller. Both
want the same thing (a controller override that spans a resolution, not a single call), and neither
is reachable from a 2ed card — the set has no modal spells and no divvy.

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

### 51. `land-subtype-permanent-filter` — 2 cards, S — **done**
Depends on: nothing.
A land's printed types live under `[kind].subtypes` (CR 305), but `PermanentFilter::subtypes`
matches against `Game::effective_subtypes`, which reads only the card's **top-level** `subtypes`.
So `filter = { types = "land", subtypes = ["Plains"] }` matches no Plains at all, and the two
mass land-hate spells have no faithful shape. *Sketch:* fold a land's `[kind].subtypes` into
`effective_subtypes` for battlefield lands, the way the catalog already unions the two — then the
existing `subtypes` axis covers both halves and `CardFilter::LandWithSubtype` gets a battlefield
twin for free.
*Cards:* flashfires, tsunami.
*Landed:* the whole fix is four lines in `Game::effective_subtypes` — a land's `[kind].subtypes`
unioned into the printed line before the CR 613.4 layers run. No new filter axis, and nothing at
either call site: `permanent_matches` already routed its `subtypes` check through
`effective_subtypes`, so the sweepers are ordinary `{ types = "land", subtypes = ["Plains"] }`
filters. `CardFilter::LandWithSubtype` got no twin either — it reads a *card* (a library search),
never a battlefield object, so it never shared the broken path. One read stays on the printed line
deliberately: `destroyed_this_way_matches` matches a snapshot of an already-destroyed permanent,
which has no live object to ask.

### 52. `blocking-creature-filter` — 1 card, S — **done**
Depends on: nothing.
`PermanentFilter` has an `attacking` axis but no `blocking` one, so "target blocking creature"
can't be expressed. `anthem_static`'s own `blocking_only` already reads `CombatState::blocks` for
Crescendo of War — this is the same read, hoisted onto the shared filter.
*Landed:* `blocking: bool` on `PermanentFilter`, one early return in `permanent_matches` reading
`CombatState::blocks` directly (not `blockers_of`, which answers the other question — who blocks a
*given* attacker — and would mean scanning every attacker to ask about one blocker). The sketch's
second half didn't happen: `anthem_static` carries its own inline filter fields rather than a
`PermanentFilter`, so `blocking_only` isn't a duplicate of this axis and dropping it would mean
converting that whole effect to take a filter — a refactor no card is asking for.
*Cards:* righteousness.

### 53. `evenly-divided-damage-and-per-target-cost` — 1 card, M — **done**
Depends on: nothing.
Fireball needs two things the DSL lacks. `DamageEffect::Target`'s `divided` splits an amount
**as the caster chooses**; Fireball divides it *evenly, rounded down*, which is a computed split
with no choice at all. And "this spell costs {1} more to cast for each target beyond the first"
is a cost modification keyed to a target count chosen during casting — the cost pipeline has no
hook that late. *Sketch:* a `divided = "evenly"` arm on the damage effect (an enum where the bool
is now), plus a `cost_per_extra_target` field on `CardDef` consulted after targets are declared
in the cast path.
*Cards:* fireball.

*Landed:* half of this increment didn't exist. "This spell costs {1} more to cast for each target
beyond the first" *is* Strive (CR 702.42), printed without the keyword's name — Twinflame's
`[cost.additional.strive]` plus `count = { strive_scaled = true }` is the same mechanic at a
different price, right down to the caster declaring the target count pre-stack because this engine
stacks the spell before it pauses to choose targets. No `cost_per_extra_target` on `CardDef`, no
cast-path hook: Fireball is a TOML-only card on that axis.

The even split is the whole engine change, and it's an enum where the bool was: `Division::None` /
`AsYouChoose` / `Evenly`, deserialized from `false` / `true` / `"evenly"`. `Evenly` returns from
`maybe_begin_damage_division` without raising the division pause at all — it pushes `total / n` to
each target itself, which is what "rounded down" means and why a Fireball for 2 among 3 targets
deals nothing to any of them.

### 54. `damage-then-gain-that-much-life` — 1 card, M — **done**
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

*Landed:* neither half became its own effect. The life gain is a `gain_life_equal_to_damage` bool
on `damage/target`, sitting beside Disintegrate's `cant_be_regenerated` and
`exile_instead_of_dying` — the same rider shape, for the same reason: what the caster gains is a
property of *this* damage landing, not of a separate step that would have to go looking for a
tally. `creature_damage_events_with_riders` and `player_damage_events` were already returning
`(events, actually_dealt)` for prevention shields, so "equal to the damage dealt" was a value
already in hand; the arm's three `return Vec::new()` prevention exits moved into a new
`single_target_damage_events` so they could return a 0 alongside their events.

The oracle's cap is not redundant with the actual-damage tally, which is where the backlog sketch
was wrong: 5 damage to a player at 2 life still *deals* all 5 (they go to -3), and only 2 comes
back. So `drain_gain` reads the target's own capacity to absorb damage — life total, loyalty, or
toughness — before any of it lands, and mins against it.

"Spend only black mana on X" is one `Option<Color>` on `Cost`. `Cost::with_x` already existed to
fold the chosen value into `generic` "so mana planning never has to know about `{X}`"; with
`x_color` set it folds into that color's pips instead, and the colored-pip payment planner — which
has always been the thing that refuses a Forest for a `{B}` — enforces the restriction with no new
code. `max_payable_x` needed nothing either, since it drives everything through `cost_at(x)`.

### 55. `rearrange-target-players-library-top` — 1 card, M — **done**
Depends on: 31 (`look-at-target-players-hand`) shares its "look at another player's hidden zone"
visibility work.
`look_at_top` always digs the resolving controller's own library. Natural Selection looks at the
top three of **target player's** library, reorders them, and may then have that player shuffle.
*Sketch:* a `whose = "target_player"` axis on `look_at_top` plus an ordering choice (the pool
already has "put back in any order" for scry/surveil — reuse that pending-choice shape) and an
optional shuffle step.
*Cards:* natural_selection.

*Landed:* the sketch picked the wrong neighbour. `look_at_top` is a *select* choice (filter some
cards out of the top N into a destination), so a `whose` axis on it would have bought nothing —
Natural Selection makes no selection at all. The choice it actually makes is the scry/surveil one,
so the two new `dig` modes lean on `PendingChoice::ArrangeTop`, and the honest cost was widening
that choice twice. It had assumed the chooser owns the library, which stopped being true, so
`player` (who answers) and `library` (whose cards) are now separate fields. And its `to_graveyard:
bool` became an `ArrangeRest` of `Bottom` / `Graveyard` / `Nowhere`, because "put them back" gives
the rest pile nowhere to go: the answer handler rejects a non-empty bottom outright, or the caster
could bury a card the card never let them bury.

That third destination is why this was really an M and not an S. A pending choice the client
renders has to reach the client, so `ArrangeRest::Nowhere` projects to a new `reorder_top`
`PendingChoiceView` (proto field 71) rather than lying about itself as a scry — a scry prompt would
offer a bottom lane the engine will refuse. The prompt reuses the same two lanes, relabelled: the
second one is "Not yet ordered", and anything left in it follows the ordered pile back onto the top.

The shuffle is a second `[[abilities.effects]]` step, not a flag on the first, and it is addressed
to the **controller** — "you may have that player shuffle" is the caster choosing whether to throw
away the ordering they just picked. `MayYesNoResume` has no "then run this" variant, so the
targeted player is baked into `MayShuffleTargetPlayersLibrary { owner }` when the pause is raised,
the same way `MayDrawUnlessPays { caster }` carries its seat.

### 56. `activate-only-during-your-turn` — 1 card, S — **done**
Depends on: nothing.
`sorcery_speed` is the only activation-timing gate the DSL has, and it is stricter than what
Disrupting Scepter prints: "Activate only during your turn" allows activation in combat, in
either end step, and with the stack non-empty, all of which `sorcery_speed` forbids (CR 602.5b vs
a plain turn check). Authoring it as `sorcery_speed` would quietly narrow a card, so the Scepter
waits. *Sketch:* a `your_turn_only` bool on the activated-ability cost fields, checked in
`ability_activation_gate` next to `sorcery_speed` — an independent axis, not a widening of it.
*Cards:* disrupting_scepter.

*Landed:* the sketch was wrong — `sorcery_speed` is not the only activation-timing gate. `ability_activation_gate` already runs an ability's `condition` as an activation restriction ("Activate only if you control five or more lands" — Temple of the False God), and `Condition::DuringYourTurn` already existed for Restless Spire's conditional first strike. Disrupting Scepter is `[abilities.condition] type = "during_your_turn"` and nothing else: zero engine lines. The new `your_turn_only` bool would have been a second spelling of a predicate the gate already evaluates. Worth remembering for #57 and anything else that reaches for a new cost flag — check `Condition` first, since the activation gate and the intervening-if evaluator are the same code path.

### 57. `until-end-of-combat-animation` — 1 card, M — **done**
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

*Landed:* the gate half was free, and for the reason #56 recorded — check `Condition` before
reaching for a cost flag. `Condition::DuringCombat` (`self.step.is_combat()`, the same predicate
`cast_only_during_combat` uses on the cast side) is two lines, and `ability_activation_gate` already
runs an ability's condition as an activation restriction. No `combat_only` cost field.

The duration half is a `ends_at_end_of_combat` bool riding `AnimateSelfUntilEndOfTurn` down to
`Event::BasePtSetUntilEndOfTurn` and onto `Permanent`, plus an End of Combat sweep in
`begin_step` next to the existing `CombatCleared` push. Not projected to the wire: the client reads
the animated P/T off the snapshot, so the duration is engine bookkeeping and the proto is untouched.

Ceiling, recorded in a `ponytail:` at the sweep: it reuses `Event::TempBoostsEnded`, which ends
*every* until-EOT effect on the Statue, so a pump cast on it mid-combat would end early too. A
dedicated event costs seven files across engine/schema/proto/server for one card; split it if a
second end-of-combat card ever lands.

### 58. `damage-the-entering-permanents-controller` — 1 card, S — **done**
Depends on: nothing.
Ankh of Mishra watches lands enter (`Trigger::PermanentEnters` with `EnterController::AnyPlayer`
already covers the watch) and then damages *that land's controller*. The two damage effects that
read a triggering object aim elsewhere: `DamageEffect::ToEnteringPermanent` hits the permanent
itself, and `DamageEffect::ToTargetController` reads an enclosing `Sequence`'s shared target, which
a trigger with no target never sets. *Sketch:* a `DamageEffect::ToEnteringPermanentController {
amount }` reading the same `TriggerContext` slot `ToEnteringPermanent` already threads.
*Cards:* ankh_of_mishra.
*Landed:* the sketch held exactly. `fill_entering_permanent` (types/effect/shared.rs) is the one
place the entering object is baked into a trigger's payload, and the new
`DamageEffect::ToEnteringPermanentController` slots in beside `ToEnteringPermanent` there with no
other plumbing — unlike its sibling it needs no `resolve_deal_damage_to_entering` choreography
(that exists only for the `then_if_subtype` rider), just a plain arm in `resolution/damage.rs`.
It reads `controller_of`, not `owner_of`, so a land stolen by a Confiscate bills the thief, which
is what the printed word says. The test needed a stocked library for the second player: an empty
one loses them the game in their draw step and their hand with it.

### 59. `land-put-into-graveyard-watch` — 1 card, M — **done**
Depends on: 58 (`damage-the-entering-permanents-controller`) — Dingus Egg's payoff is the exit-side
twin of Ankh's and wants the same "that land's controller" addressing.
There is no trigger for a *land* leaving the battlefield for a graveyard. The death watches are
creature-, enchantment-, and nonland-permanent-scoped (`CreatureDies`, `EnchantmentYouControlDies`,
`NonlandPermanentYouControlDiesIncludingThis`) — lands are the one permanent type deliberately
outside all of them, and every arm is controller-scoped besides. *Sketch:* a
`Trigger::PermanentPutIntoGraveyard { filter: PermanentFilter, controller: EnterController }`
mirroring `PermanentEnters`'s filter+scope shape, with the dying permanent's controller on the
context. *Cards:* dingus_egg.
*Landed:* the trigger is fieldless — `Trigger::LandPutIntoGraveyard`, not the sketched
`{ filter, controller }` pair. The filter+scope shape is `PermanentEnters`', where the entering
permanent is still on the battlefield for `permanent_matches` to read; a dead land is a graveyard
card, so the scanner can only ask its last-known `CardDef` — which is exactly what the two existing
death scanners do (`def.kind.types().intersects(…)`) and all Dingus Egg needs. The scope half is
moot for the one card: its text names no seat, so the scan is battlefield-wide with the watcher's
own controller on the context.
No new `DamageEffect` either. `ToTriggeringPlayer` already means "the one player this trigger
names, baked in at placement" — increment 60 filled it from `active_player`, and a second filler
(`fill_dying_permanent_controller`, off a new `TriggerContext::dying_permanent_controller`) reuses
the variant unchanged. `ToEnteringPermanentController` was the other candidate and is the wrong
one: it asks `controller_of` at *resolution*, which answers the owner once the land is in the
graveyard.
Both accepted ceilings ride on `ponytail:` notes in `queue_land_death_watchers`. The
controller-at-death is owner-based, like both sibling scanners — a land stolen with Confiscate
bills its owner. And there are three near-identical batch death scanners now, differing only in a
type filter and a controller scope; worth folding into one filter+scope scanner when a fourth
lands, not before (retrofitting the two older ones would churn Starfield Mystic and Martyr's Bond
for nothing).

### 60. `each-upkeep-payoff-addresses-that-player` — 1 card, S — **done**
Depends on: nothing.
`Trigger::EachUpkeep` fires on every player's upkeep but, per its own `ponytail:` note on
`queue_each_upkeep_triggers`, does not thread `TriggerContext::active_player` the way
`EachDrawStep` does. Copper Tablet's "deals 1 damage to **that player**" therefore has no way to
name whose upkeep it is; `DamageEffect::EachPlayer` would hit the whole table once per upkeep,
which is four times the printed damage in a four-player game. *Sketch:* thread `active_player` in
`queue_each_upkeep_triggers` (the note already sketches it) plus a
`DamageEffect::ToTriggeringPlayer { amount }`. *Cards:* copper_tablet.
*Landed:* both halves as sketched, and the threading was three lines — `TriggerWatchContextKind`
is per-watch and purely additive (it fills `TriggerContext::active_player` and leaves
`controller` alone), so swapping `EachUpkeep`'s watch to `battlefield_all_with_active_player`
costs the pool's existing each-upkeep cards nothing. The new damage mode fills its player slot
through the same walker Howling Mine uses, now renamed `fill_active_player_payoff` since it is no
longer draw-only. The stale `ponytail:` note on `queue_each_upkeep_triggers` is gone with it.

### 61. `upkeep-of-the-enchanted-permanents-controller` — 4 cards, M — **done**
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
*Landed:* no new trigger variant. Increment 60 had just taught `Trigger::EachUpkeep` to carry
the active player, so the cycle is that same watch plus one intervening-if `Condition`,
`EnchantedPermanentsControllersUpkeep`, that holds only when the Aura's host's controller is
whose upkeep it is — and `damage/to_triggering_player` from 60 addresses the same player. A
trigger filtered out at placement never reaches the stack, so it is indistinguishable from one
that never fired; a real variant is only worth it if a card ever needs to *see* the non-fire.
The condition is source-object-based, so it is handled in `ability_condition_holds` (the one
site with a source id) and falls through to `false` in `condition_holds`, exactly like
`SourceUntapped`. Type-agnostic for free: each card's own `enchant` restriction does the
typing, so all four Auras share one ability shape and one shape test.

### 62. `damage-equal-to-the-dying-creatures-toughness` — 1 card, M — **done**
Depends on: 61 (`upkeep-of-the-enchanted-permanents-controller`) — both are Aura payoffs aimed at
the host's controller, so they want the same context plumbing.
Creature Bond's `Trigger::EnchantedCreatureDies` watch already exists, but the payoff needs two
things it can't get: an amount read from the dying creature's *last-known* toughness (CR
603.6c/603.10 — the creature is gone by resolution, so no live characteristic read works), and the
dying creature's controller as the damage recipient. `Amount` has no last-known-information arm.
*Sketch:* an `Amount::DyingPermanentToughness` fed from the death snapshot `Game::apply` already
captures for the `*IncludingThis` arms, plus the increment-61 "damage the host's controller"
recipient. *Cards:* creature_bond.
*Landed:* the snapshot the sketch wanted already existed — `dying_creature_stats`, captured in
`Game::apply` the instant a creature's death event applies — it just didn't record toughness or
controller. Widening it from a `(id, power, +1/+1 counters)` tuple to a `DyingCreatureStats`
struct with both was the whole cost; `Amount::DyingEnchantedCreatureToughness` and
`DamageEffect::ToDyingEnchantedCreaturesController` are then ordinary placement-time
placeholders, filled together off one new `TriggerContext` field.
Controller, not owner: after the host dies, `controller_of` follows it to the graveyard card and
answers its *owner*, so Creature Bond on a creature stolen by Control Magic would have billed
the wrong seat without the snapshot.

### 63. `whenever-this-is-dealt-damage` — 1 card, M — **done**
Depends on: nothing.
Fungusaur grows every time it is dealt damage, from any source — combat, a burn spell, a ping.
The damage-shaped triggers in the pool all watch damage *this permanent deals*
(`DealsCombatDamageToCreature`, `DealsDamageToOpponent`, `CreatureDealtDamageByThisDies`); nothing
watches damage *received*. This is not a rename of one of those — the event is a different one,
and it fires once per damage event rather than once per combat. *Sketch:* a
`Trigger::ThisIsDealtDamage` queued off the damage-marking path with the amount on the context (a
"dealt damage" watcher that scales with the amount is the obvious next consumer, so thread it
even though Fungusaur ignores it). *Cards:* fungusaur.

*Landed:* `Trigger::ThisIsDealtDamage` as one row in the `TriggerWatch` table — self-scoped, with
the watch event's `source` set to the *damaged* permanent rather than the dealer, which is the whole
trick. It rides `Event::DamageMarked`, the choke already shared by combat damage, fight damage, and
a plain ping, so all three count with no per-path work.

*The amount was not threaded onto `TriggerContext`* — the sketch asked for it speculatively and
Fungusaur ignores it, so it stays a `ponytail:` note on the variant. Adding the slot later is a
one-line `TriggerWatchContextKind` arm.

The `DamageMarked` arm did have to widen from `source: Some(source)` to any source: the two tallies
already there (Armadillo Cloak's host watch, Vampiric Dragon's damaged-this-turn list) are
dealer-keyed and stay behind a `let Some(source) = source else { continue }`, while a receiving-end
watch has to fire for sourceless damage too.

### 64. `fixed-color-tapped-for-mana-bonus` — 1 card, S — **done**
Depends on: nothing.
`StaticEffect::TappedForManaBonus` already has the right watch and the right scope
(`LandTapScope::EnchantedHost`), but `LandTapBonusColor` offers only `AnyColor` (Fertile Ground's
"one mana of any color") and `Produced` (Mirari's Wake's "any type that land produced"). Wild
Growth adds an additional **{G}** specifically — strictly narrower than `AnyColor` and unrelated
to `Produced`, so neither approximation is faithful. *Sketch:* a
`LandTapBonusColor::Fixed(Color)` arm, credited without the `ChooseManaColor` pause `AnyColor`
raises. *Cards:* wild_growth.
*Landed:* exactly the sketch — a `Fixed(Color)` arm, one `Vec<Color>` accumulator beside the
existing `produced_bonuses` counter, credited inline. The `{ fixed = "green" }` TOML shape falls
straight out of serde's externally-tagged enum, so the DSL needed nothing. One thing came along:
the client string for this whole watch was a `literal` reading "any type that land produced" —
wrong for Fertile Ground already, and wrong for every scope. It reads its `scope`/`bonus_color`
params now.

### 65. `skip-your-draw-step-for-an-attack-shield` — 1 card, M — **done**
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

*Landed:* two of the three "things the engine lacks" turned out to be already sitting there. The
optional replacement is `PendingChoice::MayYesNo` — the draw step already pauses on a replacement
choice for dredge, so a new `MayYesNoResume::SkipDrawStepDraw` gets the offer and the resume for
free; "yes" arms the shield, "no" falls through to the dredge check and then the draw, so a player
holding both replacements still gets both choices. And `StaticEffect::CantBeAttackedBy` already
expresses the restriction — it just wanted a duration, which is one
`CombatExtras::repelled_until_next_turn` entry read in `declare_attackers` beside that static's own
scan. Only the keyword axis was genuinely new: `PermanentFilter::without_keyword`, a second
exclusion slot beside `without_flying`, because "flying and/or islandwalk" inverts to a banned set
that has to lack both at once and one bool can't say that. The card is authored inverted for the
same reason `cant_be_attacked_by` is. The shield is the one entry in `CombatExtras` that outlives
its turn: everything else there clears at the next untap whoever's it is, this one only at the
shielded player's own, which is exactly "until your next turn".

### 66. `grant-an-activated-ability-to-a-filter` — 1 card, M — **done**
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

*Landed:* the one-accessor half of the sketch held; the "make it an `Anthem` field" half didn't.
`Anthem` is a P/T-and-keyword grant whose fifteen axes all describe *characteristics*, and its
message key renders that shape — a sixteenth field carrying an activated ability would have made
every Zombie Master log line read "Other Zombies get +0/+0". It ships instead as a sibling
`StaticEffect::GrantActivatedAbility { filter, granted_ability }`, which is `GrantManaAbility`'s
own shape with the mana swapped for the `GrantedAbility` sub-table `GrantToAttached` already
carries, and it reuses `PermanentFilter` rather than duplicating `Anthem`'s parallel subtype axis.
So the card is one keyword anthem (the swampwalk half, unchanged #3 machinery) plus one grant.

`Game::granted_attachment_abilities` is now `Game::granted_activated_abilities` and returns both
kinds, attachment grants first — the rename is the whole "one accessor" ask, and keeping the
attachment block ahead of the filter block means an Aura's granted index doesn't shift when a lord
walks in. `ability_at`, `query.rs`'s activatable-action enumeration, and the activation gate all
went along for free, since each addresses grants through that single accessor.

Two things the printed text pins that the filter gets right by default: "other **Zombies**", not
"Zombies you control", so `filter.controller` stays `Any` and an opponent's Zombie is granted the
ability too; and `filter.other` reads against the *granting* permanent, which is what stops the
Master regenerating itself. A granted `target = "this"` names the host, not the lord — the
activation's source is the permanent whose index was addressed.

### 67. `spells-and-abilities-cost-more` — 1 card, M — **done**
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
*Landed:* two new statics, `tax_spell_cost` and `tax_activated_ability`, not the sketch's signed
delta. The sketch assumed the only difference between a tax and a discount was the sign; it isn't.
`Game::cost_reduction` skips every permanent whose owner isn't the caster — a reducer discounts
*your* spells — while Gloom taxes everybody's white spells, so a negative amount threaded through
that same scan would have been silently seat-scoped, and deriving "who does this reach" from the
sign of a number is the kind of cleverness nobody wants to decode at 3am. The two taxes get their
own table-wide scans instead (`Game::cost_increase`, `Game::activation_tax`), and the cast choke
now reads increase-then-reduction in CR 601.2f order — folding the tax in *after* the reduction
would have let a reducer floor the generic at 0 and eat the {3} on the way past.
The activation half turned out cheaper than "M" priced it. `Game::ability_activation_gate` is
already the single funnel every read of an activation cost goes through — the activation itself,
the three priority scans, the playability previews — so taxing `cost.mana.generic` at its
`Timing::Activated` destructure covers all of them at once, with no second place left quoting a
printed cost back untaxed. `SpellFilter::Color` and `PermanentFilter::color` both already existed
(Balefire Liege and Razorjaw Oni got there first), so neither filter needed a new arm; the whole
card is `{ color = "white" }` and `{ types = "enchantment", color = "white" }`.

### 68. `prevention-shield-top-up` — 1 card, M — **done**
Depends on: #4.
Split out of #4. Guardian Angel's first sentence is plain #4 work; its second is not: "Until end
of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage
that would be dealt to that permanent or player this turn." That is a repeatable, optional,
priority-timed payment offered by a spell that has already left the stack — there is no permanent
to hang an activated ability on, and the engine's only until-end-of-turn *offers* are triggered
(`ScheduleAtNextUpkeep`) or cost riders on a cast, never a standing "you may pay, whenever you
have priority."
*Sketch:* a turn-scoped `Game` list of standing optional payments, each remembering its target and
its effect, surfaced as a legal action wherever the priority path already enumerates activatable
abilities so the existing pay-and-resolve plumbing covers the rest; paying pushes another entry
onto `damage_prevention_shields` for the same target. Worth checking whether the same list can
express other "any time you could cast an instant, you may pay" riders before shaping it around
this one card.
*Cards:* guardian_angel.

*Landed (68 — guardian_angel):* the sketch held, and the answer to "is there anywhere to hang a
standing offer?" was the legal-action list. `Game::standing_preventions` is a turn-scoped
`Vec<StandingPrevention>` beside `damage_prevention_shields` — runtime orchestration state, cleared
at the same next-untap boundary, so "until end of turn" needs no separate expiry. The new
`MiscEffect::OfferPreventionTopUp { cost, amount }` records one on resolution and prevents nothing;
`Game::meaningful_actions` enumerates each affordable offer as a
`MeaningfulAction::PayStandingPrevention { index }` after its per-object loop (the one action that
names no object — the spell that made the offer has long left the stack), and
`Game::pay_standing_prevention` is shaped like `Game::turn_face_up`: priority check, `settle_payment`
with auto-tap, push an ordinary `PreventionShield`, keep priority. So "any time you could cast an
instant", affordability, auto-tap and the client's action bar all come free, and because the offer
is not consumed by being taken it can be bought as often as its controller can pay.

"That permanent or player" is the enclosing effects array's shared target: the two sentences are one
spell ability with two `[[abilities.effects]]` steps, and a `Sequence` runs its steps against one
`ctx`, so the offer reads the target the `prevent_next_damage` step in front of it already chose.
That is why the mode takes no `target` of its own, and the cards-crate shape test pins the two
steps together for exactly that reason. `ActionView.kind` is a plain `String`, so the wire needed
no `.proto` change — only the new `"pay_standing_prevention"` arm and its label.

*ponytail:* the offer stands even after its target has left the battlefield — paying then arms a
shield on a dead object that can never fire, so the {1} is simply wasted. Real play would never
buy it, and gating enumeration on a live target would mean re-resolving `Target::Object` liveness
on every action refresh for the one card in the pool that prints this; add the check when a second
card makes it matter.

### 69. `prevent-all-but-n` — 1 card, M — **done**
Depends on: #4, #5.
Split out of #4. Forcefield: "{1}: The next time an unblocked creature of your choice would deal
combat damage to you this turn, prevent all but 1 of that damage." The shield from #4 subtracts a
fixed count; this one is the complement — it prevents *everything except* a fixed count, so its
arithmetic at the choke is `amount.min(1)` surviving rather than `amount - points`. It also needs
#5's source-keyed shield (the chosen creature) plus an unblocked-attacker restriction #5 doesn't
have.
*Sketch:* let the shield entry carry its arithmetic — `Consume { points }` vs `AllBut { keep }` —
rather than adding a parallel list, and key it to the source with whatever #5 lands. The
"unblocked creature" filter is readable from combat state at the choke.
*Cards:* forcefield.

*Landed:* three fields on `PreventNextDamage` and three on `PreventionShield`, no new effect and no
parallel shield list. The sketch's `Consume`/`AllBut` enum was more than the arithmetic needed: one
`keep: Option<i32>` alongside the existing `amount: Option<i32>` covers all three readings at the
choke — `Some(keep)` subtracts from the other end (`amount - prevented - keep`), `None`/`None` eats
the whole hit, `None`/`Some(points)` eats what it has left. A keep-shield is then spent outright by
the hit it stood in front of, exactly as an `amount`-less shield already was, so the spend side in
`apply` needed one guard rather than a branch per arithmetic.

#5's source-keyed shield never landed, so `from_source: Option<ObjectId>` landed here instead — a
named source *replaces* the colour gate rather than joining it, since a card names either a class of
sources or one of them, never both. `combat_only` reads the step (CR 510.2 — a combat damage step
deals nothing but combat damage) rather than threading a flag through the damage events.

`target_is_source` is the one piece with no precedent: every other prevention card targets what it
protects, Forcefield targets what it stops. Rather than a second target slot, the flag swaps the
meaning of the existing one — the chosen creature becomes `from_source` and the shield goes up in
front of the ability's controller.

The `unblocked` filter axis is the mirror of `blocking`, read off the same declared-blocks list from
the other end. Targets aren't re-validated at activation in this engine (the established CR 608.2b
posture), so naming a creature that gets blocked fizzles the ability at resolution rather than
rejecting the activation; the regression test pins that with a trampler.

### 70. `damage-replaced-by-counter-removal` — 1 card, M — **done**
Depends on: #4.
Split out of #4. Rock Hydra's third ability ("{R}: Prevent the next 1 damage that would be dealt
to this creature this turn") is plain #4 work with a self target. Its second is a *replacement*
that consumes a resource on the permanent instead of a shield: "For each 1 damage that would be
dealt to this creature, if it has a +1/+1 counter on it, remove a +1/+1 counter from it and
prevent that 1 damage." Per-point, conditional on the counter still being there, so it can eat
part of a hit and let the rest through. `prevent_damage_to_self_removing_counter` (Phantom
Centaur) is the nearest existing shape, but that one removes a counter per *event*, not per point.
*Sketch:* generalise that static to a per-point loop bounded by the counters present, evaluated at
the same choke #4 spends shields at, with the counter removals riding the same event that records
the prevention. Rock Hydra's `enters with X +1/+1 counters` is already expressible.
*Cards:* rock_hydra.

*Landed:* a third counter-removal shield rather than a generalisation of the first two.
`StaticEffect::PreventDamageToSelfRemovingCounterPerPoint` registers the same
`ReplacementEffect::PreventDamageToSelfRemovingCounter` its siblings do, flagged `per_point`. The
flag's whole job is to keep it *out* of `phantom_shield_active` — that predicate means "prevents a
whole damage event," which is exactly what the Hydra doesn't do, and six damage chokes early-return
on it. Instead `Game::per_point_counter_shield` is spent inside `creature_damage_events_inner`,
ahead of the ordinary `prevent_next_damage` shields: it removes `min(damage, counters)` counters and
hands the remainder on to be dealt. All five chokes that route through that shared path get it for
free, and the leftover needed no new plumbing because the choke already returns `(events, dealt)`.

"Activate only during your upkeep" is `Condition::DuringYourUpkeep` on the existing
`[abilities.condition]` activation-restriction surface — the narrower sibling of `during_your_turn`
rather than a step axis crossed with it, since nothing else in the pool restricts an activation to
one step. The `{R}` shield is #6c's `shield_source` unchanged, and `enters with X +1/+1 counters`
was already expressible.

ponytail: the Radiance sweep in `resolution/damage.rs` hand-rolls its `DamageMarked` and so skips
the per-point shield — but it already skips every ordinary prevention shield too, so this is that
pre-existing gap, not a new one. The upgrade path is routing Radiance through
`creature_damage_events` like every other sweep.

### 71. `pay-any-amount-of-mana` — 1 card, M — **done**
Depends on: #4.
Split out of #4. Power Leak: "that player may pay any amount of mana. This Aura deals 2 damage to
that player. Prevent X of that damage, where X is the amount of mana that player paid this way."
The engine's optional payments are all for a *fixed* amount (the pay-or-suffer triggers landed in
this grind, `unless_pays`); an unbounded "any amount" needs the paying player to name a number and
the trigger to carry it forward as an `Amount` into a prevention sized by it.
*Sketch:* a `PendingChoice::PayAnyAmount` answered with a count, funded through the existing mana
payment path, whose answer is bound as the resolving ability's `x` — at which point the prevention
half is a #4 shield of `Amount::X` armed on the same player immediately before the damage.
*Cards:* power_leak.

*Landed:* the sketch priced a new `PendingChoice::PayAnyAmount` from scratch, and there was already
one on the shelf. Collective Voyage's join forces — "Starting with you, each player may pay any
amount of mana" — is exactly this pause, generic-funded and answered by `Intent::PayOptionalCostX`;
the only difference is the guest list. So `triggering_player_may_pay_any_amount_to_prevent` raises
`PendingChoice::JoinForcesPayment` with a one-seat `remaining`, and the whole answer handler,
payment settlement and Sequence-resumption came for free. The trigger half was free too: the
sketch's "upkeep of the enchanted permanent's controller" is `each_upkeep` plus
`condition = { type = "enchanted_permanents_controllers_upkeep" }`, the frame Cursed Land, Feedback,
Wanderlust and Warp Artifact already share, and the damage half is their `to_triggering_player` with
a 2 on it.

Two things the sketch got wrong. It bound the payment to the resolving ability's `x` and then armed
an `Amount::X` shield in a following step — but `PreventNextDamage` puts its shield on the
*ability's* controller (or its target), and Power Leak's Aura is not controlled by the player being
shot. Arming it in the answer handler instead, where the payer is already in hand, skips the whole
question. And "prevent X of *that* damage" is capped: an unbounded shield sized by the payment would
let someone pay 4 into a 2-damage trigger and walk away with 2 points of prevention banked against a
Lightning Bolt later that turn, since shields don't expire until the next untap step. `prevent_up_to`
carries the following step's damage down into the pause so the shield is `min(paid, cap)`.

The pending choice is still named `JoinForcesPayment`, which Power Leak is not. Renaming it costs
more than the confusion is worth today; if a third pay-any-amount card lands that is neither, that
is the moment to call it `PayAnyAmount` and let join forces be one caller of it.

### 72. `mandatory-sacrifice-or-inability-penalty` — 1 card, S — **done**
Depends on: nothing.
Split out of #20. Lord of the Pit: "At the beginning of your upkeep, sacrifice a creature other
than this creature. If you can't, this creature deals 7 damage to you." Not a pay-or-else — nothing
is optional, and the fallback fires on *inability*, not on a declined payment.
*Landed:* no new edict scope was needed. `ChoiceEffect::SacrificeOwn { filter, count }` already
means "the controller sacrifices `count` of their own matching permanents", and the existing
`Conditional { condition, then, otherwise }` supplies the "if you can't" branch. The one engine
change was a bug fix: `Game::edict_options` passed `None` as the source to `permanent_matches`, so
`filter.other` was a silent no-op on *every* edict path. It now takes `source: Option<ObjectId>`,
threaded from all eight call sites (`None` only where it is meaningless — devour hand-rolls its own
exclusion, and `CasterKeepPermanents` filters nothing). Without it the Lord counted as its own food
and the mandatory sacrifice stalled on an unanswerable pause. "You control another creature" is
spelled `you_control_at_least_creatures { count = 2 }`, exact while the Lord is on the battlefield
(the only time the trigger resolves) and recorded as a `ponytail:` comment on the card.
*Cards:* lord_of_the_pit.

### 73. `destroyed-this-way-count-from-targets` — 1 card, M — **done**
Depends on: nothing.
Split out of #1, which it never belonged to. Volcanic Eruption: "Destroy X target Mountains. This
spell deals damage equal to the number of Mountains destroyed this way to each creature and each
player." The target side is already expressible — `{ min = 1, max = 1, x_scaled = true }` is
"exactly X targets" (Curse of the Swine) — and both damage halves are plain `each_creature` /
`each_player`. What is missing is the count: `Amount::PermanentsDestroyedThisWay` reads
`ResolutionFrame::destroyed_this_way`, which only `DestroyEffect::DestroyAll` ever writes, so a
*targeted* destroy leaves it at zero. *Sketch:* have the targeted destroy path record its
successful destructions into the same resolution frame, so "destroyed this way" means the same
thing after either destroy flavor. Resist approximating it as `Amount::X`: a Mountain that left in
response, gained shroud, or regenerated (CR 701.15 — regeneration replaces the destruction, so it
was not destroyed) makes the two diverge, and this card's whole point is the symmetry between what
it destroys and what it burns.
*Cards:* volcanic_eruption.

*Landed:* the sketch held, and cost less than it reads. `resolve_destroy_all` already minted →
snapshotted → applied; the targeted path was the same three steps minus the snapshot, so the
snapshot loop moved into `Game::record_destroyed_this_way` and a `Game::resolve_destroy_target`
twin calls it. The one real difference is *accumulating* rather than clearing: the multi-target
expansion in `resolve_spell` runs one whole-ability step per chosen target, so X destroys are X
separate `run` calls and only the resolution boundary knows where the count starts over —
`resolve_top` clears the list once per stack item, next to where `resolve_spell` already clears
`returned_nonland_card_mana_value` for the same reason.

Reading the count off the *minted events* rather than the effect's targets is what makes "put into
a graveyard this way" honest for free: `mint_destroy`'s Target arm returns nothing for an
indestructible permanent and a `Regenerated` event for a shielded one, so neither reaches the
`MovedToGraveyard` arm the recorder matches on. No regeneration wiring was needed for that; it
falls out of where the recording sits.

The card is two `[[abilities]]`, not one Sequence, for Pest Infestation's reason — a shared
sequence would repeat the damage clause once per Mountain. Its `permanents_destroyed_this_way`
filter exposed a real gap: `destroyed_this_way_matches` read `CardDef::subtypes`, which never holds
a *land's* types (those live on `CardKind::Land`, CR 305.6), so a `subtypes = ["Mountain"]` filter
matched nothing. `Game::effective_subtypes` had already unioned the two halves inline; that union
became `CardDef::printed_subtypes` and both readers now share it.

### 77. `state-triggers` — 2 cards, M
Depends on: nothing.
Split out of #24. Pirate Ship and Sea Serpent's "When you control no Islands, sacrifice this
creature" is a state trigger (CR 603.8): it fires the moment the condition becomes true, checked
whenever the game checks state-based actions, and does not fire again until the condition has been
false in between. Both cards ship an `each_end_step` approximation with the same intervening-if, so
a ship whose last Island left survives until that turn's end step. *Sketch:* a `timing =
"state"` ability whose `[abilities.condition]` is scanned in the same pipeline phase that already
places triggers after the state-based sweep (`triggers.rs`'s placement runs one phase behind
`check_state_based_actions`, which is exactly CR 603.8's timing), plus a per-(object, ability)
"already fired for this condition" latch cleared when the condition next reads false — without the
latch a sacrifice that gets replaced or countered re-fires on every sweep. Resist reaching for it
until a card needs the *immediacy*: the two ships only differ from their approximation for the span
of one turn, and no 2ed card punishes that window.
*Cards:* pirate_ship, sea_serpent.

### 74. `defining-count-that-switches-on-attacking` — 1 card, M — **done**
Depends on: #2 (done).
Split out of #2. Gaea's Liege: "As long as Gaea's Liege isn't attacking, its power and toughness
are each equal to the number of Forests you control. As long as Gaea's Liege is attacking, its
power and toughness are each equal to the number of Forests defending player controls." The
defining static from #2 is the right shape, but two things are missing. First, a *condition* on
which of two counts applies: no `Condition` in the pool asks whether the ability's own source is
attacking (`PermanentFilter.attacking` gates candidates, not the source). Second, a filter
controller of "defending player" — `FilterController` is `Any`/`You`/`Opponent`, and with three
opponents at a Commander table the defending player is a specific one, read from
`Game::defender_of` the way the goad and attack-tax paths already do. *Sketch:* give
`base_power_toughness_from_amount` an optional second `{ condition, power, toughness }` arm rather
than bolting a ternary onto `Amount`, and add `FilterController::DefendingPlayer` resolved against
the source's declared defender. Note the card is also blocked on #8 for its second ability
("{T}: Target land becomes a Forest until this creature leaves the battlefield"), which is a
land-subtype change with an unusual duration. (That blocker is resolved — see 8e.)
*Cards:* gaeas_liege.

*Landed:* not a `Condition` and not a second arm. `BasePowerToughnessFromAmount` gained a
`when: DefiningPtWhen` (`Always` by default, `Attacking`, `NotAttacking`) that `defined_base_pt`
checks against `combat.attackers` before its existing `find_map` accepts the ability. So the card is
authored as two `[[abilities]]` mirroring its two printed sentences, exactly one of which answers at
any moment — no ternary `Amount`, and every other defining creature in the pool is untouched by the
`#[serde(default)]`.

`FilterController::DefendingPlayer` is the sketched one, resolved in `permanent_matches` through
`defender_of(source)` then `defender_controller`. It reads the filter's *own source*, so it matches
nothing while that source isn't attacking — the right answer, since every card printing the clause
prints a second arm for the other case, and `Opponent` would have reached three boards at a
four-player table when the card names one.

The cache bit is worth remembering: a land changing type is board-wide news. Invalidating only the
changed land left the Liege's own cached toughness stale (`left: (2, 1)`), because its P/T reads a
count of Forests. `Event::SubtypesSetWhileSourceRemains` calls `invalidate_all_battlefield`.

### 75. `half-of-a-count-amounts` — 1 card, S — **done**
Depends on: nothing.
Split out of #2, where it never belonged — this is an Aura pump, not a defining ability. Aspect of
Wolf: "Enchanted creature gets +X/+Y, where X is half the number of Forests you control, rounded
down, and Y is half the number of Forests you control, rounded up." `grant_to_attached` already
takes a live `Amount` for each of `power` and `toughness` (Sage's Reverie), so the whole card is
one existing static — what is missing is halving. The pool has `Amount::HalfX` /
`Amount::HalfXRoundedDown`, but those are scoped to a spell's chosen `{X}`, not to an arbitrary
count. *Sketch:* an `Amount::Half { of: Box<Amount>, round_up: bool }` wrapper, which subsumes the
two X-scoped variants; check whether folding them in is smaller than leaving them alone before
doing it.
*Cards:* aspect_of_wolf.

*Landed:* `Amount::Half { of, round_up }`, with `of` leaked to `&'static` the way `Amount::Scaled`
already leaks its `by` — nesting an amount inside an amount is a solved problem here, so this was
one variant, one resolver arm, one authoring key. Folding `HalfX`/`HalfXRoundedDown` in was not
smaller: `fill_cast_x` matches on those two variants by name to rewrite a `Trigger::YouCastThis`
ability's amount to a `Fixed` at placement, so subsuming them would mean teaching that rewrite to
walk inside a wrapper for no card's benefit. They stay.

The authoring key is answered ahead of the amount table's exactly-one-of match rather than joining
it, since a twelfth slot in that tuple buys nothing when the key is unambiguous on its own.

### 76. `defining-power-toughness-on-an-enchanted-host` — 1 card, M — **done**
Depends on: #2 (done).
Split out of #2. Animate Artifact: "As long as enchanted artifact isn't a creature, it's an
artifact creature with power and toughness each equal to its mana value." Three separate gaps.
`set_attached_base_p_t` takes fixed `i32`s, so it cannot express a count at all — it needs to
widen to `Amount`, and then the amount has to be *host*-relative, which no amount currently is (a
`grant_to_attached` amount resolves against the Aura's controller, not against the enchanted
permanent). There is no "this permanent's mana value" amount, only `Amount::TargetManaValue`,
which reads a target rather than an attachment host. And the whole thing is gated on "isn't a
creature", which — unlike #74's attacking switch — is a condition on the *host*, so the type-set
half (`set_attached_types` adding creature) has to be gated too or the Aura would make an already-
animated artifact stop being what it was.
*Cards:* animate_artifact.

*Landed:* two of the three gaps closed as sketched; the third turned out not to be one.
`SetAttachedBasePt`'s `power`/`toughness` widened from `i32` to `Amount`, and
`Amount::SourceManaValue` joined `SourcePower`/`SourceToughness` — one arm reading
`def_of(source).mana_value()`. The host-relative part is where the Aura's amounts are *resolved*,
not a new axis on the amount: `attachment_continuous_effects` now calls `resolve_amount(.., host,
..)` instead of pushing the literals through, so "its mana value" is the enchanted artifact's. Every
other attachment amount keeps resolving against the Aura, because `grant_to_attached`'s "+1/+1 for
each Aura you control" really is the Aura's own count.

The type half needed no gate after all. `set_attached_types`'s `add_types` is a *union*, so adding
`creature` to an artifact that already is one changes nothing — the sketch's worry only applies to
`set_types = true` (Darksteel Mutation's replace), which Animate Artifact doesn't use. That leaves
one gate, `noncreature_only` on the base-P/T set, in the shape `GrantToAttached::legendary_only`
already established for a host-property condition. Without it, enchanting Obsianus Golem would
flatten its printed 4/6 into a 6/6.

The gate reads the host's *printed* types, with a `ponytail:` on `host_is_printed_creature` naming
the ceiling. It has to ignore this very Aura's own creature-adding layer, and asking
`effective_types` from inside the attachment scan that feeds those layers would recurse. Printed
types answer the pool's case — enchanting a printed artifact creature — and miss an artifact
animated by something else; the upgrade path is an `effective_types` variant taking a source to
exclude.

## Client catch-up

The grind ran engine-first, and three increments landed server behavior the board could not show or
could not honor. This pass closed all three. Nothing here is a card change; the card pool is
unchanged from #48.

**Type-changing Auras read their own flags.** `effect.static_set_attached_types` sent only
`set_subtypes` / `add_subtypes` while `en.ts` asked for a `subtypes` param that never existed, so
every one of the five Auras using the effect rendered the same wrong sentence. The fix is on the
Rust side first: the message now carries all six of the effect's fields, including a
`card_type_words` rendering of `add_types` — `type_set_token`'s conjunctive twin, because a granted
type line reads "artifact creature" where a filter reads "artifact or creature". The formatter
composes from those flags, which is what makes Evil Presence ("is a swamp") distinguishable from
Angelic Destiny ("is an angel in addition to its other types") and Darksteel Mutation ("is an insect
artifact creature and has no abilities") — one string never could have been. An empty
`string_list_param` arrives as the literal `"none"`, absorbed in the formatter rather than by
widening the Rust param helpers.

**Winter Orb's untap caps reach the board.** `PendingChoice::DeclineUntap` carried its
`at_most_one` groups only inside the engine. They now ride the DTO, the projection, the proto
(reusing the existing `ObjectIdList`, not a new message), the gRPC map, and `types.ts`, and
`cardPickReady` — the single choke point that enables the prompt's submit button — holds it shut
while any group would leave two members untapped. A cap is a ceiling, not a quota, so keeping a
whole group tapped stays legal, and an empty `at_most_one` is still the plain Rubinia-style free
yes/no. The prompt gained a hint so a greyed-out button explains itself instead of an answer
bouncing off the server.

**Looked-at hands leave something behind.** Glasses of Urza itemizes an opponent's hand to the
looker alone, but every other read of an opponent's hand on this board is `hand_count`, so a look
left nothing but a log line. `seen-hands.ts` counts hand objects the snapshot itemized for seats
other than the viewer and renders a chip per seat, opening that hand in the existing pile overlay
via the already-generic `PileExpanded { zone, owner }`. No new overlay, no new wire field — the
cards were already arriving. The server-side gate is untouched: `has_seen_hand_card` still decides
per card what the looker gets, so a seat that never looked still sees no chip.

Both affected surfaces were updated in the same change
([`game-board`](../../openspec/specs/game-board/spec.md)).
