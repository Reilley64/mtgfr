# Card DSL and Card Pool

**Status:** Current (as of 2026-07-25)
**Module:** `crates/cards` (`data/*.toml`, `data/tokens/*.toml`), `crates/engine` (`src/de.rs`, `src/types/effect/` — `CardDef`, `Ability`, `Effect`, family enums, `Timing`), `docs/decklists/*.md`

---

## Problem Statement

Card behavior in Magic is vast and varied. Encoding it per-card in engine code would make every new card a code change and every bug a potential engine regression. The pool also needs to grow incrementally — one card at a time — with explicit tracking of what each card can and cannot do. Simultaneously, rules gaps must be surfaced as documentation rather than silent mis-modeling.

---

## Solution

Each card is a TOML file in `crates/cards/data/` that deserializes into a `CardDef` struct in `crates/engine`. `CardDef` is `Clone`, not `Copy`. Printed list-like fields (`abilities`, `keywords`, `conditional_keywords`, `identity_pips`, `colors`, `subtypes`, `otags`, `hand_ability`, `halves`) deserialize into `Arc<[T]>`, while runtime game objects and events intern each printed definition into a `CardId -> Arc<CardDef>` table and carry the small handle instead. Nested `[back]` / `[adventure]` faces are interned during deserialization as stable `CardId`s, so flip/adventure/prepare flows read and restore them without minting new handles at runtime. Card behavior is expressed as `Ability { timing, effect }` pairs; the `Effect` enum is the vocabulary. The DSL grows **only when a real card demands it** (card-dsl-and-card-pool spec). Gaps are flagged via the `approximates` field and `# ponytail:` comments rather than forced approximations. Token profiles live in `data/tokens/` and are referenced by Scryfall oracle id from creating cards.

Thirty-seven token profiles and 719 deckable card TOMLs are present as of 2026-07-26. Ten decklists live in `docs/decklists/*.md` (the five Secrets of Strixhaven decks and five additional non-SoC lists).

---

## User Stories

1. As a **card author**, I want to express a card's rules as a TOML file without writing Rust, so new cards can be added without engine changes when the required effect vocabulary already exists.
2. As a **card author**, I want a machine-readable `approximates` field and `# ponytail:` comments to flag where a card is mis-modeled, so the gap is documented and auditable.
3. As a **card author**, I want to reference a token profile by Scryfall oracle id so tokens aren't duplicated across creating cards.
4. As a **card author**, I want to flag a card as needing an engine feature that doesn't exist yet in that deck's fidelity increments backlog, rather than contorting the model.
5. As a **rules engine consumer**, I want printed definitions shared behind `CardId` handles so `Game` can still be cloned cheaply for snapshots and look-ahead without embedding fat `CardDef` values in every object and event.
6. As a **deck builder user**, I want the card catalog to surface `approximates` text so I know which cards are faithfully modeled and which have known gaps.
7. As a **deck builder user**, I want oracle tags (`otags`) for thematic search (e.g. "typal-spirit", "ramp") even for cards whose rules aren't implemented as a tag.
8. As a **test author**, I want to construct `CardDef` values inline in tests without parsing TOML, so unit tests are self-contained.
9. As a **Commander player**, I want my commander's color identity enforced at deck-build time, so I can't accidentally include off-color cards.
10. As a **player**, I want the pool's ~719 cards available for deck building, spanning the five SoC Commander precon lists and additional curated cards.

---

## Behavior

### TOML structure

Every card file opens with the verbatim Scryfall oracle text as a comment, then `name`, then top-level fields, then `[cost]`, then `[kind]`, then one or more `[[abilities]]` blocks. Each `[[abilities]]` block is preceded by a comment quoting the oracle sentence(s) it implements.

```toml
# Lightning Bolt deals 3 damage to any target.
name = "Lightning Bolt"
id = "4457ed35-7c10-48c8-9776-456485fdf070"
default_print = "7673784e-db4b-43a1-8d55-1bb9fc1e284f"
oracle = "Lightning Bolt deals 3 damage to any target."
set = "msc"

[cost]
red = 1

[kind]
type = "instant"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "damage"
mode = "target"
amount = 3
target = "any"
```

### Top-level field categories

**Identity:** `name` (registry key), `id` (Scryfall oracle id), `default_print` (Scryfall print UUID for art), `set` (set code), `oracle` (verbatim text for catalog hover), `otags` (Scryfall tagger slugs for search).

