# Political Puppets deck increments (2026-07-18)

Deck report: [political-puppets.md](political-puppets.md). Engine-wide tiers and per-card
exotics stay in [../FIDELITY_BACKLOG.md](../FIDELITY_BACKLOG.md); increment numbering is
global across files (continues after #202 in
[deathdancer-xira-increments.md](deathdancer-xira-increments.md)).

From `docs/fidelity/political-puppets.md` (Archidekt 2209176 — commander Zedruu the
Greathearted). 42 of the deck's 60 new cards need engine work; ranked S-first within
dependency order. The observability re-audit for this deck falsified fourteen pool-absence
claims plus three minor fired-upgrade notes and one unflagged engine bug — each is folded
into the increment that clears it (#203, #207, #212, #213, #215, #218, #219, #223, #228,
#230, #231). The control-donation/exchange group (#228) is the deck's centerpiece and is
staged as an XL with explicit slices, one per wave.

### 203. `vow-attack-restriction` — 3 cards, S
_Landed 2026-07-18: `Effect::GrantToAttached.cant_attack_controller` axis + `Game::host_cant_attack_controller`,
read in `declare_attackers` beside the landed `vow_protected` counter check. Re-audit fold-in landed:
`Game::is_modified` now takes `you: PlayerId` and scopes the Aura half of "modified" to Auras you
control (CR 701.29 — Silkguard regression), fixing all four call sites. vow_of_duty, vow_of_flight,
vow_of_lightning authored. Still blocked: all three keep a trimmed `approximates` for "or planeswalkers
you control" — unobservable, no planeswalker permanent exists in the pool._
Depends on: #80 part B `vow_protected` declare-attackers check (landed); grant_to_attached
(landed).
The Vows ban attacking only the Aura's controller. `grant_to_attached.cant_attack` is a
blanket ban, and Eriette's `cant_be_attacked_by { enchanted_by_you }` would be overbroad —
it covers creatures enchanted by *any* of your Auras (wrong beside an Impetus). *Sketch:* a
`cant_attack_controller = true` axis on `grant_to_attached`, read in `declare_attackers`
beside the landed `vow_protected` check — same restriction, sourced from attachment instead
of a vow counter. The +2/+2 and keyword halves are already expressible. Re-audit fold-in:
`query.rs:1299`'s "modified = any attached Aura" is falsified by a Vow you attach to an
opponent's creature — CR 701.60a counts only Auras *you* control, so scope the
`is_modified` Aura scan to the Aura's controller (Silkguard regression). *Cards:*
vow_of_duty, vow_of_flight, vow_of_lightning.

### 204. `conditional-grant-to-attached` — 1 card, S
_Landed 2026-07-19: `Effect::GrantToAttached.legendary_only` (a narrow bool, default `false`) gating
only the `keywords` slice on the host's own `CardDef::legendary`, read live at keyword recompute
(`characteristics.rs` `attachment_grants` + `modifier_sources`). Did not reuse `Condition`/
`condition_holds` — that machinery evaluates controller-scoped facts off `TriggerContext`, not an
arbitrary object's own characteristics; a `ponytail:` note (in `effect.rs` and `DSL_REFERENCE.md`)
points at growing a real host-scoped `Condition` if a second host-based gate shows up. champions_helm
authored, fully faithful (no `approximates`; corrected oracle is +2/+2 with conditional plain
hexproof, not the ticket's guessed "+1/+1 and hexproof"). Still blocked: nothing._
Depends on: grant_to_attached (landed); `anthem_static.condition` (landed pattern).
"As long as the equipped creature is legendary, it has hexproof" — `grant_to_attached` has
no condition axis. *Sketch:* an optional `condition` on the grant (or a narrow
`host_legendary = true` gate), evaluated live at keyword recompute exactly like
`anthem_static.condition` already is. *Cards:* champions_helm.

### 205. `permanent-filter-axes-2` — 2 cards, S
_Landed 2026-07-18: `PermanentFilter::mv_min` (mana-value floor, sibling of `mv_max`) and
`attacking_you` (attacker's declared defender must equal the filter's controller, via
`Game::defender_of`) added across `filter.rs`/`de.rs`/`query.rs`. austere_command and
soul_snare authored. Still blocked: soul_snare keeps a trimmed `approximates` for "or a
planeswalker you control" — unobservable, no planeswalker permanent exists in the pool._
Depends on: #70 filter-axis pattern (landed).
Two missing axes. `mv_min` for Austere Command's "mana value 4 or greater" modes (`mv_max`
and modal `choose = 2` are landed — add the sibling). `attacking_you` for Soul Snare: the
attacker's declared defender must be the filter's controller — the per-attacker defender is
already tracked for goad/tax; plain `attacking = true` would wrongly snipe creatures
attacking someone else. The "or a planeswalker you control" half stays unobservable (the
pool fields no planeswalker permanents — re-audit re-checked, not falsified). *Cards:*
austere_command, soul_snare.

### 206. `strife-counter-blocking-anthem` — 1 card, S
_Landed 2026-07-19: `CounterKind::Strife` arm (+ its `label.rs`/`ALL`/`COUNT` updates) and
`Effect::AnthemStatic.blocking_only: bool`, the sibling of `attacking_only`, checked against
`CombatState::blocks` in `characteristics.rs`'s `matching_anthems`. Re-audit correction: the
increment's own sketch guessed the blocking clause was `all_players`-scoped like the
attacking clause — verified oracle text (Scryfall, cmd/vma) says "**Blocking creatures you
control** get +1/+0", asymmetric with the unrestricted "Attacking creatures get +1/+0" — so
only the attacking `anthem_static` sets `all_players = true`; the blocking one uses the
default owner-scoped gate. crescendo_of_war authored, fully faithful, no `approximates`._
Depends on: #75 counter-kinds (landed); anthem per-counter amounts (landed).
`each_upkeep` + `put_counters { target = "this" }` and `anthem_static { all_players,
attacking_only, power = { per_counter_of_kind } }` all exist, but the counter-kind enum is
closed. *Sketch:* a `CounterKind::Strife` arm (the `effect.rs:3903` documented growth path)
plus a `blocking_only` anthem axis for "blocking creatures you control get +1/+0", sibling
of the landed `attacking_only`. *Cards:* crescendo_of_war.

### 207. `random-opponent-must-attack` — 1 card, S
_Landed 2026-07-19: `Effect::MustAttackRandomOpponent` — a fieldless begin-combat effect arm
that picks a living opponent of the controller uniformly via the injected RNG (`Game::next_u64`)
and pushes an `Event::MustAttackDeclared { object: source, defender }`, reusing the existing
`must_attack` requirement `declare_attackers` already enforces for Furygale Flocking's tokens.
Wired through `effects.rs` (`Game::run`, ahead of the pure-mint catch-all — needs `&mut self` for
the RNG draw, so also added to `resolution/mint.rs`'s "needs `&mut self`" unreachable group) and
`label.rs`; no schema/Event change needed (`MustAttackDeclared` already existed). Re-audit
fold-in: `characteristics.rs`'s `commander_identity_credit` now builds a `Mana::OfColors` mask for
a 3+-color identity instead of falling back to `Mana::Any` — the stale "no soc-pool commander is
3+ colors" comment is gone now that Ruhan (WUR) is in the pool. ruhan_of_the_fomori authored,
fully faithful, no `approximates` — Scryfall-verified oracle/mana-cost/type-line/P-T (`{1}{U}{R}{W}`,
Legendary Creature — Giant Warrior, 7/7; the increment's own sketch guessed "Wizard Warrior" /
`{2}{W}{U}{R}`, both wrong)._
Depends on: #80 part A `combat_extras.must_attack` (landed); #116 injected RNG (landed).
*Sketch:* a begin-combat effect arm that picks a living opponent at random (injected PRNG)
and pushes `(source, that player)` into the existing must-attack requirement — only the
composing arm is missing. Re-audit fold-in: `characteristics.rs:393`'s "no soc-pool
commander is 3+ colors" upgrade condition has fired (Ruhan, Zedruu, and Numot are all WUR
and Command Tower is in the deck) — switch the 3–4-color commander-identity credit from the
`Mana::Any` fallback to the existing `Mana::OfColors` (one line). *Cards:*
ruhan_of_the_fomori.

### 208. `opponent-lifegain-graveyard-watch` — 1 card, S
_Landed 2026-07-19: `Trigger::OpponentGainsLife` — the opponent-scoped twin of `YouGainLife`, fired for
every living player other than the gainer off the same `Event::LifeChanged` choke, battlefield and
graveyard-functional (`queue_graveyard_controller_triggers`) alike. punishing_fire authored, fully
faithful (no `approximates`). Still blocked: nothing._
Depends on: #121/#153 `functions_in_graveyard` + optional pay-cost trigger (landed).
*Sketch:* a "whenever an opponent gains life" trigger arm — the opponent-scoped twin of
`you_gain_life` — scanned from the graveyard. The optional pay-`{R}` trigger cost and
`return_this_to_hand` are landed. *Cards:* punishing_fire.

### 209. `combat-damage-amount-on-self-damage-trigger` — 1 card, S
_Landed 2026-07-19: the `who = "this"` placement (`Game::queue_combat_damage_triggers`) already
threaded `ctx.combat_damage` into `contextualize_effect`/`fill_combat_damage` — Venerable
Warsinger's reanimation-bound filter already read it. The actual gap was narrower than the
sketch: `fill_combat_damage`'s `CreateToken` match arm only rewrote `enters_with` (Primo, the
Unbounded's counter count), not `count` (the number of tokens minted). Replaced that bespoke arm
with the generic `map_effect_amounts` walker (already used by every sibling `fill_*` helper),
which rewrites `Amount::CombatDamageDealt` wherever it appears in a `CreateToken` — `count` and
`enters_with` alike — while keeping the `ReanimateToBattlefield` card-filter special case and
`Sequence` recursion. rapacious_one authored, fully faithful (no `approximates`). Still blocked:
nothing._
Depends on: #91/#97 (landed).
`Amount::CombatDamageDealt` exists but only fills on the batch `zero_base_power…` timing.
*Sketch:* fill it on a `deals_combat_damage_to_player` (`who = "this"`) placement too, so
"create that many 0/1 Eldrazi Spawn tokens" reads the dealt damage. The token's granted
sac-for-{C} ability is a full inline profile — landed surface. *Cards:* rapacious_one.

### 210. `opponent-picks-one-of-revealed` — 1 card, S
_Landed 2026-07-19: `Effect::RevealTopOpponentPicksOneToGraveyard { count }` +
`PendingChoice::OpponentChoosesRevealedToGraveyard`, reusing the shared #107
`choose_splitting_opponent` "which opponent" chooser and `Intent::ChooseExiledWithCard`'s
mandatory single-pick wire shape. murmurs_from_beyond authored, fully faithful. Note: the real
card is `{2}{U}` instant (not `{2}{B}` sorcery as this entry's sketch assumed) — authored per
the real Scryfall printing, oracle text unchanged. Still blocked: nothing._
Depends on: #107 shared opponent-chooser (landed); `reveal_top_split_piles` (landed).
*Sketch:* reveal the top three, the chosen opponent picks exactly one → your graveyard, the
rest → your hand. One narrower effect arm reusing the chooser pause (Fact or Fiction's
reveal machinery minus the pile split). *Cards:* murmurs_from_beyond.

### 211. `token-controller-each-other-player` — 1 card, S
_Landed 2026-07-18: `TokenController::EachOtherPlayer` (same `TargetSpec::Player` resolution as
`TargetPlayer`, but the chosen target is the one player excluded; every other living player,
caster included, gets the token). death_by_dragons authored, fully faithful. Still blocked: nothing._
Depends on: #64 token controllers (landed).
`create_token.controller` has `each_opponent`/`one_per_opponent`/`target_player` but
nothing for "each player *other than* target player" (includes the caster, excludes the
target). One new controller enum arm. *Cards:* death_by_dragons.

### 212. `noncombat-lifelink-to-players` — 1 card + engine bug, S
_Landed 2026-07-18: all three player-damage arms in `mint_damage_family` (`resolution/damage.rs`)
route through a new pure-mint `lifelink_gain` helper (the `&self` twin of `Game::gain_lifelink`), so a
lifelink source's noncombat damage to a player now gains life (CR 702.15). Secondary fix: added an
`Effect::DealDamage` arm to `contextualize_sacrifice_effect` so `Amount::SacrificedCreaturePower`
fills for a sac-power fling. brion_stoutarm authored, fully faithful (no `approximates`). Still
blocked: nothing (the `DealDamage → Target::Object` noncombat-damage-to-a-creature lifelink arm stays
unrouted — no pool card exercises it)._
Depends on: nothing.
Engine bug surfaced by the re-audit (unflagged — no `ponytail:` claims it):
`resolution/damage.rs`'s `DealDamage → Target::Player` arm emits `LifeChanged` without ever
calling `gain_lifelink`, so a lifelink source's noncombat damage to a player gains no life —
the combat and creature-damage paths already route through `gain_lifelink`
(`combat.rs:707`/`794`/`874`). *Sketch:* route the player arm through it too; regression
test mandated (Brion flings, controller gains the sacrificed creature's power as damage AND
as lifelink). Brion is then pure authoring: `lifelink` + `{R}, {T}, sacrifice another
creature` + `deal_damage { amount = "sacrificed_creature_power", target =
"player_or_planeswalker" }` (Miren/Dina sacrificed-power precedent). *Cards:*
brion_stoutarm (demoted from C for this bug).

### 213. `countered-copies-cease-to-exist` — 1 card, S
_Landed 2026-07-18: `Game::is_copy_object` predicate + a `counter_spell` copy guard
(effects.rs) — a countered copy now emits `Event::SpellCeasedToExist` instead of
`MovedToGraveyard`, checked first (ahead of flashback/escape exile and Quintorius's tuck).
flusterstorm authored, faithful (no `approximates`). The Hinder `countered_dest` rider arm
(effects.rs) is untouched — still routes a countered copy to a graveyard-destination choice
it should never reach; #214 is the one pool card that exercises it and reuses this guard
there._
Depends on: nothing.
Re-audit: `effects.rs:596`'s "no pool card counters a copy, so the distinction never
surfaces" dies with this deck — Flusterstorm both mints storm copies and counters spells,
so copy-meets-counterspell is routine. A countered still-on-stack copy must cease to exist
(CR 707.10a), not land in a graveyard where Izzet Chronarch can retrieve it (or a library
where #214's tuck would make it drawable). *Sketch:* a small copy guard in `counter_spell`
(and expose it for #214's counter-to-library path); regression — counter a storm copy,
assert no graveyard object exists. Flusterstorm itself is then pure authoring:
`counter_target_spell { filter = "instant_or_sorcery", unless_pays = 1 }` + storm via the
landed `when_you_cast_this` / `copy_triggering_spell` shape (#198 Reaping the Graves
verbatim; the storm count is already game-wide, `triggers.rs:399` holds). *Cards:*
flusterstorm (demoted from C); protects izzet_chronarch.

### 214. `counter-to-library-bottom-self-tuck` — 1 card, S
_Landed 2026-07-19: `CounteredDest::LibraryBottom` (Hinder's `LibraryTopOrBottom` sibling) —
same "would it actually reach a graveyard" gate, no pause, straight to the bottom; a countered
copy still ceases to exist (CR 707.10a) by reusing #213's `Game::is_copy_object` guard directly
in this arm (the ordinary `LibraryTopOrBottom` pause path has no such check to share). A new
`Effect::TuckSelfToLibraryBottom` (unit variant) + `Game::self_tuck_to_library_bottom` scratch
bool is the self-tuck rider, mirroring `ExileSelfWithTimeCounters`/`self_exile_time_counters`
exactly: set while the spell's own effects run, consumed (`mem::take`) in
`finish_instant_sorcery_resolution` right after the buyback fork. Real-oracle correction: the
wave brief's cost ({3}{U}) and "Then put Spell Crumple…" wording were both stale — current
Scryfall Oracle (cm2/cmd, cost `{1}{U}{U}`) reads "Counter target spell. If that spell is
countered this way, put it on the bottom of its owner's library instead of into that player's
graveyard. Put Spell Crumple on the bottom of its owner's library." (no "Then", no top choice
either way). spell_crumple authored, fully faithful (no `approximates`). Still blocked: nothing.
Depends on: #160 Hinder `countered_dest` (landed); #213 (copies vanish instead of tucking).
Hinder's `countered_dest = "library_top_or_bottom"` is a *choice*; Spell Crumple forces
bottom and additionally puts *itself* on the bottom of its owner's library instead of its
graveyard. *Sketch:* a forced `"library_bottom"` dest + a self-tuck-on-resolution rider
(sibling of buyback's return-to-hand fork in `finish_instant_sorcery_resolution`); both
paths reuse #213's copy guard. *Cards:* spell_crumple.

### 215. `tuck-without-reveal-owner-draws` — 1 card, S
_Landed 2026-07-19: `Effect::ShuffleTargetPermanentIntoLibrary { target }` — the no-reveal tuck
sibling, sharing a new `Game::shuffle_tuck_events` helper with Chaos Warp's fused reveal path (both
mint identical tuck-then-shuffle events; only Chaos Warp still needs `&mut self` to read the
post-shuffle top card). `Effect::TargetOwnerDraws { count: Amount, controller: bool }` is the
general who-draws rider: `controller` (default `false` = owner) is the axis #223 (nin_the_pain_artist)
reuses scoped to the target's controller with an `Amount::X` count — ready to reuse as-is. Real-oracle
correction: the wave brief's oracle quote was stale — current Scryfall Oracle (and every historical
printing back to Onslaught 2002) reads "The owner of target **nonland** permanent shuffles it into
their library, then draws two cards," not "nontoken." Targeting is `{ permanent = "nonland" }`, so a
token IS a legal target (unlike the brief's assumption) — landed and tested
(`oblation_on_a_token_shuffles_nothing_but_owner_still_draws_two`). That test surfaced a real gap:
`TargetOwnerDraws` resolving *after* a preceding step vanished a token target (CR 111.7) panicked on
`Game::owner_of`'s `Object::Removed` arm. Fixed generally, not just for this pairing: `apply.rs`'s
`TokenCeasedToExist` handling now also records `(token, owner)` into a new
`ResolutionFrame::vanished_permanent_owner` scratch (id-matched, so a stale entry from an unrelated
earlier vanish is harmless); a new `Game::owner_of_shared_target(object, to_controller)` reads live
`owner_of`/`controller_of` normally and falls back to that scratch only when the object is already
gone — `TargetOwnerDraws` (and any future shared-target rider) calls this instead of `owner_of`/
`controller_of` directly. oblation authored, fully faithful (no `approximates`). Still blocked:
nothing.
Depends on: Chaos Warp machinery (landed).
Re-audit: `effect.rs:2128`'s own upgrade trigger fired — "split it only when a second card
wants just the tuck half"; Oblation is that second card. *Sketch:* split the tuck half out
of `ShuffleTargetPermanentIntoLibraryThenReveal` (or add a no-reveal tuck sibling), plus a
`target_owner_draws { count }` rider for "then draws two cards". #223 reuses this rider arm
scoped to a target's *controller* with an X amount — build the who-draws axis here. *Cards:*
oblation.

### 216. `put-from-hand-on-top` — 1 card, S/M
_Landed 2026-07-19: `Effect::PutFromHandOnTop { count }`, mirroring `Discard`'s partial-hand
card-pick pause but with an ordered answer — `PendingChoice::PutFromHandOnTop` /
`Intent::PutFromHandOnTop` (`Vec<ObjectId>`, first-named ends up on top; events apply
bottom-to-top). New `Event::PutFromHandOnTop` (hand→library-top, hidden-zone move) redacts like
`CardDrawn` — public `card`/`player`, `from`/`def` hidden from every viewer but the mover (the
existing public `Event::TuckedToLibrary` wasn't reused since every current caller tucks from a
public zone; overloading it with hand-privacy branching for one new caller would've been the
wrong seam). Wire: new `WireIntentPutFromHandOnTop` / `PendingChoiceViewPutFromHandOnTop` /
`VisibleEventPutFromHandOnTop` proto messages (fields 52/60/129). brainstorm authored, fully
faithful (no `approximates`). Still blocked: nothing._
Depends on: discard's name-your-cards pick pause (landed pattern).
No effect moves chosen hand cards to the top of the library. *Sketch:*
`put_from_hand_on_top { count }` — a pause naming `count` hand cards in order (mirror of
`discard`'s pick, destination top-of-library), sequenced after `draw_cards 3`. *Cards:*
brainstorm.

### 217. `filtered-mass-damage-and-damage-each-player` — 1 card, S/M
_Landed 2026-07-19: `PermanentFilter.without_flying` (a narrow bool, matching `nonbasic`/
`nonlegendary`/`nonlair`'s existing shape) and a `filter: Option<PermanentFilter>` axis on
`DamageEachCreature`; `Effect::DamageEachPlayer` fanned out over `living_players()`, routed
through the same `lifelink_gain` helper `DealDamage`'s `Target::Player` arm uses (regression
test asserts lifelink gains once per player hit). Bug found in the same change: `DamageEachCreature`'s
existing per-creature `Amount` substitution (creature-as-source, for `SourcePower` — Wave of
Reckoning) silently broke `Amount::IfSpellKicked` (`spell_was_kicked` on a creature id reads
`false`), always resolving the "else" branch even when kicked; fixed by resolving the
kicked/unkicked branch once against the ability's true source before the per-creature loop.
breath_of_darigaaz authored, fully faithful (no `approximates`). Still blocked: nothing._
Depends on: kicker + `{ if_kicked = 4, else = 1 }` amounts (landed).
`damage_each_creature` takes no filter (needs "without flying" — a without-keyword axis on
the permanent filter) and no effect damages each *player* (`each_opponent_loses_life` is
life loss, not damage, and skips the caster). *Sketch:* give `damage_each_creature` a
filter and add `damage_each_player` routed as real damage. *Cards:* breath_of_darigaaz.

### 218. `multi-target-abilities` — 2 cards, M
_Landed 2026-07-18: triggered `ChooseTarget` widened from `optional: bool` to `count: TargetCount`
(clamped to legal-set size); >1 chosen decomposes into N `push_ability_group` stack entries, each
re-legality-checked at its own resolution (CR 608.2b). Activated second target clause modeled as a
follow-up `ChooseActivationCostTargets` pause (new `ActivationCost.graveyard_exile_target_count`),
answered by the existing `ChooseTargets` wire — zero change to `Intent::ActivateAbility`. New
`Effect::RemoveFromCombat`/`Event::RemovedFromCombat` (proto field 126) + `ChooseActivationCostTargets`
view (proto field 59, `ChooseTarget` gains `max` field 6). numot_the_devastator authored fully
faithful; spurnmage_advocate authored. Still blocked: spurnmage keeps a `ponytail:` residual — only
"attacking" is modeled (no `blocking` filter axis yet), and the graveyard-exile cost's target spec is
hardcoded to "an opponent's graveyard, any card" (the pool's one card needs only that)._
Depends on: #73 spell-side multi-target clauses (landed).
Re-audit: `effect.rs:3213`/`filter.rs:24`'s "the pool never sequences two targeting effects
under one ability" is falsified four ways (Zedruu, Chromeshell Crab, Vedalken Plotter,
Spurnmage Advocate). Spells carry two clauses + counts; triggered abilities are
single-target (only Kinetic Ooze's hardcoded second clause widens) and
`Intent::ActivateAbility` carries one `Option<Target>`. *Sketch:* widen the triggered
`ChooseTarget` path to a real `TargetCount` (Numot: "destroy up to two target lands" on a
combat-damage trigger) and give activated abilities an independent second target clause
through intent + wire (Spurnmage: two cards in an opponent's graveyard + one attacking
creature). This is the same intent/wire surface #228's donation and exchanges consume —
build once here. *Cards:* numot_the_devastator, spurnmage_advocate (zedruu,
chromeshell_crab, vedalken_plotter consume it in #228).

### 219. `each-draw-step-intervening-if` — 1 card, M — LANDED 2026-07-19
_Landed 2026-07-19: `Trigger::EachDrawStep` (the draw-step twin of `EachUpkeep`/`EachEndStep`) plus
a new `TriggerContext::active_player` field it threads (`Game::queue_each_draw_step_triggers`,
wired off `Event::StepBegan { step: Draw, .. }`); `Effect::EachDrawStepPlayerDraws` (mirrors
`AttackingPlayerDraws`'s context-filled-drawer shape, `contextualize_effect` recursing into
`Effect::Conditional`'s `then` too, not just `Sequence`). `Condition::SourceUntapped` (source-
object-based, `ability_condition_holds` arm) covers the CR 603.4 *first* check at trigger
placement. For the *second* check: rather than plumbing a new field through `StackItem`/
`Event::TriggeredAbilityOnStack`, reused the existing `Effect::Conditional` CR 608.2h resolve-time
gate (already the generic "re-check fresh at resolution" mechanism, with `SourceEnteredWithXAtLeast`/
`ColorWasSpentToCastThis` special-cased there for the same "no `TriggerContext` id" reason) — added
one `Condition::SourceUntapped` arm reading `source`'s live tapped state. howling_mine authored with
`condition = { type = "source_untapped" }` on the ability AND the same condition wrapping the payoff
in a `conditional` step, so both checks read the identical live state at two different times.
Verified oracle (current errata): "if **this artifact** is untapped," not "if Howling Mine is
untapped." Regression test confirmed by temporarily defeating the resolution-time arm (mutation
testing) before reverting. Fully faithful, no `approximates`. Still blocked: nothing._
Depends on: each_upkeep/each_end_step trigger family (landed).
Three pieces. (a) An `each_draw_step` trigger arm carrying active-player context — the
`triggers.rs:1148`/`1167` ponytails' documented `TriggerContext` active-player field, which
"that player draws an additional card" finally reads (payoff reuses
`attacking_player_draws`' shape). (b) `Condition::SourceUntapped` for the intervening if.
(c) Re-audit: `effect.rs:4089`'s skipped CR 603.4 *second* check is falsified — the pool
can tap Howling Mine in response (Magma Opus's instant-speed "tap two target permanents"),
so trigger-fired-but-Mine-now-tapped is reachable and the engine would wrongly draw. Add
the resolution-time re-check for source-state intervening-if conditions; regression with a
response-tap. *Cards:* howling_mine.

### 220. `per-permanent-combat-damage-prevention` — 2 cards, M
_Landed 2026-07-19: `Effect::PreventCombatDamageStatic { to_self, by_self }`, scanned live off the
permanent (`Game::combat_damage_prevented_to_creature` / `_by_source`) at all three combat-damage
chokes. fog_bank and guard_gomazoa authored faithfully, no `approximates`. Note: Guard Gomazoa is
1/3 (not the 0/6 this brief named — Scryfall's actual printed stats), `{2}{U}`._
Depends on: #130/#150/#159 prevention machinery (landed).
Landed prevention is turn-scoped (fog, Inkshield) or the Phantom counter shield; missing is
the *permanent static* — "prevent all combat damage that would be dealt to this creature"
(Guard Gomazoa) and "to and by" (Fog Bank). *Sketch:* a `prevent_combat_damage_static
{ to_self, by_self }` consulted at the three combat-damage chokes the #150 flag already
instruments, keyed on the source permanent — the scope generalization `state.rs:46` already
names as #130 slice 3. *Cards:* fog_bank, guard_gomazoa.

### 221. `blocks-trigger-and-block-reads` — 2 cards, M — LANDED 2026-07-19
_Landed 2026-07-19: both cards' brief-quoted oracle text had drifted from Scryfall's actual
current text (verified directly) — Gomazoa is `{2}{U}` 0/3 with a plain `{T}:` activation (no
mana cost, no "prevent all combat damage" line — that's only on the distinct Guard Gomazoa — and
no declare-blockers-step restriction; a ruling confirms it's activatable "any time [the
controller] has priority"), and Goblin Cadets is `{R}` 2/1 with no separate `{0}:` activated
ability at all — its whole oracle text is the one triggered ability: "Whenever this creature
blocks or becomes blocked, target opponent gains control of it." Landed: `Trigger::
BlocksOrBecomesBlocked` (self-referential, fired batch-deduped from `Game::declare_blockers` —
`Game::queue_blocks_or_becomes_blocked_triggers` — so a multiply-blocked attacker's "becomes
blocked" fires once, not once per blocker); `Game::attackers_blocked_by` (the reverse read of
`Game::blockers_of`); `Effect::TuckSelfAndBlockedCreatures` (Gomazoa's tap ability — tucks source
+ every blocked attacker to the top of their owners' libraries, ids minted sequentially like
`MassReturnFromGraveyard`, each owner shuffling exactly once after all of their tucks are queued).
Landed one prerequisite gap found along the way: `Game::place_targeted_ability`'s
`ThisPermanent`/`EnchantedCreature`/`ThisAurasGraveyardTarget` branch placed straight to the stack
with no second-clause pause, so a fixed-first-clause donation (`target = "this"`, Goblin Cadets)
never asked for its real second clause (the recipient opponent) — now routes through
`Game::place_ability_second_clause` like the general path does. Also landed CR 506.4c ("any time a
permanent's controller changes, it's removed from combat") at the shared control-gain apply
choke (`Game::remove_from_combat`, already used by regeneration/`RemoveFromCombat`, now also
called from the `ControlGained`/`ControlGainedUntilEndOfTurn`/`ConditionedControlGained` arms) —
Goblin Cadets' own reminder text ("This removes this creature from combat"), and a real gap for
every other control-change effect too (Zedruu, Reins of Power, …), none of which had a regression
test for it before. gomazoa.toml and goblin_cadets.toml are both fully faithful, no
`approximates`. Tests: `gomazoa_tucks_itself_and_each_creature_it_is_blocking`,
`gomazoa_tucks_only_itself_when_blocking_nothing`,
`goblin_cadets_blocks_or_becomes_blocked_trigger_fires`,
`goblin_cadets_donates_itself_to_target_opponent_when_it_becomes_blocked`._
Depends on: declare-blockers bookkeeping (landed); #228 slice 2 for Cadets' payoff.
No `blocks_or_becomes_blocked` trigger arm and no way to enumerate "each creature it's
blocking". *Sketch:* the trigger arm at the blocks/blocked choke, plus a block-assignment
read for Gomazoa's activation — tap: tuck itself and every attacker it's blocking into
their owners' libraries, each owner shuffles (multi-owner tuck+shuffle loop). Goblin Cadets
gets its trigger arm here; its "target opponent gains control of it" payoff lands with
#228. *Cards:* gomazoa, goblin_cadets (trigger half).

### 222. `life-total-set` — 1 card, M
_Landed 2026-07-19: `Effect::EachPlayerLifeBecomesHighest` (fieldless — the highest total is read
live at resolution), resolved in `mint_life_family` by routing each living player's
`highest - their_current` delta through the ordinary gain/lose choke (a zero delta emits no
event). arbiter_of_knollridge authored faithfully, no `approximates`._
Depends on: life-change choke + lifegain watchers (landed).
No effect sets a life total. *Sketch:* `each_player_life_becomes_highest` — CR 118.5 models
a set as gain/loss of the difference, so compute each player's delta to the highest total
and route it through the existing life-change choke (lifegain watchers/replacements fire
correctly); one bespoke effect arm. *Cards:* arbiter_of_knollridge.

### 223. `ability-x-surface` — 1 card, M — LANDED 2026-07-19
_Landed 2026-07-19: verified Scryfall oracle is simpler than sketched — `{X}{U}{R}, {T}: Nin deals X damage
to target creature. That creature's controller draws X cards.` (target creature only, unconditional
X-card draw, no "would die" gate; `{U}{R}` cost, Legendary Creature — Vedalken Wizard, 1/1). Authored
as `Sequence[deal_damage { amount = "x", target = "creature" }, target_owner_draws { count = "x",
controller = true }]` — both #215's rider (landed) reused exactly as its own doc anticipated. (a)
`WireIntent::ActivateAbility` gained `x: u32` (proto expand-only field 7 on `WireIntentActivateAbility`
+ `grpc/map/intent.rs` both directions); the one-click path (`Game::take_action`'s `MeaningfulAction::Activate`
arm) already had `x` in scope from `Intent::TakeAction` and just needed threading instead of a
hardcoded 0. (b) `Effect::CopyTriggeringAbility`'s `may_choose_new_targets = true` now offers a real
CR 707.10c re-pick when the copied ability actually targets, by generalizing `Game::place_targeted_ability`
to accept the copy's own `{X}`/activated-ness (`PendingChoice::ChooseTarget` grew matching `x`/`activated`
fields, mirroring `ChooseAbilityTargets`'s own, not wire-mirrored) — `unbound_flourishing.toml`'s
`approximates` deleted. (c) `Game::activate_ability`'s mana payment now feeds `settle_payment` a real
`SpellCharacteristics { has_x: cost.mana.x > 0, .. }` instead of `None`, so Elementalist's Palette's
`HasX` restricted credit funds an ability's own `{X}`, not just a spell's. (d) Protection now filters
a targeted ability's target — but at the *resolution* fizzle re-check (`Game::resolve_top`'s
`target_still_legal`, CR 608.2b), not at activation: this engine's established posture for an
illegal activated-ability target is "goes on the stack, fizzles later" (see
`deekah_grant_unblockable_lets_token_through`), not an upfront `Reject`, so `query.rs`'s
`legal_targets` (client highlight enumeration) and `place_targeted_ability`'s own legal-target scans
were also threaded with real source colors for consistency. Stale `effect.rs:9`/`mana.rs:15`
ponytails deleted. **Fully faithful (no `approximates`):** nin_the_pain_artist. Tests:
`nin_the_pain_artist_deals_x_damage_and_that_creatures_controller_draws_x_cards`,
`nin_fizzles_against_a_creature_with_protection_from_red`,
`nin_activated_ability_copied_by_unbound_flourishing_may_retarget_cr_707_10c`,
`elementalists_palette_restricted_mana_funds_nins_x_activation` (regression for (c))._
Depends on: #215 (the who-draws rider arm); {X}-activation core (landed — see below).
The classification-vs-re-audit conflict, settled by reading the code: the {X}-activation
core is **landed** — `Intent::ActivateAbility` carries `x` (`types/stack.rs:154`),
`activate_ability` pays `cost.mana.with_x(x)` and places via `push_ability_group_with_x`,
threading the chosen X so `Amount::X` resolves against it (`cast.rs:2119`); Illusionary
Mask's `activation_cost = { x = true }` is the live consumer. So the `effect.rs:9` ("only
spells choose an x") and `mana.rs:15` ("{X} in activated-ability costs aren't modeled")
ponytails are stale prose — delete them here (stale-comment cleanup). What IS missing, per
the re-audit: (a) `WireIntent::ActivateAbility` carries no `x`
(`schema/src/intent.rs:712` defaults to 0) and the one-click action path passes 0
(`lib.rs:416`) — thread a chosen X through the wire intent and action list so a client can
actually activate Nin; (b) `CopyTriggeringAbility` keeps the original's targets
(`effects.rs:1763`, `effect.rs:2611`) — Nin is exactly the targeted {X}-cost activated
ability that makes CR 707.10c's re-pick observable under Unbound Flourishing; offer the
retarget pause and delete `unbound_flourishing.toml:7`'s `approximates`; (c) `mana.rs:347`
— Elementalist's Palette's HasX credit sees no `SpellCharacteristics` at an ability
payment; feed `allows` the ability's own {X} count; (d) `query.rs:631` — a targeted
ability passes no source colors, so protection never filters its targets — Nin (a UR
source) vs a Flickering Ward-protected creature is illegal per CR 702.16b; thread the
ability source's colors through `legal_targets_for`. The card itself: `deal_damage
{ amount = "x" }` (landed) + #215's rider as `target_controller_draws` with an X amount.
*Cards:* nin_the_pain_artist.

### 224. `forecast` — 1 card, M — LANDED 2026-07-19
Depends on: `hand_ability` (landed); `once_each_turn` (landed for battlefield abilities).
`hand_ability` discards the card; Forecast (CR 702.57) reveals and *keeps* it, activatable
only during your own upkeep, once each turn. *Sketch:* a `forecast` sibling of
`hand_ability` with the keep-in-hand, upkeep-window, and once-per-turn gates. Also widen
`each_player_draws.count` from a bare `u32` to an Amount for "each player draws X" (S
rider). *Cards:* skyscribing.
_Landed 2026-07-19: `CardDef::forecast` (`Option<HandActivatedAbility>`, reusing the `hand_ability`
shape) shares `Game::activate_hand_ability`/`Intent::ActivateHandAbility` with `hand_ability` —
the engine branches on whichever field is set, so no new intent/proto surface was needed. The
forecast branch reveals (no discard, no `MovedToGraveyard`) and gates on the controller's own
upkeep (`Step::Upkeep` + `active_player == player`) plus once-each-turn, reusing the battlefield
`once_per_turn.activated` store with a fixed sentinel ability-index of 0 (a hand card carries at
most one `hand_ability`/`forecast`, so there's no real index to collide with). `EachPlayerDraws`'s
`count` widened `u32` → `Amount` so the spell mode reads `Amount::X`; the two existing
`each_player_draws` cards (Vision Skeins, Faerie Mastermind) needed no edits (their plain integer
`count` still deserializes as `Amount::Fixed`). Correction to this entry's own sketch: the real
Scryfall oracle text has **no** "if it's not your turn, discard hands at the next end step"
clause — that line isn't on Skyscribing at all (checked against Scryfall directly; the increment
brief's quoted oracle was wrong) — so there is no delayed-discard residual to flag. skyscribing.toml
is fully faithful, no `approximates`. Tests: `skyscribing_cast_each_player_draws_x`,
`skyscribing_forecast_each_player_draws_a_card_only_during_your_upkeep_once_per_turn`.

### 225. `cumulative-upkeep-nonmana` — 1 card, M — LANDED 2026-07-19
_Landed 2026-07-19: `CounterKind::Age` added (the same documented growth path as #206's Strife,
`crates/engine/src/types/effect.rs`). Cumulative upkeep (CR 702.24) is a new top-level
`CardDef::cumulative_upkeep: Option<CumulativeUpkeepCost>` field — same shape as `echo` — rather
than a `[[abilities]]` trigger, since it needs its own bespoke queue
(`Game::pending_cumulative_upkeep` / `queue_cumulative_upkeep_triggers`, mirroring
`pending_echo`): at every one of the controller's upkeeps (no "since your last upkeep" gate,
unlike Echo — CR 702.24a's trigger is bare "at the beginning of your upkeep," and it's
controller-scoped via `controller_of`, not owner-scoped like Echo's own check), it places an age
counter (`Event::KindCountersPlaced`, reused as-is) then raises a new
`PendingChoice::PayCumulativeUpkeepOrSacrifice { player, source, options, count }` scaled to
`graveyard_cards × age counters`. The non-mana cost arm needed no new `Effect`/`Cost` variant:
paying moves `count` chosen cards (validated to share one owner — CR "a single graveyard") from
`options` (every graveyard's cards, flattened across `living_players()`) to the bottom of that
owner's library by reusing `Event::TuckedToLibrary` (Mistveil Plains's own zone-move event)
directly; declining (an empty answer) reuses `Effect::SacrificeObject`, the same "declining does
something" polarity as `PayEchoOrSacrifice`. The answer side needed **zero new `Intent`**: it
rides the existing `Intent::ChooseSacrifices`/`ChoiceSacrifices` "empty declines, else name the
chosen set" wire shape already multiplexed across a dozen `PendingChoice` variants
(`crates/engine/src/pending/mod.rs`) — only the *view* is new
(`PendingChoiceView::PayCumulativeUpkeepOrSacrifice`, proto field 62, mirroring
`PayRecoverOrExile`'s own "no client form yet — deferred to Phase 5" precedent). jotun_grunt
authored and fully faithful, no `approximates` — its real printed cost is `{1}{W}` (Scryfall,
Commander 2011 `cmd`), not the ticket's assumed `{2}{W}`, and the real name carries a diaeresis
("Jötun Grunt"); the file uses the plain-ASCII spelling matching the pool's existing
snake_case-filename convention. Tests:
`jotun_grunt_cumulative_upkeep_adds_age_counter_and_scales_cost`,
`jotun_grunt_sacrificed_when_upkeep_cost_unpaid`,
`jotun_grunt_upkeep_cards_go_to_bottom_of_owners_library`. Still blocked: nothing._
Depends on: echo's pay-or-sacrifice upkeep pause (landed); #75 counter-kinds (landed).
CR 702.24: age counters (`CounterKind::Age` — the same documented growth path as #206's
strife), an upkeep pause whose cost scales per age counter (pay or sacrifice), and one
bespoke non-mana cost arm — "put two cards from a single graveyard on the bottom of their
owner's library" per age (a graveyard-card-pick payment). *Cards:* jotun_grunt.

### 226. `nonland-death-watch-shared-type-edict` — 1 card, M
_Landed 2026-07-19: a new `Trigger::NonlandPermanentYouControlDiesIncludingThis` arm
(`crates/engine/src/types/trigger.rs`) generalizes the you-control death scan to all four nonland
types and self-fires like the `*_including_this` creature arms, queued by
`Game::queue_nonland_permanent_death_watchers` (`crates/engine/src/triggers.rs`) — a sibling of
`queue_enchantment_death_watchers`, not folded into the creature-only CR 603.6c look-back path.
The dying permanent's own last-known card types ride on `TriggerContext::dying_permanent_types`,
resolved at `contextualize_effect` time (`fill_dying_permanent_types`) into a new
`PermanentFilter::shares_type_with_dying_permanent` axis (bare-string shorthand
`"shares_type_with_dying_permanent"`) that overwrites `EachPlayerSacrifices`'s `filter.types`
with the dying permanent's types before the edict ever raises a choice — no change to the edict
resolution path itself (`sacrifice_edict`/`edict_options`) was needed. martyrs_bond authored and
fully faithful (no `approximates`); note the real Scryfall printing is Commander 2011 (`cmd`),
mono-white `{4}{W}{W}`, not the ticket's assumed Conflux `{4}{W}{B}` — verified against the live
Scryfall API. Still blocked: nothing.
Depends on: `enchantment_you_control_dies` (landed shape); edict machinery (landed).
Death watches are creature-scoped (plus the narrow enchantment arm). *Sketch:* generalize
the `*_including_this` death scan with a permanent filter ("this or another **nonland
permanent** you control is put into a graveyard"), and an each-opponent-sacrifice whose
filter is computed from the dead permanent's LKI card types ("shares a card type with it")
— a dynamic filter the edict machinery doesn't take today. *Cards:* martyrs_bond.

### 227. `optional-retarget-plus-copy` — 1 card, M
_Landed 2026-07-19: `Effect::ChangeTargetOfTargetSpellOrAbility` gained an `optional` bool
(default `false`, preserving Willbender's mandatory single-target-only must-differ bend
unchanged). `optional = true` (Wild Ricochet) keeps the bent spell's current target(s) in
`legal` — re-picking them is how a player declines, CR 114.6a's plain "may," no must-differ
filter — and reaches every one of the bent spell's own independent target clauses via a new
shared `Game::spell_primary_target` lookup feeding the existing `choose_spell_targets`/
`advance_spell_target_clauses` chain (also now used by `CopyTargetSpell`'s own copy-retarget,
replacing its prior duplicate inline lookup). That chain's `player` parameter split into
`anchor` (whose perspective legality is evaluated from — the bent/copied spell's own controller)
and `chooser` (who actually answers, this ability's own controller) — the two always coincided
for a cast or a copy, but Wild Ricochet can bend an opponent's spell without becoming its
controller; `choose_spell_targets_answer` threads the real answering player through as the
chooser for clause-to-clause chaining. wild_ricochet authored, fully faithful (no
`approximates`). Still blocked: nothing.
Depends on: `change_target_of_target_spell_or_ability` (Willbender, landed);
`copy_target_spell` (landed).
The landed retarget is single-target-spells-only and mandatory ("must change if able");
Wild Ricochet needs a *may* retarget of any instant/sorcery (including multi-target sets)
followed by `copy_target_spell` sharing that same stack-object target. *Sketch:* widen the
retarget write-back to full target sets, add the optional flag, and plumb the shared target
between the two intrinsic-target steps. Its minted copy is protected by #213 if countered.
*Cards:* wild_ricochet.

### 228. `control-donation-and-exchange` — 6 cards, XL (sliced — one slice per wave) — LANDED 2026-07-19
_Slice 1 (timestamped control layer + controller gates) landed 2026-07-19. Per-entry CR 800.4a
timestamps now ride every control source: a monotonic `Game::next_control_timestamp` stamps each
of the three override registries (`control_overrides`, `permanent_control_overrides`,
`conditioned_control_overrides`) and every `ControlAttached` Aura (a new
`PlayPermissions::aura_control_timestamps`), and `Game::controller_of` collects all live sources
for a permanent and returns the highest-timestamp one (`Game::permanent_controller`) — real
"most recent wins", replacing the fixed-order `an active entry wins` stand-in. The owner-gates all
flipped to `controller_of`: blocker legality (`combat.rs`, CR 509.1a), ability activation
(`cast.rs` `ability_activation_gate`, CR 602.2), prepared-back-face cast (`cast.rs`, CR 602.2),
and both mana paths (`priority.rs` `tap_for_mana` + `available_mana`, CR 602.2/605.3) — a stolen
permanent now blocks / activates / taps for its thief. **dominus_of_fealty** authored & faithful;
note its real Scryfall oracle is an until-EOT Threaten-shape ("gain control of target permanent
until end of turn. If you do, untap it and it gains haste until end of turn"), NOT the
`gain_control_while` the ticket sketched — it reuses the landed
`gain_control_until_end_of_turn` + `untap_target` + `pump_until_end_of_turn` surface (target
`{ permanent = {} }`), so no new DSL surface. No proto/schema change (timestamps are internal,
not wire-mirrored). Regression tests in `game.rs` cover CR 800.4a precedence (`a_later_permanent_steal_outranks_an_earlier_one_cr_800_4a`,
`an_until_eot_steal_layered_over_a_permanent_steal_reverts_to_the_permanent_controller`), both
gate flips (`a_stolen_creature_blocks_for_its_thief_not_its_owner_cr_509_1a`,
`a_stolen_land_taps_for_mana_...`, `a_stolen_permanents_activated_ability_belongs_to_its_thief_cr_602_2`),
and Dominus end-to-end.
**Slice 2 (donation) landed 2026-07-19.** A new `Effect::TargetOpponentGainsControl { target, player }`
carries two independent target clauses (CR 601.2c): the donated permanent is the ability's own first
target (`{ permanent = { controller = "you" } }`, gated on `controller_of` — slice-1's flip, so a
borrowed permanent is a legal donation), and the recipient opponent is a *second* clause chosen via
the `place_ability_second_clause` path — now generalized to fire for two-target *activated* abilities
(not just Kinetic Ooze's trigger): `ChooseAbilityTargets` gained internal `x`/`spent_mana`/`activated`
fields (schema ignores them via `..`, no proto change) so the assembled donation pushes as a genuine
activated ability (CR 112.7a counterability preserved), auto-filling when only one opponent is legal.
Resolution reuses slice-1's persistent `permanent_control_overrides` write — `Event::ControlGained`
with the chosen player as controller, freshly timestamped (CR 800.4a) — so **ownership is untouched**
(CR 108.3): the donor still owns it for Zedruu's own upkeep count and for owner-correct death/zone
routing. Zedruu's upkeep uses a new `Amount::PermanentsYouOwnOpponentsControl` (owner-is-you,
controller-isn't — counts each donated permanent once regardless of opponent count) for *both* the
gain-X-life and draw-X clauses (the current Scryfall oracle gains X life, not a flat 1 — the ticket's
sketch predated that erratum). **zedruu_the_greathearted** authored & faithful (no `approximates`).
**goblin_cadets deferred to #221** (its trigger arm is unlanded; out of scope this slice). Regression
tests in `game.rs`: `zedruu_donates_a_permanent_and_target_opponent_controls_it_cr_800_4a`,
`zedruu_upkeep_draws_and_gains_life_per_owned_permanent_an_opponent_controls`,
`zedruu_donates_a_permanent_it_controls_but_does_not_own_cr_720`.
**Slice 3 (exchange) landed 2026-07-19.** A new `Effect::ExchangeControl { first, second }` carries two
independent target clauses (CR 601.2c): `first` is the ability's own "you control" target (`Effect::target`),
`second` is the "an opponent controls" clause, chosen via the same `place_ability_second_clause` path slice 2
generalized (its special-case sits beside donation's in `ability_second_target_clause`). Resolution
(`effects.rs`, beside `TargetOpponentGainsControl`) reads both permanents' `controller_of`, then mints TWO
freshly-timestamped `Event::ControlGained` — each permanent's new controller is the OTHER's prior one — so the
swap outranks any earlier steal (CR 800.4a) while ownership is untouched (CR 108.3). The two "you"/"opponent"
filters are disjoint, so no CR 601.2c distinctness gate was needed (the ticket's "two target lands, any
controller" sketch predated the current Scryfall erratum, which scopes both clauses). No proto/schema change
(reuses slice-2's `ControlGained` projection + the two-clause activation/trigger views). **vedalken_plotter
authored & faithful** (no `approximates`; current oracle is the you/opponent-scoped "target land you control and
target land an opponent controls", 1/1 for {2}{U}, not the ticket's guessed "two target lands"). **Morph turned
out to be LANDED after all** — Willbender is a real morph card in the pool (`[morph]` cost + `turned_face_up`
trigger), so `card.rs`'s stale "no morph card is in the pool" sentence was corrected — so **chromeshell_crab
authored & faithful too** (cast face down for {3}, turned up for its {4}{U} morph cost, its `may` turned-face-up
trigger exchanges a creature you control with an opponent's; 3/3 Crab Beast). Regression tests in `game.rs`:
`vedalken_plotter_exchanges_control_of_two_lands_cr_800_4a`, `an_exchange_layered_over_a_donation_reverts_correctly_cr_800_4a`,
`chromeshell_crab_turned_face_up_may_exchange_a_creature_you_control_with_an_opponents`, and
`chromeshell_crab_declining_its_may_leaves_control_unchanged`.
**Slice 4 (mass two-player swap + elimination sweep) landed 2026-07-19.** A new
`Effect::ExchangeAllCreaturesUntilEndOfTurn { target }` (`target = "opponent"`, a single cast-time
target — both creature *sets* are computed at resolution, not targeted). Resolution (`resolution/control.rs`,
control family) snapshots every creature the caster controls and every creature the target opponent controls
*before* any swap (CR 800.4a — the first steal can't feed the second), untaps them all, hands each to the OTHER
player via freshly-timestamped until-EOT `ControlGainedUntilEndOfTurn` (the slice-1 `control_overrides` layer that
reverts at cleanup, CR 514.2 — a swap layered over a donation reverts to the donated-to controller, not the
owner), and grants `Keyword::Haste` via `TempBoost`. Ownership untouched (CR 108.3). No proto/schema change (reuses
existing `Untapped`/`ControlGainedUntilEndOfTurn`/`TempBoost` events). Plus the CR 800.4a **elimination sweep**,
implemented in the `Event::PlayerLost` apply arm (so it fires for lethal-damage eliminations too, not just
concede): the owner-removal loop already drops everything the leaver owns (a permanent they own but another
controls leaves with them); slice 4 adds dropping every control override whose new controller is the leaving
player, so a permanent they stole returns to its owner. The stale `priority.rs` concede ponytail ("permanents stay
on the battlefield") is deleted — the sweep now runs. **reins_of_power** authored & faithful (no `approximates`;
current Scryfall oracle untaps BOTH creature sets up front and is `{2}{U}{U}`, not the ticket's assumed `{3}{U}{U}`
/ "untap all creatures you control" only). Regression tests in `game.rs`:
`reins_of_power_swaps_all_creatures_between_two_players_until_eot_cr_800_4a`,
`reins_mass_steal_layered_over_a_donation_reverts_to_the_donated_controller_cr_800_4a`,
`a_stolen_creature_from_reins_can_attack_for_its_thief`, and
`when_a_player_leaves_permanents_they_own_under_others_control_leave_and_their_control_effects_end_cr_800_4a`.
This also **unblocks #229** (`mass-steal-until-eot`, Insurrection) — Reins' snapshot-then-mint-until-EOT primitive
is the same shape a `gain_control_all_until_end_of_turn { filter }` would reuse (Insurrection loops it over every
creature of any controller); not built here.
**Slice 5 (client catch-up) landed 2026-07-19 — #228 fully LANDED.** Three client deliverables, all
minimal catch-up (no redesign): **(1) Controller-vs-owner rendering.** `layout()` already grouped the
battlefield by `controller` (`controls(zone, who)`), so a donated/exchanged/stolen permanent already
rendered in its controller's row; added the missing **owner badge** — a pure `foreignOwnerSeat(owner,
controller)` (`cardBadges.ts`, returns the owner seat when it differs, else null) drives a seat-coloured
bar down the card's left edge in `boardDraw.ts` (`drawStatusBadges`), and `board.tsx`'s `bfMarkers`
gained a `data-owner` attribute for e2e. ponytail: the bar encodes the owner by seat colour only, no
player name. **(2) Donation / exchange targeting.** Both flow through surface that already existed:
donation's first clause ("target permanent you control") is the ability's own activation target, and
its second clause ("target opponent") + exchange's "an opponent controls" clause arrive as
`choose_ability_targets` — already form-mapped. The one gap: donation's second clause is a *player*
target (no card art), so `ChooseSpellTargetsForm` now routes a single non-card target to a shared
`SeatTargetPick` dialog (extracted from `ChooseTargetForm`) emitting a `{target, player}` answer →
`choose_targets` intent. **(3) Two-clause activated-ability intent through RPC.** `bun run gen`
regenerated the wire; `protoMap.ts` is a generic structural walker so the new views/intents round-trip
without per-arm mapping. Added the #218 `choose_activation_cost_targets` view to `types.ts` +
`ChooseActivationCostTargetsForm` (reuses `CardPickPrompt`, answers `choose_targets`) so the exhaustive
`FORMS` record and wire type both cover it. Client tests: `cardBadges.test.ts` (owner badge),
`layout.test.ts` (donated permanent renders under its controller, badged by owner). No proto/schema
change; server untouched._
Depends on: #218 multi-target-abilities.
The deck's centerpiece: every landed control effect (`gain_control`,
`gain_control_until_end_of_turn`, `gain_control_while`, `control_attached`) hands control
to the *ability's own controller* — nothing can give a permanent to someone else, exchange
control, or count owned-but-donated permanents. Reading the code softens the re-audit's A1
("no separate controller field") slightly: `permanent_control_overrides` (Entrancing
Melody) is already a persistent control layer over true ownership, so donation must land in
that layer — NOT as an owner rewrite like `apply.rs:1527`'s reanimation shape — and death/
zone routing then stays owner-correct for free (feeds Martyr's Bond, Journey to Nowhere's
"under its owner's control"). Still honestly XL: timestamps, gate flips, three new effect
families, an elimination sweep, and client catch-up.
*Slices:*
1. **Timestamped control layer + controller gates (M).** Per-entry CR 800.4a timestamps
   across the three override registries + `ControlAttached` so "most recent wins" is real
   (`core.rs:362`, `state.rs:71` — re-audit A2), and flip the owner-gates to
   `controller_of`: blocker legality (`combat.rs:30`, CR 509.1a — re-audit A3) and ability
   activation / tap_for_mana / available_mana (`cast.rs:1686`/`1298`, CR 602.2 — re-audit
   A4). Lands **dominus_of_fealty** (demoted from C): its Besmirch-shaped optional upkeep
   steal is landed surface, but stealing *any permanent* is only worth doing once the thief
   can tap/activate it.
2. **Donation (M).** Generalize the control-change event to carry an arbitrary new
   controller; `target_opponent_gains_control` — "target opponent gains control of target
   permanent you control" (two independent targets on an activated ability, via #218); and
   Zedruu's upkeep Amount, `permanents_you_own_opponents_control` (owners are already
   tracked; compare `owner_of` vs `controller_of`). Lands **zedruu_the_greathearted** and
   **goblin_cadets**' payoff (trigger arm from #221).
3. **Exchange (M).** `exchange_control` — two target clauses (yours / an opponent's),
   swapped permanently in the control layer. Lands **vedalken_plotter** (ETB, two lands)
   and **chromeshell_crab** (turn-face-up trigger, two creatures — morph itself is landed,
   #163; delete `types/card.rs:1602`'s stale "no morph card is in the pool" sentence).
4. **Mass two-player swap + elimination sweep (M).** Reins of Power: until-EOT exchange of
   all creatures between you and target opponent + untap + haste (composes the Besmirch
   revert machinery over the timestamped layer; blocking with stolen creatures = slice 1's
   gate). Plus the `priority.rs:100` sweep: when a player leaves, permanents they *own*
   under others' control leave with them and control effects they own end (CR 800.4a) —
   newly observable once donation exists. Lands **reins_of_power**.
5. **Client catch-up (M).** Controller-vs-owner rendering on the board (a donated permanent
   sits in its controller's row with an owner badge), donation/exchange targeting forms,
   and the two-clause activated-ability intent from #218 wired through the RPC layer.

### 229. `mass-steal-until-eot` — 1 card, M — LANDED 2026-07-19
`Effect::GainControlAllUntilEndOfTurn { filter }` — the mass, one-sided, all-creatures-of-any-controller
twin of the landed single-target `gain_control_until_end_of_turn`, reusing #228 slice 4's snapshot-then-
mint-freshly-timestamped-until-EOT-steal + untap + haste primitive (Reins of Power's
`ExchangeAllCreaturesUntilEndOfTurn`) without the two-player swap: `filter` (`creature` for Insurrection) is
evaluated against EVERY creature on the battlefield regardless of controller, including the caster's own (no
`you`/`opponent` scoping, unlike `UntapAll`, which is hardcoded to the ability's own controller — see
`resolution/control.rs`). Resolution (beside `ExchangeAllCreaturesUntilEndOfTurn` in `resolution/control.rs`)
snapshots the matching set before minting any event, untaps them all, hands each to the caster via a
freshly-timestamped `Event::ControlGainedUntilEndOfTurn` (so a mass steal layered over a donated permanent
outranks the donation this turn and reverts to the donated-to controller at cleanup, not the owner — CR
514.2 / CR 800.4a), and grants `Keyword::Haste` via `Event::TempBoost`. Ownership is untouched (CR 108.3). No
new event/proto/schema surface (reuses `Untapped`/`ControlGainedUntilEndOfTurn`/`TempBoost`, and the default
`_ => execute_effect` fallthrough in `effects.rs::run` — no destroy-style "this way" bookkeeping needed).
Current Scryfall oracle text matches the ticket's sketch verbatim ("Untap all creatures and gain control of
them until end of turn. They gain haste until end of turn."), but the cost does not: `{5}{R}{R}{R}` (cmc 8,
Commander Masters), not the ticket's guessed `{5}{R}{R}`. **insurrection** authored & faithful (no
`approximates`). Regression tests in `game.rs`:
`insurrection_steals_all_creatures_until_eot_with_haste_cr_800_4a` (four controllers, including the caster's
own creature, all stolen/untapped/hasted, all revert at cleanup),
`insurrection_layered_over_a_donation_reverts_to_the_donated_controller_cr_800_4a`, and
`a_stolen_creature_from_insurrection_can_attack_for_its_thief` (re-audit A4's falsifier, confirmed: the
stolen creature's declared-attacker gate reads `controller_of`, so it attacks for the thief this turn).

### 230. `clash` — 4 cards, L (takes the wave XL slot) — LANDED 2026-07-18
_Slice 1 (clash core + Lash Out) landed 2026-07-18. Slice 2 landed 2026-07-18: the three riders
(whirlpool_whelm / pollen_lullaby / scattering_stroke) are authored & faithful. New surface:
`tuck_permanent_into_library.second_from_top` (Whirlpool Whelm), `skip_next_untap_opponent_creatures`
(Pollen Lullaby, with a per-permanent skip-next-untap mark consumed at the controller's next untap
step), `schedule_colorless_mana_for_countered_spell_next_main_phase` (Scattering Stroke), and a
Main-phase (`Main1`, controller-scoped) arm on `Game::fire_delayed_triggers`. That same
`StepBegan{Main1}` hook added the `first_main_phase` trigger timing, which made
`advanced_reconstruction.toml` faithful (its "upkeep stands in for first main phase" approximation
is gone). Residual (ponytail, unobservable for the pool): Scattering Stroke reads the countered
spell's PRINTED mana value (an {X} spell's chosen X counts as 0) and approximates "your next main
phase" as your next precombat main._
Depends on: #107 opponent-chooser (landed); #150 fog (landed); delayed-trigger drain
(landed).
CR 701.22 — no clash primitive exists (Keen Duelist's mutual reveal is unrelated).
*Slices:*
1. **Clash core + Lash Out (M).** A `clash` sub-action: controller picks an opponent
   (shared #107 chooser), both reveal their top card, each pauses on keep-top-or-bottom,
   MV compare sets a resolution-scoped won-the-clash flag read by a new
   `Condition::WonClash` in a following `conditional` step. Lash Out's rider needs damage
   to the targeted creature's *controller* (a target-controller damage read). Lands
   **lash_out**.
2. **The three riders (M).** **whirlpool_whelm** — bounce-or-library-top choice on a win;
   **pollen_lullaby** — fog is landed (#150), plus a new delayed "creatures that player
   controls don't untap during their next untap step"; **scattering_stroke** — delayed
   "you may add {C}×MV at the beginning of your next main phase". Re-audit fold-in:
   `effect.rs:2726`'s `fire_at` covers only Upkeep/End — Scattering Stroke is the third
   timing; add a Main-phase arm to `Game::fire_delayed_triggers` (which also unblocks
   fixing `advanced_reconstruction.toml:25`'s "upkeep stands in for first main phase"
   approximation).

### 231. `opponent-repeat-draw-loop` — 1 card, L (takes the wave XL slot) — LANDED 2026-07-19
Depends on: cross-player pay/choice pause shapes (#102/#107, landed patterns).
Trade Secrets: "target opponent draws two cards, then you may draw up to four cards"; the
opponent "may repeat this process as many times as they choose". Two choice shapes with no
sibling: an opponent-answered repeat-or-stop pause and a caster-answered draw-up-to-N count
pause, looped until decline — genuinely L for one card, but the declinable-draw half is
engine-global. Re-audit fold-in: `arcane_denial.toml:6` collapsed "may draw up to two" to a
mandatory draw on the claim that "drawing is strictly beneficial everywhere in this pool" —
this deck's group-draw density (Howling Mine, Skyscribing, Vision Skeins, Murmurs from
Beyond) makes draw-out-the-library states realistic and Trade Secrets makes declinable
draws mandatory machinery anyway; restore Arcane Denial's printed "may draw up to two" once
the pause exists. *Slices:*
1. **Draw-up-to-N pause (M)** — the caster-answered count choice, consumed by a plain
   `may_draw_up_to` effect; restore arcane_denial's printed text with it.
2. **The opponent repeat loop (M)** — opponent-answered repeat-or-stop wrapping the
   draw-two + draw-up-to-four sequence until decline.
3. **Client forms (S)** — count-picker and repeat prompts over the wire.
*Cards:* trade_secrets.

Slice 1 (draw-up-to-N pause) landed 2026-07-19: `Effect::MayDrawUpTo { count }` pauses its
resolving controller on a new `PendingChoice::MayDrawUpTo { player, max }` count choice
(new `Intent::ChooseDrawCount`, clamped `0..=max`), draws exactly the chosen number, and is
documented in DSL_REFERENCE. arcane_denial is faithful — printed "may draw up to two" restored,
ponytail note deleted. Wire surface added (`WireIntentChooseDrawCount` field 53,
`PendingChoiceViewMayDrawUpTo` field 63) but no client form yet. Slices 2 (opponent repeat
loop) and 3 (client forms) remain; trade_secrets is not authored this wave (its file stays
absent, lands with slices 2/3).

Slice 2 (opponent repeat loop) landed 2026-07-19: **oracle correction** — the live Scryfall text
is "Target opponent draws two cards, then you draw up to four cards. That opponent may repeat this
process as many times as they choose." (not the ticket's sketched "you draw a card, then target
opponent draws two"). The target opponent's two-card draw is mandatory/immediate
(`target_player_draws { opponent = true }`); the caster's "up to four" is its own declinable count
pause, new `Effect::MayDrawUpToThenOpponentMayRepeat { count }` → `PendingChoice::TradeSecretsCasterDraw
{ player, max, opponent, source }` (reuses `Intent::ChooseDrawCount`'s wire shape, dispatched
alongside `MayDrawUpTo`) → once answered, the *target opponent* is paused on
`PendingChoice::TradeSecretsRepeat { player, caster, max, source }` (reuses `Intent::AnswerMay`,
dispatched alongside `DanceExileMore`); `yes` re-runs the mandatory two-card draw then re-raises
`TradeSecretsCasterDraw` (self-rescheduling, like `dance_exile_more`), `no` stops. Both new
`PendingChoice` variants have full schema/wire projections (`PendingChoiceView`, `dto.rs`,
`answer_protocol.rs`, proto fields 64/65) so every exhaustive match compiles; no client form yet.
trade_secrets is authored and fully faithful (no `approximates`) — `crates/cards/data/trade_secrets.toml`,
set `cmd` (Commander 2011), `{1}{U}{U}` sorcery. Note: Trade Secrets is banned in Commander (and
in Oathbreaker/Predh) per its Scryfall legalities — authored anyway per this backlog's
any-card-built-faithfully posture; flagging here since it's this deck's proving ground. Slice 3
(count-picker/repeat client forms) is the only remaining piece of #231.

Slice 3 (client forms) landed 2026-07-19 (Phase 5 client catch-up): `may_draw_up_to` and
`trade_secrets_caster_draw` share one count-picker form (a button per count `0..=max`, one click
answers via the new `draw_count` AnswerInput → `WireIntentChooseDrawCount`);
`trade_secrets_repeat` is a Repeat/Stop yes-no reusing the `may` answer (`AnswerMay`). All three
render in the corner panel. With this, all three slices are in — #231 is LANDED and
trade_secrets is playable end to end.
