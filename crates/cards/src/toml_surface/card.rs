use std::sync::Arc;

use serde::Deserialize;

use crate::de::{TimingName, arc_strs, one_u8};
use crate::toml_surface::{CostToml, KindToml};
use crate::{
    Ability, AlternativeCost, Amount, BecomesTargetedScope, CardDef, CastXMax, CasterScope, Color,
    CombatDamageScope, Condition, Cost, CounterKind, CumulativeUpkeepCost, Effect, EnterAsCopy,
    EnterController, EscapeCost, HandActivatedAbility, Keyword, PermanentFilter, SacrificeCost,
    SpellFilter, SpendToCastPredicate, Suspend, intern_card_def,
};

/// An `[[abilities]]` table as spelled in TOML — flat: the `timing` string plus every cost
/// piece, trigger sibling, and the `[[abilities.effects]]` list beside it. [`Ability`]'s
/// `Deserialize` impl folds this into the nested runtime shape (`Timing::Activated`'s
/// [`crate::ActivationCost`], the [`crate::Trigger`] a tag names, a one-or-many `effects`
/// list). Sibling fields are read only by the timings that print them; the rest are ignored.
#[derive(Deserialize)]
#[cfg_attr(
    feature = "card-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(deny_unknown_fields)]
pub struct AbilityToml {
    pub(crate) timing: TimingName,
    #[serde(default)]
    pub(crate) taps_self: bool,
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "CostToml"))]
    pub(crate) activation_cost: Cost,
    #[serde(default)]
    pub(crate) sacrifice: SacrificeCost,
    #[serde(default)]
    pub(crate) pay_life: Amount,
    /// +1/+1 counters removed from the source as part of the activation cost (CR 118
    /// "remove a counter" cost — Steelbane Hydra's "Remove a +1/+1 counter from this
    /// creature").
    #[serde(default)]
    pub(crate) remove_counters: u8,
    /// Which counter kind `remove_counters` removes; unset (the default) is the +1/+1
    /// path above (staff_of_the_storyteller's "remove a story counter" sets this).
    #[serde(default)]
    pub(crate) remove_counters_kind: Option<CounterKind>,
    /// "Remove X storage counters from this land" (fungal_reaches.toml) — the removal
    /// count is a player-declared `{X}` instead of `remove_counters`'s fixed count.
    #[serde(default)]
    pub(crate) remove_counters_x: bool,
    #[serde(default)]
    pub(crate) self_damage: u8,
    #[serde(default)]
    pub(crate) loyalty: Option<i32>,
    /// "Activate only once each turn" (CR 602.2b) on an activated ability, or "this
    /// ability triggers only once each turn" (CR) on a triggered one — one TOML key
    /// feeding whichever struct `timing` resolves to (`ActivationCost` or [`Ability`]).
    #[serde(default)]
    pub(crate) once_each_turn: bool,
    /// "Activate only as a sorcery" (CR 602.5b): restricts activation to a legal
    /// sorcery-speed moment (Ozolith, the Shattered Spire's counter ability).
    #[serde(default)]
    pub(crate) sorcery_speed: bool,
    /// "Activate only during an opponent's turn" (CR 602.5b — Nettling Imp): someone other
    /// than the activating player must be the active player. Composable with the siblings
    /// below the same way the `cast_only_*` card restrictions are with each other.
    #[serde(default)]
    pub(crate) only_during_opponents_turn: bool,
    /// "Activate only during your turn" (CR 602.5b — Instill Energy) — the mirror of the
    /// sibling above. Not the same as `sorcery_speed`, which also demands an empty stack and
    /// a main phase; this one only asks whose turn it is, so it activates in combat.
    #[serde(default)]
    pub(crate) only_during_your_turn: bool,
    /// "…before attackers are declared" (CR 602.5b — Nettling Imp): the activation-side twin
    /// of `cast_only_before_attackers`. The window runs up to and including the
    /// declare-attackers step and shuts the moment the declaration is made.
    #[serde(default)]
    pub(crate) only_before_attackers: bool,
    /// "Activate only during your upkeep" (CR 602.5b — Cyclopean Tomb): a step-and-controller
    /// window rather than the phase-shaped ones above. The controller half is what makes it
    /// *your* upkeep.
    #[serde(default)]
    pub(crate) only_during_your_upkeep: bool,
    /// "Only this creature's owner may activate this ability" (CR 602.5c — Personal
    /// Incarnation): the pool's one activation restriction keyed to ownership rather than
    /// control.
    #[serde(default)]
    pub(crate) only_owner_may_activate: bool,
    /// "Return this to its owner's hand" as part of the cost (Rootha, Mercurial
    /// Artist's "Return Rootha to its owner's hand").
    #[serde(default)]
    pub(crate) return_self: bool,
    /// "Mill a card" as part of the cost (Millikin's "Mill a card").
    #[serde(default)]
    pub(crate) mill_self: u8,
    /// "Discard a card" as part of the cost (Wild Mongrel's "Discard a card").
    #[serde(default)]
    pub(crate) discard_cost: u8,
    /// "Exile this artifact"/"exile this permanent" as part of the cost (Perpetual
    /// Timepiece's "Exile this artifact").
    #[serde(default)]
    pub(crate) exile_self: bool,
    /// "Exile N target cards from an opponent's graveyard" as an additional cost
    /// (Spurnmage Advocate's "Exile two target cards from an opponent's graveyard").
    #[serde(default)]
    pub(crate) graveyard_exile_target_count: u8,
    #[serde(default)]
    pub(crate) condition: Option<Condition>,
    #[serde(default)]
    pub(crate) optional: bool,
    /// The minimum Class level this ability requires to function (CR 717.5 — a Class's
    /// level-gated abilities). `0` (the default, omitted in TOML) is unconditional.
    #[serde(default)]
    pub(crate) min_level: u8,
    /// The cost to accept an `optional` trigger (CR 603.2c "you may pay …"), e.g. Trudge
    /// Garden's "you may pay {2}." Ignored for a non-optional ability. Same `[cost]`-table
    /// shape as a spell's own top-level cost (§2); omitted = a plain free "may".
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "CostToml"))]
    pub(crate) cost: Cost,
    /// The permanent filter for a `you_sacrifice`/`any_player_sacrifices`/
    /// `permanent_enters` trigger (Smothering Abomination's "a creature", Mazirek's
    /// "another permanent", Ajani's Chosen's "an enchantment"). Ignored for every other
    /// trigger/timing.
    #[serde(default)]
    pub(crate) filter: PermanentFilter,
    /// Which tap a `permanent_becomes_tapped` trigger watches — `false` (the default) is
    /// every tap there is (Lifetap: an attack, an Icy Manipulator, a mana ability alike),
    /// `true` narrows to a land tapped *for mana* (Manabarbs, CR 106.11 — a tap that actually
    /// produced mana). Ignored for every other trigger/timing.
    #[serde(default)]
    pub(crate) for_mana: bool,
    /// Whose permanent a `permanent_enters` trigger watches — `you` (default,
    /// constellation's "an enchantment you control"), `opponent` (Archaeomancer's Map's
    /// "a land an opponent controls"), or `any_player`. Ignored for every other
    /// trigger/timing.
    #[serde(default)]
    pub(crate) controller: EnterController,
    /// Who a `deals_combat_damage_to_player` trigger watches (Leitmotif Composer's
    /// `this`, Ohran Frostfang's `your_creatures`, Curiosity Crafter's `your_tokens`,
    /// Contaminant Grafter's batch-once `your_creatures_batch`). Ignored for every other
    /// trigger/timing.
    #[serde(default)]
    pub(crate) who: CombatDamageScope,
    /// Which permanent a `becomes_targeted` trigger watches — `this` (default, Goldspan
    /// Dragon) or `creature_you_control` (Venerated Rotpriest). Ignored for every other
    /// trigger/timing.
    #[serde(default)]
    pub(crate) targeted: BecomesTargetedScope,
    /// The spell filter for a `cast_spell` trigger (Monologue Tax's "a spell", Sram
    /// Senior Edificer's "an Aura, Equipment, or Vehicle spell"). Named distinctly from
    /// `filter` (a [`PermanentFilter`], taken by the sacrifice triggers above). Ignored
    /// for every other trigger/timing.
    #[serde(default)]
    pub(crate) spell_filter: SpellFilter,
    /// Whose cast a `cast_spell` trigger watches — `you` (default), `opponent`
    /// (Monologue Tax, Mangara), or `any_player`. Ignored for every other trigger/timing.
    #[serde(default)]
    pub(crate) caster: CasterScope,
    /// Whose draw a `player_draws` trigger watches — `you` (default), `opponent`
    /// (Faerie Mastermind), or `any_player`. Ignored for every other trigger/timing.
    #[serde(default)]
    pub(crate) drawer: CasterScope,
    /// Restricts a `cast_spell`/`player_draws` trigger to exactly the watched player's
    /// Nth spell/draw that turn (Monologue Tax/Mangara's "their second spell each turn",
    /// Faerie Mastermind's "their second card each turn" — `2`). `None` (the default,
    /// omitted in TOML) fires on every matching cast/draw. Ignored for every other
    /// trigger/timing.
    #[serde(default)]
    pub(crate) nth_each_turn: Option<u8>,
    /// Restricts a `cast_spell` trigger to a spell cast from its controller's hand (CR
    /// 601's default cast zone) — Dirgur Focusmage's "you cast … from your hand". `false`
    /// (the default, omitted in TOML) fires on a cast from any zone (flashback/escape,
    /// the command zone, an impulse-play permission). Ignored for every other
    /// trigger/timing.
    #[serde(default)]
    pub(crate) from_hand: bool,
    /// The attacker-count threshold for a `you_attack_with_creatures`/
    /// `opponent_attacks_you_with_creatures`/`creature_enchanted_by_your_aura_attacks`
    /// trigger (Firemane Commando/Mangara/Tomik's "two or more creatures" — `2`; Killian,
    /// Decisive Mentor's "one or more" — `1`). Ignored for every other trigger/timing.
    #[serde(default)]
    pub(crate) at_least: u8,
    /// Which cast a `spend_mana_to_cast` trigger accepts (Study Hall/Opal Palace's
    /// `commander`, Path of Ancestry's `creature_sharing_type_with_commander`). Ignored for
    /// every other trigger/timing; the field is required only when `timing =
    /// "spend_mana_to_cast"`, defaulting to `commander` otherwise (unread).
    #[serde(default = "crate::de::default_spend_predicate")]
    pub(crate) spend_predicate: SpendToCastPredicate,
    /// The ability's effect(s), always the array-of-tables `[[abilities.effects]]` form
    /// (even a single-effect ability uses a one-element list). An ordered list runs as one
    /// resolution, sharing the ability's target/`{X}` (Faithless Looting's "draw two cards,
    /// then discard two cards"); a one-element list is just that effect (no Sequence
    /// wrapper).
    #[serde(default)]
    pub(crate) effects: Vec<Effect>,
}

