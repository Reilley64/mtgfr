# Card DSL and Card Pool

**Status:** Current (as of 2026-07-20)
**Module:** `crates/cards` (`data/*.toml`, `data/tokens/*.toml`), `crates/engine` (`src/de.rs`, `src/types.rs` — `CardDef`, `Ability`, `Effect`, `Timing`), `docs/decklists/*.md`

---

## Problem Statement

Card behavior in Magic is vast and varied. Encoding it per-card in engine code would make every new card a code change and every bug a potential engine regression. The pool also needs to grow incrementally — one card at a time — with explicit tracking of what each card can and cannot do. Simultaneously, rules gaps must be surfaced as documentation rather than silent mis-modeling.

---

## Solution

Each card is a TOML file in `crates/cards/data/` that deserializes into a `CardDef` struct in `crates/engine`. `CardDef` is `Copy` and `&'static` — all ability slices are interned at load time. Card behavior is expressed as `Ability { timing, effect }` pairs; the `Effect` enum is the vocabulary. The DSL grows **only when a real card demands it** (card-dsl-and-card-pool spec). Gaps are flagged via the `approximates` field and `# ponytail:` comments rather than forced approximations. Token profiles live in `data/tokens/` and are referenced by Scryfall oracle id from creating cards.

Thirty-five token profiles and 618 deckable card TOMLs are present as of 2026-07-20. Eight precon decklists live in `docs/decklists/*.md` (the five Secrets of Strixhaven decks and three additional non-SoC lists).

---

## User Stories

1. As a **card author**, I want to express a card's rules as a TOML file without writing Rust, so new cards can be added without engine changes when the required effect vocabulary already exists.
2. As a **card author**, I want a machine-readable `approximates` field and `# ponytail:` comments to flag where a card is mis-modeled, so the gap is documented and auditable.
3. As a **card author**, I want to reference a token profile by Scryfall oracle id so tokens aren't duplicated across creating cards.
4. As a **card author**, I want to flag a card as needing an engine feature that doesn't exist yet in that deck's fidelity increments backlog, rather than contorting the model.
5. As a **rules engine consumer**, I want `CardDef` to be `Copy` so `Game` can be cheaply cloned for snapshots and look-ahead.
6. As a **deck builder user**, I want the card catalog to surface `approximates` text so I know which cards are faithfully modeled and which have known gaps.
7. As a **deck builder user**, I want oracle tags (`otags`) for thematic search (e.g. "typal-spirit", "ramp") even for cards whose rules aren't implemented as a tag.
8. As a **test author**, I want to construct `CardDef` values inline in tests without parsing TOML, so unit tests are self-contained.
9. As a **Commander player**, I want my commander's color identity enforced at deck-build time, so I can't accidentally include off-color cards.
10. As a **player**, I want the pool's ~618 cards available for deck building, spanning the five SoC Commander precon lists and additional curated cards.

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

**Miscellaneous:** `uncounterable`, `modal`/`choose`/`choose_max`/`choose_max_if_commander`, `enchant`, `enchant_graveyard`, `cycling`/`cycling_sacrifice`, `hand_ability`, `forecast`, `functions_in_graveyard`, `subtypes`, `cast_only_during_combat`, `enter_as_copy`, `hand_ability`.

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
- `"activated"`: activated ability; requires a `[abilities.cost]` sub-table with `taps_self`, `mana`, `pay_life`, `sacrifice`, `discard`, `x`.
- `"static"`: continuous effect active while the permanent is on the battlefield.
- `"each_upkeep"` / `"your_upkeep"` / `"each_end_step"` / `"your_end_step"` / `"begin_combat"` / `"declare_attackers"` / `"this_attacks"` / `"this_attacks_or_blocks"` / `"this_leaves_battlefield"` / `"turned_face_up"` / `"this_dies"` / etc.: self-referential triggered timings.
- Trigger watch events also reference player-scoped timings: `"cast_spell"`, `"draw_card"`, `"gain_life"`, etc.

### Effect vocabulary (representative sample)

The `Effect` enum grows only from real cards. Currently implemented effect types include:

**Damage:** `deal_damage` (to target or player), `damage_each_creature` (mass damage), `fight` (two creatures deal damage to each other), `damage_player_for_each` (Torment of Hailfire shape).

**Targeting / removal:** `exile_target`, `destroy_target`, `return_to_hand`, `counter_target`, `tap_target`, `tap_all`, `destroy_all`, `exile_all`.

**Counters:** `put_plus_one_counters`, `remove_counter`, `proliferate`, `enters_with_counters`.

**Token creation:** `create_token` (references a token profile by id, or an inline creature definition; `count` takes an `Amount`; `controller` specifies recipient: `you`/`each_opponent`/`each_player`/`one_per_opponent`; optional `required_defender` for goad-on-create).

**Card draw / library:** `draw_cards`, `mill`, `search_library`, `scry`, `surveil`, `arrange_top`.

**Life:** `gain_life`, `lose_life`, `each_opponent_drain` (drain each opponent), `gain_life_target_controller`.

**Mana:** `add_mana` (one or more `Mana` values; optional `repeat: Amount` for scaled production).

**Pump / anthem:** `pump_until_end_of_turn` (target creature gets +N/+M until EOT), `anthem_static` (continuous anthem for matching permanents; `power`, `toughness`, `keywords` axes; optional `subtypes`/`colors`/`attacking_only`/`condition`/`self_only`/`exclude_source` filters), `grant_to_attached` (pump the enchanted/equipped permanent; takes `Amount`).

