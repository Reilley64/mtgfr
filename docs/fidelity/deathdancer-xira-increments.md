# Deathdancer Xira deck increments (2026-07-17)

Deck report: [deathdancer-xira.md](deathdancer-xira.md). Engine-wide tiers and per-card
exotics stay in [../FIDELITY_BACKLOG.md](../FIDELITY_BACKLOG.md); increment numbering is
global across both.

From `docs/fidelity/deathdancer-xira.md` (Archidekt 2209179 — commander Xira Arien).
26 of the deck's 65 new cards need engine work; ranked S-first within dependency order. The
observability re-audit for this deck falsified nineteen pool-absence claims — each is folded
into the increment that clears it (#168, #169, #174, #180, #181, #182, #183, #184, #185,
#186, #187, #192, #196, #198).

### 167. `sorcery-card-filter` — 1 card, S
Depends on: nothing.
`CardFilter::Sorcery` — the sorcery-only half of the existing `InstantOrSorcery` arm (which
already reads `CardKind::Spell` + speed directly), plus its `de.rs` string. Every consumer
(graveyard-card targets, search/return filters) inherits it. *Cards:* anarchist.

_Landed 2026-07-17: `CardFilter::Sorcery` added (matches `CardKind::Spell { speed: Sorcery }`),
with the `card_filter_label` arm and a `DSL_REFERENCE` filter-axis entry. `filter = "sorcery"`
parses via the enum's existing `rename_all = "snake_case"` — no `de.rs` string arm needed.
anarchist authored, fully faithful (no `approximates`). Still blocked: nothing._

### 168. `negated-filter-axes` — 3 cards, S
Depends on: #63 color-filter-axis (landed).
"Nonartifact, nonblack creature" targeting. *Sketch:* a `not_color` arm on `ColorFilter`
(read off `Game::colors_of`) and the `exclude: TypeSet` generalization of the `noncreature`
bool that `filter.rs:643`'s ponytail names (an artifact creature via `also` must fail
`nonartifact`). Clears the `filter.rs:550` and `filter.rs:643` ponytail ceilings. *Cards:*
terror, shriekmaw, ashes_to_ashes.

_Landed 2026-07-17: `ColorFilter::NotColor(Color)` (reads `Game::colors_of`) and
`PermanentFilter::exclude: TypeSet` (generalizing the old `noncreature` bool, which is now sugar
for `exclude = "creature"`), plus `de.rs` TOML keys (`exclude`, `not_color`), a
`permanent_filter_label` arm, and `DSL_REFERENCE` entries. shriekmaw's ETB target upgraded to the
real filter (`approximates`/ponytail deleted); terror authored, fully faithful. ashes_to_ashes
authored faithful for the exile-two-nonartifact-creatures clause; its real oracle text is
"Exile two target nonartifact creatures" (no "nonblack" — the wave brief's paraphrase was wrong,
verified against Scryfall) at `{1}{B}{B}`, not `{2}{B}{B}`. The "deals 5 damage to you" rider is
modeled as losing 5 life (an `approximates` note cites #173 spell-self-damage for the real
damage routing). Still blocked: nothing for #168 itself._

### 169. `devoid-real-colors` — 1 card + observers, S
Depends on: #168 (its consumers make this observable).
Devoid must actually zero the card's colors: `smothering_abomination.toml`'s "Devoid is
flavor" ponytail dies the moment a `not_color = "black"` target scan exists — Terror and
Shriekmaw CAN target the colorless Abomination. *Sketch:* honor the landed
`CardDef::colors` empty-vs-explicit distinction by giving devoid cards an explicit empty
color list (`colors = []` meaning "explicitly colorless", distinct from "derive from pips"),
or a `devoid` flag consulted by `colors_of`. *Cards:* smothering_abomination (existing pool).

_Landed 2026-07-17: `CardDef::devoid` bool (kept `Copy`) checked first in `color_identity`, returning
all-false before cost-pip/`colors` derivation (CR 702.114a), plus the `de.rs` TOML key and a
`DSL_REFERENCE` entry. smothering_abomination authored with `devoid = true`, "Devoid is flavor"
ponytail deleted — now colorless despite `{2}{B}{B}` and a legal `not_color = "black"` target. Still
blocked: nothing._

### 170. `fear-keyword` — 2 cards, S
Depends on: nothing (skulk/shadow `can_block` pattern, landed).
`Keyword::Fear` + one `Game::can_block` clause: a would-be blocker must be an artifact
creature or black (CR 702.36). *Cards:* shriekmaw, avatar_of_woe.

_Landed 2026-07-17: `Keyword::Fear` (CR 702.36b) + `Game::can_block` clause (blocker must be an
artifact creature or black), plus the `wire_keyword`/`keyword_label` schema arms and a
`DSL_REFERENCE` keyword entry. shriekmaw and avatar_of_woe authored with fear fully faithful.
shriekmaw's ETB target was upgraded to the real "nonartifact, nonblack creature" filter by #168
negated-filter-axes and is now fully faithful. avatar_of_woe's cost reduction landed with #171
graveyard-count-conditions below — both cards are now fully faithful._

### 171. `graveyard-count-conditions` — 2 cards, S
Depends on: #71 condition-extensions (landed pattern).
Two Condition arms: `cards_in_your_graveyard_at_least { count }` (threshold — whole-graveyard
count, evaluated live so the anthem flips as the graveyard changes) and
`creature_cards_in_all_graveyards_at_least { count }` (sum the existing per-graveyard
creature-card walk over every player). Consumers: `anthem_static self_only` + conditional
cost reduction. *Cards:* werebear, avatar_of_woe.

_Landed 2026-07-17: both `Condition` arms added (`crates/engine/src/types/effect.rs`) with live
`condition_holds` arms in `triggers.rs` (`CardsInYourGraveyardAtLeast` reads the controller's own
graveyard; `CreatureCardsInAllGraveyardsAtLeast` sums `graveyard_cards` over every living player,
reusing the `graveyard_functional_watchers` all-graveyards-scan pattern). Both consumers were
already-landed mechanisms: `AnthemStatic.condition` (Bloodghast's shape) for werebear's Threshold
+3/+3, and `Cost.reduce_own_generic` via the existing `Amount::IfCondition` (Mortality Spear's
shape) for avatar_of_woe's {6} discount — no new consumer plumbing needed. werebear authored
faithful (real oracle has no trample, unlike this brief's paraphrase — verified against Scryfall);
avatar_of_woe's `approximates`/ponytail deleted, fully faithful._

### 172. `search-to-graveyard` — 1 card, S
Depends on: #76 multi-pick search (landed).
`SearchDest::Graveyard`: a found card moves library→owner's graveyard through the normal
move choke (firing graveyard observability). One dest arm + one route line; count/shuffle
semantics unchanged. *Cards:* buried_alive.

_Landed 2026-07-18: added `SearchDest::Graveyard`, routed in `search_library`'s answer handler
through the same `Event::Milled` library-to-graveyard choke mill effects use — so the arrival is
never "put into a graveyard from the battlefield" (CR 700.4) and can't fire Dies (reuses #183's
from-battlefield gate). `label.rs` gets "into your graveyard" text; the two `reveal_until`/
`reveal_top_cards` exhaustive matches get `ponytail:`-noted no-op arms (no pool card searches to
graveyard via a top-down reveal). `buried_alive` authored faithfully — `{2}{B}` sorcery (verified
against Scryfall), `search_library` `filter = "creature"`, `to_zone = "graveyard"`, `count = 3` —
no `approximates`. Regression `tutored_to_graveyard_creature_does_not_fire_dies` covers #183's
previously-untested path._

### 173. `spell-self-damage` — 1 card, S
Depends on: `Game::damage_player` (landed).
`Effect::DealDamageToSelf { amount }` routed through `Game::damage_player` against the
ability's controller, so prevention shields and damage triggers see real damage (the painland
`self_damage` cost rider is modeled as life loss and stays as-is). *Cards:* ashes_to_ashes.