/// One entry of `CardDef::conditional_keywords` as spelled in TOML — an
/// `{ condition, keyword }` table folded into a `(Condition, Keyword)` tuple at load.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(
    feature = "card-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(deny_unknown_fields)]
pub struct ConditionalKeywordToml {
    pub condition: Condition,
    #[cfg_attr(feature = "card-schema", schemars(with = "String"))]
    pub keyword: Keyword,
}

/// A card's top-level TOML table.
///
/// This mirrors the DSL's top-level card shape with owned fields, then folds into the interned
/// `CardDef` representation used by the engine.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(
    feature = "card-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(deny_unknown_fields)]
pub struct CardToml {
    /// Scryfall oracle id — required on top-level pool TOMLs (enforced at registry load).
    /// Nested faces/tokens may omit it (`""`).
    #[serde(default)]
    pub id: String,
    /// Scryfall card UUID for the default Printing — required on top-level pool TOMLs.
    #[serde(default)]
    pub default_print: String,
    /// Printed card name and card-pool registry key. The filename is arbitrary; this field
    /// is what authors, tests, and the catalog use to find a card.
    pub name: String,
    #[serde(
        default,
        deserialize_with = "crate::toml_surface::deserialize_cost_toml"
    )]
    /// Printed mana cost. Omit the table for free cards such as lands and most token
    /// profiles.
    pub cost: CostToml,
    /// Printed card kind/type line as a `[kind]` table.
    pub kind: KindToml,
    /// An Aura's enchant subject restriction (CR 303.4a) — `enchant = { … }`, the same
    /// [`PermanentFilter`] table/shorthand shape as any other filter field; absent means
    /// "any creature" (every ordinary Aura).
    #[serde(default)]
    pub enchant: Option<PermanentFilter>,
    /// Animate Dead's cast-time "enchant creature card in a graveyard" (CR 303.4a) —
    /// `enchant_graveyard = true`; absent (`false`) for every ordinary card.
    #[serde(default)]
    pub enchant_graveyard: bool,
    #[serde(default)]
    /// Legendary supertype; Commander deck validation reads this when identifying legal
    /// commanders.
    pub legendary: bool,
    /// Snow supertype (CR 205.4g) — `snow = true`; absent (`false`) for every ordinary card.
    #[serde(default)]
    pub snow: bool,
    /// "This spell can't be countered" (CR 701.5g) — `uncounterable = true`; absent
    /// (`false`) for every ordinary card.
    #[serde(default)]
    pub uncounterable: bool,
    #[serde(default)]
    /// A modal spell or modal triggered ability ("Choose N"). For modal spells, each
    /// `timing = "spell"` ability is one mode.
    pub modal: bool,
    #[serde(rename = "choose", default = "one_u8")]
    pub modal_choose: u8,
    /// CR 700.2d "choose one or more" — the max of the range; `None` keeps the count
    /// fixed at exactly `modal_choose` (every "choose one"/"choose two" card).
    #[serde(rename = "choose_max", default)]
    pub modal_choose_max: Option<u8>,
    /// Gates `modal_choose_max` on the caster controlling a commander at cast time
    /// (Nexus Mentality's "if you control a commander as you cast this spell, you may
    /// choose both instead"). `false` for every ordinary modal card.
    #[serde(rename = "choose_max_if_commander", default)]
    pub modal_choose_max_if_commander: bool,
    #[serde(default)]
    pub keywords: Vec<Keyword>,
    /// A keyword granted only while a `Condition` holds (Primordial Hydra's trample at
    /// ten-or-more +1/+1 counters) — `conditional_keywords = [{ condition = { type =
    /// "..." }, keyword = "..." }]` in TOML. Empty for every ordinary card.
    #[serde(default)]
    pub conditional_keywords: Vec<ConditionalKeywordToml>,
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Vec<AbilityToml>"))]
    /// Authored rules text as ability blocks. Each block has a `timing` and one or more
    /// `effects`; multiple effects fold into `Effect::Sequence` in order.
    pub abilities: Vec<Ability>,
    #[serde(default)]
    /// Extra color-identity pips (CR 903.4) that the simplified model would otherwise
    /// drop, such as pips in trimmed activated abilities. Deck-building only.
    pub identity: Vec<Color>,
    /// Explicit colors (CR 105.2a) overriding the cost-pip derivation — a token's stated
    /// color, since it has no mana cost to derive one from. `colors = ["green"]` in
    /// TOML; empty (every ordinary card) derives color from cost pips as usual.
    #[serde(default)]
    pub colors: Vec<Color>,
    /// Devoid (CR 702.114a) — `devoid = true`; absent (`false`) for every ordinary card.
    #[serde(default)]
    pub devoid: bool,
    #[serde(default)]
    /// Unconditional enters-tapped replacement (CR 614.13), usually for lands.
    pub enters_tapped: bool,
    #[serde(default)]
    pub enters_tapped_unless: Option<Condition>,
    /// A CR 614.12 as-enters replacement choice (Overgrown Tomb) —
    /// `enters_tapped_unless_you_pay_life = 2`; absent for a card without one.
    #[serde(default)]
    pub enters_tapped_unless_you_pay_life: Option<u8>,
    /// A printed conditional free-cast permission (CR 118.5) — `free_cast_if = { .. }`
    /// with the same `Condition` table shape as `enters_tapped_unless`; absent for a
    /// card without one.
    #[serde(default)]
    pub free_cast_if: Option<Condition>,
    /// A printed non-mana alternative cost (CR 601.2f) — `alternative_cost = { condition =
    /// { .. }, rider = { .. } }`; absent for a card without one.
    #[serde(default)]
    pub alternative_cost: Option<AlternativeCost>,
    /// "Cast this spell only during combat" (CR 601.3e) — `cast_only_during_combat = true`;
    /// absent (`false`) for every ordinary card.
    #[serde(default)]
    pub cast_only_during_combat: bool,
    /// "Cast this spell only before attackers are declared" (CR 601.3e — Master Warcraft)
    /// — `cast_only_before_attackers = true`; absent (`false`) for every ordinary card.
    #[serde(default)]
    pub cast_only_before_attackers: bool,
    /// "Cast this spell only during combat before blockers are declared" (CR 601.3e — Blaze
    /// of Glory) — the declare-blockers half of `cast_only_before_attackers`, open until the
    /// first defending player declares. Pair it with `cast_only_during_combat`, which is the
    /// other half of Blaze of Glory's printed sentence; alone it leaves the pre-combat main
    /// phase open. Absent (`false`) for every ordinary card.
    #[serde(default)]
    pub cast_only_before_blockers: bool,
    /// "Cast this spell only during an opponent's turn" (CR 601.3e — Siren's Call) —
    /// `cast_only_during_opponents_turn = true`; the cast-side twin of an activated ability's
    /// `only_during_opponents_turn`, composable with `cast_only_before_attackers` (together
    /// they are Siren's Call's printed restriction). Absent (`false`) for every ordinary card.
    #[serde(default)]
    pub cast_only_during_opponents_turn: bool,
    /// "Cast this spell only before the combat damage step" (CR 601.3e — Berserk) —
    /// `cast_only_before_combat_damage = true`: legal from untap through declare blockers, and
    /// closed for the rest of the turn from the first combat damage step on. Absent (`false`)
    /// for every ordinary card.
    #[serde(default)]
    pub cast_only_before_combat_damage: bool,
    /// "Cast this spell only during the declare blockers step" (CR 601.3e — False Orders) —
    /// `cast_only_during_declare_blockers = true`: a single step rather than everything up to
    /// one, open before *and* after the declaration (False Orders rearranges a declaration
    /// that already happened). Absent (`false`) for every ordinary card.
    #[serde(default)]
    pub cast_only_during_declare_blockers: bool,
    /// "Cast this spell only during your declare attackers step" (CR 601.3e — Camouflage) —
    /// `cast_only_during_declare_attackers = true`: the attack-side twin of the window above,
    /// and narrower still, since it is closed on every other player's turn as well as in every
    /// other step. Absent (`false`) for every ordinary card.
    #[serde(default)]
    pub cast_only_during_declare_attackers: bool,
    #[serde(default)]
    /// Machine-readable fidelity note for modeled divergences. Set this whenever a
    /// `# ponytail:` comment marks a deliberate simplification; leave absent for faithful
    /// cards.
    pub approximates: Option<String>,
    /// Verbatim Scryfall Oracle text for catalog hover/read-the-card display. The engine
    /// never parses this; behavior comes from abilities, keywords, and other DSL fields.
    #[serde(default)]
    pub oracle: Option<String>,
    /// Every Scryfall set code with a printing of this oracle, used by coverage and
    /// catalog search. Pure metadata; gameplay never reads it.
    #[serde(default)]
    pub sets: Vec<String>,
    /// Printed non-land subtypes, such as creature, artifact, and enchantment subtypes.
    /// Land types live under `[kind].subtypes`.
    #[serde(default)]
    pub subtypes: Vec<String>,
    /// Scryfall Tagger oracle-tag slugs for thematic catalog search. Pure metadata.
    #[serde(default)]
    pub otags: Vec<String>,
    /// Cycling {N} (CR 702.29a) — `cycling = { generic = N }`; absent for a card with none.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub cycling: Option<Cost>,
    /// A sacrifice folded into the cycling cost (CR 702.29b — Edge of Autumn's
    /// "Cycling—Sacrifice a land"), same [`SacrificeCost`] table/shorthand shape as an
    /// activation sacrifice cost. Absent (`SacrificeCost::None`) for ordinary cycling.
    #[serde(default)]
    pub cycling_sacrifice: SacrificeCost,
    /// Flashback (CR 702.34) — `[flashback]` with the same `[cost]`-table shape (may carry
    /// a `[flashback.additional]` rider); absent for a card without flashback.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub flashback: Option<Cost>,
    /// Echo (CR 702.31) — `[echo]` with the same `[cost]`-table shape; absent for a card
    /// without echo.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub echo: Option<Cost>,
    /// Cumulative upkeep (CR 702.24) — `[cumulative_upkeep]` (`graveyard_cards = N`);
    /// absent for a card without cumulative upkeep.
    #[serde(default)]
    pub cumulative_upkeep: Option<CumulativeUpkeepCost>,
    /// Recover (CR 702.59) — `[recover]` with the same `[cost]`-table shape as `[echo]`;
    /// absent for a card without recover.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub recover: Option<Cost>,
    /// Bestow (CR 702.103) — `[bestow]` with the same `[cost]`-table shape as `[echo]`;
    /// absent for a card without bestow.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub bestow: Option<Cost>,
    /// Morph (CR 702.37) — `[morph]` with the same `[cost]`-table shape as `[bestow]` (the
    /// card's morph cost); absent for a card without morph.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub morph: Option<Cost>,
    /// Evoke (CR 702.74) — `[evoke]` with the same `[cost]`-table shape as `[echo]`;
    /// absent for a card without evoke.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub evoke: Option<Cost>,
    /// Delve (CR 702.66) — `delve = true`; absent (`false`) for a card without delve.
    #[serde(default)]
    pub delve: bool,
    /// Escape (CR 702.19) — `[escape]` (an `[escape.cost]` sub-table plus `exile`/
    /// `plus_one_plus_one_counters`); absent for a card without escape.
    #[serde(default)]
    pub escape: Option<EscapeCost>,
    /// Retrace (CR 702.83) — `retrace = true`; absent (`false`) for a card without
    /// retrace.
    #[serde(default)]
    pub retrace: bool,
    /// Cast-from-graveyard alternative cost for a permanent (CR 118.9) —
    /// `[graveyard_cast_cost]` with the same `[cost]`-table shape as `[flashback]`; absent
    /// for a card without it.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub graveyard_cast_cost: Option<Cost>,
    /// Cascade (CR 702.85) — `cascade = true`; absent (`false`) for a card without
    /// cascade.
    #[serde(default)]
    pub cascade: bool,
    /// Demonstrate (CR 702.147) — `demonstrate = true`; absent (`false`) for a card
    /// without demonstrate.
    #[serde(default)]
    pub demonstrate: bool,
    /// Devour N (CR 702.82) — `devour = N`; absent for a card without devour.
    #[serde(default)]
    pub devour: Option<u32>,
    /// CR 603.6e — this card's triggered abilities fire from its owner's graveyard rather
    /// than the battlefield (Squee, Nether Traitor). `false` for every ordinary card.
    #[serde(default)]
    pub functions_in_graveyard: bool,
    /// A "prepare" DFC's back face (soc/sos) — an inline `[back]` `CardDef` table, parsed
    /// via `CardDef`'s own impl and interned below. Absent for ordinary cards.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CardToml>"))]
    pub back: Option<CardDef>,
    /// An adventure card's adventure half (CR 715, soc/sos) — an inline `[adventure]`
    /// `CardDef` table (its own `cost`, `kind`, `abilities`), parsed like `back` and
    /// interned below. Absent for ordinary cards.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CardToml>"))]
    pub adventure: Option<CardDef>,
    /// A split card's two castable halves (CR 709, Fire // Ice) — `[[half]]` tables, each
    /// its own inline `CardDef` (name, oracle, `cost`, `kind`, `abilities`) parsed like
    /// `adventure`. Empty for every non-split card.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Vec<CardToml>"))]
    pub half: Vec<CardDef>,
    /// Suspend N—[cost] (CR 702.62, Rousing Refrain) — a `[suspend]` table whose `cost`
    /// sub-table is leaked to `'static` by the `Suspend` impl. Absent for ordinary cards.
    #[serde(default)]
    pub suspend: Option<Suspend>,
    /// Enter-as-a-copy replacement (CR 706/707.2) — an inline `enter_as_copy = { .. }`
    /// table (`until_eot`/`extra_counters`/`gains_haste`, all optional). Absent for a card
    /// without it.
    #[serde(default)]
    pub enter_as_copy: Option<EnterAsCopy>,
    /// Encore [cost] (CR 702.140, Angel of Indemnity) — an `[encore]` table with the same
    /// `[cost]`-table shape as `[flashback]`, leaked to `'static` below. Absent for a card
    /// without encore.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<CostToml>"))]
    pub encore: Option<Cost>,
    /// A hand-activated, discard-this-card ability (CR 113.6/602.5e, Magma Opus) — zero or
    /// more `[[hand_ability]]` tables (`[hand_ability.cost]` + `[[hand_ability.effects]]`
    /// each), one per typecycling type (CR 702.29d — Valley Rannet's mountaincycling and
    /// forestcycling). Empty for a card without one.
    #[serde(default)]
    pub hand_ability: Vec<HandActivatedAbility>,
    /// Forecast (CR 702.57, Skyscribing) — a `[forecast]` table (`[forecast.cost]` +
    /// `[[forecast.effects]]`), the reveal-and-keep sibling of `hand_ability`. Absent for
    /// a card without one.
    #[serde(default)]
    pub forecast: Option<HandActivatedAbility>,
    /// "You may choose not to untap this during your untap step" (CR 502.2 — Rubinia
    /// Soulsinger) — `may_choose_not_to_untap = true`; absent (`false`) for every ordinary
    /// permanent.
    #[serde(default)]
    pub may_choose_not_to_untap: bool,
    /// Dredge N (CR 702.52) — `dredge = N` for a dredger; absent (`None`) otherwise.
    #[serde(default)]
    pub dredge: Option<u8>,
    /// Vanishing N (CR 702.63) — `vanishing = N` for a vanishing permanent; absent
    /// (`None`) for every other card.
    #[serde(default)]
    pub vanishing: Option<u8>,
    /// A non-mana cast-time cap on {X} (CR 601.2b — Open the Way's player-count bound) —
    /// `cast_x_max = "player_count"`; absent (`None`) for every ordinary {X} spell.
    #[serde(default)]
    #[cfg_attr(feature = "card-schema", schemars(with = "Option<String>"))]
    pub cast_x_max: Option<CastXMax>,
}

