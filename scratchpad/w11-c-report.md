# Wave 11, slice C — report

Increments 38, 60, 126, 127, 128. **All five landed.** Test file: `crates/engine/tests/leg_w11_c.rs`
(17 tests, green). Also re-ran and kept green: `leg_bands_with_other` (24), `leg_rampage` (10),
`leg_divide_combat_damage_in_the_damage_step` (6), `leg_combat_restrictions` (8),
`leg_attacking_or_blocking_filter` (6), `leg_source_keyed_prevention` (16),
`leg_any_player_may_activate` (8).

## 38 — `damage-reduction-replacement` (Forethought Amulet) — LANDED

- New `StaticEffect::ReplaceDamageToYou { source: SpellFilter, at_least, becomes }`.
- New `ReplacementEffect::ReplaceDamageToYou` + registry arm + `replaced_damage_to_player()` query
  in `crates/engine/src/replacements.rs`.
- Read in `player_damage_events_inner` (`resolution/damage.rs`) **after** the prevention shields
  take their bite — CR 615.9 rewrites the amount rather than subtracting from it.
- `crates/cards/data/forethought_amulet.toml` created (upkeep `pay_or_else` sacrifice + the static).
  Faithful, no `approximates`.
- 6 tests: rewrite at 3, rewrite at 4 (Psionic Blast → still 2), control case without the amulet,
  sub-threshold Shock untouched, permanent source (Orcish Artillery) untouched, opponent unprotected.

## 60 — `damage-redirection` (Nova Pentacle, Shimian Night Stalker) — LANDED

The sketch asked for "a redirection registry parallel to the prevention shields". **It already
exists** as `PreventionShield::redirect_to` (Jade Monolith, Reverberation). Only two new redirect
*destinations* were needed, as flags on `MiscEffect::PreventNextDamage`:

- `redirect_to_source` — "is dealt to **this creature** instead" (Shimian). No `TargetSpec` can name
  the ability's own source when the target slot already holds the watched attacker.
- `redirect_to_target` — "is dealt to **target creature** … instead" (Nova Pentacle). The targeted
  creature is the damage's new home, so the *shield* stands in front of the controller.

Wired in `resolution/resolve_misc.rs`. `message.rs` needed no change — the `PreventNextDamage` arm
already renders flag-agnostically.

- `shimian_night_stalker.toml` — faithful, no `approximates`.
- `nova_pentacle.toml` — carries an `approximates`: the redirected creature is named by the
  artifact's controller, not "an opponent's choice". That needs slice E's **#80** `chosen_by`
  routing; the "source of your choice" half is approximated the same way Jade Monolith / Forcefield
  already are.
- 3 tests.

## 126 — `finish-cr-510s-damage-assignment-rules` — LANDED (sketch was wrong; see below)

Two halves.

**CR 510.1c lethal order.** `Game::lethal_order_respected` in `pending/handlers/combat.rs`, called
from `assign_damage`'s validation.

> **The increment's sketch was wrong** and I only found it because four `leg_bands_with_other` tests
> and several `game.rs` divisions went red on the first implementation. CR 510.1c constrains the
> *damage assignment order*, which CR 509.2 lets the dividing player announce freely as blockers are
> declared. It is **not** the declaration order `blockers_of` returns, and the engine records no
> order at all. Walking `blockers_of` as the sketch says makes legal MTG illegal — e.g. `game.rs`'s
> "pour 1 into the 2/2 (survives) and 3 into the 3/3 (dies) — the opposite of what
> lethal-in-declaration-order would do", which is a documented, correct expectation.

So the rule is enforced in its order-independent form: an assignment is realizable under *some*
order exactly when **at most one** recipient is left short of lethal while still taking damage
(everything at lethal sorts ahead of it, every zero sorts behind). Two half-killed blockers is the
shape that is refused. Deathtouch reads lethal as 1 (CR 702.2b), so it never trips.

