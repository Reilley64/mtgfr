# Mirror Mastery Engine Increments

Ranked increments for the 25 Section D cards that need engine work. Numbering continues from the highest number in `docs/FIDELITY_BACKLOG.md` (currently #166).

---

## 183. `land-enters-with-counters-static` — 3 cards (Vivid Crag / Vivid Creek / Vivid Grove), S — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** S — the `enters_with_counters` static today only fires in `Game::resolve_spell` (the cast-resolution choke, `crates/engine/src/effects.rs:368`). It does not fire at `Event::LandPlayed` (`crates/engine/src/apply.rs:892`), so a land printed with "enters with N counters" gets none. The three Vivid lands in this deck are authored with an ETB trigger workaround (functionally equivalent; see `crates/cards/data/vivid_crag.toml`'s ponytail note). A future card that needs true as-enters counters on a land (a *slowland*-style "enters with a lore counter" or anything another effect reads between the enter event and the trigger resolution) would need this gap closed by moving the static into the `LandPlayed` apply path.

**Example cards:** Vivid Crag, Vivid Creek, Vivid Grove (all worked around with ETB trigger); future as-enters-counter lands would land here.

**Sketch:** Hoist the `enters_with_counters(def)` block out of `resolve_spell` into a shared `Game::apply_enters_with_counters(perm)` helper called from both the spell-resolution path and the `Event::LandPlayed` apply path. No card TOML change needed for the existing Vivid lands (the ETB workaround is functionally equivalent), but switching them back to the static form is a one-line cleanup.

---

## 184. `storage-land-x-counter-mechanic` — 1 card, M — **LANDED**

**Depends on:** none

**Effort:** M — storage lands (Fungal Reaches and its cycle) print two abilities: "{1}, {T}: Put a storage counter on this land." and "{1}, Remove X storage counters from this land: Add X mana in any combination of {R} and/or {G}." The first is trivial (`put_counters` on a `remove_counters_kind = "storage"` cost). The second needs two new pieces: (a) a variable counter-removal cost — `remove_counters` today is a fixed `u8`; (b) an any-combination-of-N-colors mana credit that lets the controller pick per-mana which of the listed colors each point produces (a `single_color = true`-style lock, but choosing *per point* across multiple colors, not one color for all).

**Example cards:** Fungal Reaches

**Sketch:** Add `CounterKind::Storage` (already trivially extensible). Widen `remove_counters` to accept an `Amount` (or add a sibling `remove_counters_x = true`). For the mana credit, add a `ChooseManaColorPerCredit` pending choice: the controller picks a color from `mana` for each credit in the batch, with X credits and a `[[red, green]]` palette. The `add_mana` effect would carry `mana = [["red", "green"]]`, `count = "x"`, `per_credit_choice = true`.

**Landed (2026-07-22):** the sketch's part (a) held — `ActivationCost::remove_counters_x: bool`
(sibling to the fixed `remove_counters`) plus `CounterKind::Storage`; `ability_activation_gate`
trivially passes the fixed-count check for an X cost (`remove_counters == 0` there), and
`activate_ability` bounds-checks the player's chosen `x` against `counters_of_kind` once X is
known (CR 602.2b). Part (b)'s `ChooseManaColorPerCredit` pending choice was **not needed** —
`Mana::OfColors` (already in the engine for triome-style restricted-set credits) is exactly the
"any combination of these colors, chosen per credit at payment time" primitive the sketch was
looking for. `add_mana`'s existing `mana = [["red", "green"]]` + `repeat = "x"` spelling mints X
`Mana::OfColors([R,G])` credits directly, no new pending choice, no new `Effect`/`Amount` variant.
Fungal Reaches (`crates/cards/data/fungal_reaches.toml`) is fully faithful, all three abilities,
no `approximates`.

---

## 185. `gain-control-of-all-owned-creatures` — 1 card, S — **LANDED**

**Depends on:** none

**Effort:** S — Homeward Path's "{T}: Each player gains control of all creatures they own" needs a new effect that iterates every battlefield creature and reassigns control to its owner. The existing `gain_control` is single-target; there's no `gain_control_all` or owner-revert effect. The mechanic is one tactical purpose (undoing threaten/steal effects across the whole board) so a single dedicated `Effect::RevertAllCreaturesToOwners` is sufficient.

**Example cards:** Homeward Path

**Sketch:** New `Effect::RevertAllCreaturesToOwners` no-arg variant; in resolution, iterate `self.battlefield()`, and for each creature whose `controller != owner`, set controller back to owner (firing whatever observe events the engine uses for control changes). Wire to a `timing = "activated"` `{T}` ability on Homeward Path.

**Landed 2026-07-21:** the sketch held exactly as written — `Effect::RevertAllCreaturesToOwners` snapshots the battlefield, filters creatures where `controller_of != owner_of`, and mints one `Event::ControlGained` per mismatch naming the owner. A fresh `Game::stamp_control_timestamp()` per event (the same one `GainControl` uses) means CR 800.4a timestamp precedence resolves the until-EOT-layer interaction the sketch flagged as a risk for free — no engine change needed there, no residual gap. Homeward Path is fully faithful, no `approximates`.

---

## 186. `put-from-hand-on-top-of-library` — 1 card, S — **LANDED**

**Depends on:** none

**Effort:** S — Brainstorm's "Draw three cards, then put two cards from your hand on top of your library in any order" needs a new effect that moves N chosen cards from the controller's hand to the top of their library in a chosen order. The engine has library→hand (`draw_cards`), library→bottom (`reveal_until`/`look_at_top`), graveyard→hand (`return_from_graveyard_to_hand`), permanent→library (`tuck_permanent_into_library`), but no hand→library effect. Brainstorm is the flagship card; the same effect unlocks Browse, Sylvan Library's pick-2-from-top + put-back-from-hand edge, and any "put N cards from your hand on top/bo" effect.

**Example cards:** Brainstorm.

**Sketch:** New `Effect::PutFromHandOnTopOfLibrary { count: u8 }` (or `target-count` shaped). At resolution, pause on a `PendingChoice::ArrangeHand` over `count` cards from the controller's hand; the chosen order goes on top in that order. Pair with a `draw_cards count = 3` step before it in the same `[[abilities.effects]]` sequence (Brainstorm is one spell with two effects in series).


**Landed (verified 2026-07-22 by inspection, wave 7 planner):** `Effect::PutFromHandOnTop` ships (`crates/engine/src/types/effect.rs:3351`, `types/stack.rs:1275`/`:2735`) and `crates/cards/data/brainstorm.toml` authors `draw_cards count = 3` + `put_from_hand_on_top count = 2` with no `approximates`.
---

## 187. `split-card-two-castable-faces` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — Fire // Ice (and the split-card family) print two independent castable halves on one card. Each half has its own mana cost, type, and oracle effect; in every zone except the stack they're a single object; on the stack they're just the chosen half. The DSL today has modal spells (`modal = true`, modes are abilities of one card with one cost) and double-faced cards (`[adventure]` casts one face from exile then exiles itself, but the adventure shares the card object, not a true split). Neither fits: a split card needs two `[face]` tables, each independently castable from the hand, and a `default_hand_face` for zones that don't track which half is "up."

**Example cards:** Fire // Ice.

**Sketch:** Add `[split.faces]` (or `[face.left]` / `[face.right]`) — each a full `[cost]` + `[kind]` + `[[abilities]]` block. `validate_cast` accepts either face's cost; the unchosen face is invisible on the stack. Hand/graveyard/library display the fused oracle (both halves). Color identity is the union. This is foundational for the entire split/aftermath/exploit family.

**Landed shape (2026-07-22):** smaller than the sketch — no `default_hand_face`, no fused-oracle
synthesis, no `validate_cast` cost fan-out. The **top-level card table is the fused card** (CR
709.4: name `"Fire // Ice"`, the combined `{2}{R}{U}` cost, `type = "instant"`), and the two halves
are inline `[[half]]` card tables parsed exactly like `[adventure]` / `[back]`. Pieces:

- `CardDef::halves: &'static [CardDef]` (one new `Copy`-safe field) + `[[half]]` in `de.rs`.
- `Intent::CastSplitHalf { player, source, half, target, x }` / `Event::SplitHalfSpellCast`, both
  modeled directly on the adventure pair; `Game::cast_split_half` mirrors `cast_adventure`.
- `validate_cast` rejects the fused card outright (CR 709.4a — only a half is ever cast).
- `PlayPermissions::split_halves_on_stack: Vec<(spell, fused CardDef)>` — the same off-`Copy`
  registry shape as `adventure_fronts`. `Game::create_object` swaps the fused def back in whenever a
  card is created *from* a half's spell id, which is the one choke covering resolution, being
  countered, and a tuck alike. (A first attempt used a leaked `split_parent` back-pointer on the
  half's `CardDef`; that could not be made a fixpoint — the restored fused card's own halves lost
  the pointer, so casting the *second* half of a recurred split card left the bare half in the
  graveyard. The registry has no such depth limit; regression test
  `casting_a_second_half_of_the_same_split_card_still_leaves_the_whole_card_behind`.)

No `approximates`. **Client debt:** a split card in hand needs two cast affordances (one per half)
sending `WireIntent::CastSplitHalf { half }`; the catalog projection exposes no per-half costs yet
(the same gap `adventure` already carries).

---

## 188. `color-spent-to-cast-accounting` — 1 card, M — **LANDED (2026-07-22)**

**Landed shape:** premise correction — the accounting already existed. `Spell::spent_colors:
[bool; Color::COUNT]` is written at cast-time payment settlement and `Condition::
ColorWasSpentToCastThis` already read it, but *only* off a resolved permanent
(`as_permanent(source)` — Court Hussar's "unless {W} was spent to cast it"). Firespout asks the
same question from a spell still on the stack, so the resolution-time evaluator in
`crates/engine/src/effects.rs` gained a fallback to the new `Game::spell_spent_colors`
(`crates/engine/src/core.rs`, sibling of `spell_was_kicked`). No new `Effect` variant: the card is
two `conditional` effects each wrapping a `damage_each_creature`, which is exactly the sketch's
"more general `Condition::SpellSpentColor` wrap" alternative. The only genuinely new surface is
`PermanentFilter::with_flying` (`types/filter.rs`, `de.rs`, `query.rs`) — the positive sibling of
the existing `without_flying`, kept as its own bool rather than folded in. `firespout.toml` is
fully faithful, no `approximates`.

**Depends on:** none

**Effort:** M — Firespout's "if {R} was spent to cast this spell ... and if {G} was spent" (and other "spent-color-matters" cards like granted-by-Specters or "color of mana spent to cast") needs the engine to record which colors of mana went into a specific spell's payment. Today's `ManaSpent` events aggregate the total paid but don't carry per-spell color accounting that an effect at resolution can read. The CR-correct hook is CR 601.2h "as it's cast" — record a snapshot of paid colors per spell.

**Example cards:** Firespout.

**Sketch:** Add `Spell::colors_spent: smallvec::SmallVec<[Color; 6]>` populated at cast-time payment. At resolution, the effect reads it: Firespout's two conditionally-fired `damage_each_creature` arms (`amount = 3`, one gated on `colors_spent.contains(Red)`, one on `.contains(Green)`, the first matching "without flying" and the second "with flying"). New `Effect::DamageEachCreature { amount, opponents_only, color_spent_gate: Option<Color> }` (or a more general `Condition::SpellSpentColor` wrap on the existing effect).

---

## 189. `multi-target-per-modal-mode` — 1 card, S — **LANDED (2026-07-21)**

**Landed shape:** kept the existing clause axis (`ChooseSpellTargets { clause }`, already used by
Magma Opus). Hull Breach mode 2 is authored as one ability whose `[[abilities.effects]]` is two
`destroy_target` steps with different target specs (artifact, enchantment) — deserialized into one
`Effect::Sequence`. `Game::modal_target_clauses` (was `modal_multi_target`) reads a chosen mode's
independent clauses via the new `mode_target_clauses`/`modal_clause_ability` helpers
(`crates/engine/src/types/stack.rs`); `validate_modes` defers a multi-clause mode's target(s) to
the post-cast pause chain the same way it already deferred a multi-target mode's; `Game::cast`'s
existing `choose_spell_targets`/`advance_spell_target_clauses` chain answers clause 0 then clause 1
unchanged. Resolution (`crates/engine/src/effects.rs`) gained `modal_clause_steps`, splitting the
mode's `Sequence` into per-clause `(Ability, Target)` steps reading `spell.targets`/
`spell.targets_second` respectively, mirroring the non-modal per-ability clause alternation already
used for Magma Opus. **Key subtlety:** a step's `target()` reports its real spec even when it's a
rider *sharing* the enclosing sequence's one chosen target (Prismari Command's "target player
draws two cards, then discards two cards" — the discard step is also `TargetSpec::Player`, not
`None`) — `mode_target_clauses` distinguishes a shared rider from a genuinely independent clause by
comparing each step's spec against the immediately preceding one; same spec back-to-back folds into
the prior clause, a spec change starts a new one. No wire/proto/schema change — `ChooseSpellTargets`
already carries `clause`. See `.agents/skills/card-dsl/DSL_REFERENCE.md`'s modal section for the
authoring-facing note.

**Depends on:** none

**Effort:** S — Hull Breach's third mode "Destroy target artifact and target enchantment" needs one modal mode (ability) to carry two independent target clauses. Today's modal sugar scopes each `[[abilities]]` block to one shared target list — Quandrix Command and Casualties of War both have one target per mode. The fix is two-pronged: (a) extend the modal target-collection loop so an ability with multiple `[[abilities.effects]]` each carrying their own `target` accumulates a target clause per effect (already supported in `ChooseSpellTargets { clause }`'s `clause` index — `Magma Opus` uses clause 0 and 1 for its two target clauses, but it's non-modal), and (b) thread that through the modal-mode filter so a chosen mode contributes all its target clauses.

**Example cards:** Hull Breach.

**Sketch:** The existing `clause` axis on `ChooseSpellTargets` already handles per-effect target lists for non-modal spells. Modal needs the same: when mode N is chosen, iterate its effects in printed order, each effect with its own `target` contributes a target clause. Hull Breach mode 2 then has two `[[abilities.effects]]` blocks (one `destroy_target target = { permanent = { types = "artifact" } }`, one `destroy_target target = { permanent = { types = "enchantment" } }`) and the engine asks for both targets when mode 2 is selected.

---

## 190. `alternative-cost-with-rider` — 1 card, M — **LANDED (2026-07-22)**

**Landed shape:** essentially the sketch. New `CardDef::alternative_cost: Option<AlternativeCost>`
(`types/card.rs`) where `AlternativeCost { condition: Option<Condition>, rider: &'static Effect }`
— the `rider` is leaked to `'static` like every other nested `Effect` a `Copy` struct holds. Cast
declares it: `Intent::Cast` / `CastInputs` gained `alternative_cost: bool` (the same opt-in shape
`evoked` uses, threaded through `cast_cost` / `cast` / `cast_with_kind` and every call site).
`cast_cost` returns `Cost::FREE` outright when it's declared — CR 601.2f replaces the whole mana
cost, so no reduction/kicker/delve folds on top — and `Game::cast` fires the rider at cast time
alongside the zeroed settlement, not at resolution. `validate_cast_cost_picks`
(`crates/engine/src/playable.rs`) rejects declaring it on a card with no alternative cost *or*
when the printed condition doesn't hold right now (unlike evoke, which has no condition to
re-check). The rider is a new `Effect::OpponentGainsLife { amount }`
(`crates/engine/src/resolution/life.rs`). Wire: `WireIntent::Cast` / `WireIntentCast` gained
`alternative_cost` (proto field 14).

