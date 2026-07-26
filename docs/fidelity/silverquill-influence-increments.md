# Silverquill Influence deck increments (2026-07-26)

Deck report: [silverquill-influence.md](silverquill-influence.md). This file is the sole
engine-capability backlog for this deck (ranked increments plus the concrete Silverquill cards they
unblock).

From `docs/decklists/silverquill_influence.md` (official Wizards `soc` precon; commander Killian,
Decisive Mentor). After the documentation re-audit, 28 of the deck's 84 nonbasic cards still need
engine work. The largest surfaced gap is controller-vs-owner trigger scanning once `Animate Dead`
puts opponent-owned permanents under your control; the remaining clusters are goad-versus-tax
combat legality, non-cast Aura move legality, Darksteel Mutation's missing type-stripping layer,
Herald of Amity's resolution-time cast, and Defacing Duskmage's all-player life-loss sequencing.

### 1. `controller-scoped-battlefield-trigger-watches` — 20 cards, XL
**Depends on:** none.
**Cards:** `ajanis_chosen.toml`, `archon_of_suns_grace.toml`, `breena.toml`,
`combat_calligrapher.toml`, `defacing_duskmage.toml`, `doomwake_giant.toml`,
`eiganjo_dynastorian.toml`, `eriette_of_the_charmed_apple.toml`, `firemane_commando.toml`,
`hateful_eidolon.toml`, `herald_of_amity.toml`, `keen_duelist.toml`,
`killian_decisive_mentor.toml`, `kor_spiritdancer.toml`, `mangara_the_diplomat.toml`,
`nils_discipline_enforcer.toml`, `pearl_ear_imperial_advisor.toml`, `scriv_the_obligator.toml`,
`sram_senior_edificer.toml`, `starfield_mystic.toml`
**Sketch:** Silverquill's `animate_dead.toml` makes opponent-owned permanents you control routine
play, but many battlefield trigger scanners still key "you/opponent" and trigger controller from
`owner_of(id)` on live permanents. The visible choke points are the generic watch-table dispatch
(`triggers.rs` all-battlefield/all-battlefield-except-player watches), permanent-enters watchers
and self-ETBs, attack watch/batch triggers, spell-cast watchers, player-draw watchers, Hateful
Eidolon's `an_enchanted_creature_dies`, and Starfield Mystic's enchantment-death watcher. Switch
those live battlefield paths to `controller_of(id)` while keeping owner-based look-back only for
objects that have already left the battlefield. Add regression tests that reanimate an
opponent-owned watcher with `Animate Dead`, then exercise the trigger from the new controller's
perspective.
**Slices:**
1. **Generic watch-table controller cleanup (M) — LANDED (2026-07-26).** The three live-battlefield
   watch-dispatch arms in `triggers.rs` (`ControlledPlayer` filter, `AllBattlefield`,
   `AllBattlefieldExceptPlayer`) now read `controller_of(id)` instead of `owner_of(id)`, so a
   stolen/reanimated permanent's table-driven "your …" watches (e.g. Keen Duelist's upkeep) fire
   for its current controller, never its original owner. Regression:
   `stolen_upkeep_watcher_fires_for_its_controller_not_its_owner`. Slices 2–3 still pending.