impl From<CardToml> for CardDef {
    fn from(card: CardToml) -> Self {
        CardDef {
            id: Box::leak(card.id.into_boxed_str()),
            default_print: Box::leak(card.default_print.into_boxed_str()),
            name: Box::leak(card.name.into_boxed_str()),
            cost: card.cost.into(),
            kind: card.kind.into(),
            enchant: card.enchant,
            enchant_graveyard: card.enchant_graveyard,
            legendary: card.legendary,
            snow: card.snow,
            uncounterable: card.uncounterable,
            modal: card.modal,
            modal_choose: card.modal_choose,
            modal_choose_max: card.modal_choose_max,
            modal_choose_max_if_commander: card.modal_choose_max_if_commander,
            keywords: Arc::from(card.keywords),
            conditional_keywords: Arc::from(
                card.conditional_keywords
                    .into_iter()
                    .map(|raw| (raw.condition, raw.keyword))
                    .collect::<Vec<_>>(),
            ),
            abilities: Arc::from(card.abilities),
            identity_pips: Arc::from(card.identity),
            colors: Arc::from(card.colors),
            devoid: card.devoid,
            enters_tapped: card.enters_tapped,
            enters_tapped_unless: card.enters_tapped_unless,
            enters_tapped_unless_you_pay_life: card.enters_tapped_unless_you_pay_life,
            free_cast_if: card.free_cast_if,
            alternative_cost: card.alternative_cost,
            cast_only_during_combat: card.cast_only_during_combat,
            cast_only_before_attackers: card.cast_only_before_attackers,
            cast_only_before_blockers: card.cast_only_before_blockers,
            cast_only_during_opponents_turn: card.cast_only_during_opponents_turn,
            cast_only_before_combat_damage: card.cast_only_before_combat_damage,
            cast_only_during_declare_blockers: card.cast_only_during_declare_blockers,
            cast_only_during_declare_attackers: card.cast_only_during_declare_attackers,
            approximates: card.approximates.map(|s| &*Box::leak(s.into_boxed_str())),
            oracle: card.oracle.map(|s| &*Box::leak(s.into_boxed_str())),
            sets: arc_strs(card.sets),
            subtypes: arc_strs(card.subtypes),
            otags: arc_strs(card.otags),
            cycling: card.cycling,
            cycling_sacrifice: card.cycling_sacrifice,
            flashback: card.flashback,
            echo: card.echo,
            cumulative_upkeep: card.cumulative_upkeep,
            recover: card.recover,
            bestow: card.bestow,
            morph: card.morph,
            evoke: card.evoke,
            delve: card.delve,
            escape: card.escape,
            retrace: card.retrace,
            graveyard_cast_cost: card.graveyard_cast_cost,
            cascade: card.cascade,
            demonstrate: card.demonstrate,
            devour: card.devour,
            functions_in_graveyard: card.functions_in_graveyard,
            // Intern nested faces once at load so later lookups can reuse stable CardIds.
            back: card.back.map(intern_card_def),
            adventure: card.adventure.map(intern_card_def),
            suspend: card.suspend,
            enter_as_copy: card.enter_as_copy,
            // Leak the encore cost once at load so the `CardDef` can keep sharing the same
            // nested `Cost` handle as today.
            encore: card.encore.map(|cost| &*Box::leak(Box::new(cost))),
            hand_ability: Arc::from(card.hand_ability),
            forecast: card.forecast,
            may_choose_not_to_untap: card.may_choose_not_to_untap,
            dredge: card.dredge,
            vanishing: card.vanishing,
            cast_x_max: card.cast_x_max,
            halves: Arc::from(
                card.half
                    .into_iter()
                    .map(intern_card_def)
                    .collect::<Vec<_>>(),
            ),
        }
    }
}