**Residual:** `Game::playable_from_zone`'s `affordable` closure hardcodes `alternative_cost:
false`, so a spell castable *only* via its alternative cost never appears in the enumerated
`LegalAction` list (a direct `Intent::Cast { alternative_cost: true, .. }` submit works and is
tested). This is the identical pre-existing gap `evoked` already has — one shared follow-up, not a
per-card residual, so `invigorate.toml` carries no `approximates` for it. It *does* carry one for
the rider itself: `OpponentGainsLife` always picks the lowest-seat-index living opponent instead of
letting the caster choose which one, which needs a `PendingChoice::ChooseOpponent` to fix.

**Depends on:** none

**Effort:** M — Invigorate's "If you control a Forest, rather than pay this spell's mana cost, you may have an opponent gain 3 life" is a full alternative cost (CR 601.2f, like escalate/kicker/phyrexian-but-not-phyrexian). The DSL today has `free_cast_if` (a binary gate — `{0}` if a condition holds, full cost otherwise) and `flashback`/`escape`/`retrace` (alt costs from graveyard only). There's no general "alternative cost with a rider" mechanism: the alt cost is "an opponent gains 3 life," not a mana cost at all. Force of Will, Snuff Out, and the entire "pitch spell" cycle share this gap.

**Example cards:** Invigorate.

**Sketch:** Add `CardDef::alternative_cost: Option<AlternativeCost>` where `AlternativeCost { condition: Option<Condition>, rider: Option<Effect> }`. The casting UI offers "pay normal cost OR pay alt cost (which fires `rider` at cast time)." `rider` for Invigorate is `{ type = "target_player_gains_life", amount = 3, opponent = true }` (new `Effect::TargetPlayerGainsLife` or reuse `GainLife` with `who = "opponent_of_caster"`). `condition = "you_control_forest"`. The card's hand-display and stack-display show both options.

---

## 191. `counter-to-library-bottom-and-self-tuck` — 1 card, S — **LANDED**

**Depends on:** none

**Effort:** S — Spell Crumple has two distinct gaps: (a) `countered_dest` only accepts `"library_top_or_bottom"` (a player's-choice superset of bottom-only), not `"library_bottom"` (Spell Crumple puts the countered spell on the bottom, no choice); (b) the resolving spell tucks itself to the bottom of its owner's library ("Put Spell Crumple on the bottom of its owner's library"), and there's no `tuck_self_on_resolve` for a resolving instant/sorcery. (a) is one new enum variant. (b) is one new `Effect::TuckSelfOnResolve { to_top: bool }` variant.

**Example cards:** Spell Crumple.

**Sketch:** (a) Add `CounteredDest::LibraryBottom` and route it through `effects.rs`'s countered-dest resolution arm. (b) Add `Effect::TuckSelfOnResolve { to_top: bool }`, fired at the resolution fork where the spell would normally go to the graveyard. Pair with `countered_dest = "library_bottom"` for the rider. Hinder (already in the pool as `library_top_or_bottom`) stays unchanged.


**Landed (verified 2026-07-22 by inspection, wave 7 planner):** `CounteredDest::LibraryBottom` (`crates/engine/src/types/effect.rs:4254`) and `Effect::TuckSelfToLibraryBottom` both ship; `crates/cards/data/spell_crumple.toml` uses `countered_dest = "library_bottom"` + `tuck_self_to_library_bottom`, no `approximates`.
---

## 192. `exile-self-on-resolve` — 1 card, S

**Depends on:** none

**Effort:** S — Vengeful Rebirth's "Exile Vengeful Rebirth" rider needs a resolving instant/sorcery to exile itself on resolution instead of going to the graveyard. Today's `exile_self = true` is an activated-ability *cost* field (not a spell-resolution effect), and `exile_self_with_time_counters` adds suspend/scream counters (wrong mechanic). The fix is a new `Effect::ExileSelfOnResolve` variant, fired at the resolution fork where the spell moves to the graveyard. Distinct from flashback/escape's implicit exile (those are gated by `CardDef::flashback`/`escape` at the cast-from-graveyard choke; this is for any spell that exiles itself on resolve regardless of where it was cast from).

**Example cards:** Vengeful Rebirth, Beacon of Immortality, any "exile ~this~" sorcery.

**Sketch:** New `Effect::ExileSelfOnResolve` (no args). Wire to the same `apply.rs` resolution fork that routes flashback's exile and `exile_self_with_time_counters`. Authoring: one trailing `[[abilities.effects]] type = "exile_self_on_resolve"` on the spell's `timing = "spell"` ability.

**Landed 2026-07-21:** the sketch held exactly as written — `Effect::ExileSelfOnResolve` mirrors `TuckSelfToLibraryBottom`'s shape one-for-one: a `Game::self_exile_on_resolve: bool` scratch flag set in `resolve_misc.rs`, consumed via `std::mem::take` in `finish_instant_sorcery_resolution` to mint `Event::MovedToExile`, and cleared alongside the other two self-move marks in the `spell.copy` early return (CR 707.10a). Vengeful Rebirth authors the rider faithfully, but the card is not fully faithful: "If you return a nonland card to your hand this way, Vengeful Rebirth deals damage equal to that card's mana value to any target" needs two independent single-target clauses on one spell (a graveyard-card target plus an unrelated any-target), which the engine's multi-target machinery (`StackItem::targets_second`) only supports when both clauses are *multi*-target (Magma Opus), not when either is single-target. `vengeful_rebirth.toml` carries a precise `approximates` note naming only that residual clause; the return-from-graveyard and self-exile rider are both faithful. New increment **#203** tracks the residual gap.

**Still blocked:** nothing — Vengeful Rebirth's conditional damage clause landed with **#203** on 2026-07-22 and the card is now fully faithful (`approximates` deleted).

---

## 193. `creature-mana-value-at-least-filter` — 1 card, S

**Depends on:** none

**Effort:** S — Fierce Empath's "search your library for a creature card with mana value 6 or greater" needs a `{ creature_with_mana_value_at_least = N }` card-filter variant. Today only `_at_most` MV filters exist (`creature_with_mana_value_at_most`, etc.). Mirror the existing `_at_most` enum arms and the `CardFilter` deserialization in `de.rs`.

**Example cards:** Fierce Empath (and any "power X+ / mana value X+" tutor).

**Sketch:** Add `CardFilter::CreatureWithManaValueAtLeast(u32)` (and the search-library filter shape), evaluated against `CardDef::mana_value()` in the existing `matches_card_filter` choke.

**Landed 2026-07-21:** the sketch held, with the narrower `u8` bound the existing `_at_most` siblings all use (not `u32` — `CardDef::mana_value()` is cast to `u32` for the comparison, same as every other MV-gated arm). `CardFilter::CreatureWithManaValueAtLeast(u8)` mirrors `CreatureWithManaValueAtMost` one-for-one (`matches`, `label.rs`, `DSL_REFERENCE.md`). Fierce Empath is fully faithful — no `approximates`.

---

## 194. `multi-target-untap` — 1 card, S — **LANDED**

**Depends on:** none

**Effort:** S — Garruk Wildspeaker's +1 "untap two target Forests" needs an `untap_target` effect that accepts a `count` (and a target spec). The DSL today has no untap-target effect at all (`UntapTarget` is not in the reference or the effect enum's multi-target list). Add `Effect::UntapTarget { target, count }`, register it in `target_count()`, and surface an `untap_target` TOML effect.

**Example cards:** Garruk Wildspeaker (+1), any "untap two target [permanents]" planeswalker/clicker.

**Sketch:** New `Effect::UntapTarget { target: TargetSpec, count: TargetCount }`; TOML `type = "untap_target"` with `target = { permanent = { types = "land", subtypes = ["Forest"], controller = "you" } }`, `count = { min = 2, max = 2 }`.


**Landed (verified 2026-07-22 by inspection, wave 7 planner):** `crates/cards/data/garruk_wildspeaker.toml`'s +1 authors `type = "untap_target"` with `count = { min = 2, max = 2 }`, no `approximates`. (The heading's "untap two target Forests" was a misquote — the real card reads "+1: Untap two target lands.")
---

## 195. `amount-source-power-plus-toughness` — **CLOSED — dead variant**

**Closed 2026-07-21:** the increment (and the deck report line it came from) misquoted Magmatic Force's oracle text. Live Scryfall (`https://api.scryfall.com/cards/named?exact=Magmatic+Force`, oracle id `ea73153b-1ecb-4593-a9ec-ce4f54e69824`) reads:

```
At the beginning of each upkeep, this creature deals 3 damage to any target.
```

Not "…where X is this creature's power plus its toughness" — there is no variable-X amount on the real card, so `Amount::SourcePowerPlusToughness` is **not implemented** and should not be added on this card's account. The real text is expressible today with `Trigger::EachUpkeep` (already landed) + `deal_damage { amount = 3, target = "any" }` — no engine gap. Magmatic Force reclassifies to Section C (authoring-only) and is now in the pool (`crates/cards/data/magmatic_force.toml`), faithful, no `approximates`.

---

## 196. `each-player-precombat-main-timing` — 1 card, S — **LANDED**

**Depends on:** none

**Effort:** S — Magus of the Vineyard's real oracle text is "At the beginning of **each player's first main phase**, that player adds {G}{G}" (not "precombat main phase" — the increment heading's wording was stale; verified live Scryfall, oracle id `e6788809-6b34-4701-89af-b422ba35efb2`). Landed as `Trigger::EachPlayerFirstMainPhase` (TOML `timing = "each_player_first_main_phase"`), queued alongside the existing controller-only `Trigger::FirstMainPhase` at the `Step::Main1` choke (`Game::queue_each_player_first_main_phase_triggers`, mirroring `queue_each_draw_step_triggers`). Because the payoff ("**that player** adds {G}{G}") needs to know whose main phase it is, `TriggerContext::active_player` rides along exactly like Howling Mine's `EachDrawStepPlayerDraws`, and `Effect::AddMana` gained a `recipient: Option<PlayerId>` field (context-filled via `fill_add_mana_recipient`, `None` everywhere else — every other `add_mana` ability is unchanged).

**Example cards:** Magus of the Vineyard.

**Landed 2026-07-21 as:** `Trigger::EachPlayerFirstMainPhase` (`crates/engine/src/types/trigger.rs`), `Game::queue_each_player_first_main_phase_triggers` (`crates/engine/src/triggers.rs`), `Effect::AddMana::recipient` + `fill_add_mana_recipient` (`crates/engine/src/types/effect.rs`), the mana-mint arm (`crates/engine/src/resolution/mana.rs`). `crates/cards/data/magus_of_the_vineyard.toml`, faithful, no `approximates`.

---

## 197. `graveyard-card-filter-color-axis-and-second-trigger-target` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — Nucklavee's "Return target instant card ... and target sorcery card" needs two engine pieces: (a) a `colors` axis on graveyard card filters (today `card_in_graveyard = { whose, filter }` has type/subtype/MV but no color — you can't say "red sorcery" or "blue instant"); (b) a second independent target clause for a triggered ability (today a triggered ability carries at most one extra target clause, hardwired to `double_counters_on_target_creatures`). Two `[[abilities.effects]]` blocks both resolve against the single shared target, so you can't pick two distinct cards.

**Example cards:** Nucklavee (red sorcery + blue instant), any "return target instant and target sorcery" wizard.

**Sketch:** (a) Add `colors` to the `CardInGraveyard`/card-filter shape. (b) Generalize the triggered-ability second-target clause to an array, OR add `Effect::ReturnTwoFromGraveyard { target_a, target_b }` with two independent `TargetSpec`s.

**Premise correction + landed shape (2026-07-22):** half of (b) does not exist. Scryfall's oracle
text is **two separate triggered abilities**, not one ability with two targets: "When this creature
enters, you may return target red sorcery card from your graveyard to your hand." / "When this
creature enters, you may return target blue instant card from your graveyard to your hand." So no
second-target-clause work was needed at all. What was needed:

- (a) as written: two new `CardFilter` arms, `sorcery_with_color` / `instant_with_color`, plus their
  `card_filter_label` text.
- A real bug in `Game::place_pending_triggers` / `choose_order`: two simultaneous triggers from one
  source raise `PendingChoice::OrderTriggers`, and the answer path *fabricated* a single merged
  `Ability` from the group, discarding each trigger's own `optional` and `[abilities.cost]`. It now
  keeps the group queued and re-splits its real abilities in the chosen order, so Nucklavee's two
  independent "you may" prompts and targets survive ordering.

---

## 198. `tokens-scaled-by-combat-damage-dealt` — 1 card, M — **LANDED**

**Depends on:** `combat-damage-trigger-controller-draws` (172) — same "this player dealt combat damage to an opponent" routing.

**Effort:** M — Rapacious One's "Whenever it deals combat damage to a player, create X 0/1 colorless Eldrazi Spawn creature tokens, where X is the damage dealt" needs (a) a "dealt combat damage to a player" trigger that fires per combat-damage event and (b) a `create_token` whose `count` is bound to the damage amount of that event. Today `create_token.count` is an `Amount` but no amount binds to the triggering event's damage value (Edric's controller-draws routing exists for *events* but not for token counts).

**Example cards:** Rapacious One (and any "X 0/1 Spawn where X is the damage dealt" Eldrazi).

**Sketch:** Add a `combat_damage_dealt_to_player` trigger and an `Amount::TriggeringEventDamage` (or pass the damage amount into the effect context at the combat-damage choke), then `count = "triggering_event_damage"` on the `create_token` effect.


**Landed (verified 2026-07-22 by inspection, wave 7 planner):** `Amount::CombatDamageDealt` (`crates/engine/src/types/effect.rs:255`) plus `timing = "deals_combat_damage_to_player"` ship; `crates/cards/data/rapacious_one.toml` authors `create_token count = "combat_damage_dealt"`, no `approximates`. Landed without #172 — the dependency note was wrong (this is a self-referential "whenever *this creature*" trigger, not Edric's any-creature routing).
---

## 199. `hand-ability-array-multiple-landcycling` — 1 card, S — **LANDED (2026-07-21)**

**Depends on:** none

**Effort:** S — Valley Rannet has Mountaincycling AND Forestcycling (two discard-cost, hand-activated landcycling abilities). Today `hand_ability` is a single top-level table, not an array — the deserializer reads one `hand_ability` key and drops the second. Either widen `CardDef::hand_ability` to a `Vec<HandAbility>` (deserialized as `[[hand_ability]]`) or add typed `mountaincycling`/`forestcycling` shortcut fields mirroring the existing `cycling` field. The `[[hand_ability]]` array widening is the cleaner general fix (any two-landcycling or multi-type-cycling card).

**Example cards:** Valley Rannet; any card with two landcycling abilities.

**Sketch:** Change `CardDef::hand_ability: Option<HandAbility>` to `Vec<HandAbility>`, update `de.rs` to read `[[hand_ability]]`, update every existing single-`hand_ability` TOML to `[[hand_ability]]` form (mechanically: `[hand_ability]` → `[[hand_ability]]`), and update callers that read `.hand_ability` (iterate instead).

**Landed shape:** `CardDef::hand_ability` is `&'static [HandActivatedAbility]` (leaked slice, `CardDef: Copy`), deserialized from `[[hand_ability]]` (`crates/engine/src/de.rs`, `Vec<HandActivatedAbility>` interned to `'static`). `Game::activate_hand_ability` and `Intent::ActivateHandAbility`/`WireIntent::ActivateHandAbility` gained an `index: usize`/`u32` field selecting the entry (`forecast` always activates at index 0; proto `WireIntentActivateHandAbility.index` field 3, `client/src/wire/types.ts` updated). `Game::push_hand_ability_actions` (was `hand_ability_listable`) offers one `MeaningfulAction::ActivateHandAbility { card, index }` per affordable entry; `schema::snapshot::action_view` disambiguates the two entries' labels by their own effect ("Discard: Search your library for a Mountain or Forest card…" vs the forestcycling one) when a card has more than one. The 7 pre-existing single-entry `[hand_ability]` TOMLs (Chartooth Cougar, Elvish Aberration, Magma Opus, Noble Templar, Shoreline Ranger, Twisted Abomination, Wirewood Guardian) converted mechanically to one-entry `[[hand_ability]]` with unchanged behavior. `crates/cards/data/valley_rannet.toml` is faithful — two `[[hand_ability]]` blocks (mountaincycling index 0, forestcycling index 1), no `approximates`. Tests: `valley_rannet_mountaincycling_fetches_a_mountain`, `valley_rannet_forestcycling_fetches_a_forest`, `chartooth_cougar_mountaincycling_fetches_any_mountain_typed_land` (regression), `magma_opus_hand_ability_*` (regression). `.agents/skills/card-dsl/DSL_REFERENCE.md`'s `hand_ability`/`forecast` rows updated for the array form and `index`.

---

## 200. `search-library-all-players-fan-out` — 1 card, M — **LANDED (2026-07-21)**

**Depends on:** none

**Effort:** M — Veteran Explorer's "When this creature dies, each player may search their library for up to two basic land cards, put them onto the battlefield, then shuffle" needs `search_library` to fan out to every player. Today `search_library` only offers `searcher = "you"` (default) and `searcher = "target_controller"` (Path to Exile). The per-player fan-out idiom exists for *other* effects (`each_player_sacrifices`, `mass_return_from_graveyard { all_players = true }`, `each_player_draws`) but not for library searches. Add `searcher = "all_players"` so each player searches their own library and makes their own picks (a `may`/up-to-N per player, with a `None` choice ending a player's turn at the keyboard).