**Rules identity:** `legendary`, `colors` (explicit color override; empty = derive from cost pips), `devoid`, `identity_pips` (extra color-identity pips the simplified model would otherwise drop).

**Fidelity:** `approximates` (machine-readable gap note for the catalog and audits), `# ponytail:` inline comment at the divergence point.

**Alternative casts / graveyard mechanics:** `flashback`, `echo`, `cumulative_upkeep`, `recover`, `delve`, `escape`, `retrace`, `graveyard_cast_cost`, `cascade`, `demonstrate`, `devour`, `bestow`, `morph`, `evoke`, `adventure`, `back`, `suspend`, `encore`, `dredge`.

**Entry modifiers:** `enters_tapped`, `enters_tapped_unless`, `may_choose_not_to_untap`, `free_cast_if`.

**Miscellaneous:** `uncounterable`, `modal`/`choose`/`choose_max`/`choose_max_if_commander`, `enchant`, `enchant_graveyard`, `cycling`/`cycling_sacrifice`, `hand_ability`, `forecast`, `functions_in_graveyard`, `subtypes`, `cast_only_during_combat`, `cast_x_max` (non-mana cast-time `{X}` ceiling; `"player_count"` — Open the Way), `enter_as_copy`, `hand_ability`.

### `[cost]`

Fields: `generic`, `white`/`blue`/`black`/`red`/`green` (colored pips), `colorless` ({C} pips), `x` (bool or integer count for {X}), `hybrid` (array of two-color pairs for {A/B} symbols), `phyrexian` (array of two-value pairs for {A/P}). All optional; default 0/false. No `[cost]` = free (lands, tokens).

The `[cost.additional]` sub-table encodes additional costs like `kicker`/`buyback`/`strive`/`replicate` (optional extra costs the player may pay at cast) and `discard_land = true` (retrace).

### `[kind]`

Discriminates on `type`:

- `"creature"`: requires `power` and `toughness` (i32); optional `also` for dual-type creatures (e.g. `also = "artifact"` for artifact creatures). Creature subtypes go in top-level `subtypes`.
- `"instant"` / `"sorcery"`: collectively `CardKind::Spell`.
- `"enchantment"` / `"artifact"` / `"planeswalker"`: non-creature permanents. Planeswalker requires `loyalty: i32`.
- `"aura"`: permanent Aura; the `enchant` top-level field supplies the attach filter.
- `"land"`: optional `produces` (what mana it taps for), `subtypes` (Forest/Island/Plains/Swamp/Mountain for basic subtypes), `basic: true`.
- `"token"`: used only in `data/tokens/*.toml` files; not a deckable card type.

### `[[abilities]]`

Each ability block has a `timing` field and one or more `[[abilities.effects]]` entries. Optional: `condition` (intervening-if clause), `optional` (bool — "you may" trigger), `trigger` (what event fires this — for triggered abilities), `target` (what the whole ability targets, for targeted activated abilities).

### Timing variants

- `"spell"`: the card's own spell effect; fires on resolution.
- `"etb"`: triggered on entering the battlefield (ETB trigger, CR 603.6a).
- `"as_enters"`: "As this permanent enters, …" (CR 614.12) — a replacement effect, not a trigger. Watched off the same entry events as `"etb"` and queued ahead of it, but `Game::place_pending_triggers` runs the effect inline instead of placing it on the stack, so no player holds priority between the entry and the choice it raises.
- `"activated"`: activated ability; requires a `[abilities.cost]` sub-table with `taps_self`, `mana`, `pay_life`, `sacrifice`, `discard`, `x`.
- `"static"`: continuous effect active while the permanent is on the battlefield.
- `"each_upkeep"` / `"your_upkeep"` / `"each_end_step"` / `"your_end_step"` / `"begin_combat"` / `"declare_attackers"` / `"this_attacks"` / `"this_attacks_or_blocks"` / `"this_leaves_battlefield"` / `"turned_face_up"` / `"this_dies"` / etc.: self-referential triggered timings.
- Trigger watch events also reference player-scoped timings: `"cast_spell"`, `"draw_card"`, `"gain_life"`, etc.

### Effect vocabulary (representative sample)