_Landed 2026-07-17: `Effect::DealDamageToSelf { amount }` resolves as real damage against the
ability's controller — mirrors `DealDamage`'s `Target::Player` arm (`Event::LifeChanged` +
`Event::DamageDealtToPlayer` marker), NOT `Game::damage_player`, which is combat-only (commander
damage/Inkshield shields) and would misroute a sorcery's damage. ashes_to_ashes switched from
`lose_life` to `deal_damage_to_self`, `approximates`/ponytail deleted, fully faithful. Still blocked:
nothing (no noncombat player-damage prevention primitive exists yet — a general engine gap, not this
card's)._

### 174. `regeneration-sba-consult` — 2 cards + pool regenerators, S
Depends on: landed RegenerateShield/Event::Regenerated machinery (per-card exotic
"regeneration", whose follow-up names exactly this).
At the CR 704.5g lethal-marked-damage SBA destroy, add the same shield guard DestroyTarget
resolution carries: shield present → consume one, emit `Event::Regenerated` (tap, remove
from combat, heal marked damage) instead of the graveyard move. Mirror
`Permanent::regeneration_shields` into the snapshot. Clears the `effects.rs:3013` and
`effect.rs:967` ponytails — lethal combat damage is the primary destroy a regenerator must
survive, and this deck's regenerators activate their own shields. *Cards:*
twisted_abomination, golgari_grave_troll.

_Landed 2026-07-18: `check_state_based_actions`'s CR 704.5g lethal-marked-damage destroy now
consults `Permanent::regeneration_shields` the same way `DestroyTarget` does — a positive-toughness
creature with a shield present is regenerated (tapped, removed from combat, damage healed, one
shield consumed) instead of moving to the graveyard; CR 704.5f's 0-toughness death is unaffected
(not a "destroy", not shield-replaceable). The `effects.rs`/`effect.rs` ponytails naming this gap
are deleted. No snapshot layer exists in this engine (`check_state_based_actions` reads live `Game`
state directly), so the brief's "mirror into the snapshot" step was a non-issue. `twisted_abomination`
authored faithfully: `{B}: Regenerate this creature.` (`regenerate_shield`, `target = "this"`) and
`Swampcycling {2}` (`hand_ability` + `search_library` with `land_with_subtype = ["Swamp"]`, the same
shape Noble Templar/Shoreline Ranger already use) — no `approximates` needed. `golgari_grave_troll`
also authored faithfully: contrary to this section's own note above (double-checked, wrong), the
real Golgari Grave-Troll *does* regenerate — "This creature enters with a +1/+1 counter on it for
each creature card in your graveyard.", "{1}, Remove a +1/+1 counter from this creature: Regenerate
this creature.", "Dredge 6" — all three landed already (`enters_with_counters` with a
graveyard-scoped `per_permanent` count, `remove_counters` on the activation cost, `regenerate_shield`,
and the existing `dredge` top-level field), no new engine surface. No `approximates` needed.
Depends on: #73 multi-target clause machinery (landed).
A `count` target-count field on `ReturnFromGraveyardToHand` mirroring `return_to_hand`'s,
consumed with `target = { card_in_graveyard = { whose = "yours", filter = "land" } }` and
`count = { max = 3 }`. *Cards:* life_from_the_loam.

_Landed 2026-07-17: `count: TargetCount` (`#[serde(default)]` = `{1,1}`) added to
`ReturnFromGraveyardToHand`, wired through the existing `target_count()` multi-target machinery — no
`effects.rs`/`de.rs` resolve changes needed (the generic per-target expansion already handled it), a
`DSL_REFERENCE` `count` note added. life_from_the_loam authored: the "return up to three target land
cards" clause is fully faithful. Still blocked: Dredge 3 trimmed with an `approximates`/ponytail note
citing #200 dredge (replacement-draw keyword unlanded)._

### 176. `opponent-graveyard-target` — 1 card, S
Depends on: card_in_graveyard target spec (landed).
An `"opponents"` arm on the `card_in_graveyard` `whose` enum — legal targets restricted to
graveyards of the chooser's living opponents. *Cards:* nezumi_graverobber.

_Landed 2026-07-17: `GraveyardScope::Opponents` (serde `"opponents"`) + its legal-target arm in
`query.rs`'s `CardInGraveyard` match — scoped to living players other than the chooser (excludes the
chooser and eliminated players, CR 800.4a), plus a `DSL_REFERENCE` `whose` note. Pure engine/DSL arm;
no pool card consumes it yet. Still blocked: nezumi_graverobber awaits #201 flip-cards XL to author the
front face that uses `whose = "opponents"` — #201 now unblocked on this prereq._

### 177. `discard-activation-cost` — 1 card, S
Depends on: mill_self/exile_self cost-field pattern (#82, landed).
A `discard_cost = N` activated-cost field: the activating intent names N cards from the
activator's hand, rejected if short (CR 602.2b), discarded on activation (firing
you_discard watchers). Mirrors the sacrifice cost's name-your-payment shape. *Cards:*
wild_mongrel.

_Landed 2026-07-17: `ActivationCost::discard_cost` (u8) plus a `discard_cost: Vec<ObjectId>`
slot on `Intent::ActivateAbility`/`WireIntent::ActivateAbility`/`Intent::TakeAction` (the
Cast-side `TakeAction.discard_cost` slot doubles for activation, since a given action id is
never both). Validated (count, distinct, in-hand) and paid via the normal discard choke
(`Event::MovedToGraveyard` + `Event::Discarded`, so `you_discard` watchers fire) in
`Game::activate_ability`. wild_mongrel authored: the discard cost and +1/+1-until-EOT pump are
faithful; "becomes the color of your choice until end of turn" is trimmed with an
`approximates` note citing #196 color-set-until-eot._

### 178. `nonmana-buyback-cost` — 1 card, S
Depends on: #149 buyback (landed); #92 Intent::Cast sacrifice_cost validation (landed).
Let `[cost.additional.buyback]` carry a `sacrifice = { filter }` rider beside/instead of
mana pips; opting into `bought_back` requires naming one matching permanent in the cast
intent's existing sacrifice slot, validated and paid like the optional additional-sacrifice.
The `bought_back` return-to-hand fork is untouched. (`mana.rs:188`/`cast.rs:227` buyback
ponytails stay literally true.) *Cards:* constant_mists.

_Landed 2026-07-18: no new type needed — `[cost.additional.buyback]` is already a `[cost]`-shaped
table, so it already carried its own nested `[cost.additional.buyback.additional]` `sacrifice`
rider (the existing `SacrificeAdditionalCost`). `Game::cast_cost` (`cast.rs`) now folds that rider
into the returned cost's `additional.sacrifice` when `bought_back` is chosen, so
`Game::validate_cast_cost_picks` and the existing `sacrifice_cost` pay loop in `Game::cast` require
and pay it exactly like any other additional-sacrifice cost — no parallel validation/payment path.
constant_mists (sth) authored faithful, no `approximates`. Still blocked: nothing — ponytail notes
the fold overwrites rather than merges with a base spell's own separate `additional.sacrifice`
(no pool card combines both yet)._

### 179. `cast-only-during-combat` — 1 card, S
Depends on: Game::cast legality checks (landed).
A `cast_only_during_combat` CardDef flag checked in the cast legality guard: legal only
between begin-combat and end-of-combat, mirroring the sorcery-speed timing gate. *Cards:*
cauldron_dance (with #197).

_Landed 2026-07-18: added the Copy-preserving `bool` field `CardDef::cast_only_during_combat`
(de.rs `deny_unknown_fields` key + all CardDef literals) and a `Step::is_combat()` helper
(types/card.rs) covering the five combat steps. `Game::cast_timing_ok` (playable.rs) guard-returns
`false` when the flag is set and the step isn't combat — layered on top of, not instead of, the
ordinary instant/sorcery-speed gate. Tests: `cast_only_during_combat_rejects_in_main_phase`,
`cast_only_during_combat_allows_in_combat`. cauldron_dance's TOML carries the flag; its ability
authoring lands with #197. Still blocked: nothing._

### 180. `creature-type-lexicon` — deck-wide, S
Depends on: nothing.
Extend `CREATURE_TYPES` (`stack.rs:1364` ponytail: "the pool's own creature types") with the
deck's new printed types — Kavu, Imp, Plant, Avatar, Treefolk, Mutant — plus Boar (pool
Krosan Tusker already prints it), so choose-a-creature-type choices can name them. *Cards:*
flametongue_kavu, stinkweed_imp, shambling_shell, avatar_of_woe, wickerbough_elder,
twisted_abomination.