**Example cards:** Veteran Explorer (and any "each player may search" card).

**Sketch:** Add `searcher = "all_players"` to the `search_library` effect's fan-out enum; the engine already pauses on `PendingChoice::SearchLibrary` per search — chain one per player in turn order, threading the per-player `matches`/`remaining` state.

**Landed 2026-07-21:** the sketch held — `SearchScope::AllPlayers` chains one `PendingChoice::SearchLibrary` per player in APNAP order (`ResolutionFrame::search_fanout` carries the still-to-search queue plus the fixed filter/destination/count template; `Game::continue_search_fanout` pops the next player once the current one's search fully ends, i.e. their own shuffle has happened). Each player's decline ("fail to find") is legal and doesn't abort the others' turns at the search. The existing per-player `private_items` gate in `crates/schema/src/projection/choice.rs` already keyed the `SearchLibrary` projection off the pausing player, so the fan-out inherited correct per-player privacy for free — no opponent ever sees another player's library contents. Veteran Explorer is fully faithful, no `approximates`.

---

## 167. `copy-creature-spell` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**This heading was a misquote — corrected 2026-07-22.** Riku of Two Reflections does **not**
copy creature spells; there is no `Effect::CopyCastCreatureSpell` and none was added. Verified
live against Scryfall (print `716d0b3b-bac9-4fb8-882e-bd6171864043`, oracle id
`f697c78d-7e4f-4320-bfc6-2a25e6d7dc94`) — the real oracle text is:

> Whenever you cast an instant or sorcery spell, you may pay {U}{R}. If you do, copy that spell.
> You may choose new targets for the copy.
> Whenever another nontoken creature you control enters, you may pay {G}{U}. If you do, create a
> token that's a copy of that creature.

**Example cards:** Riku of Two Reflections

**Landed 2026-07-22:** both abilities used entirely existing machinery. Ability one (the
instant-or-sorcery copy) is `Effect::CopyTriggeringSpell { may_choose_new_targets: true, .. }` off
a `cast_spell`/`instant_or_sorcery` trigger — no new code. Ability two ("create a token that's a
copy of *that creature*") is untargeted (CR 603.3b's implicit "that permanent," not a chosen
target): `Effect::CreateTokenCopy` gained a context-filled `entering: Option<ObjectId>` field
(`#[serde(skip)]`, `TargetSpec::None` on the `target` axis in TOML), threaded through
`fill_entering_permanent` at trigger placement exactly like `deal_damage_to_entering_permanent`/
`attach_self_to_entering` already do; resolution reads `entering` in preference to a chosen
target. Both `optional`+cost triggers reuse the generic `PendingChoice::PayCost` "you may pay …
if you do" pause — no new pending-choice or ability-cost plumbing. The existing `token = "nontoken"`
`PermanentFilter` axis (§7) covers "another nontoken creature you control enters" with no new
filter surface. Riku of Two Reflections is fully faithful, no `approximates`. DSL surface change:
`create_token_copy`'s `target` field documented for its `"none"`-on-`permanent_enters` case
(`.agents/skills/card-dsl/DSL_REFERENCE.md`).

---

## 168. `cast-from-exile-permission` — 1 card, L — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** L — new zone-permission subsystem. Exile-with-permission is a per-card tracking state (which cards in exile have "you may cast this" permissions, and for how long — until EOT, this turn, indefinitely). Needs `Game::grant_exile_cast_permission`, a turn-scoped or indefinite expiry, and validation in `Game::validate_cast` (check the exiled card's permission before allowing the cast).

**Example cards:** Intet, the Dreamer

**Sketch:** Intet's "Whenever Intet deals combat damage to a player, you may pay {2}{U}. If you do, exile the top card of your library face down. You may look at that card for as long as it remains exiled. You may play that card until your next turn" needs:
1. Exile-face-down support (cosmetic, no gameplay impact if we treat it as face-up exile for the owner only)
2. Grant cast-from-exile permission with "until your next turn" expiry
3. `validate_cast` check: if casting from exile, verify the card has a valid permission

This is a foundation for future "exile and cast" effects (Prosper, Light Up the Stage, etc.). Start minimal: permission is a `HashMap<ObjectId, ExileCastPermission>` on `Game`, cleaned at turn boundaries or when the card leaves exile.

**Premise correction + landed shape (2026-07-22):** the Sketch quotes the wrong oracle text twice.
Live Scryfall reads "You may **play that card without paying its mana cost** for as long as **Intet
remains on the battlefield**" — not "until your next turn", and not at normal cost. That makes this
S, not L: the whole impulse-draw subsystem (`Effect::ExileTopMayPlay`, `PlayPermissions::
play_from_exile`, `Game::may_play_from_exile`, the exile-zone branch of `Game::playable_zone`)
already existed and was reused wholesale. Deltas:

- Two `serde(default)` flags on `Effect::ExileTopMayPlay`: `face_down` (CR 701.9; the existing
  `Card::face_down` + wire redaction already covers "you may look at that card") and
  `free_while_source`.
- One new registry, `PlayPermissions::play_from_exile_free_while_source: Vec<(card, player,
  source)>`, with **no** cleanup expiry — `Game::may_play_from_exile_free_while_source` requires the
  source to still be a permanent, so the duration is read live and a stale entry stops matching by
  itself. `Game::cast_cost` returns `Cost::FREE` for it (CR 118.5).
- Deliberately *not* folded into `Game::may_cast_from_exile_free`: that permission also waives
  timing (CR 601.3e), while Intet's is an ordinary "you may play that card" and keeps normal timing.

No `approximates`, and no client wire debt — the permission is engine bookkeeping the client already
reads back as playability.

---

## 169. `retrace-validation` — 1 card, S — **LANDED**

**Depends on:** none

**Effort:** S — retrace is already in the DSL (`CardDef::retrace`, CR 702.83), but has never been authored in the pool. Validate that the existing implementation works (cast from graveyard by discarding a land card as additional cost), write the first retrace card.

**Example cards:** Call the Skybreaker

**Sketch:** Call the Skybreaker has `retrace = true`. The existing `CardDef::retrace` field routes through `[cost.additional]` `discard_land = true`. Confirm `Game::validate_cast` allows graveyard casts with retrace, the discard is required and paid, and the spell returns to graveyard (no exile). If it works, this is authoring-only (reclassify to C); if broken, fix the validation/cost-payment path.

**Landed 2026-07-21:** the graveyard-cast/discard/no-exile mechanics already worked (proven by the pool's other retrace card, Throes of Chaos, from an earlier wave). The drive-out found one real defect: `discard_land`'s additional cost was unconditional, so casting a retrace card **from hand** wrongly demanded a land discard too (CR 702.83a's discard only applies to the graveyard cast). Fixed by threading `Zone` into `Game::cast_additional_cost_gate` and `Game::validate_cast_cost_picks` so the discard-a-land rider only engages when `zone == Zone::Graveyard`. Reclassifies to Section C (authoring + a one-line engine fix, not a new mechanic).

---

## 170. `cost-reduction-scaled-by-source-counters` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — extend `Cost::reduce_own_generic` (which today takes static `Amount` variants) to support reading the casting permanent's own +1/+1 counter count. New `Amount::SourceCounters` variant, resolved at cast time with `source` = the spell's own object (for a creature already on the battlefield casting another spell, this would read that creature's counters; for Animar, the reduction applies to *other* spells, so this is a static ability on Animar reducing other spells' costs scaled by Animar's counters).

**Example cards:** Animar, Soul of Elements

**Sketch:** Animar's "Creature spells you cast cost {1} less to cast for each +1/+1 counter on Animar" is an other-spells cost reducer (`Effect::ReduceSpellCost`, already exists, Tomik uses it) with `amount = { source_counters = true }`. Route `Amount::SourceCounters` → `Game::resolve_amount` reads `Game::counters_on(source)`. Animar also needs protection from white/blue (keywords) and a creature-cast trigger placing a +1/+1 counter on itself (standard trigger + `put_counters` effect).

**Premise correction (landed):** no new `Amount` variant was needed. `Amount::PerCounterOnSource` already exists and `Game::cost_reduction` already calls `resolve_amount` with the reducer permanent itself as the source object — exactly "counters on Animar." `Effect::ReduceSpellCost { filter = "creature" }` (Marauding Raptor) plus `amount = "per_counter_on_source"` (Zimone, Infinite Analyst) already compose to the exact ability. Authoring-only: also confirmed Scryfall's oracle text is protection from **white and black** (not "white and blue" as this doc's Sketch said), a two-entry `keywords = [{ protection = "white" }, { protection = "black" }]`, and the growth trigger is an ordinary `timing = "cast_spell"` / `spell_filter = "creature"` / `put_counters` ability — no engine change, no `approximates` note.

---

## 171. `opponent-prediction-choice` — 1 card, M — **LANDED (2026-07-22)**

**Landed shape:** premise correction — there is no "opponent guesses" here and no
`PendingChoice::OpponentGuessesCard`; every player (CR 101.4 APNAP, including the attacker) names
a card for themselves. The pool's first "name a card" (CR 201.2/201.3, 703.2j). Modeled on the
existing `CouncilsDilemmaVote` fan-out rather than a new mechanism: new
`PendingChoice::ChooseCardName { player, source, remaining: Vec<PlayerId> }` raised by
`pending/raise/fanout.rs::next_card_name` (from `resolution/pause_edict.rs`, seeded with
`Game::apnap_order()`), answered by `pending/handlers/fanout.rs::answer_choose_card_name`, which
reveals the top card (`Event::RevealedTopOfLibrary` — CR 701.30, a reveal is not a zone change),
routes it to hand on a name match and to the library bottom otherwise, then re-prompts the next
seat. New fieldless `Effect::EachPlayerNamesCardThenRevealsTop`; the answer is a new free-text
`Intent::ChooseCardName { player, name }` validated shape-only (trimmed non-empty, ≤200 chars) —
the engine deliberately does *not* check the name against the card pool, since CR 201.3 lets you
name any card, including one not in the game.

**Visibility:** the projection `PendingChoiceView::ChooseCardName { player, source }` deliberately
drops `remaining` and never carries the name — the name is only ever an inbound intent, and the
top card becomes public solely via the `RevealedTopOfLibrary` event that the reveal itself
authorizes. `Event::SearchedToHand`'s existing redaction keeps the card hidden from non-owners on
a hit.

**Client debt:** `PendingChoiceView::ChooseCardName` has no prompt form yet — the client needs a
free-text card-name input wired to `Answer::CardName`. Until then this choice can only be answered
by a direct wire intent, and a Conundrum Sphinx attack will stall a real table.

**Residual:** the fan-out asks one seat at a time, so naming is sequential — a later seat sees an
earlier seat's public reveal before naming. Recorded as `conundrum_sphinx.toml`'s `approximates`;
fixing it needs an everyone-commits-before-anyone-reveals primitive no other pool card wants yet.

**Depends on:** none

**Effort:** M — new `PendingChoice::OpponentGuessesCard` and validation logic. The active player names a card, an opponent guesses yes/no (or the opponent names a card and you validate), then a condition forks based on correctness.

**Example cards:** Conundrum Sphinx

**Sketch:** Conundrum's "Whenever Conundrum Sphinx attacks, each player names a card. Then each player reveals the top card of their library. If the card a player revealed is the card they named, that player puts it into their hand. If it's not, that player puts it on the bottom of their library" is a complex multi-step:
1. Attack trigger
2. Each player names a card (new choice type, free text or card-name selection)
3. Each player reveals their top card
4. Fork: if match → hand, else → bottom

This is the pool's first "name a card" effect. Start with a `PendingChoice::NameCard` (string input), store the named cards in a map, then resolve the reveal + conditional move per player.

---

## 172. `combat-damage-trigger-controller-draws` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — extend `Trigger::CombatDamageDealt` (currently "whenever a creature you control deals combat damage") with a payload routing "that creature's controller draws a card" instead of "you draw." The trigger is templated to fire per attacking creature, but the draw goes to that creature's controller (not the trigger's owner, relevant if the attacker is temporarily stolen).

**Example cards:** Edric, Spymaster of Trest

**Sketch:** Edric's "Whenever a creature deals combat damage to one of your opponents, its controller may draw a card" is a combat-damage watch that fires per attacker dealing damage to an Edric-controller opponent, with the draw going to the attacker's controller. New `Effect::TargetControllerDraws` or route through existing `DrawCards` with `controller = "triggering_creature_controller"`.

**Landed (2026-07-22):** the existing `deals_combat_damage_to_player` watch gained a fourth `who` scope, `"any_creature_damaging_your_opponent"` (`CombatDamageScope`) — the only scope that reads the *damaged* player, firing for any creature's damage as long as it landed on one of the watcher's opponents (CR 102.3). Two new `TriggerContext` fields carry CR 603.10a last-known information out of `Game::queue_combat_damage_triggers`: `combat_damage_recipient` and `combat_damage_source_controller`. The payoff is a new `Effect::DamagingCreatureControllerMayDraw { drawer, count }`, whose `drawer` is baked in at trigger placement by `fill_combat_damage_source_controller` and who answers the `MayYesNo` pause itself (the same shape `target_player_may_draw` uses). Edric, Spymaster of Trest is faithful, no `approximates` note. Residual: the drawer is locked in at placement, not re-read at resolution — a control change during the combat damage step would draw the old controller (flagged with a `ponytail:` comment).

---

## 173. `triplicate-combat-damage` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — extend combat damage resolution to allow a creature to deal its damage to multiple players simultaneously. Hydra Omnivore's "Whenever this creature deals combat damage to an opponent, it deals that much damage to each other opponent" is a reflexive trigger (not a replacement), so it's two steps: normal combat damage, then a triggered ability dealing the same amount to each other opponent.

**Premise correction (landed):** the sketch below misquoted the card as "to a player" / "each other **player**". Live Scryfall oracle (oracle id `8f504855-f3df-4284-a189-e799bcddf620`) reads "Whenever this creature deals combat damage to an **opponent**, it deals that much damage to each other **opponent**" — the Hydra's own controller is never splashed, and neither is the player who already took the combat damage.

**Example cards:** Hydra Omnivore

**Sketch:** Model as a combat-damage-dealt trigger with `Effect::DamageEachOtherOpponent { amount = "combat_damage_dealt" }` (read from `TriggerContext::combat_damage`). New `Amount::CombatDamageDealt` variant, filled by the combat-damage trigger context the same way `AttackerDrawsControllerCounters` is filled today.

**Landed (2026-07-22):** exactly the sketched shape, minus the `Amount` work (`Amount::CombatDamageDealt` and its `fill_combat_damage` rewriter already shipped with #202/Rapacious One). New `Effect::DamageEachOtherOpponent { amount, damaged }` mints the same player-damage event pair `damage_each_player` uses (life loss + `DamageDealtToPlayer` + lifelink per player, CR 702.15e; nothing at all at 0 damage, CR 120.8), skipping both the ability's controller (not an opponent, CR 102.3) and `damaged` — the player who took the combat damage, baked in at placement by the new `fill_combat_damage_recipient` off `TriggerContext::combat_damage_recipient`. Hydra Omnivore is faithful, no `approximates` note.

---

## 174. `vanishing-keyword` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — Vanishing N (CR 702.63): enters with N time counters, remove one at each upkeep, sacrifice when the last is removed. Mirror structure of Suspend (which adds time counters and casts when zero), but in reverse (starts with counters, sacrifices when zero). New `CardDef::vanishing = N`, upkeep trigger removes a counter, state-based action sacrifices at zero.

**Example cards:** Deadwood Treefolk

**Sketch:** Deadwood Treefolk has `vanishing = 3`. At each controller upkeep, remove a time counter; when zero, sacrifice. Also has an ETB "return up to two target creature cards from your graveyard to your hand" (standard multi-target return effect). Vanishing is the new mechanic; the ETB is expressible today.

**Premise correction (landed):** the second ability is *not* a two-target ETB return — Scryfall's oracle text is "When this creature enters **or leaves the battlefield**, return **another** target creature card from your graveyard to your hand." It is authored as two `[[abilities]]` blocks (`timing = "etb"` and `timing = "this_leaves_battlefield"`), each `return_from_graveyard_to_hand` at a `card_in_graveyard` target with a new `other = true` flag ("another"), which excludes the ability's own source card — the case that bites on the leaves half, once the Treefolk itself is the creature card in that graveyard. Vanishing landed as `CardDef::vanishing = N`: the counters ride the existing enters-with-counters choke (`stack::enters_with_counters` answers for a vanishing card, so no new ETB site), the tick is done directly in the upkeep step (same shortcut suspend's tick takes), and the sacrifice is a real trigger via `Game::queue_self_sacrifice_trigger` (evoke's fabricated single-ability shape, renamed from `queue_evoke_sacrifice` and now shared). No state-based action, no new `Effect` variant. Deadwood Treefolk is faithful, no `approximates` note.

**Residual (not this increment):** enters-with-counters — vanishing's included — still only fires at the cast-resolution choke and the land special action; a permanent reanimated, blinked, or searched onto the battlefield enters with no counters. Flagged as a `ponytail:` note on `Game::push_enters_with_counters`.

---

## 175. `forced-attack-direction` — **CLOSED — dead variant**

**Depends on:** none

**Effort:** M — extend the Vow pattern ("can't attack you") to its inverse: "must attack [specific player] if able." Needs a per-creature forced-attack target stored on the permanent, checked during declare attackers (if able, must attack that player or their planeswalker).

**Example cards:** Death by Dragons

**Sketch:** Death by Dragons' "each opponent creates a 5/5 red Dragon creature token with flying. Each Dragon created this way attacks its controller each combat if able" creates tokens under each opponent with a "must attack its controller" rider. New `Effect::CreateTokenForEachOpponent` (under each opponent, not under you) with `token_gains_ability` granting a static "must attack [specific player]." The static needs a `forced_attack_target: Option<Player>` field on the permanent, checked in `declare_attackers`.


**Closed 2026-07-22 (wave 7 planner):** the increment misquotes the card. Live Scryfall (`https://api.scryfall.com/cards/named?exact=Death+by+Dragons`, oracle id `88e912e3-0548-4a4a-94c1-c804093ca1b0`, `{4}{R}{R}` Sorcery) reads in full:

```
Each player other than target player creates a 5/5 red Dragon creature token with flying.
```

There is no "each Dragon created this way attacks its controller each combat if able" clause, so no forced-attack subsystem is owed on this card's account. `crates/cards/data/death_by_dragons.toml` is already in the pool and faithful (`TokenController::EachOtherPlayer`, `crates/engine/src/types/filter.rs:826`), no `approximates`. Reopen only if a real "must attack [player]" card enters the pool.
---

## 176. `storm-keyword` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — Storm (CR 702.40): when you cast this spell, copy it for each other spell cast this turn before it. The turn-scoped spell-count tally already exists (`Game::spells_cast_this_turn`); storm mints that many copies when the spell resolves (or when it's cast? — check CR). New `CardDef::storm = true`, cast-time check of the tally, mint copies.

**Example cards:** Hunting Pack

**Sketch:** Hunting Pack has `storm = true`. When cast, count `spells_cast_this_turn.len() - 1` (exclude itself), mint that many copies of the spell via the existing `mint_spell_copies`. Each copy creates a 4/4 Beast token independently. Storm is a cast trigger (CR 702.40a), so the copies go on the stack above the original spell.

**Landed:** authoring-only, reclassifies to Section C — no engine change was needed. The briefed premise (a new `CardDef::storm` field) was wrong: the Storm keyword's `Trigger::YouCastThis` + `Effect::CopyTriggeringSpell { count = "spells_cast_before_this_this_turn", last_known_information = true }` machinery already landed for Reaping the Graves' storm ability, and Hunting Pack (targetless — no `may_choose_new_targets`) is a strict subset of it. The only real gap was pool data: `crates/cards/data/hunting_pack.toml` (verbatim oracle re-fetched live, cmd printing) and a new 4/4 green Beast token `crates/cards/data/tokens/beast_4_4.toml` (the existing `beast.toml` is the 3/3 Beast Within/Garruk token — wrong body for this card). Tests: `hunting_pack_storm_copies_for_each_earlier_spell_this_turn`, `hunting_pack_with_no_earlier_spells_creates_one_beast` (`crates/engine/tests/game.rs`); the existing Reaping the Graves storm tests are untouched and still green.

---

## 177. `annihilator-keyword` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — Annihilator N (CR 702.86): whenever this creature attacks, defending player sacrifices N permanents. New `CardDef::annihilator = N`, attack trigger, the defending player sacrifices N (via the existing multi-sacrifice choice, `PendingChoice::SacrificeNPermanents`).

**Example cards:** Artisan of Kozilek

**Sketch:** Artisan has `annihilator = 2` and a cast trigger "When you cast this spell, you may return target creature card from your graveyard to the battlefield." The cast trigger is expressible today (`timing = "on_cast"`, `Effect::ReanimateTarget`); annihilator is the new keyword. Attack trigger → defending player chooses and sacrifices 2 permanents.

**Premise correction (landed):** annihilator needed **no engine work at all** — the effort estimate was wrong. Eldrazi Conscription already grants annihilator 2 through `defending_player_sacrifices`, whose `defender` is filled from the `Attacks` trigger context; that context takes its defending player from `Event::AttackerDeclared`, already resolved through `Game::defender_controller`, so #201's planeswalker-as-defender case (CR 508.1a) routes annihilator to the planeswalker's controller for free — covered by a new regression test. No `CardDef::annihilator` field was added: Artisan of Kozilek is authored as `timing = "attacks"` + `defending_player_sacrifices count = 2` plus a `timing = "when_you_cast_this"`, `optional = true` `reanimate_to_battlefield` for the cast trigger. Faithful, no `approximates` note.

---

## 178. `free-cast-if-opponent-condition` — 1 card, S — **LANDED (2026-07-21)**

**Depends on:** none

**Effort:** S — extend `CardDef::free_cast_if` (which today has `Condition` arms like `controls_lands_with_subtype`) to support opponent-checks. New `Condition::OpponentControlsNLands { at_least = 7 }` (or more generally, `OpponentMeets { condition }`), checked at cast time across all opponents (true if any opponent meets it).

**Example cards:** Avatar of Fury

**Sketch:** Avatar of Fury's "If an opponent controls seven or more lands, Avatar of Fury costs {0} to cast" is `free_cast_if = { type = "opponent_controls_n_lands", at_least = 7 }`. Route through the existing free-cast permission gate in `Game::cast_cost`. Likely S effort if the condition is a simple opponent iteration; M if it needs a general opponent-condition wrapper.

**Landed 2026-07-21 (premise correction):** the title/sketch above misread the oracle text — re-verified against Scryfall (print `f9badf70-5e86-4d43-b457-5cbf821d97df`), Avatar of Fury reads "If an opponent controls seven or more lands, **this spell costs {6} less to cast**," a conditional own-cost *reduction* (CR 601.2f-shaped), not a free cast. `CardDef::free_cast_if` was never touched. Landed via the already-supported `reduce_own_generic = { condition = {...}, then = 6 }` shape (the `avatar_of_woe.toml` template) and `Amount::IfCondition`. The one real engine gap was the condition: the existing `Condition::OpponentsControlLands` *sums* lands across every opponent, which is wrong for "an opponent controls" (a per-opponent existential read) — a 4-player pod with three opponents on 3 lands each would wrongly cross 7. Added `Condition::AnOpponentControlsLands { at_least }` (`crates/engine/src/types/effect.rs`), evaluated per-opponent in `crates/engine/src/triggers.rs` mirroring `OpponentControlsLandsWithSubtype`. `crates/cards/data/avatar_of_fury.toml` is faithful (flying, firebreathing, the cost reduction) — no `approximates`, no `ponytail:` note.

---

## 179. `multi-player-x-tracking` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — extend X-cost resolution to store the chosen X in a per-player map, then each player independently resolves their own X. Collective Voyage's "each player may search their library for up to X basic land cards" means the caster picks X, pays X, but then *each* player (including opponents) searches for up to X basics.

**Example cards:** Collective Voyage

**Sketch:** Collective Voyage has `cost.x = true` and an effect "each player may search their library for up to X basic land cards and put them onto the battlefield. Then each player shuffles." The X is chosen/paid by the caster, but the effect grants *each* player the search (not just the caster). This is a "may search up to X" with X shared across all players, resolved per player. New `Effect::EachPlayerSearchLibrary { up_to_count = "x", ... }` or extend `SearchLibrary` with an `each_player = true` flag.

**The briefed premise was wrong.** Collective Voyage does **not** have `cost.x = true` and the
caster does not pick X. Live oracle text (Scryfall oracle id
`00f82f84-81a5-45e5-9c35-9d627a180950`, `{G}` Sorcery, printing `3f2d44f0-c71e-4061-bb7c-1f91fbce8f51`,
set `c16`): "Join forces — Starting with you, each player may pay any amount of mana. Each player
searches their library for up to X basic land cards, where X is the total amount of mana paid this
way, puts them onto the battlefield tapped, then shuffles." It is a **join forces** card: X is the
*sum* every player contributes, so there is no per-player X map to track — one shared tally.

**Landed 2026-07-22:** modeled by reusing the council's-dilemma fan-out skeleton wholesale. Three new
pieces: `Effect::JoinForcesPayMana` (each living player in turn order starting with the caster
pauses on a pay-any-amount choice, CR 101.4; the payment is settled as that much generic, CR 202.2),
`Amount::ManaPaidThisWay` (reads the resolution-scoped `ResolutionFrame::join_forces_mana`), and
`SearchLibrary.count_amount: Option<Amount>` — a dynamic cap that overrides `count`, resolved once as
the search begins so every seat of the existing `SearchScope::AllPlayers` fan-out searches for the
same X. `Game::turn_order_from` was extracted as a shared helper (it also deleted the duplicate
ordering in the vote arm). No new answering intent was needed: `Intent::PayOptionalCostX` already
existed but had **no wire projection at all** — that pre-existing debt is now closed by an additive
optional `x` on `WireIntent::PayOptionalCost` / `WireIntentPayOptionalCost`, which also unblocks
Decree of Justice's X rider on the existing `PayCost` pause. The pause projects as the new
`PendingChoiceView::PayAnyAmountOfMana { player, source, max }` (`max` from
`Game::max_payable_x`), with a client count-picker form reusing the draw-up-to shape.

`crates/cards/data/collective_voyage.toml` is new and faithful, with no `approximates`.


---

## 180. `reveal-creature-card-from-hand-additional-cost` — 1 card, S — **LANDED (2026-07-22)**

**Depends on:** none

**The briefed premise was wrong.** Disaster Radius does not sacrifice a land and does not read
"the number of lands you control." Live oracle text (Scryfall oracle id
`3322c865-bb2a-4201-9115-979883cd7894`, `{5}{R}{R}` sorcery, printing `cmd` — cross-checked
against `scratchpad/deck.json`'s own `oTags`/`text`): "As an additional cost to cast this spell,
reveal a creature card from your hand. Disaster Radius deals X damage to each creature your
opponents control, where X is the revealed card's mana value." No land sacrifice, no lands-you-
control count, and the sweep is opponents-only, not "each creature."

**Example cards:** Disaster Radius

**Landed:** `AdditionalCost::reveal_creature_from_hand: bool` (CR 601.2g) — a bare bool, not a
filtered struct, since only a creature-card reveal exists in the pool (widen to a `CardFilter` if
a future card reveals a different card type). `Game::cast_additional_cost_gate` rejects the cast
outright (CR 601.2f) if the caster has no other creature card in hand; `Game::cast` then records
the pick via `Game::highest_creature_mana_value_in_hand` (revealing has no cost or downside, so a
rational caster always reveals the biggest X — the same "no real choice" idiom
`Condition::HandHasLandWithSubtype`'s reveal-lands path already uses) onto the new
`Spell::revealed_creature_mana_value` field (the `sacrifice_count` sibling), read by
`Amount::RevealedCreatureManaValue` via `Game::revealed_creature_mana_value`. `Effect::
DamageEachCreature { amount = "revealed_creature_mana_value", opponents_only = true }` (no new
effect needed). One correctness fix along the way: `Effect::DamageEachCreature`'s mint path
substitutes each hit creature in as `Amount`'s resolution `source` (needed for
`Amount::SourcePower`), so a naive `Amount::RevealedCreatureManaValue` read there would silently
see 0 (an off-battlefield spell id's mana value, not the spell's own record) — fixed by resolving
it once against the true ability source before the substitution, mirroring the existing
`Amount::IfSpellKicked` rewrite immediately above it. `crates/cards/data/disaster_radius.toml` —
no `approximates`. Tests: `disaster_radius_cant_be_cast_without_a_creature_card_in_hand`,
`disaster_radius_deals_revealed_creatures_mana_value_to_each_opponents_creature`
(`crates/engine/tests/game.rs`).

---

## 181. `exile-reveal-put-in-hand` — 1 card, S — **LANDED (2026-07-21)**

**Depends on:** none

**The briefed premise was wrong.** Prophetic Bolt does not exile, does not filter for lands, and the pick is not optional. Live oracle text (Scryfall print `9f482559-c09f-4261-9715-76fc11014a20`, oracle id `d5e4bf5e-1a66-4315-8c82-640c27977b88`, set `cmd`, `{3}{U}{R}` instant): "Prophetic Bolt deals 4 damage to any target. Look at the top four cards of your library. Put one of those cards into your hand and the rest on the bottom of your library in any order." That's a `deal_damage` clause plus a plain `look_at_top` with a mandatory (`min = 1`) single pick and no filter — a strict subset of what Dig Through Time already exercises (`count = 7, up_to = 2, min = 2`) and Quandrix Apprentice's "may" variant.

**Example cards:** Prophetic Bolt

**Landed:** authoring-only, reclassifies to Section C — no engine change was needed. `Effect::DealDamage { target = "any" }` (Chain Lightning's shape) followed by `Effect::LookAtTop { count = 4, up_to = 1, min = 1, dest = "hand", rest = "bottom" }` (default `filter = AnyCard`) as two effects on one `timing = "spell"` ability landed the card faithfully on the first pass; `min` was already enforced correctly by `Game::select_from_top` (Dig Through Time's existing mandatory-minimum tests already covered that path, and the new test's declined-choice assertion confirms it holds for `min = 1` too). `crates/cards/data/prophetic_bolt.toml` — no `approximates`, only the standing "bottom order is PRNG-shuffled, not player-chosen" `ponytail:` note the rest of the `look_at_top` family already carries. Test: `prophetic_bolt_deals_four_then_puts_one_of_top_four_in_hand` (`crates/engine/tests/game.rs`).

---

## 182. `exile-all-cards-matching-from-zone` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — Trench Gorger's "When Trench Gorger enters, you may search your library for any number of land cards, exile them, then shuffle. If you do, Trench Gorger has base power and toughness each equal to the number of cards exiled this way" needs:
1. A zone-sweep effect (search library, exile all matching cards)
2. Power/toughness set to the count exiled (new characteristic-defining ability or static effect)

**Example cards:** Trench Gorger

**Sketch:** New `Effect::ExileCardsFromLibrary { filter = "land", up_to = "any" }` + a static effect `SetPowerToughness { amount = "exiled_by_this_ability" }` that reads a stored count. The count must be tracked per permanent (how many lands *this* Gorger exiled), likely via a custom counter or a `HashMap<ObjectId, usize>` in `Game` state.

**Landed (2026-07-22):** the "still unvalidated" premise held only in part — no `HashMap<ObjectId, usize>` was needed, and no new zone-sweep effect either. `Effect::SearchLibrary` already carried both axes Trench Gorger needed as generalizations of prior cards: `to_zone = SearchDest::Exile` and `count = "any"` (TOML sugar for `u8::MAX`, `de::count_or_any`). The one real gap was the P/T read: `Amount::CardsExiledBySearchThisWay` (the land-agnostic generalization of `NonlandCardsExiledThisWay`'s "this way" idiom) reads a resolution-scoped counter (`ResolutionFrame::cards_exiled_by_search_this_way`, reset when the search begins, incremented per accepted pick in `Game::search_library`) — no per-permanent map, no new counter kind. The CR 613.3(7b) *continuous* base-P/T set is `Effect::SetOwnBasePtFromAmount { amount }`, minting `Event::BasePtSetIndefinite` (the indefinite sibling of the reanimate-with-modification `set_base_pt` field already on `Permanent`, applied through the existing `pt_layers`/`apply_pt_layers` 7b-then-7c ordering exactly as sketched) — a no-op if the source has already left the battlefield (CR 608.2c). One real bug surfaced by the "exile zero lands" test and fixed in the same change: `Game::invalidate_characteristics_cache` was missing a `BasePtSetIndefinite` arm, so a set that changed power but left toughness's *value* unchanged (or vice versa) could read a stale memoized value for whichever characteristic a prior state-based check had already cached — added alongside `BasePtSetUntilEndOfTurn` in the same match arm. Trench Gorger (`crates/cards/data/trench_gorger.toml`) is fully faithful — trample + the ETB — no `approximates`.

---

## 201. `planeswalker-as-attack-defender` — 4 cards, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — this increment replaces the briefed `controller-restricted-attack-ban`, whose premise was checked and found already landed: `grant_to_attached` carries `cant_attack_controller` (`crates/engine/src/types/effect.rs`, read live in `Game::declare_attackers` via `Game::host_cant_attack_controller`, `crates/engine/src/combat.rs:376`), and `vow_of_flight.toml` / `vow_of_lightning.toml` already ship with it. The Vow cycle's *real* residual is the second half of the same sentence — "or planeswalkers **you control**" — and it is unrepresentable for one reason: **a planeswalker can never be attacked**. `Intent::DeclareAttackers` carries `Vec<(ObjectId, PlayerId)>`, so an attack's defender is always a player id; `Game::attack_tax_owed`/`attacker_tax_owed` (`crates/engine/src/combat.rs`) likewise key on `PlayerId`. Planeswalker permanents themselves already exist (`CardKind::Planeswalker`, `quintorius_history_chaser.toml`, `garruk_wildspeaker.toml`), loyalty is paid and damaged (`deal_damage_to_planeswalker_removes_loyalty`), and CR 306.6/508.1a's "redirect to the planeswalker's controller" combat model is simply absent. Every "can't attack you **or planeswalkers you control**" residual in the pool is downstream of this one gap, not of any missing `grant_to_attached` axis.

**Example cards:** Vow of Flight, Vow of Lightning, Vow of Wildness (all three keep an `approximates` note on the planeswalker half until this lands), Nils, Discipline Enforcer (its `counter_scaled_attack_tax` charges per defending *player* only)

**Sketch:** Widen the attack defender from `PlayerId` to a `Defender { Player(PlayerId), Planeswalker(ObjectId) }` (CR 508.1a). `declare_attackers` validates a planeswalker defender as a battlefield planeswalker controlled by an opponent of the declarer; combat damage to it routes through the existing `Event::LoyaltyChanged` path `deal_damage` already uses. `host_cant_attack_controller` and `attacker_tax_owed` then take a `Defender` and resolve it to its controller, which makes the Vow cycle's and Nils's "or planeswalkers you control" fall out for free — both read the *controller* of the thing being attacked, which is the same player either way. Client-side this is a new defender shape on the declare-attackers intent, so scope the wire/projection change with it.

**Landed 2026-07-22:** the sketch held. `Defender { Player(PlayerId), Planeswalker(ObjectId) }`
(`crates/engine/src/types/stack.rs`) replaced the bare `PlayerId` in `Intent::DeclareAttackers`,
`Intent::TakeAction` and `CombatState::attack_targets`. `Game::defender_controller` is the single
choke every "who is being attacked" read routes through, so the Vow cycle, Nils and Soul Snare fell
out for free as predicted. `declare_attackers` validates a planeswalker defender per CR 508.1a (on
the battlefield, not phased out, a planeswalker, controlled by a live opponent); combat damage to it
mints `Event::LoyaltyChanged` rather than `Event::DamageMarked` (CR 120.3c/306.8), honours
protection/prevention and feeds lifelink, and trample overflow past a blocker spills onto the
planeswalker, not the player (CR 510.1c). CR 800.4a's attacker retain drops attacks aimed at a
removed planeswalker as well as a removed player.

`Event::AttackerDeclared` is **flat**, not a bare `Defender`: `{ object, defender: PlayerId,
defender_planeswalker: Option<ObjectId> }`. `project_event` has no `&Game`, so it cannot resolve a
planeswalker to its controller; every downstream consumer (turn yields, attack triggers, the wire
projection) wants the defending *player* and now reads it directly, while `apply` recombines the two
fields into a `Defender` for `attack_targets`. `WireAttack` and `VisibleEventAttackerDeclared` each
gained an **additive optional** field 3 (`defender_planeswalker`); field 2 keeps its old meaning, so
older clients are unaffected (`docs/WIRE_COMPAT.md`).

**Cards reconciled:** `vow_of_flight.toml`, `vow_of_duty.toml`, `vow_of_lightning.toml`,
`vow_of_wildness.toml` and `soul_snare.toml` all lost their `approximates` (and Vow of Wildness its
paired `# ponytail:`) — the planeswalker half is now modeled. `nils_discipline_enforcer.toml`'s
header ponytail became the verbatim oracle text and its second ability's comment was restored to the
full sentence. The stale inline notes on `GrantToAttached::cant_attack_controller`,
`CantBeAttackedBy`, `CounterScaledAttackTax` (`crates/engine/src/types/effect.rs`) and
`Trigger::OpponentAttacksYouWithCreatures` were deleted; `MyriadTokenCopies`'s ponytail was reworded
to its true residual (`Event::TokenEnteredAttacking` still carries a `PlayerId`).

**Client debt created:** `client/src/wire/types.ts` was hand-patched (codegen cannot run in this
worktree — `client/node_modules` is absent, so `protoc-gen-es`/`protoc-gen-effect-grpc` are not on
PATH). The board's attack interaction still only offers *players* as attack targets: a planeswalker
cannot be picked as a defender in the UI, and the action log renders `defender_planeswalker` not at
all. `client/src/api/generated.ts` (tracked, unimported) is stale and needs `bun run gen`.


---

## 202. `planeswalker-as-damage-and-effect-target` — 1 card, M — **LANDED**

**Depends on:** none (independent of #201)

**Effort:** M — the two ponytail claims flagged by the observability re-audit were both verified stale, but for a reason neither note states. Both say "no planeswalker permanent exists in the pool"; that has been false since `quintorius_history_chaser.toml`, and Garruk Wildspeaker makes two. What is actually missing is narrower and split across the two cards:
1. `crates/cards/data/nils_discipline_enforcer.toml` — its "or planeswalkers you control" is blocked by the attack-defender model, **not** by planeswalker existence. Tracked in #201, not here.
2. `crates/cards/data/volcanic_torrent.toml` — "deals X damage to each creature **and planeswalker** your opponents control" is blocked here: `Effect::DamageEachCreature`'s sweep filters `is_creature_on_battlefield` (`crates/engine/src/resolution/damage.rs`), so planeswalkers are dropped from every mass-damage effect. Single-target damage already reaches a planeswalker (`TargetSpec::CreatureOrPlaneswalker`, `Effect::DealDamage` → `Event::LoyaltyChanged`), so only the untargeted fan-out is missing.

**Example cards:** Volcanic Torrent (`approximates` residual until this lands); every future "damage to each creature and planeswalker" sweeper

**Sketch:** Add `include_planeswalkers: bool` (default `false`, so every existing sweeper is unchanged) to `Effect::DamageEachCreature`, and widen its battlefield filter to `is_creature_on_battlefield(id) || (include_planeswalkers && is_planeswalker_on_battlefield(id))`. Damage to a planeswalker in the sweep reuses the loyalty-removal branch `deal_damage` already has, including its protection/prevention filters. Rename the effect to `damage_each_permanent` only if a second non-creature axis ever appears — one bool is cheaper than a new effect. Then drop the `ponytail:` note and `approximates` on `volcanic_torrent.toml`.

**Landed 2026-07-21:** the sketch held, with the planeswalker predicate inlined as a closure rather than a new `is_planeswalker_on_battlefield` helper (one call site family). `Effect::DamageEachCreature` gained `include_planeswalkers: bool` (`#[serde(default)]`), the sweep filter widened, and a swept planeswalker's share mints `Event::LoyaltyChanged` instead of `Event::DamageMarked` (CR 120.3c/306.9). Protection still filters planeswalkers out (CR 702.16d); Tajic's "other creatures" prevention (CR 615) explicitly exempts them. `volcanic_torrent.toml` lost its `ponytail:` note — no `approximates` field existed — and its planeswalker half is faithful. Nils, Discipline Enforcer's own planeswalker gap stays with #201, untouched.

**Still blocked:** nothing for Volcanic Torrent. #201 (`attack-planeswalker-defender`) remains open for Nils.

---

## 203. `two-independent-single-target-clauses-on-one-spell` — 1 card, M — **LANDED (2026-07-22)**

**Depends on:** none

**Effort:** M — Vengeful Rebirth's "Return target card from your graveyard to your hand. If you return a nonland card to your hand this way, [it] deals damage equal to that card's mana value to any target" needs a spell with two *independently*-targeted single-target clauses (a graveyard-card target for the return, an unrelated any-target for the damage). Today's two-target machinery (`StackItem::targets_second`, `Game::ability_second_target_clause`) only fires when both clauses are *multi*-target (Magma Opus's "4 damage divided as you choose" + "tap two target permanents"); no card in the pool yet pairs one single-target clause with a second, independent single-target clause on the same spell. The mana-value binding itself is already solved (`ExileTargetGraveyardCardRecordManaValue` / `Amount::TargetManaValue` read a just-moved graveyard card's mana value into a later step), so the gap is purely the second-target slot, not the amount.

**Example cards:** Vengeful Rebirth (`approximates` residual until this lands).

**Sketch:** Extend the `targets_second` slot (or add a sibling) so a spell can carry two single-target clauses, not just two multi-target ones — likely widening `StackItem`'s second-target field to accept either shape rather than adding a third parallel field. Vengeful Rebirth then becomes `effects = [{ type = "return_from_graveyard_to_hand", target = ... }, { type = "conditional", condition = "returned_card_nonland", then = [{ type = "deal_damage", amount = "returned_card_mana_value", target = "any_second" }] }, { type = "exile_self_on_resolve" }]` (exact field names TBD by whoever lands this).

**LANDED (2026-07-22):** no new target slot was needed — the *modal* path already split a `Effect::Sequence` into independent single-target clauses (Hull Breach mode 2, #189), so the fix was to let non-modal spells read the same machinery: `mode_target_clauses` → `ability_target_clauses` and `modal_clause_steps` → `ability_clause_steps` (`types/stack.rs`, `cast.rs`), plus a new `Game::spell_target_clauses` that `spell_multi_target` / `spell_target_clause` now delegate to. A non-modal spell ability with >1 targeted step yields one `ChooseSpellTargets` pause per clause (clause 0 → `Spell::targets`, clause 1 → `Spell::targets_second`); single-clause abilities are unchanged, and a scan of all card TOMLs confirmed no existing non-modal spell has two targeted steps, so nothing else moved. The `conditional` wrapper in the sketch proved unnecessary: new `Amount::ReturnedNonlandCardManaValue` reads `ResolutionFrame::returned_nonland_card_mana_value` (written on every `Event::ReturnedToHand` apply, `None` for a land or a non-graveyard bounce, cleared at each `resolve_spell` entry so a fizzled return can't leak a stale value), and `0` damage is no damage at all (CR 120.8) — which *is* the "if you return a nonland card" gate. `vengeful_rebirth.toml` is now fully faithful; its `approximates` is deleted.

**Still blocked:** nothing.

---

## Priority Rank (cards unblocked ÷ effort)

1. ~~**#169 `retrace-validation`** (S, 1 card)~~ — LANDED 2026-07-21
2. ~~**#178 `free-cast-if-opponent-condition`** (S, 1 card)~~ — LANDED 2026-07-21 (premise correction: own-cost reduction, not free cast)
3. ~~**#180 `reveal-creature-card-from-hand-additional-cost`** (S, 1 card) — Disaster Radius~~ — LANDED 2026-07-22 (premise correction: reveal-a-creature, not land-sacrifice)
4. ~~**#181 `exile-reveal-put-in-hand`** (S, 1 card)~~ — LANDED 2026-07-21 (authoring-only; premise correction: look-at-top, no exile/filter)
5. ~~**#185 `gain-control-of-all-owned-creatures`** (S, 1 card) — Homeward Path~~ — LANDED 2026-07-21
6. ~~**#183 `land-enters-with-counters-static`** (S, 3 cards) — Vivid Crag / Vivid Creek / Vivid Grove~~ — LANDED 2026-07-22
7. ~~**#186 `put-from-hand-on-top-of-library`** (S, 1 card) — Brainstorm~~ — LANDED (verified 2026-07-22)
8. ~~**#189 `multi-target-per-modal-mode`** (S, 1 card) — Hull Breach~~ — LANDED 2026-07-21
9. ~~**#191 `counter-to-library-bottom-and-self-tuck`** (S, 1 card) — Spell Crumple~~ — LANDED (verified 2026-07-22)
10. ~~**#192 `exile-self-on-resolve`** (S, 1 card) — Vengeful Rebirth~~ — LANDED 2026-07-21 (card residual on #203)
11. ~~**#184 `storage-land-x-counter-mechanic`** (M, 1 card) — Fungal Reaches~~ — LANDED 2026-07-22
12. ~~**#167 `copy-creature-spell`** (M, 1 card) — Riku's signature ability~~ — LANDED 2026-07-22 (premise correction: Riku copies instants/sorceries, not creature spells)
13. ~~**#170 `cost-reduction-scaled-by-source-counters`** (M, 1 card) — Animar's signature ability~~ — LANDED 2026-07-22 (premise correction: `Amount::PerCounterOnSource` + `reduce_spell_cost` already covered it; authoring-only)
14. ~~**#171 `opponent-prediction-choice`** (M, 1 card) — Conundrum Sphinx~~ — LANDED 2026-07-22 (premise correction: every player names a card for themselves, no opponent guess)
15. **#172 `combat-damage-trigger-controller-draws`** (M, 1 card) — Edric's group-hug draw
16. **#173 `triplicate-combat-damage`** (M, 1 card)
17. ~~**#174 `vanishing-keyword`** (M, 1 card) — Deadwood Treefolk~~ — LANDED 2026-07-22 (premise correction: the second ability is an enters-*or-leaves* trigger returning *another* target creature card)
18. ~~**#175 `forced-attack-direction`** (M, 1 card)~~ — CLOSED 2026-07-22 (dead variant: Death by Dragons has no forced-attack clause)
19. ~~**#176 `storm-keyword`** (M, 1 card) — Hunting Pack~~ — LANDED 2026-07-22
20. **#177 `annihilator-keyword`** (M, 1 card)
21. **#179 `multi-player-x-tracking`** (M, 1 card)
22. ~~**#182 `exile-all-cards-matching-from-zone`** (M, 1 card) — Trench Gorger~~ — LANDED 2026-07-22
23. ~~**#187 `split-card-two-castable-faces`** (M, 1 card) — Fire // Ice~~ — LANDED 2026-07-22
24. ~~**#188 `color-spent-to-cast-accounting`** (M, 1 card) — Firespout~~ — LANDED 2026-07-22 (premise correction: `Spell::spent_colors` already existed; only the on-stack read and `with_flying` were missing)
25. ~~**#190 `alternative-cost-with-rider`** (M, 1 card) — Invigorate~~ — LANDED 2026-07-22
26. ~~**#168 `cast-from-exile-permission`** (L, 1 card) — Intet, future-proofing foundation~~ — LANDED 2026-07-22 (premise correction: the real oracle text is free-cast, source-scoped — S, not L)
27. **#201 `planeswalker-as-attack-defender`** (M, 4 cards) — the Vow cycle + Nils; best cards-per-effort of the M tier
28. ~~**#202 `planeswalker-as-damage-and-effect-target`** (M, 1 card) — Volcanic Torrent~~ — LANDED 2026-07-21
29. ~~**#203 `two-independent-single-target-clauses-on-one-spell`** (M, 1 card) — Vengeful Rebirth~~ — LANDED 2026-07-22

---

## Notes

- Four increments (#169, #180, #181, #182) were flagged for validation — #169 and #181 landed and reclassified to Section C (authoring-only, no engine change); #180 and #182 are still unvalidated.
- The deck's three legendaries (Riku, Animar, Intet) are all Section D — the commanders drive the engine work.
- Storm (#176) and Annihilator (#177) are evergreen-ish keywords that will unblock future cards beyond this deck.
- ~~Cast-from-exile (#168) is foundational L-tier work but only blocks 1 card here~~ — landed 2026-07-22 and was S, not L: the impulse-draw subsystem already covered it and Intet's real text is a free-cast permission scoped to Intet's presence, not a until-your-next-turn normal-cost one.
- Wave-2 added four S-tier increments (#186, #189, #191, #192) and three M-tier (#187, #188, #190) from the instants/sorceries bucket.
- Wave-11 (2026-07-22) closed the last three open engine increments — #197 (Nucklavee), #187 (Fire // Ice) and #168 (Intet) — leaving every card in this deck faithful (72/72).
- Wave-3 added #201/#202 from the planeswalker re-audit: the briefed `controller-restricted-attack-ban` gap does not exist (`grant_to_attached.cant_attack_controller` already ships), and both stale planeswalker ponytail notes were falsified by *existence* long before Garruk — the real blockers are the attack-defender model (#201) and mass-damage fan-out (#202).

---

## Per-card exotics

- **exotic: Vow of Wildness** — **LANDED (2026-07-22)**, authoring-only (Section C), no engine
  change, exactly as predicted. Aura: enchanted creature gets +3/+3,
  has trample, and can't attack you or planeswalkers you control. Authored off the in-pool
  `vow_of_flight.toml` / `vow_of_lightning.toml` (`grant_to_attached` with
  `cant_attack_controller = true`); carries the same "or planeswalkers you control" residual
  as its siblings until #201 lands.