2. **Permanent-enters and attack watcher cleanup (M) — LANDED (2026-07-26).**
   `queue_permanent_enters_triggers`, `queue_self_permanent_enters_trigger`,
   `queue_watch_attack_triggers`, and `queue_batch_attack_triggers` now read `controller_of(id)`
   (graveyard-functional watchers are unaffected — `controller_of` returns the owner off the
   battlefield). Regressions: `stolen_doomwake_constellation_fires_for_its_new_controller` (enters
   path) and `stolen_breena_attack_watch_fires_for_its_new_controller` (attack path). Slice 3 still
   pending (Killian also needs #2; Herald also needs #5).
3. **Cast / draw / death special scanners (L).** Fix `queue_cast_spell_triggers`,
   `queue_player_draws_triggers`, `queue_an_enchanted_creature_dies_triggers`, and
   `queue_enchantment_death_watchers`; cover Sram, Kor Spiritdancer, Mangara, Pearl-Ear, Defacing
   Duskmage, Hateful Eidolon, and Starfield Mystic.

### 2. `goad-versus-attack-tax-if-able` — 8 cards, L
**Depends on:** none.
**Cards:** `coercive_impetus.toml`, `ghostly_prison.toml`, `ghoulish_impetus.toml`,
`killian_decisive_mentor.toml`, `martial_impetus.toml`, `nils_discipline_enforcer.toml`,
`parasitic_impetus.toml`, `redemption_arc.toml`
**Sketch:** `combat.rs` still treats attack taxes as an implicit auto-payment and then forces goaded
creatures to attack even when their controller cannot pay that tax. Silverquill makes that
shortcut false immediately by pairing `Ghostly Prison` / `Nils` with Killian and the whole goad
Aura package. Rework declare-attackers so "if able" is evaluated against the actual payable tax
set, not a hypothetical free attack. The shared seam is `attack_tax_owed` / `declare_attackers`;
the cleanest result is a real attack-tax payment choice rather than today's invisible generic
auto-plan. Tests should cover both "cannot pay, so the goaded creature is not forced to attack" and
"can pay, so the goaded creature must attack a legal non-goader."

### 3. `noncast-aura-attachment-legality` — 2 cards, M — **LANDED** (2026-07-26)
Landed: added `Game::noncast_attach_legal` (reuses `attachment_host_legal` + `protection_blocks_source`)
and routed both non-cast Aura moves through it — Ajani's Chosen's
`AttachTriggeringAuraToMintedToken` (guard-return, Aura stays put if the minted token fails its
enchant restriction) and Gift of Immortality's delayed `ReturnThisAuraAttachedTo` (returns nothing,
Aura stays in the graveyard, if the reanimated creature now has protection from the Aura). Ajani's
Chosen still needs increment #1's controller-scoped watch fix, so it stays unchecked on the deck
report.
**Depends on:** none.
**Cards:** `ajanis_chosen.toml`, `gift_of_immortality.toml`
**Sketch:** most Aura moves already re-check `attachment_host_legal`, but two Silverquill helpers
still bypass it: Ajani's Chosen's `AttachTriggeringAuraToMintedToken` and Gift of Immortality's
delayed `ReturnThisAuraAttachedTo`. Route both through the same legality choke that ordinary
attach/deploy paths already use (`apply.rs` + `maybe_pause_attach_deployed_aura`). The key
regressions are (a) Ajani's Chosen should not be able to snap `animate_dead.toml` onto the Cat
token, and (b) Gift returning onto a creature that gained protection from white should stay in the
graveyard instead of entering-and-then-falling-off through an illegal temporary attachment.

### 4. `attached-type-removal-layer` — 1 card, M — **LANDED** (2026-07-26)
Landed: `SetAttachedTypes`/`ContinuousEffectKind::SetTypes` gained a `set_types` flag (CR 613.4
"loses all other card types") that makes `add_types` the host's complete card-type list instead of
a union; `darksteel_mutation.toml` now sets `add_types = ["artifact", "creature"]` + `set_types =
true`, so mutating an enchantment creature (Doomwake Giant) leaves exactly an artifact creature.
**Depends on:** none.
**Cards:** `darksteel_mutation.toml`
**Sketch:** the current layer-4-ish type pipeline in `characteristics.rs` unions added card types
onto the printed line and replaces creature subtypes, but it has no "strip other card types"
surface. That is enough for additive Auras like Angelic Destiny, but not for Darksteel Mutation on
an enchantment creature like `doomwake_giant.toml`. Extend `ContinuousEffectKind::SetTypes` (or a
small sibling) so an attached effect can say "replace the host's card types with exactly these"
while still reusing the existing subtype replacement / lose-all-abilities logic. Regression should
assert that a mutated Doomwake is an artifact creature Insect with no enchantment type left.

### 5. `resolution-time-free-cast-from-dig` — 1 card, M
**Depends on:** none.
**Cards:** `herald_of_amity.toml`
**Sketch:** `Effect::Dig(ExileTopCastMatchingFree)` currently exiles the top eight, raises a choice,
grants a one-shot permission, and bottoms the rest. That means Herald's chosen Aura is cast at a
later priority window this turn, not during the ETB ability's own resolution. Add a pause/resume
path that keeps the chosen card in the resolving dig effect: choose the Aura, cast it without
paying its mana cost as part of the same resolution, then bottom the rest afterward. The current
chooser machinery in `pending/handlers/dig.rs` is already close; the missing piece is a
resolution-resume cast rather than a delayed `pending_next_cast` permission.

### 6. `simultaneous-each-player-life-change` — 1 card, S — **LANDED** (2026-07-26)
Landed: `LifeEffect::EachPlayerLoses { amount }` mints one simultaneous life-loss touching every
living player (controller included) in seat order; `defacing_duskmage.toml` now uses the single
`each_player_loses` effect instead of the each-opponent-then-you sequence, and its ponytail note is
gone.
**Depends on:** none.
**Cards:** `defacing_duskmage.toml`
**Sketch:** `Vandal's Edit` currently resolves "Each player loses 2 life" as
"each opponent loses 2, then you lose 2". Life totals match, but the event shape is not actually
simultaneous. Add a narrow life-family effect/event for uniform all-player life loss so later
watchers and replacements see one CR-shaped simultaneous event instead of a hand-authored sequence.
Keep the scope tight: Silverquill only needs one all-living-players loss pattern, not a whole
general simultaneous-life DSL.
