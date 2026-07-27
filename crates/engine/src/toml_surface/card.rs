use std::sync::Arc;

use serde::Deserialize;

use crate::de::{arc_strs, one_u8};
use crate::{
    Ability, AlternativeCost, CardDef, CardKind, CastXMax, Color, Condition, Cost,
    CumulativeUpkeepCost, EnterAsCopy, EscapeCost, HandActivatedAbility, Keyword, PermanentFilter,
    SacrificeCost, Suspend, intern_card_def,
};

/// One entry of `CardDef::conditional_keywords` as spelled in TOML — an
/// `{ condition, keyword }` table folded into a `(Condition, Keyword)` tuple at load.
#[derive(Clone, Debug, Deserialize)]
pub struct ConditionalKeywordToml {
    pub condition: Condition,
    pub keyword: Keyword,
}

/// A card's top-level TOML table.
///
/// This mirrors the DSL's top-level card shape with owned fields, then folds into the interned
/// `CardDef` representation used by the engine.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardToml {
    /// Scryfall oracle id — required on top-level pool TOMLs (enforced at registry load).
    /// Nested faces/tokens may omit it (`""`).
    #[serde(default)]
    pub id: String,
    /// Scryfall card UUID for the default Printing — required on top-level pool TOMLs.
    #[serde(default)]
    pub default_print: String,
    pub name: String,
    #[serde(default)]
    pub cost: Cost,
    pub kind: CardKind,
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
    pub legendary: bool,
    /// Snow supertype (CR 205.4g) — `snow = true`; absent (`false`) for every ordinary card.
    #[serde(default)]
    pub snow: bool,
    /// "This spell can't be countered" (CR 701.5g) — `uncounterable = true`; absent
    /// (`false`) for every ordinary card.
    #[serde(default)]
    pub uncounterable: bool,
    #[serde(default)]
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
    pub abilities: Vec<Ability>,
    #[serde(default)]
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
    #[serde(default)]
    pub approximates: Option<String>,
    #[serde(default)]
    pub oracle: Option<String>,
    #[serde(default)]
    pub sets: Vec<String>,
    #[serde(default)]
    pub subtypes: Vec<String>,
    #[serde(default)]
    pub otags: Vec<String>,
    /// Cycling {N} (CR 702.29a) — `cycling = { generic = N }`; absent for a card with none.
    #[serde(default)]
    pub cycling: Option<Cost>,
    /// A sacrifice folded into the cycling cost (CR 702.29b — Edge of Autumn's
    /// "Cycling—Sacrifice a land"), same [`SacrificeCost`] table/shorthand shape as an
    /// activation sacrifice cost. Absent (`SacrificeCost::None`) for ordinary cycling.
    #[serde(default)]
    pub cycling_sacrifice: SacrificeCost,
    /// Flashback (CR 702.34) — `[flashback]` with the same `[cost]`-table shape (may carry
    /// a `[flashback.additional]` rider); absent for a card without flashback.
    #[serde(default)]
    pub flashback: Option<Cost>,
    /// Echo (CR 702.31) — `[echo]` with the same `[cost]`-table shape; absent for a card
    /// without echo.
    #[serde(default)]
    pub echo: Option<Cost>,
    /// Cumulative upkeep (CR 702.24) — `[cumulative_upkeep]` (`graveyard_cards = N`);
    /// absent for a card without cumulative upkeep.
    #[serde(default)]
    pub cumulative_upkeep: Option<CumulativeUpkeepCost>,
    /// Recover (CR 702.59) — `[recover]` with the same `[cost]`-table shape as `[echo]`;
    /// absent for a card without recover.
    #[serde(default)]
    pub recover: Option<Cost>,
    /// Bestow (CR 702.103) — `[bestow]` with the same `[cost]`-table shape as `[echo]`;
    /// absent for a card without bestow.
    #[serde(default)]
    pub bestow: Option<Cost>,
    /// Morph (CR 702.37) — `[morph]` with the same `[cost]`-table shape as `[bestow]` (the
    /// card's morph cost); absent for a card without morph.
    #[serde(default)]
    pub morph: Option<Cost>,
    /// Evoke (CR 702.74) — `[evoke]` with the same `[cost]`-table shape as `[echo]`;
    /// absent for a card without evoke.
    #[serde(default)]
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
    pub back: Option<CardDef>,
    /// An adventure card's adventure half (CR 715, soc/sos) — an inline `[adventure]`
    /// `CardDef` table (its own `cost`, `kind`, `abilities`), parsed like `back` and
    /// interned below. Absent for ordinary cards.
    #[serde(default)]
    pub adventure: Option<CardDef>,
    /// A split card's two castable halves (CR 709, Fire // Ice) — `[[half]]` tables, each
    /// its own inline `CardDef` (name, oracle, `cost`, `kind`, `abilities`) parsed like
    /// `adventure`. Empty for every non-split card.
    #[serde(default)]
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
    pub cast_x_max: Option<CastXMax>,
}

impl From<CardToml> for CardDef {
    fn from(card: CardToml) -> Self {
        CardDef {
            id: Box::leak(card.id.into_boxed_str()),
            default_print: Box::leak(card.default_print.into_boxed_str()),
            name: Box::leak(card.name.into_boxed_str()),
            cost: card.cost,
            kind: card.kind,
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