CR 702.22j/k are an explicit exception — when banding moved the choice to the other seat, that seat
divides freely with no order. The check returns early on `banding_division_shifter(recipients)`,
which I had to widen from `fn` to `pub(crate)` in `combat.rs` (B's file — one-word additive edit).

ponytail comments left on two known gaps, neither in scope for #126: damage assigned by *another*
creature in the same batch is not counted toward a recipient's lethal reading, and a trampler
holding damage back (CR 702.19b) is not made to bring every blocker to lethal first.

**CR 510.4 double-strike re-ask.** Landed one level up from the sketch, in `priority.rs`'s
`Step::CombatDamage` arm, which clears `self.combat.damage` before calling
`divide_or_deal_combat_damage`. Routing it there rather than into `divide_or_deal_combat_damage`
itself kept me out of B's `combat.rs` for the behavioral change. Safe because nothing but the two
batches writes that table, `assign_damage` re-enters `divide_or_deal_combat_damage` directly rather
than through `advance_step`, and `owes_a_division` requires `deals_this_batch`. I did update the now
stale ponytail on `owes_a_division`'s doc comment in `combat.rs`.

## 127 — `unanswerable-division-past-max-blockers` — LANDED

`Game::next_undivided_division` (`combat.rs`) now raises a division only when the recipient count is
in `2..=MAX_BLOCKERS` (was `>= 2`). A gang block wider than the 8-recipient ceiling falls through to
the default lethal-in-order split instead of raising a choice `assign_damage` can never accept —
which was a hard softlock of the combat damage step. ponytail doc names the ceiling and the upgrade
path (lifting `MAX_BLOCKERS` means giving up `Event: Copy`).

## 128 — `costless-permanent-regeneration-shield` (Clergy of the Holy Nimbus) — LANDED

- New fieldless `StaticEffect::RegeneratesInsteadOfBeingDestroyed`.
- Folded into `Game::regeneration_shield_available` (`core.rs`) — the single choke all seven destroy
  call sites and the lethal-damage SBA already ask, so no call site changed. `Event::Regenerated`'s
  `saturating_sub` leaves the (already zero) counted shields alone, which is exactly what makes the
  replacement standing rather than one-shot. `cant_be_regenerated_this_turn` still gates it, so
  Clergy's own second ability (#25) works against it.
- `clergy_of_the_holy_nimbus.toml`: first ability scripted, `approximates` line **removed** — the
  card is now faithful.
- 5 tests: destroy spell replaced, replaced *twice* (proves it is standing), control case (an
  ordinary creature dies to the same spell), the opponents-only "can't be regenerated this turn"
  beats it, lethal combat damage (CR 704.5g) is replaced too.

## Collateral edit outside my slice's files

`crates/engine/tests/leg_any_player_may_activate.rs` — the three Clergy tests hardcoded
`ability_index: 0` / `ability: 0`. Adding the static regeneration ability as `[[abilities]]` #0
shifted the activated one to index 1 (`ability_activation_gate` indexes the raw `def.abilities`
slice, statics included). Bumped to 1; all 8 green. Any other slice adding a static ability ahead of
an activated one on an already-tested card will hit the same thing.

## For the wave retrospective

1. **Tree breakage from siblings was the dominant cost of this slice.** I was blocked from building
   for long stretches by, in order: `PumpEffect::GrantSelfKeywordsUntilNextUpkeep`,
   `MiscEffect::GrantSpendManaAsAnyTypeForOneSpellThisTurn`, `PumpEffect::ThatCreatureBecomesColor`,
   `CounterKind::Hatchling`, `backfire.toml`'s `enchanted_creature_deals_damage_to_you` timing,
   `de.rs`'s missing `FilterOwner` import, `StaticEffect::CounterMaximum`. Several were "new enum
   variant committed to the working tree without the `message.rs` / `resolution/*` arm", which E0004s
   the whole workspace for everyone. A card TOML naming a timing that does not exist yet is worse:
   it compiles fine and instead fails *every* test in *every* slice at registry parse time, with a
   panic that looks like your own card is broken.
2. **`crates/engine/tests/game.rs` is currently broken by siblings** and I could not verify against
   it: `ChoiceEffect::PayOrElse` is missing `extra_generic` at lines 28969/29157 and
   `MiscEffect::ScheduleAtNextUpkeep` is missing `your_upkeep` at 29514/29782. Whoever added those
   fields owes `game.rs` the update. I hand-checked every `Intent::AssignDamage` division in
   `game.rs` against my new CR 510.1c rule and all of them stay legal.
3. **Increment sketches that name a concrete data structure deserve a skim of the existing code
   first.** #60's "build a redirection registry" was already built, and #126's "walk `blockers_of`'s
   declaration order" would have shipped a rules bug that the banding suite caught. Both cost less
   than an hour, but a one-line "check whether this already exists" pass on the backlog would have
   caught them at authoring time.
4. `DSL_REFERENCE.md` needed no edit for any of the five: it lists top-level `CardToml` keys and
   effect `type` families, not per-effect `mode`s or nested flags. Those live in the generated
   `crates/cards/schema/card.schema.json`, which the orchestrator regenerates.

## New increments filed

None. Nothing in 38/60/126/127/128 hit a DSL wall that needed a 165–169 entry. Nova Pentacle's
remaining gap is already covered by the existing **#80**.

## Docs ticked

- `docs/fidelity/leg.md`: Forethought Amulet, Nova Pentacle, Shimian Night Stalker, Clergy of the
  Holy Nimbus.
- `docs/fidelity/leg-increments.md`: `### 38`, `### 60`, `### 126`, `### 127`, `### 128` marked
  `— **LANDED** (wave 11)`, each with a *Landed:* note (126's records the sketch correction).