_Landed 2026-07-17: `CREATURE_TYPES` extended with Avatar, Boar, Imp, Kavu, Mutant, Plant,
Treefolk (inserted alphabetically, no dupes), so choose-a-creature-type pools name them. Pure
engine const — no DSL surface, no card TOML changes. Still blocked: nothing (const stays
pool-scoped per its ponytail; widen again as new types are printed)._

### 181. `colored-sacrifice-unless-pay-label` — 1 card, S
Depends on: echo (landed).
Render the full Cost (colored pips) in the SacrificeSelfUnlessPay label — Echo {2}{R} is the
colored consumer the `label.rs:1035` ponytail waited for. *Cards:* keldon_vandals.

_Landed 2026-07-17: `Cost::mana_label()` (`mana.rs`) renders full pip text (`{X}`/generic/`{C}`/WUBRG/
hybrid), backed by a new `Color::letter()`; `label.rs`'s `SacrificeSelfUnlessPay` label now uses it and
the generic-only ponytail is deleted. Internal helper, no TOML surface. keldon_vandals' echo `{2}{R}`
labels faithfully (card already carried no `approximates`). Still blocked: nothing._

### 182. `return-this-zone-guard` — regression, S
Depends on: nothing.
`ReturnThisToHand` "finds the source wherever it now lives" (`effect.rs:1884`) — guard on
the source still being in the graveyard, since Nezumi Graverobber can exile it while the
trigger is pending (pulling a card back out of exile is wrong). Regression test with the
Graverobber race. *Cards:* nezumi_graverobber (interaction), existing pool return-this cards.

**LANDED (2026-07-18):** Root-caused in the shared helper, not per-caller. New
`Game::return_this_source(source, allowed: &[Zone]) -> Option<ObjectId>` resolves the live current id
and returns `None` unless its zone is in the effect's allowlist, folding in the pre-existing
`Object::Removed` left-the-game guard. `Effect::ReturnThisFromGraveyardToBattlefield` passes
`&[Zone::Graveyard]` (Nether Traitor, Teacher's Pest always fire with the source already a graveyard
card); `Effect::ReturnThisToHand` passes `&[Zone::Graveyard, Zone::Battlefield]` (covers both its pool
shapes — Angelic Destiny's graveyard death/upkeep trigger and Flickering Ward's battlefield `{W}`
self-bounce). No TOML changed — all consumers (nezumi_graverobber, angelic_destiny, flickering_ward,
nether_traitor, teachers_pest) were already faithful; this was a pure engine race fix. Regression test
`return_this_is_a_noop_if_source_was_exiled_before_it_resolves` (Nezumi exiles the orphaned Aura out of
the graveyard before its pending `ReturnThisToHand` resolves — stays exiled, not returned to hand).

### 183. `dies-means-from-battlefield` — correctness, M
Depends on: nothing; #200 dredge assumes it.
Dies triggers are keyed off `MovedToGraveyard`, which carries no source zone
(`triggers.rs:107`) — a creature discarded (Wild Mongrel), milled (dredge), or tutored to
the graveyard (Buried Alive) must NOT fire Dies. Carry/check a from-battlefield source zone
on the Dies path; regression tests for discard, mill, and search-to-graveyard arrivals of
dies-trigger creatures. *Cards:* protects skullclamp, abyssal_gatekeeper, penumbra_bobcat,
yavimaya_elder, hissing_iguanar, carrion_feeder interactions deck-wide.

_Landed 2026-07-17: root-cause fix with no event/schema shape change — `enqueue_triggers`'s
`MovedToGraveyard` arm, `batch_creature_deaths`, and `queue_enchantment_death_watchers` now gate the
Dies/death-watch paths on `batch_trigger_scratch.permanents_put_into_graveyard_from_battlefield` (the
pre-move live-`Permanent` set the engine already captured for `ThisAuraLeaves`, CR 700.4). No card TOML
changed — the affected pool cards were already correctly authored, just exposed to the engine bug.
Regression tests: `discarded_creature_does_not_fire_dies` (Wild Mongrel discards Yavimaya Elder,
Hissing Iguanar watching — red before), `milled_creature_does_not_fire_dies` (mill uses its own
`Event::Milled`, was never bugged — locked in). Still blocked: search-to-graveyard (#172) untested (no
DSL effect moves a library card straight to graveyard yet), but the gate keys off source-object-kind
so it is correct-by-construction once #172 lands. #200 dredge now unblocked._

### 184. `death-watch-through-player-loss` — 1 card, S/M
Depends on: nothing.
A death simultaneous with its controller's elimination currently suppresses other players'
death-watch (`triggers.rs:114` ponytail) — Hissing Iguanar is exactly the "whenever another
creature dies" watcher the comment defers to. Fire surviving players' death-watch for deaths
coincident with a player loss. Also add the `triggers.rs:3188` cross-controller APNAP
ordering regression test (two controllers triggering off one death). *Cards:*
hissing_iguanar.

_Landed 2026-07-18: captured the dying creature's LKI (`(id, def, owner)`) into a new
`BatchTriggerScratch::dying_creature_lki` at the `apply.rs` `MovedToGraveyard` creature-death choke
(alongside the existing `dying_creature_stats`), so a same-batch `PlayerLost` that tombstones the slot
to `Object::Removed` can't strand the death-watch scan. `triggers.rs`'s `Object::Removed` guard now
looks up that LKI and still runs `queue_watch_death_triggers` for surviving watchers (CR 603.6e), with a
new `include_self: bool` param passed `false` so the dying creature's own Dies/`*IncludingThis` self-arms
stay suppressed per CR 800.4a while cross-controller watches (Hissing Iguanar) fire. The old blanket-
suppress ponytail at `triggers.rs:114` is deleted. `hissing_iguanar.toml` was already faithful — pure
engine fix, no TOML change. Tests: `death_watch_fires_for_survivor_when_owner_is_eliminated` (RED pre-
fix), `dead_creatures_own_dies_still_suppressed_on_owner_loss` (over-correction guard),
`two_controllers_death_watch_orders_apnap` (APNAP lock-in). Still blocked: none for this card.
`queue_enchanted_creature_dies_triggers` (Aura-watches-enchanted, e.g. Hateful Eidolon) has the same
coincident-loss skip and is left alone — no pool card exercises it; follow-up if one appears._

### 185. `player-loss-pending-purge` — correctness, M
Depends on: nothing.
Purge a departed player's pending triggers/choices on PlayerLost (`apply.rs:1987` ponytail:
"no pool card lets a player die with those outstanding") — Phyrexian Arena's upkeep drain
can eliminate its controller mid-upkeep with echo/Genesis/All Hallow's Eve triggers
outstanding. Regression: Arena kills its controller during upkeep with another upkeep
trigger queued; the game continues cleanly. *Cards:* phyrexian_arena.