The `Effect` enum grows only from real cards. Every leaf effect is authored as nested **`type` (family) + `mode` (leaf)**. Structural composers (`sequence`, `conditional`, `choose_one`) are the only effects with no `mode`. Family and mode vocabulary live in `crates/engine/src/types/effect/`; grow it only when a real card demands a new leaf.

Top-level families: `damage`, `draw`, `life`, `destroy`, `exile`, `sacrifice`, `control`, `counters`, `mana`, `mill`, `pump`, `reveal`, `token`, `zone`, `copy`, `dig`, `choice`, `static`, `misc`, plus structural `sequence` / `conditional` / `choose_one`.

Representative modes by family:

**`damage`:** `target` (to creature/player/planeswalker), `each_creature` (mass damage), `to_self`.

**`destroy`:** `target`, `all`, `triggering_damaged_creature`.

**`exile`:** `target`, `all`, `all_graveyards`, `graveyard`, `until_source_leaves`.

**`sacrifice`:** `source`, `enchanted_creature` (plus engine-internal object sacrifice helpers).

**`control`:** `tap_target`, `tap_all`, `gain_control`, `gain_control_while`.

**`counters`:** `put_counters`, `remove_counter`, `move_counters`.

**`token`:** `create` (references a token profile by id or inline definition), `create_copy`, `create_treasure`.

**`draw`:** `cards`, `target_player`, `each_player`; **`mill`:** `cards`.

**`dig`:** `search_library`, `scry`, `surveil`, `look_at_top`, `distribute_top`, `cascade`, `clash`.

**`life`:** `gain`, `lose`, `each_opponent_drain`, `gain_target_controller`.

**`mana`:** `add` (one or more `Mana` values; optional `repeat: Amount` for scaled production).

**`pump`:** `pump_until_end_of_turn` (target creature +N/+M until EOT); **`static`:** `anthem` (continuous anthem for matching permanents; `power`, `toughness`, `keywords` axes; optional filters), `grant_to_attached`, `enters_with_counters`, `grant_mana_ability` (grants every matching permanent an activated mana ability from an inline `filter` + `[…cost]` + `mana` list — e.g. Goldspan Dragon giving your Treasures "{T}, Sacrifice this artifact: Add two mana of any one color"; `single_color = true` locks all mana credits to one named color, so activating pauses on a `ChooseManaColor` choice rather than producing independent wildcards, CR 106.4).