**Control / copy:** `gain_control`, `gain_control_while` (condition-scoped steal), `copy_target_spell`, `create_token_copy`.

**Zone change:** `reanimate_to_battlefield`, `return_permanent_to_hand`, `put_on_top_of_library`, `exile_with_linked_return` (blink), `exile_dead_creature_create_copy_with_subtype`.

**Special:** `sequence` (run multiple effects in order), `conditional` (if condition then effect else effect), `may_yes_no` (optional trigger — wraps another effect), `choose_mode` (modal dispatch), `clash`, `populate`, `draw_until_hand_size`.

### Amount

`Amount` is the polymorphic numeric type used anywhere a count or numeric value appears. Variants include `Fixed(n)`, `X` (the cast's {X} value), `TargetPower`, `LifeGainedThisTurn`, `CreaturesDiedThisTurn`, `SpellsCastThisTurn`, `CommanderColorCount`, `PerPermanentMatching { filter }`, `AurasAttachedToSource`, `NontokenCreaturesEnteredThisTurn`, `TriggeringSpellManaValue`, `CombatDamage`, `SacrificedCreaturePower`, `PermanentsDestroyedThisWay`, and others. `Amount` can appear as `count` on `create_token`, `draw_cards`, `gain_life`, activated cost `pay_life`, etc.

### Token profiles

Token profiles live in `data/tokens/*.toml`. They are full `CardDef` instances with `[kind] type = "token"`. They carry `colors`, `subtypes`, and optional abilities (e.g. a Pest token that gains life on death). Creating cards reference them by oracle id: `token = "uuid"`. The `install_token_defs` / `token_def` APIs load and query the registry. Current tokens: 35 profiles covering Angel, Beast, Cat, Dragon, Elemental, Food, Fractal, Goat, Inkling, Insect, Myr, Pest, Saproling, Snake, Soldier, Spirit, Treasure, Thopter, Zombie, and others.

### Fidelity discipline

- **`approximates` + `# ponytail:` are both required** when a card diverges from oracle text. The `approximates` field is machine-readable (catalog, audit scripts); the inline comment is for human reviewers. Silence means faithful.
- **Flag, don't force-script.** When a card needs an `Effect` the DSL cannot express, it is noted in the active deck's `docs/fidelity/<slug>-increments.md` with an effort estimate and increment number, and the card carries an `approximates` note. No card should contort the engine.
- **Oracle text comment first.** Every card file opens with verbatim oracle text as a comment, then `name`. Reviewers read what the card does before seeing how it's modeled.

### Precon decklists and card pool scope

Eight decklists live in `docs/decklists/*.md`:

- Five Secrets of Strixhaven (`soc`) Commander precons: Witherbloom Pestilence, Silverquill Influence, Quandrix Unlimited, Prismari Artistry, Lorehold Spirit.
- Three additional lists: Political Puppets, Enchantress Rubinia, Deathdancer Xira.

These are the **first faithful target** (card-dsl-and-card-pool spec): every card in these lists should be faithfully representable in the DSL, with `approximates` notes for known gaps. The north star (card-dsl-and-card-pool spec) is any card, faithfully — the SoC decks are the proving ground, not the ceiling.

### Deck-builder legality

- A Commander deck must have exactly one legendary creature commander, 99 other cards, singleton (except basic lands), and every card's **color identity** within the commander's.
- Color identity is derived from cost pips + `colors` override + hybrid/phyrexian pips + `identity_pips` (extra pips for trimmed abilities).
- Legality is validated server-side on save; the response returns all problems at once.

---

## Implementation Decisions

- **`CardDef` is `Copy` and `&'static`.** `intern` / `static_slice` in `de.rs` leak owned vecs into static slices at load time (a bounded, load-once pool). This enables zero-cost `Clone` of `Game` and eliminates per-card heap allocation at runtime.
- **`Effect` enum grows only from real card demand (card-dsl-and-card-pool spec).** New behavior = new `Effect` variant + `Game::run` arm + `Event::apply` arm + TOML authoring. The DSL never anticipates future cards.
- **Token profiles are pre-loaded into a `OnceLock<HashMap<&'static str, CardDef>>` before deckable cards.** `install_token_defs` must be called before any card TOML that references a token by id is deserialized. `cards` crate's `load` function handles this ordering.
- **The `card-dsl` feature flag gates all DSL deserialization.** The engine can be compiled without TOML parsing (e.g. for pure engine tests that construct `CardDef` inline). The feature adds `serde` derives and `de.rs`.
- **`de.rs` holds only structurally-divergent deserializers.** Types whose TOML spelling matches their Rust shape use serde derives on the definitions in `types.rs`. Only when the TOML spelling differs structurally (flat cost table, `instant`/`sorcery` as separate strings, folded `Timing::Activated`) does `de.rs` provide a manual impl.
- **`otags` and `set` are pure catalog metadata** — the engine never reads them. They exist for deck-builder search (`set`/`subtypes` + Postgres catalog search, accounts-decks-and-catalog spec) and Scryfall tagger integration.
- **`oracle` is catalog metadata** — the engine never parses it; rules behavior comes from `abilities`/`keywords` only.
- **`approximates` is surfaced in the card catalog** so the deck builder and audits see the same gap the engine runs. An absent `approximates` field means the card is faithful.

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
- **Split cards / Aftermath** — not in the current pool or DSL.
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