_Landed 2026-07-18: the `Event::PlayerLost` handler now purges the departed player's outstanding
pending work (CR 800.4a) after the existing object/stack/combat cleanup — `pending_trigger_groups`
by `controller` (the actual reachable gap: a still-queued same-player `TriggerGroup` would otherwise
place a zombie ability referencing a tombstoned source), plus `pending_echo`,
`pending_enter_bonus_counters`, `exile_time_counters` filtered by removed-object, and
`pending_choice`/`pending_sequence`/`pending_spell_finish`/`pending_demonstrate_opponent_copy`/the
four `delayed_triggers` queues scoped to the departed player. `self_exile_time_counters` left alone
with a `ponytail:` (consumed synchronously, never survives a `PlayerLost` batch). The old
`apply.rs` ponytail is deleted. Pure engine hygiene — `phyrexian_arena` needed no TOML change.
Regressions `phyrexian_arena_controller_dies_mid_upkeep_with_pending_trigger` (RED pre-fix) and
`player_lost_purge_is_scoped_to_the_departed_player` (a survivor's own queued trigger still fires)._

### 186. `trample-vs-prevention` — correctness, M
Depends on: nothing.
Prevented combat damage isn't counted as assigned, so a trampler carries the prevented share
to the player (`combat.rs:660`/`combat.rs:668` ponytails) — Brawn grants all your creatures
trample from the graveyard, making trampler-vs-protection (pool Flickering Ward hosts) and
trampler-vs-Phantom Centaur live. Count the prevented share as assigned to the blocker
(CR 615.8/702.19); regression both chokes. *Cards:* brawn, rootbreaker_wurm.

**LANDED (2026-07-18):** `Game::assign_attacker_damage` (combat.rs) now splits the single `dealt`
counter into `assigned` (every amount the lethal-split committed to a blocker, prevented or not) and
`dealt` (only damage that actually lands, still driving lifelink). Trample overflow is
`power - assigned` per CR 510.1c/702.19e; the protection guard and the Phantom Centaur self-shield
guard both add to `assigned` before their `continue`, so a prevented share no longer leaks to the
defending player. Both `combat.rs` ponytails deleted. brawn and rootbreaker_wurm were already
faithful (no `approximates`); regression tests added in game.rs (Spirit-Mantle-protected blocker,
Phantom Centaur blocker) alongside the existing trample-carries-excess test.

### 187. `exile-instead-of-death-as-placed-trigger` — correctness, S/M
Depends on: nothing.
`spawn.rs:151`'s rider skips the graveyard entirely, so Dies triggers and
`permanents_died_this_turn` never fire — but the printed grant (Serra Paragon) is a dies
trigger: the permanent really dies first, and Hissing Iguanar now watches every creature
death table-wide. Model the rider as a real placed trigger (die → then exile from graveyard).
*Cards:* hissing_iguanar × serra_paragon (existing pool).

**LANDED (2026-07-18):** `spawn.rs`'s `graveyard_or_command` no longer redirects a
`serra_recursion`-tagged permanent to exile — it dies for real (battlefield → graveyard, CR 400),
same as any other death. `Game::apply`'s `MovedToGraveyard` choke captures the tag (last-known
information, `BatchTriggerScratch::serra_recursion_deaths`, mirroring `dying_creature_attachments`)
before it tombstones; `Game::enqueue_triggers` fabricates a real CR 603.6 placed trigger off it
(`Trigger::ThisAuraLeaves` reused as the timing — it's the exact oracle condition), landing on the
stack via the normal APNAP `pending_trigger_groups` path — respondable, unlike the old redirect.
Its payoff is a new synthetic `Effect::ExileGraveyardObjectGainLife { object, amount }` (the
graveyard-scoped twin of the existing `ExileObject`, engine-internal only — never TOML-authored),
guard-returning a no-op if the card already left the graveyard by the time it resolves. The old
`pending_serra_lifegain`/`take_serra_lifegain_events` SBA-drain machinery (now dead, since the
rider no longer routes through `Event::MovedToExile`) is deleted. serra_paragon needed no TOML
change (the grant was already authored as `play_from_graveyard_once_per_turn`); hissing_iguanar
is now faithful *in combination* with it too — its "whenever another creature dies" watch sees a
Serra-recursion creature's real death, and `permanents_died_this_turn` is incremented by the same
unconditional `apply.rs` code every other battlefield death already goes through, no separate
wiring needed. Regression tests in game.rs: `serra_paragon_recursion_card_exiles_and_gains_2_on_death`
(updated for the new two-stack-object resolution shape), `serra_paragon_recursion_death_fires_a_death_watch_before_the_exile`
(Hissing Iguanar sees the real death, then the rider still exiles + gains life), and
`serra_paragon_recursion_rider_is_a_noop_if_the_card_already_left_the_graveyard` (an instant response
— Lorehold Charm's reanimate mode — moves the card out of the graveyard before the placed trigger
resolves; no panic, no double-exile, no life gain).

### 188. `recover-keyword` — 1 card, M
Depends on: functions_in_graveyard death scan (landed); echo's pay-or-consequence
PendingChoice (landed).
`CardDef::recover = Cost`. When a creature is put into its owner's graveyard from the
battlefield, each recover card already in that graveyard queues a pay-or-exile choice
(echo's pay-or-sacrifice shape retargeted): pay moves the card graveyard→hand, decline
exiles it (CR 702.59a). *Cards:* grim_harvest.

**LANDED (2026-07-18):** `CardDef::recover: Option<Cost>` (`[recover]` TOML key, same `[cost]`-table
shape as `[echo]`). A creature's death from the battlefield now queues every recover card already in
that owner's graveyard (reusing #183's from-battlefield gate) into `Game::pending_recover`, drained one
at a time — after Echo's queue — into a `PendingChoice::PayRecoverOrExile` (`Game::pay_recover`): pay
settles mana and returns the card graveyard→hand, decline exiles it. CR 702.59a's "N simultaneous deaths
→ only the first trigger has effect" falls out of the drain's zone guard (a card already gone is silently
skipped). Wire projection (`PendingChoiceView::PayRecoverOrExile`) lands engine/wire-only. grim_harvest
authored fully faithful, no `approximates` (verified against Scryfall: Instant `{1}{B}`, Recover `{2}{B}`).
Still blocked: client pay-or-exile form (Phase 5 client catch-up — answerable over the wire today).

### 189. `fixed-count-sacrifice-cost` — 1 card, M
Depends on: #92 additional-cost-sacrifice (landed, optional one_or_more shape); flashback
(landed).
A mandatory `count = N` form on AdditionalCost's sacrifice shape: the cast intent must name
exactly N distinct matching permanents or the cast is rejected (CR 601.2f/602.2b), wired
through the flashback cast's cost assembly so `[flashback.additional]` accepts it
("Flashback—Sacrifice three creatures"). *Cards:* dread_return.