**`copy`:** `target_spell`, `this_spell`, `copy_triggering_spell`. Cast-time self-copy riders (Gravestorm, Storm, and Plumb the Forbidden's reflexive "When you do") live on a `timing = "when_you_cast_this"` ability using `copy_triggering_spell` (with a `count` `Amount` such as `permanents_died_this_turn`, `spells_cast_before_this`, or `spell_sacrifice_count`): the copies mint on the stack *above* the still-unresolved original (CR 706.9), unlike resolution-time `this_spell`. A reflexive "When you do" (Plumb) additionally carries a `condition` (`spell_sacrificed_to_cast`) so the trigger doesn't happen at all when nothing was sacrificed — no zero-count copy trigger reaches the stack — whereas a keyword Gravestorm/Storm trigger always fires and simply copies zero times when its count is 0.

**`zone`:** `reanimate_to_battlefield`, `return_to_hand`, `flicker_target`, `exile_dead_creature_create_copy_with_subtype`.

**`choice`:** `may_sacrifice`, `may_draw_up_to`, `discard`, `proliferate`, `may_return_from_graveyard` (return a matching card from your graveyard to your hand), `may_put_counter_on_creature`. `may_return_from_graveyard` takes an optional `mandatory` bool (default `false`): the optional "you may return" consumers (Deadly Brew, Witch of the Moors) leave it `false` and the choice is declinable, while a mandatory "you return" (Witherbloom Command mode 0) sets `mandatory = true` so declining is rejected when a legal card exists — with no legal card the effect simply does nothing (no pause) either way. `may_put_counter_on_creature` (fieldless) is a resolution-time optional "you may put a +1/+1 counter on a creature" chosen during resolution over any battlefield creature and never advertised as a stack target (Zimone's Hypothesis' primer); it carries no `then`, so its follow-up runs as the next `Sequence` step regardless, and it is answered by `Intent::ChooseCopyTarget` (one object or none), projected onto the generic `ChooseCopyTarget` pick-or-decline view.

**`misc`:** `fight`, `counter_target_spell`, `schedule_at_next_upkeep`.

**Structural:** `sequence` (run multiple effects in order), `conditional` (if condition then effects else effects), `choose_one` (modal dispatch).

### Amount

`Amount` is the polymorphic numeric type used anywhere a count or numeric value appears. Variants include `Fixed(n)`, `X` (the cast's {X} value), `TargetPower`, `LifeGainedThisTurn`, `CreaturesDiedThisTurn`, `SpellsCastThisTurn`, `CommanderColorCount`, `PerPermanentMatching { filter }`, `AurasAttachedToSource`, `NontokenCreaturesEnteredThisTurn`, `TriggeringSpellManaValue`, `CombatDamage`, `SacrificedCreaturePower`, `PermanentsDestroyedThisWay`, and others. `Amount` can appear as `count` on `create_token`, `draw_cards`, `gain_life`, activated cost `pay_life`, etc.

### Token profiles

Token profiles live in `data/tokens/*.toml`. They are full `CardDef` instances with `[kind] type = "token"`. They carry `colors`, `subtypes`, and optional abilities (e.g. a Pest token that gains life on death). Creating cards reference them by oracle id: `token = "uuid"`. The `install_token_defs` / `token_def` APIs load and query the registry, and token creation interns the chosen definition into a `CardId` before attaching it to a live object or event. Current tokens: 37 profiles covering Angel, Beast, Cat, Dragon, Elemental, Food, Fractal, Goat, Inkling, Insect, Myr, Pest, Saproling, Snake, Soldier, Spirit, Treasure, Thopter, Zombie, and others.

### Fidelity discipline

- **`approximates` + `# ponytail:` are both required** when a card diverges from oracle text. The `approximates` field is machine-readable (catalog, audit scripts); the inline comment is for human reviewers. Silence means faithful.
- **Flag, don't force-script.** When a card needs an `Effect` the DSL cannot express, it is noted in the active deck's `docs/fidelity/<slug>-increments.md` with an effort estimate and increment number, and the card carries an `approximates` note. No card should contort the engine.
- **Oracle text comment first.** Every card file opens with verbatim oracle text as a comment, then `name`. Reviewers read what the card does before seeing how it's modeled.

### Precon decklists and card pool scope

Ten decklists live in `docs/decklists/*.md`:

- Five Secrets of Strixhaven (`soc`) Commander precons: Witherbloom Pestilence, Silverquill Influence, Quandrix Unlimited, Prismari Artistry, Lorehold Spirit.
- Five additional lists: Political Puppets, Mirror Mastery, Enchantress Rubinia, Deathdancer Xira,
  Heavenly Inferno.

These are the **first faithful target** (card-dsl-and-card-pool spec): every card in these lists should be faithfully representable in the DSL, with `approximates` notes for known gaps. The north star (card-dsl-and-card-pool spec) is any card, faithfully — the SoC decks are the proving ground, not the ceiling.

### Deck-builder legality

- A Commander deck must have exactly one legendary creature commander, 99 other cards, singleton (except basic lands), and every card's **color identity** within the commander's.
- Color identity is derived from cost pips + `colors` override + hybrid/phyrexian pips + `identity_pips` (extra pips for trimmed abilities).
- Legality is validated server-side on save; the response returns all problems at once.

---

## Implementation Decisions

- **`CardDef` is `Clone`, not `Copy`; runtime uses `CardId`.** Once a card enters a `Game` it is interned via `intern_card_def` and referenced by `CardId` through shared `Arc<CardDef>` lookups. `CardDef`'s own list-like DSL fields now load into shared `Arc<[T]>` storage, while `Effect::Sequence` / `ChooseOne` / `Conditional` payload lists deserialize into `Arc<[Effect]>`, so authored card text and runtime contextualizers both avoid the older plain-slice leaks. Nested back/adventure/split faces are interned as `CardId`s during load.
- **`Effect` enum grows only from real card demand (card-dsl-and-card-pool spec).** New behavior = new `Effect` variant + `Game::run` arm + `Event::apply` arm + TOML authoring. The DSL never anticipates future cards.
- **Token profiles are pre-loaded into a `OnceLock<HashMap<&'static str, CardDef>>` before deckable cards.** `install_token_defs` must be called before any card TOML that references a token by id is deserialized. `cards` crate's `load` function handles this ordering, and token creation interns the selected profile before storing it on a live object/event.
- **The `card-dsl` feature flag gates all DSL deserialization.** The engine can be compiled without TOML parsing (e.g. for pure engine tests that construct `CardDef` inline). The feature adds `serde` derives and `de.rs`.
- **`de.rs` holds only structurally-divergent deserializers.** Types whose TOML spelling matches their Rust shape use serde derives on the definitions in `types/effect/`. Only when the TOML spelling differs structurally (flat cost table, `instant`/`sorcery` as separate strings, folded `Timing::Activated`) does `de.rs` provide a manual impl.
- **`otags` and `set` are pure catalog metadata** — the engine never reads them. They exist for deck-builder search (`set`/`subtypes` + Postgres catalog search, accounts-decks-and-catalog spec) and Scryfall tagger integration.
- **`oracle` is catalog metadata** — the engine never parses it; rules behavior comes from `abilities`/`keywords` only.
- **`approximates` is surfaced in the card catalog** so the deck builder and audits see the same gap the engine runs. An absent `approximates` field means the card is faithful.
- **`grant_mana_ability` is read live off the static scan, never resolved off the stack.** The granted `[…cost]` + `mana` pair is synthesized into an activated ability on each matching permanent (the mana twin of `grant_to_attached`), so it appears and disappears with the granting permanent. `single_color` is the granted twin of `ManaEffect::Add`'s own `single_color`: both reuse the `ChooseManaColor` pause and then emit one credit per `mana` entry in the chosen color (CR 106.4).

---

## Testing Decisions

- **Card TOML tests**: the `cards` crate's tests deserialize a sample of known cards and assert `CardDef` fields match expected values (correct cost pips, correct ability count, correct effect type).
- **Inline `CardDef` in engine tests**: the engine's own unit and integration tests construct `CardDef` values directly (no TOML parsing) using struct literal syntax, keeping tests self-contained and avoiding the `card-dsl` feature.
- **Fidelity regression tests**: for each card in `docs/decklists/*.md`, a CI test verifies the card TOML is present and parses without error. Presence of `approximates` is tracked but not a failure.
- **Effect roundtrip test**: for each `Effect` variant, at least one card TOML in the pool should exercise it (verified by the fidelity audit tooling).
- **Token profile tests**: `install_token_defs` is called with the full token set; `token_def(id)` returns the correct profile for known ids.
- The `.agents/skills/card-dsl/SKILL.md` and `DSL_REFERENCE.md` are the authoring guide for card authors; the skill specifies the full field reference and non-negotiable discipline.

---

## Out of Scope

- **CR 613 full layers (type-changing, lose-all-abilities, dependency ordering)** — partially implemented (7b base-set, 7c additive mods). Full layer stack deferred (engine-core-and-event-model spec); schedule via a deck's fidelity increments when needed.
- **Replacement effects / damage prevention (general)** — specific patterns (combat damage prevention shields, commander redirect) are implemented. General CR 614 framework is a backlog item for decks that need it.
- **Sideboard / wish effects** — cards that retrieve cards from outside the game are not implemented; no sideboard concept exists in the engine.
- **Partner commanders** — a deck has exactly one commander. Partner/Partner With is not yet modeled.
- **Aftermath / fused split-card casting** — not in the current pool. The DSL supports split halves via `[[half]]`, but only one half is cast at a time; fused-both-halves and aftermath-specific rules are still out of scope.
- **Sagas** — not in the current pool or DSL.
- **Class enchantments** — Class leveling is a known gap; add to a deck's increments when a grind includes Classes.
- **Complete `ponytail:` debt** — deliberate approximations stay on cards as `approximates` / `# ponytail:`; engine work is scheduled per-deck under `docs/fidelity/` when a grind needs it.

---

## Further Notes

- See `2026-07-20-engine-core-and-event-model.md` for how `CardDef` and `Effect` are consumed at runtime by `Game::run`.
- See `2026-07-20-choices-actions-and-resolution.md` for how effect types map to `PendingChoice` variants.
- `CONTEXT.md` defines **card**, **effect**, **ability**, **timing**, **keyword**, **populate**, and related terms.
- Per-deck `docs/fidelity/<slug>-increments.md` files (created by `fidelity-grind`) are the living engine-capability backlogs.
- `.agents/skills/card-dsl/DSL_REFERENCE.md` is the complete authoring field reference.
- The `just engine-cr-index` recipe regenerates `docs/CR_INDEX.md` from CR citations across the engine; check it after adding new rules behaviors.