**LANDED (2026-07-18):** `AdditionalCost::sacrifice` is now `Option<SacrificeAdditionalCost>` (was
`Option<PermanentFilter>`) — a `{ filter, count: SacrificeAdditionalCostCount }` pair, with
`SacrificeAdditionalCostCount::{OneOrMore, Exactly(u8)}` (`types/mana.rs`). TOML `sacrifice = {
count = "one_or_more", filter }` still spells the optional shape (Plumb the Forbidden, unchanged);
`sacrifice = { count = N, filter }` spells the new mandatory fixed-N shape (`de.rs`'s
`RawSacrificeCount` untagged enum picks string-vs-integer). `playable.rs`'s `sacrifice_valid` check
gates `Exactly(n)` on `inputs.sacrifice_cost.len() == n` in addition to the existing
distinct/control/filter checks it already ran for the optional shape — no separate validation path.
Routes through the flashback cast for free: `cast_cost`'s `base = def.flashback...` already carries
`base.additional` (including the new sacrifice shape) into the priced `Cost`, so
`[flashback.additional] sacrifice = { count = 3, filter = "creature" }` needed no cast.rs/cast_cost
changes. `dread_return.toml` authored faithful, no `approximates` (verified `{2}{B}{B}` against
Scryfall — the brief's `{2}{B}` sketch undercounted a pip). 6 new tests in `game.rs`: flashback with
exactly 3 (succeeds, sacrifices, reanimates, exiles on resolve), too few (2), too many (4), a
duplicate, a non-creature (all four rejected `CannotPayCost`), and the front-face `{2}{B}{B}` cast
from hand needing no sacrifice at all.

### 190. `nonmana-cycling-cost` — 1 card, M
Depends on: cycling/hand_ability (landed); ActivationCost sacrifice shape (#78, landed).
Let cycling (and `hand_ability.cost`) carry the ActivationCost-style sacrifice field;
Intent::Cycle names the sacrificed permanent, validated like an activation's sacrifice pick
(CR 702.29b — cycling is an activated ability) and paid before the discard+draw goes on the
stack. The `effect.rs:3933` `at_most` ponytail can be re-pointed here or the sibling added —
the front half already parses as the negated `you_control_lands at_least = 5`. *Cards:*
edge_of_autumn.

_Landed 2026-07-18: added scalar `CardDef::cycling_sacrifice: SacrificeCost` (reusing the existing
`SacrificeCost` enum/TOML shape, `#[serde(default)]` → `None`) and threaded `sacrifice: Option<ObjectId>`
through `Intent::Cycle`, `Game::cycle`, `MeaningfulAction::Cycle`, and the `WireIntent::Cycle` projection
(`#[serde(default)]`, expand-only wire-compatible). Extracted `validate_sacrifice_cost` /
`pay_sacrifice_events` out of `activate_ability` so cycling reuses the exact same activation-sacrifice
validation/payment choke (validated up front per CR 118.9, paid before the discard). The brief's cast-side
sacrifice sketch was wrong: real oracle has NO cast-side sacrifice — the front half is a plain resolve-time
conditional, so added `Condition::YouControlLandsAtMost { at_most }` (the sibling the old `you_control_lands`
ponytail called for) driven by existing `Effect::Conditional`; that ponytail is deleted. `edge_of_autumn`
authored fully faithful (no `approximates`). Tests: `edge_of_autumn_cycling_requires_naming_a_land`,
`edge_of_autumn_cycling_pays_sacrifice_then_discards_and_draws`,
`edge_of_autumn_searches_a_basic_land_when_controlling_four_or_fewer_lands`,
`edge_of_autumn_does_nothing_when_controlling_five_or_more_lands`. Still blocked: none._

### 191. `reflexive-pay-to-copy` — 1 card, M
Depends on: mint_spell_copies + CR 707.10c retarget (landed); cross-player pay-choice
shapes (#102/#107/#153, landed).
Chain Lightning's chain: after the damage step in the same Sequence, pause the damaged
player (or damaged permanent's controller) on a pay-{R}{R} MayPay choice; on payment, mint
one copy under THAT player via mint_spell_copies with the standard retarget pause. The copy
chains naturally since it resolves the same rider. *Cards:* chain_lightning.

_Landed 2026-07-18: new `Effect::MayPayToCopyThis { cost, count }` (types/effect.rs) reads the
enclosing Sequence's shared target to find the payer (player target pays themself; permanent
target's controller pays via `Game::controller_of`), then raises `PendingChoice::PayCost`
directly. `Game::pay_optional_cost` (pending/handlers.rs) special-cases a `CopyThisSpell`
payload to mint inline under the PAYER via `mint_spell_copies` (mirroring `answer_may`'s
existing optional-copy shape) instead of placing a fresh ability — this also means the mint
bypasses `CopyThisSpell`'s own "is this spell itself a copy" storm-guard, so a minted copy's
own resolution correctly re-offers the same rider and the chain continues with zero dedicated
bookkeeping. chain_lightning.toml authored (dmr printing, oracle verified against Scryfall —
"may choose A NEW TARGET for that copy," singular, matching CR 707.10c's ordinary retarget, not
the brief's plural). Faithful, no `approximates`; one `ponytail:` collapsing the oracle's two
"may"s (may pay, then separately may copy) into one pay-mints-unconditionally step, since
declining to copy after paying is never distinguishable from never paying. 6 new tests in
game.rs cover the baseline hit, pay-and-copy, decline, a 2-link chain, an unaffordable payer,
and the permanent's-controller-pays half._

### 192. `conditional-free-cast` — 1 card, M
Depends on: #86 free-cast permission plumbing (landed).
A top-level `free_cast_if = <Condition>` table: Game::cast accepts a cast-free intent flag
when the condition holds at cast time (CR 118.5). Needs an
`opponent_controls_lands_with_subtype` Condition arm plus an `all` (AND) composition with
the you-scoped arm — the `effect.rs:3133` "no combinators" ponytail dies here ("If an
opponent controls a Plains and you control a Swamp"). *Cards:* massacre.

**LANDED (2026-07-17):** `CardDef::free_cast_if: Option<Condition>` gates `Game::cast_cost` to
`Cost::FREE` when it holds (checked with `TriggerContext::of(player)`, same "always take the
strictly-better option" modeling `Condition::HandHasLandWithSubtype` already uses — no decline
pause). New `Condition::OpponentControlsLandsWithSubtype { subtypes, count }`, the opponent-scoped
twin of `ControlsLandsWithSubtype` (holds when *some* living opponent individually meets the
threshold). `Condition::All` (the AND combinator) turned out to already be landed, generically, for
Zimone, All-Questioning — the `effect.rs:3133` "no combinators" ponytail was stale prose on
`Effect::Conditional`'s own doc comment (trimmed) rather than a missing `Condition` arm; no new
combinator code was needed. massacre authored fully faithful, no `approximates` (real oracle cost
is `{2}{B}{B}`, not the brief's `{B}{B}` — verified against Scryfall).

### 193. `combat-damage-to-creature-trigger` — 1 card, M
Depends on: deals_combat_damage_to_player trigger plumbing (landed).
A `deals_combat_damage_to_creature` trigger arm scanned at the creature-combat-damage choke,
threading the damaged creature through TriggerContext (like dying_enchanted_creature), plus
a `destroy_triggering_damaged_creature` payoff reading it (no-op if it already left,
CR 603.10a). *Cards:* stinkweed_imp (#194 wants the same context slot).

**LANDED (2026-07-18):** `Trigger::DealsCombatDamageToCreature` (fieldless, self-scoped) fires off a new
`Event::CombatDamageDealtToCreature` pushed alongside `DamageMarked` at both combat-damage-to-creature
chokes (`assign_attacker_damage`'s per-blocker loop and `deal_creature_damage`'s `combat = true` path) —
never on `fight`'s noncombat path. The damaged creature rides `TriggerContext::damaged_creature`;
`Effect::DestroyTriggeringDamagedCreature` reads it (ordinary destroy — indestructible ignores, regen
shield replaces; no-op if it already left, CR 603.10a). Bug found + regression-tested: the enqueue arm's
`owner_of(source)` panicked when a blocker traded lethal combat damage with its attacker in the same SBA
sweep that dropped its own controller to 0 (CR 800.4a) — guarded with the same `Object::Removed` check as
the `Dies` path. `VisibleEvent::CombatDamageDealtToCreature` mirror added (public, CR 510.4). stinkweed_imp
authored fully faithful, no `approximates`. Still blocked: nothing for this card (#194 needs its own
turn-scoped damage set, not this single-fire slot).

### 194. `damaged-by-memory-death-watch` — 1 card, M
Depends on: #193 (damage context, generalized to all damage this source deals).
A turn-scoped damaged-by set: at every creature-damage choke record (source, victim); a
`creature_dealt_damage_by_this_dies` trigger arm checks the dying creature against the
source's set at the death choke (CR 603.10a LKI), payoff `put_counters target = "this"`.
Cleared with the other turn tallies. *Cards:* vampiric_dragon.

**LANDED (2026-07-18):** `Trigger::CreatureDealtDamageByThisDies` (fieldless, self-scoped) fires at both
creature-death chokes (`MovedToGraveyard`'s from-battlefield creature arm and `TokenCeasedToExist`'s
creature arm) for every source whose `Game::damaged_this_turn` tally recorded the dying object as a
victim. The tally itself is a single `Vec<(source, victim)>` populated at the one shared
`Event::DamageMarked { source: Some(_), .. }` arm in `enqueue_triggers` — already the common choke behind
both combat creature-damage (`assign_attacker_damage`/`deal_creature_damage`) and noncombat creature
damage (`fight`/`Effect::DealDamage`), so no new emitter call sites were needed. Cleared alongside
`permanents_died_this_turn` at every Untap step. Same `Object::Removed` guard as #193/`Dies` (a source
eliminated in the same SBA sweep as its victim doesn't get its own trigger). `vampiric_dragon` authored
fully faithful (Flying, the death-watch, and its own `{1}{R}: deal 1 damage to target creature` ability
that populates the tally) — no `approximates`. Verified oracle/cost/P-T against the live Scryfall API
(`{6}{B}{R}` 5/5, oracle id `e8941499-2b31-417b-b2d2-10a144826703`), not memory.

### 195. `minus-one-counters` — 1 card, M
Depends on: #75 counter-kinds (landed).
A `MinusOneMinusOne` CounterKind wired into P/T computation beside plus-counters, accepted
by `enters_with_counters` kind and `remove_counters_kind`. ponytail: skip the CR 704.5r
+1/+1↔-1/-1 annihilation SBA until a card can observe both kinds on one creature, ceiling
noted at the P/T site. *Cards:* wickerbough_elder.

**LANDED (2026-07-18):** `CounterKind::MinusOneMinusOne` added (COUNT→7, in `ALL` so it
proliferates / moves / removes generically). `Game::pt_layers` (characteristics.rs) emits a
`PtDelta` subtracting `kind_counters[MinusOneMinusOne]` from power/toughness, sibling to the
plus-counters layer; `label.rs` gets the `"-1/-1"` arm. `enters_with_counters` and
`remove_counters_kind` needed no code change (already generic over `CounterKind`). wickerbough_elder
authored fresh, fully faithful, no `approximates` (real card is `{3}{G}` 4/4 Treefolk Shaman — the
brief's quoted cost/type was stale; verified against Scryfall). Still blocked: CR 704.5r's
+1/+1↔-1/-1 annihilation SBA (ponytail-noted at the `pt_layers` site — add when a pool card puts both
kinds on one creature). 3 engine tests in game.rs (P/T reduce, restore on removal, wickerbough
integration).

### 196. `color-set-until-eot` — 1 card, M
Depends on: five-color choice pause machinery (landed); #169 (colors_of override plumbing
shared).
`set_own_color_until_end_of_turn`: pause on the existing color choice, record an until-EOT
color SET consulted by `Game::colors_of` ahead of derived/added colors (CR 613.3c layer-5
set, unlike `add_colors`' union), cleared at cleanup. Makes color runtime state — the
`filter.rs:648` "exact for every pool card" claim retires. *Cards:* wild_mongrel.

**LANDED (2026-07-18):** New `Effect::SetOwnColorUntilEndOfTurn` reuses the existing
`PendingChoice::ChooseColor` picker (added an `until_end_of_turn: bool` so the same wire prompt
now answers into either Flickering Ward's indefinite `Permanent::chosen_color` or a new
until-end-of-turn `Permanent::set_color_eot`). `Game::colors_of` consults `set_color_eot` first,
short-circuiting with an exact single-color return (a CR 613.3c layer-5 SET, replacing rather
than unioning the derived/added colors) before falling through to cost-pip/`added_colors_eot`
derivation; cleared alongside the other until-EOT boosts at `Event::TempBoostsEnded`
(`priority.rs`'s cleanup scan and `apply.rs`'s handler both updated). New
`Event::ColorSetUntilEndOfTurn` (engine + schema `VisibleEvent` + `redact_for` projection).
wild_mongrel.toml is fully faithful — both the `ponytail` and `approximates` notes deleted. The
`filter.rs:648` "exact for every pool card" comment trimmed to describe the runtime override.
Regression tests: `wild_mongrel_discard_cost_becomes_the_chosen_color_until_end_of_turn` (SET
replaces green, doesn't union), `wild_mongrel_color_choice_reverts_at_cleanup`. Two pre-existing
tests (`wild_mongrel_discard_cost_pumps_until_end_of_turn`,
`discarded_creature_does_not_fire_dies`) updated to answer the now-real color-choice pause that
Wild Mongrel's ability raises. Files: `crates/engine/src/{types/{effect,stack,card,filter}.rs,
effects.rs, characteristics.rs, apply.rs, priority.rs, label.rs, pending/{mod,handlers}.rs}`,
`crates/schema/src/{event.rs, projection/{event,choice}.rs}`, `crates/cards/data/wild_mongrel.toml`,
`crates/engine/tests/game.rs`, `.agents/skills/card-dsl/DSL_REFERENCE.md`.

### 197. `cauldron-dance-package` — 1 card, M
Depends on: #179; fire_delayed_triggers Step::End drain (#74, landed);
Event::ReanimatedToBattlefield read-back (landed); delayed SacrificeObject (#74/#88, landed).
Two effect shapes: (a) delayed return-to-hand of the just-reanimated creature at the next
end step — read the ObjectId off this resolution's own ReanimatedToBattlefield event and
schedule a ReturnToHandObject payload, exactly how ScheduleReturnThisAuraAttachedToReanimated
reads back and schedules, guard-return if it already left; (b) `PutCreatureFromHand` — the
creature sibling of put_land_from_hand: optional pick over matching hand cards, deploy via
the normal enter-battlefield choke, grant keywords, schedule a SacrificeObject at Step::End
(the token-copy cleanup machinery verbatim). *Cards:* cauldron_dance.

_Landed 2026-07-18: added `Effect::ScheduleReturnReanimatedToHand` (reads back this resolution's own
`Event::ReanimatedToBattlefield`, grants haste via an until-EOT `TempBoost` — exact here since the
creature always leaves the battlefield this same end step — then schedules the new
`Effect::ReturnObjectToHand` delayed payload at `Step::End`, mirroring
`ScheduleReturnThisAuraAttachedToReanimated`'s read-back-then-schedule shape) and
`Effect::PutCreatureFromHand` (the creature sibling of `PutLandFromHand`: a new
`PendingChoice::PutCreatureFromHand`/`Intent::PutCreatureFromHand` pair — carrying `source` so the
answer can schedule its own delayed `SacrificeObject` at `Step::End` — offering the controller's hand
creature cards; on acceptance it deploys via the existing `Event::PutOntoBattlefieldFromHand` ETB
choke, grants haste, and schedules the sacrifice). Both effects, `PendingChoiceView::PutCreatureFromHand`,
and `WireIntent::PutCreatureFromHand` are documented/projected through `crates/schema/`. `cauldron_dance`
now authors its full three-clause ability (`reanimate_to_battlefield` →
`schedule_return_reanimated_to_hand` → `put_creature_from_hand`) and is fully faithful — no
`approximates`. Tests: `cauldron_dance_reanimates_with_haste_and_returns_at_end_step`,
`cauldron_dance_reanimated_return_is_noop_if_it_already_left`,
`cauldron_dance_puts_a_creature_from_hand_with_haste_and_sacrifices_at_end`,
`cauldron_dance_hand_put_is_optional_and_empty_hand_is_a_noop`. Still blocked: nothing._

### 198. `storm` — 1 card, M
Depends on: #83 storm-copy-count machinery (landed copy_this_spell/retarget).
A game-wide turn-scoped cast tally (all players), snapshotted onto the Spell at cast
(pre-increment) so "each spell cast before it this turn" is exact, exposed as
`Amount::SpellsCastBeforeThisThisTurn`. The `effect.rs:2423` resolution-rider timing
ponytail is falsified by a targeted storm spell (copies must survive the original being
countered and get per-copy target legality, CR 702.40a triggers on cast) — either put the
copies on the stack from a real cast trigger, or document the exact residual if the rider
shape is kept. *Cards:* reaping_the_graves.

_Landed 2026-07-18: routed Storm through the already-landed `Trigger::YouCastThis` +
`Effect::CopyTriggeringSpell` cast-trigger primitive (a genuinely separate stack object, so copies
structurally survive the original being countered) rather than the `CopyThisSpell` resolution-rider shape.
Added `Amount::SpellsCastBeforeThisThisTurn`, computed as `sum(all players' spells_cast_this_turn) - 1`
and snapshotted into `TriggerContext::spells_cast_before_this` when the `Event::SpellCast` arm places the
trigger (immune to responses and to the storm spell's own later copies), then baked to `Amount::Fixed`
by new `fill_spells_cast_before_this`. Added `CopyTriggeringSpell::last_known_information: bool` (default
`false`) relaxing the "must still be on the stack" guard for CR 702.40a's documented Storm exception;
the existing `SpellCopied` LKI fallback (built for Surge to Victory) needed no change, and Thunderclap
Drake's default-`false` no-op-if-countered behavior is unchanged. `reaping_the_graves` authored fully
faithful ({2}{B} instant, verified against Scryfall — not the sorcery the brief guessed; no `approximates`).
Tests: `storm_copies_for_each_prior_spell_this_turn`, `storm_count_is_zero_as_first_spell`,
`storm_count_snapshot_ignores_responses`, `storm_copies_survive_countered_original`. Still blocked:
`Effect::CopyThisSpell`'s residual resolution-rider timing gap (Ominous Harvest / Plumb the Forbidden)
is out of scope and retained — its ponytail now points at this route._

### 199. `animate-dead-aura` — 1 card, M
Depends on: AttachSelfToReanimated / ScheduleReturnThisAuraAttachedToReanimated machinery
(landed).
Close Animate Dead's intake residual: type it as a real Aura whose resolution reanimates the
enchanted creature card and attaches (the landed reanimate-attach event path), with the
printed "loses/gains enchant ability" self-rewrite still modeled implicitly as staying
attached (that half of the note stays if the literal ability rewrite stays out of scope —
trim, don't delete, unless fully modeled). *Cards:* animate_dead (existing pool).

**LANDED (2026-07-18):** animate_dead retyped `[kind] type = "aura"` (was `"enchantment"`); the "typed as
an enchantment, not an aura kind" half of the `approximates` deleted, the loses/gains-enchant-ability
residual trimmed and kept. As a real Aura, CR 704.5m's orphan-Aura SBA now applies to it. Three guards
made that work: `required_target` checks `enchant_graveyard` ahead of the ordinary `CardKind::Aura` arm;
`resolve_spell` routes an `enchant_graveyard` Aura through a new extracted `resolve_permanent_enter` helper
(same generic entry as Creature/Enchantment — enters unattached, its own ETB `reanimate_to_battlefield` +
`attach_self_to_reanimated` do the attach) instead of the immediate-attach arm; and `check_state_based_actions`
exempts an `enchant_graveyard` Aura from the CR 704.5m orphan sweep for the brief pre-ETB window while its
cast-time target still sits in the graveyard (the sweep runs before the ETB ability is even placed, per
`pipeline.rs`). New test `animate_dead_dies_via_sba_when_host_leaves_first` replaces the old
`animate_dead_no_op_if_host_already_gone` (which asserted the pre-Aura, non-CR-accurate behavior). Still
blocked: the literal "loses enchant creature card in a graveyard / gains enchant creature put onto the
battlefield" text-rewrite is still modeled implicitly as staying-attached (CR 303.4 makes it observationally
identical — note retained).

**Follow-up (2026-07-18):** the trimmed residual is closed — the ETB self-rewrite is modeled
literally: `Permanent::enchant_rewrite_host` records the reanimated object at attach, and
`Game::attachment_host_legal` holds an `enchant_graveyard` Aura to exactly that object (CR 704.5m).
`approximates` deleted; Animate Dead is fully faithful. Tests:
`animate_dead_rewritten_enchant_holds_it_to_the_reanimated_creature`,
`animate_dead_trigger_does_nothing_if_the_aura_leaves_first` (intervening-if, CR 603.4).

### 200. `dredge` — 5 cards, L (takes the wave XL slot)
Depends on: #183 (milled dies-trigger creatures must not fire Dies); functions_in_graveyard
idiom (landed); replacement pipeline precedent (#128, landed).
CR 702.52: a top-level `dredge = N` card field. At every draw choke, scan the drawing
player's graveyard for dredge cards; if any, pause on a per-draw ChooseDredge (decline =
normal draw). Accepting mills N off the owner's library (fewer than N makes the option
illegal, CR 702.52a) and moves the dredger graveyard→hand instead of drawing. A replacement,
not a trigger — no stack item. New PendingChoice + draw-choke fork — state-machine surface.
Slices: (1) the draw-choke fork + ChooseDredge for single draws; (2) multi-draw sequencing
(each draw of "draw three" offers dredge separately); (3) client projection of the choice.
*Cards:* golgari_grave_troll, golgari_thug, stinkweed_imp, shambling_shell,
life_from_the_loam.

**Progress 2026-07-18 (slice 1 of 3 landed — NOT complete):** Built the single-draw dredge fork.
New `dredge = N` top-level `CardDef` field (`Option<u8>`, Copy-safe; TOML key + `de.rs` DTO + DSL
reference). New `PendingChoice::ChooseDredge { player, eligible: Vec<(ObjectId, u8)>, from_draw_step }`
and `Intent::ChooseDredge { player, dredger: Option<ObjectId> }`. Both single-draw chokes fork:
`Effect::DrawCards { count: 1 }` in `run` (effects.rs) and the turn-based draw step (priority.rs).
When the drawing player has an eligible dredger (library ≥ N, CR 702.52a) the engine pauses; accept
mills N via `mill_events` (milled creatures do NOT die — #183) and returns the dredger graveyard→hand
instead of drawing, decline draws normally. `from_draw_step` picks the resume path (advance_step vs
deferred sequence). Minimal schema projection added (`PendingChoiceView::ChooseDredge`, `Answer::Dredge`,
`WireIntent::ChooseDredge`, action-log line). **life_from_the_loam is now faithful** (Dredge 3;
`approximates` + dredge ponytail deleted). 5 engine tests in game.rs (accept / decline / library-too-
small / milled-Dies-regression / natural-draw-step).
**Progress 2026-07-18 (slice 2 of 3 landed — NOT complete):** Multi-draw dredge sequencing (CR 702.52 /
121.2). Shape A (remaining-count on the pause): added `remaining: u8` to `PendingChoice::ChooseDredge`
and a `Game::draw_with_dredge(player, remaining, from_draw_step, events)` helper (zones.rs) that draws
one card at a time — pausing on `ChooseDredge` before any draw the player has an eligible dredger for,
else batch-drawing the rest (no un-milled draw can create a dredger). `Effect::DrawCards { count }` now
calls it for all N (effects.rs, slice-2 ponytail deleted); `answer_choose_dredge` (handlers.rs) resolves
one draw then re-enters `draw_with_dredge` for `remaining - 1`, re-checking eligibility against the
now-live graveyard/library each time (a dredger returned to hand drops out; library shrinking below N
disqualifies it, CR 702.52a). Draw-step path forks `remaining: 1` (draws one, advances the step) —
unchanged. `#183` milled-Dies regression re-verified in the multi-draw path. No new TOML/DSL surface
(the `remaining` field is internal engine state; schema projection reads `{ player, eligible, .. }`
unchanged, so no `card-dsl` reference change). 4 new engine tests (offered-per-draw / accept-then-second-
finds-none / library-drops-below-N mid-sequence / milled-Dies regression). stack.rs slice-1 ponytail
deleted.
**Progress 2026-07-18 (slice 3 of 3 landed — #200 mechanism COMPLETE):** Client rendering of the
ChooseDredge choice. New `ChooseDredgeForm` (prompt-forms.tsx) reusing `CardPickPrompt` (count 1 +
decline), mirroring the MaySacrifice/DeclineUntap "pick one object or decline" precedent: accepting
emits `Answer::Dredge { dredger: Some(id) }`, the "Draw normally" decline emits `dredger: None`. New
`AnswerInput` variant `{ kind: "dredge", dredger }` + `choiceIntent` arm (choice.ts) mapping to
`WireIntent::ChooseDredge`; registry entry `choose_dredge: ChooseDredgeForm` in `FORMS`; `choose_dredge`
added to `FULLSCREEN_KINDS` (promptForm.ts) so the picker isn't double-wrapped in panel chrome. Hint is
generic ("mill this dredger's dredge value") — the wire label carries only the card name (the choice
projection drops the per-dredger mill count); ponytail: widen the projection if a per-dredger count in
the label is wanted. Client tests: 2 `choiceIntent` cases (accept-with-id / decline-null → the right
`choose_dredge` payload) in choice.test.ts, `choose_dredge` added to promptForm.test.ts's kind list +
a fullscreen-chrome assertion. (A runtime FORMS-resolves-to-form check isn't possible here — this
project's vitest has no DOM, so prompt-forms.tsx can't be imported; the `Record<kind, …>` exhaustiveness
is the compile-time gate instead.)
**Residual landed 2026-07-18:** golgari_thug and shambling_shell authored — both fully faithful, no
`approximates`. golgari_thug (rvr, `{1}{B}`, Human Warrior): its real oracle text is dies → tuck a
target graveyard creature card to the top of the library + Dredge 4 (no "can't block" clause — the
increment brief's guessed oracle was wrong; verified against Scryfall). shambling_shell (rvr, `{1}{B}{G}`,
Plant Zombie): its real oracle is "Sacrifice this creature: Put a +1/+1 counter on target creature."
+ Dredge 3 (also not the brief's guessed "gets +1/+1 until end of turn" pump-with-a-sac-a-creature-cost
text). Both compose only already-landed surface: `dredge = N`, `Effect::TuckFromGraveyard { to_top }`,
`Effect::PutCounters`, `SacrificeCost::This`. All 5 dredge pool cards (life_from_the_loam,
golgari_grave_troll, stinkweed_imp, golgari_thug, shambling_shell) are now on disk — #200 fully closed,
mechanism and pool both complete.

### 201. `flip-cards` — 1 card, L (takes the wave XL slot)
Depends on: #176; `[back]` inline-def machinery (landed); morph's def-reveal precedent.
CR 712 Kamigawa flip cards: reuse the `[back]` inline card-table for the flipped face and a
`Permanent::flipped` flag consulted by def_of — like morph's face-down override but one-way
and permanent; counters/attachments/tapped state persist (CR 712.5). A `flip_source` effect
arm sets it, fired from a conditional step gated on the targeted graveyard being empty at
resolution. Slices: (1) flipped def-swap + flip_source; (2) the graveyard-emptiness
conditional gate + Nighteyes' reanimate-from-any-graveyard back face; (3) client projection
(flipped face rendering). *Cards:* nezumi_graverobber.

**Progress 2026-07-18 (slice 1 of 3 landed — NOT complete):** Built the flipped def-swap mechanism.
New `Permanent::flipped: bool` (Copy-safe, runtime state, defaulted `false` in `fresh_permanent`).
`Game::def_of`'s `Permanent` arm now returns `p.def.back.copied().unwrap_or(p.def)` when `flipped`
— the single seam every characteristic accessor (name, types, subtypes, abilities) reads through, so
the whole set flips at once (CR 712). `pt_base` (characteristics.rs) was the one accessor reading
`p.def.kind` directly rather than `def_of`; re-pointed at `def_of(object)` so P/T flips too. New
`Effect::FlipSource` (serde tag `flip_source`, no de.rs change — the `#[serde(tag = "type")]` derive
handles the unit variant; `target()` no-target arm; `label.rs` arm; resolves in `execute_effect`,
guard-returning a no-op if the source isn't a live/unflipped permanent) emits new
`Event::Flipped { object }`, applied in `apply.rs` (sets the flag) with a characteristics-cache
invalidation. Schema projects `Event::Flipped → VisibleEvent::Flipped` (public, straight mirror, no
redaction). No pool card is faithful this slice — proven with a constructed `FLIPPER_FRONT`/
`FLIPPER_BACK` test pair. 3 engine tests in game.rs: `flip_source_swaps_to_back_face`,
`flip_preserves_identity_counters_and_tapped` (CR 712.5 — same ObjectId, +1/+1 counter applied on
top of the back P/T, tapped persists), `unflipped_permanent_uses_front_face` (regression).
Exhaustive-match sites touched: `Effect` (variant + `target()` + `label` + `execute_effect`),
`Event` (variant + `apply` + cache invalidation + schema `VisibleEvent` + projection).
*Slices 2–3 still owe:* slice 2 = the graveyard-emptiness conditional gate on the flip step +
Nighteyes-the-Devourer's reanimate-from-any-graveyard back face + authoring `nezumi_graverobber`
(uses #176's `whose = "opponents"` on its front face); slice 3 = client rendering of the flipped
face. DSL reference updated (`flip_source` effect; `[back]` now doubles as the CR 712 flipped face).

**Progress 2026-07-18 (slice 2 of 3 landed — still NOT complete):** Authored `nezumi_graverobber`
**fully faithful** (both faces). Front (Nezumi Graverobber, 2/1 Rat Rogue, `{1}{B}`) is an `exile_target`
over `card_in_graveyard { whose = "opponents", filter = "any_card" }` (#176) followed by a `conditional`
step gating a `flip_source` on the new `Condition::TargetCardOwnerGraveyardEmpty` — a target-based arm
(serde tag `target_card_owner_graveyard_empty`, no `de.rs` change; unit variant). It reuses existing
target-owner plumbing, adding none: the special-cased `Effect::Conditional` resolve site (`effects.rs`,
alongside `TargetPowerAtLeast`) reads the shared `target`'s owner via `owner_of` (which follows the
exiled card's `Object::Moved` lineage to recover the owner) and checks `graveyard_cards(owner).is_empty()`.
Guard-return-first: no legal target exiled ⇒ `is_some_and` false ⇒ no flip. Back (**Nighteyes the
Desecrator** — legendary 4/2 Rat Wizard, `{4}{B}`; the brief's "Nighteyes the Devourer / 4/2 Demon /
{2}{B}" was wrong, corrected against Scryfall) is pure authoring: `reanimate_to_battlefield` over
`card_in_graveyard { whose = "any", filter = "creature" }`. Exhaustive-match sites touched: `Condition`
(variant + `condition_holds` `=> false` target-based arm + `Effect::Conditional` resolve special-case).
3 engine tests in game.rs: `nezumi_graverobber_flips_when_opponent_graveyard_emptied`,
`nezumi_graverobber_does_not_flip_if_graveyard_still_has_cards`,
`nighteyes_reanimates_a_creature_from_any_graveyard`; slice-1's flip tests still pass. DSL reference
updated (`target_card_owner_graveyard_empty` condition). #182 (`return-this-zone-guard`) can now use the
real `nezumi_graverobber` for its interaction test. *Slice 3 still owes:* client rendering of the flipped
face.

**Progress 2026-07-18 (slice 3 of 3 landed — #201 COMPLETE):** Wired the flipped face to the client.
The only real gap was printing identity: a flip card's `[back]` inline def carries no `id`/`default_print`
(both faces share one physical Scryfall print/image), so a flipped permanent's `ObjectView.card_id`/`print`
went empty and the client's `imageUrlByPrint`/oracle-by-card_id lookups broke. Fix is server-only:
`Game::front_def_of` returns the printed *front* def (ignoring the flip swap), and the `ObjectView` builder
in `snapshot.rs` falls back to it for `card_id`/`print` whenever the live (back) face supplies an empty one
— name/type/P-T still flip via `def_of`, unflipped permanents are a no-op (`front == def`). **Zero client
change:** `board.tsx` already reads `name`/`print`/`card_id` off `ObjectView` and renders art by `print`
(the single Kamigawa image shows both faces, so the front print is correct for the flipped read); no
name-keyed oracle cache. Schema test `a_flipped_kamigawa_permanent_keeps_the_front_printing_identity`
drives Nezumi's real flip and asserts back-face name/P-T + front-face `card_id`/`print`. Follow-up (polish,
not fidelity): a 180° visual rotate cue for the flipped read is unimplemented — left as a client nicety, no
wire field needed.

### 202. `countdown-exile-payload` — 1 card, L (takes the wave XL slot)
Depends on: exile_self_with_time_counters / suspend tick (landed);
mass_return_from_graveyard (landed, your-graveyard-only).
All Hallow's Eve: widen `exile_self_with_time_counters` with an `on_expiry` effects array —
when the last scream counter is removed at the upkeep tick, move the card to its owner's
graveyard and resolve the payload instead of granting the suspend cast permission. Add an
`all_players` scope to `mass_return_from_graveyard` (per-player scan of the existing walk,
APNAP order). Note the counter kind is "scream" (CounterKind widen, routine). Slices:
(1) the expiry-payload hook (M); (2) the symmetric all-players return (S). *Cards:*
all_hallows_eve.

_LANDED 2026-07-17. Slice 1: `Effect::ExileSelfWithTimeCounters` widened with an `on_expiry`
effects array, `CounterKind::Scream` added, and the upkeep tick forks on the last counter — a card
with a non-empty `on_expiry` goes to its owner's graveyard and resolves the payload; an empty
payload still grants the suspend free-cast (Rousing Refrain regression green). Slice 2: added an
`all_players: bool` scope to `Effect::MassReturnFromGraveyard` — when true it scans EVERY player's
graveyard in APNAP order (deterministic id mint) and returns each player's matching cards under
that player's OWN control. NOTE on fidelity: the current Scryfall/Forge oracle is the symmetric
"each player returns all creature cards from their graveyard to the battlefield" (per-owner
control), NOT the errata'd "under your control" the sketch assumed — the resolver and card were
built to the verified current oracle. `all_hallows_eve.toml` authored `{2}{B}{B}` sorcery, fully
faithful (no approximates): its sole effect self-exiles with two scream counters whose `on_expiry`
is `mass_return_from_graveyard { filter = creature, all_players = true }`. Tests
`mass_return_creatures_from_all_graveyards`, `all_hallows_eve_returns_all_graveyard_creatures_on_expiry`;
Replenish (`all_players: false`) and slice-1 tests stay green._
