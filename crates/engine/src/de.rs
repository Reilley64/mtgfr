//! Deserialization of card definitions from the TOML card DSL (the `card-dsl` feature).
//!
//! Most types deserialize via derives on their definitions in `lib.rs`; this module holds
//! the handful whose TOML spelling differs structurally from their Rust shape (a flat
//! `[cost]` table of color names, the `instant`/`sorcery` split of [`CardKind::Spell`],
//! the flat ability table that folds into [`Timing::Activated`]), plus the interning
//! helpers that turn owned TOML data into the `&'static` slices that keep [`CardDef`]
//! `Copy` — a bounded, load-once pool that lives for the program's lifetime anyway.
//! See [`Effect`]'s doc comment for the invariant these helpers exist to satisfy.
//!
//! CR citations appear on individual fields where the DSL encodes a rules concept
//! (e.g. commander identity mana, target counts); see `docs/CR_INDEX.md`.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;
use serde::de::{self, Deserializer, IntoDeserializer, Visitor};

use crate::{
    Ability, ActivationCost, AdditionalCost, Amount, AmountZone, CardDef, CardFilter, CardKind,
    CasterScope, Color, ColorFilter, CombatDamageScope, Condition, Cost, CounterKind,
    CumulativeUpkeepCost, EdictScope, Effect, EnterAsCopy, EnterController, EscapeCost,
    FilterController, GrantedAbility, HandActivatedAbility, Keyword, LandProduces, Mana, ManaPool,
    Parity, PermanentFilter, ProtectionScope, ReanimateBecomes, SacrificeAdditionalCost,
    SacrificeAdditionalCostCount, SacrificeCost, SpellFilter, SpellSpeed, SpendToCastPredicate,
    Suspend, TargetCount, Timing, TokenFilter, Trigger, TypeSet,
};

/// Token profiles loaded from `cards/data/tokens/` before deckable cards deserialize. Keyed by
/// Scryfall oracle id; [`token_profile`] resolves `token = "<id>"` against this map.
static TOKEN_DEFS: OnceLock<HashMap<&'static str, CardDef>> = OnceLock::new();

/// Install the token-profile registry used by [`token_profile`]. Call once from the `cards` crate
/// after loading `data/tokens/*.toml` and before parsing deckable card TOMLs. Panics if called twice.
pub fn install_token_defs(defs: HashMap<&'static str, CardDef>) {
    TOKEN_DEFS
        .set(defs)
        .unwrap_or_else(|_| panic!("install_token_defs called more than once"));
}

/// Look up a token profile by Scryfall oracle id after [`install_token_defs`].
pub fn token_def(id: &str) -> Option<CardDef> {
    TOKEN_DEFS.get().and_then(|m| m.get(id).copied())
}

// ── Interning + serde defaults (referenced by the derives in lib.rs) ────────────────

/// Leak an owned `Vec<T>` into the `&'static [T]` a `Copy` [`CardDef`]/[`Effect`] field needs.
/// The one place that actually calls `Box::leak` on a plain vec-to-slice; every other site in
/// this module (and [`static_slice`] below) should go through this rather than leaking directly.
pub(crate) fn intern<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

/// One entry of `CardDef::conditional_keywords` as spelled in TOML — an `{ condition, keyword }`
/// table, folded into a `(Condition, Keyword)` tuple at load (see [`intern`]).
#[derive(Deserialize)]
struct ConditionalKeywordRaw {
    condition: Condition,
    keyword: Keyword,
}

pub(crate) fn static_slice<'de, D, T>(d: D) -> Result<&'static [T], D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + 'static,
{
    Ok(intern(Vec::<T>::deserialize(d)?))
}

/// Leak one owned `Effect` into the `&'static Effect` a nested `Copy` field needs (a single-value
/// sibling of [`static_slice`] — `Effect` can't hold itself by value, so
/// [`Effect::ScheduleAtNextUpkeep`]'s `then` is the one-element leaked case instead).
pub(crate) fn static_effect<'de, D>(d: D) -> Result<&'static Effect, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(&*Box::leak(Box::new(Effect::deserialize(d)?)))
}

/// Leak one owned [`Cost`] into the `&'static Cost` a `Copy` field needs (the `Cost` sibling of
/// [`static_effect`] — [`Suspend::cost`] can't hold a `Cost` by value without bloating a `Copy`
/// [`CardDef`], since `Cost` embeds an [`AdditionalCost`]).
pub(crate) fn leaked_cost<'de, D>(d: D) -> Result<&'static Cost, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(&*Box::leak(Box::new(Cost::deserialize(d)?)))
}

/// `deserialize_with` for [`Effect::GrantToAttached`]'s `granted_ability`: leak the one owned
/// [`GrantedAbility`] the sub-table spells into the `&'static` a `Copy` [`Effect`] needs. Only
/// called when the key is present (a `#[serde(default)]` absent key stays `None`), so it always
/// yields `Some`.
pub(crate) fn opt_static_granted_ability<'de, D>(
    d: D,
) -> Result<Option<&'static GrantedAbility>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(&*Box::leak(Box::new(GrantedAbility::deserialize(d)?))))
}

/// `deserialize_with` for [`Effect::ReanimateToBattlefield`]'s `becomes`: leak the one owned
/// [`ReanimateBecomes`] the sub-table spells into the `&'static` a `Copy` [`Effect`] needs. Only
/// called when the key is present (an absent `#[serde(default)]` key stays `None`).
pub(crate) fn opt_static_reanimate_becomes<'de, D>(
    d: D,
) -> Result<Option<&'static ReanimateBecomes>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(&*Box::leak(Box::new(ReanimateBecomes::deserialize(
        d,
    )?))))
}

/// Intern a list of owned strings (subtypes, type-filter names) into a `&'static [&'static
/// str]`. Unlike [`static_slice`], `&str` can't derive `Deserialize<'static>` directly (same
/// borrow-vs-`'static` problem as `CardDef::name` — see the module doc), so this leaks each
/// string too rather than delegating to it.
pub(crate) fn intern_strs(strings: Vec<String>) -> &'static [&'static str] {
    let leaked: Vec<&'static str> = strings
        .into_iter()
        .map(|s| &*Box::leak(s.into_boxed_str()))
        .collect();
    intern(leaked)
}

/// `deserialize_with` for a `&'static [&'static str]` field (land subtypes, and the card-filter /
/// [`Condition`] arms that filter or gate on them) — TOML spells it as a plain array of strings.
pub(crate) fn static_str_slice<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<&'static [&'static str], D::Error> {
    Ok(intern_strs(Vec::<String>::deserialize(d)?))
}

/// serde default for a `CounterReplacement`'s `times` (the multiplicative identity).
pub(crate) fn one() -> i32 {
    1
}

/// serde default for `modal_choose`: a modal spell chooses one mode unless it says more.
pub(crate) fn one_u8() -> u8 {
    1
}

/// serde default for [`Effect::LookAtTop`]'s `up_to`: the printed "put *that card*" ⇒ one.
pub(crate) fn one_u32() -> u32 {
    1
}

/// serde default for [`Effect::LookAtTop`]'s `filter`: a filterless look sees any card.
pub(crate) fn any_card_filter() -> CardFilter {
    CardFilter::AnyCard
}

/// serde default for an edict's `scope`: "each player" is the common wording.
pub(crate) fn all_players() -> EdictScope {
    EdictScope::AllPlayers
}

/// serde default for an edict's `filter`: a creature is the common sacrifice.
pub(crate) fn creature_edict() -> PermanentFilter {
    PermanentFilter::of(TypeSet::CREATURE)
}

/// A token profile reference on `create_token` (and siblings): a Scryfall oracle id string
/// (`token = "37c4adc8-…"`) resolved against the registry installed by [`install_token_defs`].
/// Token characteristics live in `cards/data/tokens/*.toml`; after resolve the effect embeds a
/// full [`CardDef`] so mint paths stay pool-agnostic.
pub(crate) fn token_profile<'de, D: Deserializer<'de>>(d: D) -> Result<CardDef, D::Error> {
    let id = String::deserialize(d)?;
    if id.is_empty() {
        return Err(de::Error::custom(
            "token profile id is empty — expected a Scryfall oracle id from data/tokens/",
        ));
    }
    token_def(&id).ok_or_else(|| {
        de::Error::custom(format!(
            "unknown token profile id {id:?} — add data/tokens/<name>.toml and ensure \
             install_token_defs ran before loading deckable cards"
        ))
    })
}

/// An `add_mana` effect spells its batch as one symbol per mana produced
/// (`mana = ["colorless", "colorless"]` for Sol Ring), not as pool component counts.
/// A `deserialize_with` on the [`Effect::AddMana`] `mana` field rather than a `Deserialize`
/// on [`ManaPool`] itself — the pool is runtime game state (events, replays), and its
/// canonical serde shape shouldn't be a card-DSL spelling.
pub(crate) fn mana_batch<'de, D: Deserializer<'de>>(d: D) -> Result<ManaPool, D::Error> {
    let mut pool = ManaPool::default();
    for symbol in Vec::<Mana>::deserialize(d)? {
        pool.add(symbol, 1);
    }
    Ok(pool)
}

/// The default `repeat`/`count` for an amount-bearing field that omits one — a single copy.
pub(crate) fn one_amount() -> Amount {
    Amount::Fixed(1)
}

/// The default for an amount-bearing field that omits one and means "none" rather than "one" —
/// `create_token`'s `enters_with` (no counters unless a card says otherwise).
pub(crate) fn zero_amount() -> Amount {
    Amount::Fixed(0)
}

/// The default `spend_predicate` for an ability that isn't a `spend_mana_to_cast` trigger (the
/// field is unread there) — an arbitrary variant so the derive has a default.
pub(crate) fn default_spend_predicate() -> SpendToCastPredicate {
    SpendToCastPredicate::Commander
}

// ── Types whose TOML spelling differs structurally from their Rust shape ────────────

/// A card's top-level TOML table. Manual rather than derived because of the
/// `name: &'static str` field: serde infers a `&str` field as borrowed from the input
/// (pinning the impl to `Deserialize<'static>`, which `toml::from_str` can't use), when
/// it's really an owned string interned at load.
impl<'de> Deserialize<'de> for CardDef {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Card {
            /// Scryfall oracle id — required on top-level pool TOMLs (enforced at registry load).
            /// Nested faces/tokens may omit it (`""`).
            #[serde(default)]
            id: String,
            /// Scryfall card UUID for the default Printing — required on top-level pool TOMLs.
            #[serde(default)]
            default_print: String,
            name: String,
            #[serde(default)]
            cost: Cost,
            kind: CardKind,
            /// An Aura's enchant subject restriction (CR 303.4a) — `enchant = { … }`, the same
            /// [`PermanentFilter`] table/shorthand shape as any other filter field; absent means
            /// "any creature" (every ordinary Aura).
            #[serde(default)]
            enchant: Option<PermanentFilter>,
            /// Animate Dead's cast-time "enchant creature card in a graveyard" (CR 303.4a) —
            /// `enchant_graveyard = true`; absent (`false`) for every ordinary card.
            #[serde(default)]
            enchant_graveyard: bool,
            #[serde(default)]
            legendary: bool,
            /// "This spell can't be countered" (CR 701.5g) — `uncounterable = true`; absent
            /// (`false`) for every ordinary card.
            #[serde(default)]
            uncounterable: bool,
            #[serde(default)]
            modal: bool,
            #[serde(rename = "choose", default = "one_u8")]
            modal_choose: u8,
            /// CR 700.2d "choose one or more" — the max of the range; `None` keeps the count
            /// fixed at exactly `modal_choose` (every "choose one"/"choose two" card).
            #[serde(rename = "choose_max", default)]
            modal_choose_max: Option<u8>,
            /// Gates `modal_choose_max` on the caster controlling a commander at cast time
            /// (Nexus Mentality's "if you control a commander as you cast this spell, you may
            /// choose both instead"). `false` for every ordinary modal card.
            #[serde(rename = "choose_max_if_commander", default)]
            modal_choose_max_if_commander: bool,
            #[serde(default)]
            keywords: Vec<Keyword>,
            /// A keyword granted only while a `Condition` holds (Primordial Hydra's trample at
            /// ten-or-more +1/+1 counters) — `conditional_keywords = [{ condition = { type =
            /// "..." }, keyword = "..." }]` in TOML. Empty for every ordinary card.
            #[serde(default)]
            conditional_keywords: Vec<ConditionalKeywordRaw>,
            #[serde(default)]
            abilities: Vec<Ability>,
            #[serde(default)]
            identity: Vec<Color>,
            /// Explicit colors (CR 105.2a) overriding the cost-pip derivation — a token's stated
            /// color, since it has no mana cost to derive one from. `colors = ["green"]` in
            /// TOML; empty (every ordinary card) derives color from cost pips as usual.
            #[serde(default)]
            colors: Vec<Color>,
            /// Devoid (CR 702.114a) — `devoid = true`; absent (`false`) for every ordinary card.
            #[serde(default)]
            devoid: bool,
            #[serde(default)]
            enters_tapped: bool,
            #[serde(default)]
            enters_tapped_unless: Option<Condition>,
            /// A printed conditional free-cast permission (CR 118.5) — `free_cast_if = { .. }`
            /// with the same `Condition` table shape as `enters_tapped_unless`; absent for a
            /// card without one.
            #[serde(default)]
            free_cast_if: Option<Condition>,
            /// "Cast this spell only during combat" (CR 601.3e) — `cast_only_during_combat = true`;
            /// absent (`false`) for every ordinary card.
            #[serde(default)]
            cast_only_during_combat: bool,
            #[serde(default)]
            approximates: Option<String>,
            #[serde(default)]
            oracle: Option<String>,
            #[serde(default)]
            set: String,
            #[serde(default)]
            subtypes: Vec<String>,
            #[serde(default)]
            otags: Vec<String>,
            /// Cycling {N} (CR 702.29a) — `cycling = { generic = N }`; absent for a card with none.
            #[serde(default)]
            cycling: Option<Cost>,
            /// A sacrifice folded into the cycling cost (CR 702.29b — Edge of Autumn's
            /// "Cycling—Sacrifice a land"), same [`SacrificeCost`] table/shorthand shape as an
            /// [`ActivationCost::sacrifice`]. Absent (`SacrificeCost::None`) for ordinary cycling.
            #[serde(default)]
            cycling_sacrifice: SacrificeCost,
            /// Flashback (CR 702.34) — `[flashback]` with the same `[cost]`-table shape (may carry
            /// a `[flashback.additional]` rider); absent for a card without flashback.
            #[serde(default)]
            flashback: Option<Cost>,
            /// Echo (CR 702.31) — `[echo]` with the same `[cost]`-table shape; absent for a card
            /// without echo.
            #[serde(default)]
            echo: Option<Cost>,
            /// Cumulative upkeep (CR 702.24) — `[cumulative_upkeep]` (`graveyard_cards = N`);
            /// absent for a card without cumulative upkeep.
            #[serde(default)]
            cumulative_upkeep: Option<CumulativeUpkeepCost>,
            /// Recover (CR 702.59) — `[recover]` with the same `[cost]`-table shape as `[echo]`;
            /// absent for a card without recover.
            #[serde(default)]
            recover: Option<Cost>,
            /// Bestow (CR 702.103) — `[bestow]` with the same `[cost]`-table shape as `[echo]`;
            /// absent for a card without bestow.
            #[serde(default)]
            bestow: Option<Cost>,
            /// Morph (CR 702.37) — `[morph]` with the same `[cost]`-table shape as `[bestow]` (the
            /// card's morph cost); absent for a card without morph.
            #[serde(default)]
            morph: Option<Cost>,
            /// Evoke (CR 702.74) — `[evoke]` with the same `[cost]`-table shape as `[echo]`;
            /// absent for a card without evoke.
            #[serde(default)]
            evoke: Option<Cost>,
            /// Delve (CR 702.66) — `delve = true`; absent (`false`) for a card without delve.
            #[serde(default)]
            delve: bool,
            /// Escape (CR 702.19) — `[escape]` (an `[escape.cost]` sub-table plus `exile`/
            /// `plus_one_plus_one_counters`); absent for a card without escape.
            #[serde(default)]
            escape: Option<EscapeCost>,
            /// Retrace (CR 702.83) — `retrace = true`; absent (`false`) for a card without
            /// retrace.
            #[serde(default)]
            retrace: bool,
            /// Cast-from-graveyard alternative cost for a permanent (CR 118.9) — `[graveyard_cast_cost]`
            /// with the same `[cost]`-table shape as `[flashback]`; absent for a card without it.
            #[serde(default)]
            graveyard_cast_cost: Option<Cost>,
            /// Cascade (CR 702.85) — `cascade = true`; absent (`false`) for a card without
            /// cascade.
            #[serde(default)]
            cascade: bool,
            /// Demonstrate (CR 702.147) — `demonstrate = true`; absent (`false`) for a card
            /// without demonstrate.
            #[serde(default)]
            demonstrate: bool,
            /// Devour N (CR 702.82) — `devour = N`; absent for a card without devour.
            #[serde(default)]
            devour: Option<u32>,
            /// CR 603.6e — this card's triggered abilities fire from its owner's graveyard rather
            /// than the battlefield (Squee, Nether Traitor). `false` for every ordinary card.
            #[serde(default)]
            functions_in_graveyard: bool,
            /// A "prepare" DFC's back face (soc/sos) — an inline `[back]` `CardDef` table, parsed
            /// via `CardDef`'s own impl and leaked to `'static` below. Absent for ordinary cards.
            #[serde(default)]
            back: Option<CardDef>,
            /// An adventure card's adventure half (CR 715, soc/sos) — an inline `[adventure]`
            /// `CardDef` table (its own `cost`, `kind`, `abilities`), parsed like `back` and leaked
            /// to `'static` below. Absent for ordinary cards.
            #[serde(default)]
            adventure: Option<CardDef>,
            /// Suspend N—[cost] (CR 702.62, Rousing Refrain) — a `[suspend]` table whose `cost`
            /// sub-table is leaked to `'static` by the `Suspend` impl. Absent for ordinary cards.
            #[serde(default)]
            suspend: Option<Suspend>,
            /// Enter-as-a-copy replacement (CR 706/707.2) — an inline `enter_as_copy = { .. }`
            /// table (`until_eot`/`extra_counters`/`gains_haste`, all optional). Absent for a card
            /// without it.
            #[serde(default)]
            enter_as_copy: Option<EnterAsCopy>,
            /// Encore [cost] (CR 702.140, Angel of Indemnity) — an `[encore]` table with the same
            /// `[cost]`-table shape as `[flashback]`, leaked to `'static` below. Absent for a card
            /// without encore.
            #[serde(default)]
            encore: Option<Cost>,
            /// A hand-activated, discard-this-card ability (CR 113.6/602.5e, Magma Opus) — an
            /// `[hand_ability]` table (`[hand_ability.cost]` + `[[hand_ability.effects]]`).
            /// Absent for a card without one.
            #[serde(default)]
            hand_ability: Option<HandActivatedAbility>,
            /// Forecast (CR 702.57, Skyscribing) — a `[forecast]` table (`[forecast.cost]` +
            /// `[[forecast.effects]]`), the reveal-and-keep sibling of `hand_ability`. Absent for
            /// a card without one.
            #[serde(default)]
            forecast: Option<HandActivatedAbility>,
            /// "You may choose not to untap this during your untap step" (CR 502.2 — Rubinia
            /// Soulsinger) — `may_choose_not_to_untap = true`; absent (`false`) for every ordinary
            /// permanent.
            #[serde(default)]
            may_choose_not_to_untap: bool,
            /// Dredge N (CR 702.52) — `dredge = N` for a dredger; absent (`None`) otherwise.
            #[serde(default)]
            dredge: Option<u8>,
        }

        let card = Card::deserialize(d)?;
        Ok(CardDef {
            id: Box::leak(card.id.into_boxed_str()),
            default_print: Box::leak(card.default_print.into_boxed_str()),
            name: Box::leak(card.name.into_boxed_str()),
            cost: card.cost,
            kind: card.kind,
            enchant: card.enchant,
            enchant_graveyard: card.enchant_graveyard,
            legendary: card.legendary,
            uncounterable: card.uncounterable,
            modal: card.modal,
            modal_choose: card.modal_choose,
            modal_choose_max: card.modal_choose_max,
            modal_choose_max_if_commander: card.modal_choose_max_if_commander,
            keywords: intern(card.keywords),
            conditional_keywords: intern(
                card.conditional_keywords
                    .into_iter()
                    .map(|raw| (raw.condition, raw.keyword))
                    .collect(),
            ),
            abilities: intern(card.abilities),
            identity_pips: intern(card.identity),
            colors: intern(card.colors),
            devoid: card.devoid,
            enters_tapped: card.enters_tapped,
            enters_tapped_unless: card.enters_tapped_unless,
            free_cast_if: card.free_cast_if,
            cast_only_during_combat: card.cast_only_during_combat,
            approximates: card.approximates.map(|s| &*Box::leak(s.into_boxed_str())),
            oracle: card.oracle.map(|s| &*Box::leak(s.into_boxed_str())),
            set: Box::leak(card.set.into_boxed_str()),
            subtypes: intern_strs(card.subtypes),
            otags: intern_strs(card.otags),
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
            // Leak the back face to `'static` (like the rest of the interned card data) so a
            // `Copy` `&'static CardDef` reference can live on the front `CardDef`.
            back: card.back.map(|def| &*Box::leak(Box::new(def))),
            // Leak the adventure half to `'static`, like the back face above.
            adventure: card.adventure.map(|def| &*Box::leak(Box::new(def))),
            suspend: card.suspend,
            enter_as_copy: card.enter_as_copy,
            // Leak the encore cost to `'static` (like `suspend`'s cost) so a `Copy` `&'static Cost`
            // reference can live on the `CardDef`.
            encore: card.encore.map(|cost| &*Box::leak(Box::new(cost))),
            hand_ability: card.hand_ability,
            forecast: card.forecast,
            may_choose_not_to_untap: card.may_choose_not_to_untap,
            dredge: card.dredge,
        })
    }
}

/// `[cost]`'s `x` key: the common case `x = true` (a single `{X}`) or an integer count of
/// `{X}` symbols (`x = 3` for Astral Cornucopia's `{X}{X}{X}`, CR 107.3). `false`/absent means
/// no `{X}`. Untagged so TOML's own scalar type picks the arm.
#[derive(Deserialize)]
#[serde(untagged)]
enum XPips {
    Bool(bool),
    Count(u8),
}

impl Default for XPips {
    fn default() -> Self {
        XPips::Bool(false)
    }
}

impl From<XPips> for u8 {
    fn from(pips: XPips) -> u8 {
        match pips {
            XPips::Bool(false) => 0,
            XPips::Bool(true) => 1,
            XPips::Count(n) => n,
        }
    }
}

/// A `[cost]` table spells each color by name (`white = 1`) rather than as the
/// [`Cost::colored`] WUBRG array; every field is optional.
impl<'de> Deserialize<'de> for Cost {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize, Default)]
        #[serde(default, deny_unknown_fields)]
        struct Pips {
            generic: u8,
            white: u8,
            blue: u8,
            black: u8,
            red: u8,
            green: u8,
            colorless: u8,
            x: XPips,
            /// Hybrid mana pips (CR 107.4e — `{a/b}`): a list of two-color arrays, one per
            /// hybrid symbol (`hybrid = [["black", "green"]]` for one `{B/G}`).
            hybrid: Vec<[Color; 2]>,
            /// `[cost.additional]` — an additional cost paid alongside mana (CR 601.2f).
            additional: AdditionalCost,
            /// A spell's own board-derived generic reduction (Blasphemous Act's "costs {1} less
            /// ... for each creature on the battlefield"), e.g.
            /// `reduce_own_generic = "per_creature_on_battlefield"`.
            reduce_own_generic: Option<Amount>,
        }

        let pips = Pips::deserialize(d)?;
        let mut hybrid = Vec::with_capacity(pips.hybrid.len());
        for [a, b] in pips.hybrid {
            if a == b {
                return Err(de::Error::custom(
                    "a hybrid pip's two colors must differ (spell a mono pip as a colored cost)",
                ));
            }
            // Normalize to WUBRG order so either spelling interns identically, mirroring
            // Mana::Either's dual-symbol normalization below.
            hybrid.push(if a.index() < b.index() {
                (a, b)
            } else {
                (b, a)
            });
        }
        Ok(Cost {
            generic: pips.generic,
            colored: [pips.white, pips.blue, pips.black, pips.red, pips.green],
            colorless: pips.colorless,
            x: pips.x.into(),
            hybrid: intern(hybrid),
            additional: pips.additional,
            reduce_own_generic: pips.reduce_own_generic,
        })
    }
}

/// `[cost.additional]` spells the pay-life rider as `pay_life`: either the marker string
/// `pay_life = "x"` (Toxic Deluge's "pay X life" — the chosen `{X}` funds it, mirroring `[cost]`'s
/// own `x = true` chooser) or a fixed integer `pay_life = 3` (Deep Analysis's flashback "Pay 3
/// life"). The two are mutually exclusive — one card never spells both. `sacrifice = { count =
/// "one_or_more", filter = "creature" }` spells an optional "sacrifice any number of permanents"
/// cost (Plumb the Forbidden); `sacrifice = { count = 3, filter = "creature" }` spells a mandatory
/// fixed-count sacrifice cost (Dread Return's Flashback—Sacrifice three creatures, CR 601.2f/
/// 602.2b) — `count` is either the `"one_or_more"` marker or a positive integer.
/// `kicker = { generic = 5 }` spells Kicker (CR 702.33) — the same table shape as `[cost]`.
/// `buyback = { generic = 3 }` spells Buyback (CR 702.27) — same table shape.
/// `strive = { generic = 2, red = 1 }` spells Strive (CR 702.42) — same table shape, the
/// per-extra-target cost. `replicate = { generic = 2 }` spells Replicate (CR 702.108) — same
/// table shape, the per-payment cost.
impl<'de> Deserialize<'de> for AdditionalCost {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        /// `pay_life` is a string marker (`"x"`) or a fixed count (`3`); untagged so TOML's own
        /// scalar type picks the arm.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum PayLife {
            Marker(String),
            Fixed(u8),
        }

        /// `count` is either the `"one_or_more"` marker (Plumb the Forbidden) or a fixed integer
        /// (Dread Return's Flashback—Sacrifice three creatures); untagged so TOML's own scalar
        /// type picks the arm, mirroring `PayLife` above.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawSacrificeCount {
            Marker(String),
            Fixed(u8),
        }

        #[derive(Deserialize, Default)]
        #[serde(default, deny_unknown_fields)]
        struct RawSacrifice {
            count: Option<RawSacrificeCount>,
            filter: PermanentFilter,
        }

        #[derive(Deserialize, Default)]
        #[serde(default, deny_unknown_fields)]
        struct Raw {
            discard: u8,
            /// Retrace's "discard a land card" (CR 702.83a) — `discard_land = true`.
            discard_land: bool,
            pay_life: Option<PayLife>,
            sacrifice: Option<RawSacrifice>,
            /// `[cost.additional.kicker]` — Kicker (CR 702.33), the same table shape as `[cost]`.
            kicker: Option<Cost>,
            /// `[cost.additional.buyback]` — Buyback (CR 702.27), the same table shape as
            /// `[cost]`.
            buyback: Option<Cost>,
            /// `[cost.additional.strive]` — Strive (CR 702.42), the same table shape as `[cost]`.
            strive: Option<Cost>,
            /// `[cost.additional.replicate]` — Replicate (CR 702.108), the same table shape as
            /// `[cost]`.
            replicate: Option<Cost>,
        }

        let raw = Raw::deserialize(d)?;
        let (pay_life_x, pay_life) = match raw.pay_life {
            None => (false, 0),
            Some(PayLife::Marker(ref s)) if s == "x" => (true, 0),
            Some(PayLife::Marker(other)) => {
                return Err(de::Error::custom(format!(
                    "cost.additional.pay_life: unsupported string {other:?} (only \"x\" is modeled)"
                )));
            }
            Some(PayLife::Fixed(n)) => (false, n),
        };
        let sacrifice = match raw.sacrifice {
            None => None,
            Some(RawSacrifice {
                count: Some(RawSacrificeCount::Marker(ref s)),
                filter,
            }) if s == "one_or_more" => Some(SacrificeAdditionalCost {
                filter,
                count: SacrificeAdditionalCostCount::OneOrMore,
            }),
            Some(RawSacrifice {
                count: Some(RawSacrificeCount::Fixed(n)),
                filter,
            }) if n > 0 => Some(SacrificeAdditionalCost {
                filter,
                count: SacrificeAdditionalCostCount::Exactly(n),
            }),
            Some(_) => {
                return Err(de::Error::custom(
                    "cost.additional.sacrifice: count must be \"one_or_more\" or a positive integer",
                ));
            }
        };
        Ok(AdditionalCost {
            discard: raw.discard,
            discard_land: raw.discard_land,
            pay_life_x,
            pay_life,
            sacrifice,
            kicker: raw.kicker.map(|c| &*Box::leak(Box::new(c))),
            buyback: raw.buyback.map(|c| &*Box::leak(Box::new(c))),
            strive: raw.strive.map(|c| &*Box::leak(Box::new(c))),
            replicate: raw.replicate.map(|c| &*Box::leak(Box::new(c))),
        })
    }
}

/// A `[kind]` table spells instants and sorceries as their own `type` tags
/// (`type = "instant"`) rather than as [`CardKind::Spell`]'s `speed` field.
impl<'de> Deserialize<'de> for CardKind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        enum Kind {
            Creature {
                power: i32,
                toughness: i32,
                /// Additional card types (Artifact Creature, Enchantment Creature) — a list of
                /// type names. Empty for a plain creature.
                #[serde(default)]
                also: TypeSet,
            },
            Instant,
            Sorcery,
            Enchantment,
            Aura,
            Artifact,
            Planeswalker {
                loyalty: i32,
            },
            Land {
                /// Optional sugar for a free "{T}: Add one mana" base tap; omitted for a
                /// fetch-only land or a land whose mana is all explicit `add_mana` abilities.
                #[serde(default)]
                produces: Option<LandProduces>,
                /// Printed land types (CR 305 — "Forest", "Island", …). Empty for a land with
                /// none (a check land, an untyped scry land).
                #[serde(default)]
                subtypes: Vec<String>,
                /// The "Basic" supertype (CR 205.4a) — `basic = true` in TOML for the five
                /// basics. Independent of `subtypes`: a nonbasic dual can carry the same type
                /// strings without being basic.
                #[serde(default)]
                basic: bool,
            },
        }

        Ok(match Kind::deserialize(d)? {
            Kind::Creature {
                power,
                toughness,
                also,
            } => CardKind::Creature {
                power,
                toughness,
                also,
            },
            Kind::Instant => CardKind::Spell {
                speed: SpellSpeed::Instant,
            },
            Kind::Sorcery => CardKind::Spell {
                speed: SpellSpeed::Sorcery,
            },
            Kind::Enchantment => CardKind::Enchantment,
            Kind::Aura => CardKind::Aura,
            Kind::Artifact => CardKind::Artifact,
            Kind::Planeswalker { loyalty } => CardKind::Planeswalker { loyalty },
            Kind::Land {
                produces,
                subtypes,
                basic,
            } => CardKind::Land {
                produces,
                subtypes: intern_strs(subtypes),
                basic,
            },
        })
    }
}

/// A mana symbol in TOML: a bare string — a color name, `"colorless"` (`{C}`), or `"any"` —
/// or a color array (`["green", "blue"]`) for a fixed choice among 2–4 distinct colors: exactly
/// two normalizes to [`Mana::Either`] (a dual's "either of two colors"), three or four to
/// [`Mana::OfColors`] (a triome's "{G}, {W}, or {U}" — Treva's Ruins). Color spellings delegate
/// to [`Color`]'s derive so they live in exactly one place.
impl<'de> Deserialize<'de> for Mana {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ManaVisitor;

        impl<'de> Visitor<'de> for ManaVisitor {
            type Value = Mana;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a mana symbol (a color name, \"colorless\", or \"any\") or a 2-to-4-color \
                     array (a fixed choice of colors)",
                )
            }

            fn visit_str<E: de::Error>(self, symbol: &str) -> Result<Mana, E> {
                Ok(match symbol {
                    "colorless" => Mana::Colorless,
                    "any" => Mana::Any,
                    color => Mana::Color(Color::deserialize(color.into_deserializer())?),
                })
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Mana, A::Error> {
                let hint = &"2 to 4 distinct colors";
                let mut colors: Vec<Color> = Vec::new();
                while let Some(color) = seq.next_element::<Color>()? {
                    colors.push(color);
                }
                if colors.len() < 2 || colors.len() > 4 {
                    return Err(de::Error::invalid_length(colors.len(), hint));
                }
                let mut mask: u8 = 0;
                for &color in &colors {
                    let bit = 1 << color.index();
                    if mask & bit != 0 {
                        return Err(de::Error::custom(
                            "a color-choice mana symbol's colors must be distinct (spell a mono \
                             producer as one color)",
                        ));
                    }
                    mask |= bit;
                }
                if colors.len() == 2 {
                    // Normalize to WUBRG order so ["green", "blue"] == ["blue", "green"].
                    return Ok(if colors[0].index() < colors[1].index() {
                        Mana::Either(colors[0], colors[1])
                    } else {
                        Mana::Either(colors[1], colors[0])
                    });
                }
                Ok(Mana::OfColors(mask))
            }
        }

        d.deserialize_any(ManaVisitor)
    }
}

/// A land's `produces` sugar in TOML: a [`Mana`] symbol (any of its spellings, including a
/// dual's two-color array), the literal string `"commander_identity"` — "one mana of any
/// color in your commander's color identity" (CR 903.4, Command Tower) — or the literal string
/// `"opponent_colors"` — "one mana of any color that a land an opponent controls could produce"
/// (Exotic Orchard).
impl<'de> Deserialize<'de> for LandProduces {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ProducesVisitor;

        impl<'de> Visitor<'de> for ProducesVisitor {
            type Value = LandProduces;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a mana symbol, \"commander_identity\", \"opponent_colors\", or a two-color array",
                )
            }

            fn visit_str<E: de::Error>(self, symbol: &str) -> Result<LandProduces, E> {
                match symbol {
                    "commander_identity" => return Ok(LandProduces::CommanderIdentity),
                    "opponent_colors" => return Ok(LandProduces::OpponentColors),
                    _ => {}
                }
                Ok(LandProduces::Mana(Mana::deserialize(
                    symbol.into_deserializer(),
                )?))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, seq: A) -> Result<LandProduces, A::Error> {
                Ok(LandProduces::Mana(Mana::deserialize(
                    de::value::SeqAccessDeserializer::new(seq),
                )?))
            }
        }

        d.deserialize_any(ProducesVisitor)
    }
}

/// `{ protection = "<value>" }`: a color name (`"red"`, …) for the common fixed-color case, or
/// one of the non-color qualities `"creatures"` / `"multicolored"`.
impl<'de> Deserialize<'de> for ProtectionScope {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ScopeVisitor;

        impl<'de> Visitor<'de> for ScopeVisitor {
            type Value = ProtectionScope;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a color name, \"creatures\", or \"multicolored\"")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<ProtectionScope, E> {
                match value {
                    "creatures" => return Ok(ProtectionScope::Creatures),
                    "multicolored" => return Ok(ProtectionScope::Multicolored),
                    _ => {}
                }
                Ok(ProtectionScope::Color(Color::deserialize(
                    value.into_deserializer(),
                )?))
            }
        }

        d.deserialize_str(ScopeVisitor)
    }
}

/// A numeric quantity in TOML: a plain number (`amount = 3`), a keyword string for a derived
/// value (`"x"`, `"half_x"`, `"half_x_rounded_down"`, `"twice_x"`, `"per_creature_you_control"`, `"source_power"`,
/// `"source_toughness"`, `"target_power"`, `"target_mana_value"`, `"per_counter_on_source"`, `"life_gained_this_turn"`,
/// `"spells_cast_this_turn"`, `"commander_casts_from_command_zone"`, `"creatures_died_this_turn"`,
/// `"nontoken_creatures_entered_this_turn"`,
/// `"sacrificed_creature_power"`, `"commander_color_count"`, `"total_power_you_control"`,
/// `"triggering_spell_mana_value"`, `"spell_sacrifice_count"`, `"permanents_died_this_turn"`,
/// `"past_votes"`, `"present_votes"`, `"total_mana_value_milled_this_way"`,
/// `"exiled_card_mana_value_this_way"`, `"combat_damage_dealt"`, `"spells_cast_before_this_this_turn"`),
/// or a table for a filtered count
/// (`{ per_permanent = <filter>, zone = "graveyard" }`), a per-kind counter count
/// (`{ per_counter_of_kind = "charge" }`), a conditional amount
/// (`{ condition = <Condition>, then = <Amount> }` — 0 when `condition` doesn't hold), a
/// kicked-branch amount (`{ if_kicked = <Amount>, else = <Amount> }` — CR 702.33d), or a
/// "destroyed this way" count (`{ permanents_destroyed_this_way = <filter> }`, filter optional
/// — defaults to matching every destroyed permanent), or a count of Auras attached to the
/// effect's source (`{ auras_attached_to_source = {} }`).
impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct AmountVisitor;

        const KEYWORDS: &[&str] = &[
            "x",
            "half_x",
            "half_x_rounded_down",
            "twice_x",
            "per_creature_you_control",
            "per_creature_on_battlefield",
            "source_power",
            "source_toughness",
            "target_power",
            "target_toughness",
            "target_mana_value",
            "per_counter_on_source",
            "life_gained_this_turn",
            "spells_cast_this_turn",
            "cards_in_target_player_hand",
            "cards_in_your_hand",
            "commander_casts_from_command_zone",
            "creatures_died_this_turn",
            "nontoken_creatures_entered_this_turn",
            "sacrificed_creature_power",
            "sacrificed_creature_toughness",
            "commander_color_count",
            "total_power_you_control",
            "permanents_you_own_opponents_control",
            "triggering_spell_mana_value",
            "triggering_spell_mana_spent",
            "spell_sacrifice_count",
            "permanents_died_this_turn",
            "nonland_cards_exiled_this_way",
            "past_votes",
            "present_votes",
            "total_mana_value_milled_this_way",
            "exiled_card_mana_value_this_way",
            "auras_you_controlled_attached_to_dying_creature",
            "greatest_instant_or_sorcery_mana_value_cast_this_turn",
            "one_plus_instants_and_sorceries_cast_this_turn",
            "instant_or_sorcery_cards_in_your_graveyard",
            "combat_damage_dealt",
            "triggering_damage_dealt",
            "spells_cast_before_this_this_turn",
        ];

        impl<'de> Visitor<'de> for AmountVisitor {
            type Value = Amount;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a number, a derived-amount keyword, or a per-permanent table")
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Amount, E> {
                let n = i32::try_from(n).map_err(|_| {
                    E::invalid_value(de::Unexpected::Signed(n), &"an amount that fits in i32")
                })?;
                Ok(Amount::Fixed(n))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Amount, E> {
                Ok(match s {
                    "x" => Amount::X,
                    "half_x" => Amount::HalfX,
                    "half_x_rounded_down" => Amount::HalfXRoundedDown,
                    "twice_x" => Amount::TwiceX,
                    "per_creature_you_control" => Amount::PerCreatureYouControl,
                    "per_creature_on_battlefield" => Amount::PerCreatureOnBattlefield,
                    "source_power" => Amount::SourcePower,
                    "source_toughness" => Amount::SourceToughness,
                    "target_power" => Amount::TargetPower,
                    "target_toughness" => Amount::TargetToughness,
                    "target_mana_value" => Amount::TargetManaValue,
                    "per_counter_on_source" => Amount::PerCounterOnSource,
                    "life_gained_this_turn" => Amount::LifeGainedThisTurn,
                    "spells_cast_this_turn" => Amount::SpellsCastThisTurn,
                    "cards_in_target_player_hand" => Amount::CardsInTargetPlayerHand,
                    "cards_in_your_hand" => Amount::CardsInYourHand,
                    "commander_casts_from_command_zone" => Amount::CommanderCastsFromCommandZone,
                    "creatures_died_this_turn" => Amount::CreaturesDiedThisTurn,
                    "nontoken_creatures_entered_this_turn" => {
                        Amount::NontokenCreaturesEnteredThisTurn
                    }
                    "sacrificed_creature_power" => Amount::SacrificedCreaturePower,
                    "sacrificed_creature_toughness" => Amount::SacrificedCreatureToughness,
                    "commander_color_count" => Amount::CommanderColorCount,
                    "total_power_you_control" => Amount::TotalPowerYouControl,
                    "permanents_you_own_opponents_control" => {
                        Amount::PermanentsYouOwnOpponentsControl
                    }
                    "triggering_spell_mana_value" => Amount::TriggeringSpellManaValue,
                    "triggering_spell_mana_spent" => Amount::TriggeringSpellManaSpent,
                    "spell_sacrifice_count" => Amount::SpellSacrificeCount,
                    "permanents_died_this_turn" => Amount::PermanentsDiedThisTurn,
                    "nonland_cards_exiled_this_way" => Amount::NonlandCardsExiledThisWay,
                    "past_votes" => Amount::PastVotes,
                    "present_votes" => Amount::PresentVotes,
                    "total_mana_value_milled_this_way" => Amount::TotalManaValueMilledThisWay,
                    "exiled_card_mana_value_this_way" => Amount::ExiledCardManaValueThisWay,
                    "auras_you_controlled_attached_to_dying_creature" => {
                        Amount::AurasYouControlledAttachedToDyingCreature
                    }
                    "greatest_instant_or_sorcery_mana_value_cast_this_turn" => {
                        Amount::GreatestInstantOrSorceryManaValueCastThisTurn
                    }
                    "one_plus_instants_and_sorceries_cast_this_turn" => {
                        Amount::OnePlusInstantsAndSorceriesCastThisTurn
                    }
                    "instant_or_sorcery_cards_in_your_graveyard" => {
                        Amount::InstantOrSorceryCardsInYourGraveyard
                    }
                    "combat_damage_dealt" => Amount::CombatDamageDealt,
                    "triggering_damage_dealt" => Amount::TriggeringDamageDealt,
                    "spells_cast_before_this_this_turn" => Amount::SpellsCastBeforeThisThisTurn,
                    other => return Err(E::unknown_variant(other, KEYWORDS)),
                })
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Amount, A::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Table {
                    #[serde(default)]
                    per_permanent: Option<PermanentFilter>,
                    #[serde(default)]
                    zone: AmountZone,
                    #[serde(default)]
                    per_counter_of_kind: Option<CounterKind>,
                    #[serde(default)]
                    condition: Option<Condition>,
                    #[serde(default)]
                    then: Option<Amount>,
                    /// `{ if_kicked = 5, else = 1 }` — [`Amount::IfSpellKicked`] (CR 702.33d).
                    #[serde(default)]
                    if_kicked: Option<Amount>,
                    #[serde(default, rename = "else")]
                    otherwise: Option<Amount>,
                    /// `{ permanents_destroyed_this_way = <filter> }` — [`Amount::PermanentsDestroyedThisWay`].
                    /// A separate key from `per_permanent` (rather than reusing it) so an
                    /// empty `{}` filter table still selects this arm.
                    #[serde(default)]
                    permanents_destroyed_this_way: Option<PermanentFilter>,
                    /// `{ auras_attached_to_source = {} }` — [`Amount::AurasAttachedToSource`]. A
                    /// bare `{}` presence flag (no fields of its own), matching the
                    /// `permanents_destroyed_this_way` table-vs-nullary-keyword split.
                    #[serde(default)]
                    auras_attached_to_source: Option<de::IgnoredAny>,
                }
                let t = Table::deserialize(de::value::MapAccessDeserializer::new(map))?;
                match (
                    t.per_permanent,
                    t.per_counter_of_kind,
                    t.condition,
                    t.then,
                    t.if_kicked,
                    t.otherwise,
                    t.permanents_destroyed_this_way,
                    t.auras_attached_to_source,
                ) {
                    (Some(filter), None, None, None, None, None, None, None) => {
                        Ok(Amount::PerPermanentMatching {
                            filter,
                            zone: t.zone,
                        })
                    }
                    (None, Some(kind), None, None, None, None, None, None) => {
                        Ok(Amount::PerCounterOfKindOnSource { kind })
                    }
                    (None, None, Some(condition), Some(then), None, None, None, None) => {
                        Ok(Amount::IfCondition {
                            condition,
                            then: &*Box::leak(Box::new(then)),
                        })
                    }
                    (None, None, None, None, Some(if_kicked), Some(otherwise), None, None) => {
                        Ok(Amount::IfSpellKicked {
                            then: &*Box::leak(Box::new(if_kicked)),
                            else_: &*Box::leak(Box::new(otherwise)),
                        })
                    }
                    (None, None, None, None, None, None, Some(filter), None) => {
                        Ok(Amount::PermanentsDestroyedThisWay { filter })
                    }
                    (None, None, None, None, None, None, None, Some(_)) => {
                        Ok(Amount::AurasAttachedToSource)
                    }
                    _ => Err(de::Error::custom(
                        "an amount table needs exactly one of `per_permanent`, `per_counter_of_kind`, \
                         `condition`+`then`, `if_kicked`+`else`, `permanents_destroyed_this_way`, or \
                         `auras_attached_to_source`",
                    )),
                }
            }
        }

        d.deserialize_any(AmountVisitor)
    }
}

/// A [`TargetCount`] (CR 601.2c). Two spellings:
/// - a bare integer `N` (`count = 6`) ⇒ an exact "N target" (`{ min: N, max: N }`);
/// - a table `{ min, max, x_scaled, sacrifice_scaled }` (`count = { min = 1, max = 2 }`) ⇒ an
///   explicit "up to"/"one or two" range. `min` and `max` both default to 0, so a scaled count
///   needs neither. `x_scaled` (CR 601.2b, default `false`) marks `min`/`max` as placeholders the
///   spell's chosen `{X}` substitutes at cast time (see [`TargetCount::x_scaled`]'s own doc for
///   the exact rule): `{ min = 0, max = 0, x_scaled = true }` is "up to X target(s)" (Silkguard);
///   `{ min = 1, max = 1, x_scaled = true }` is "exactly X target(s)" (Curse of the Swine).
///   `sacrifice_scaled` (default `false`) is the sibling for a spell whose X is defined by an
///   additional sacrifice cost rather than chosen as `{X}` (see
///   [`TargetCount::sacrifice_scaled`]'s own doc): `{ sacrifice_scaled = true }` is "exactly X
///   target(s)" where X is the number sacrificed (Immoral Bargain). `strive_scaled` (default
///   `false`) is Strive's own sibling (see [`TargetCount::strive_scaled`]'s own doc): `{
///   strive_scaled = true }` is "exactly N target(s)" where N is the caster's declared Strive
///   target count (Twinflame).
///
/// ponytail: no pool card needs a *fixed* range yet (Aether Gale is exactly six); the table form
/// is here so "up to N"/"one or two" cards don't need a new deserializer when they land.
impl<'de> Deserialize<'de> for TargetCount {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct CountVisitor;

        impl<'de> Visitor<'de> for CountVisitor {
            type Value = TargetCount;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a target count: an integer N, or a { min, max, x_scaled, sacrifice_scaled, \
                     strive_scaled } range",
                )
            }

            fn visit_u64<E: de::Error>(self, n: u64) -> Result<TargetCount, E> {
                let n = u8::try_from(n).map_err(|_| {
                    E::invalid_value(
                        de::Unexpected::Unsigned(n),
                        &"a target count that fits in u8",
                    )
                })?;
                Ok(TargetCount {
                    min: n,
                    max: n,
                    x_scaled: false,
                    sacrifice_scaled: false,
                    strive_scaled: false,
                })
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<TargetCount, E> {
                let n = u64::try_from(n).map_err(|_| {
                    E::invalid_value(de::Unexpected::Signed(n), &"a non-negative target count")
                })?;
                self.visit_u64(n)
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<TargetCount, A::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Table {
                    #[serde(default)]
                    min: u8,
                    #[serde(default)]
                    max: u8,
                    #[serde(default)]
                    x_scaled: bool,
                    #[serde(default)]
                    sacrifice_scaled: bool,
                    #[serde(default)]
                    strive_scaled: bool,
                }
                let t = Table::deserialize(de::value::MapAccessDeserializer::new(map))?;
                if t.min > t.max {
                    return Err(de::Error::custom("target count min exceeds max"));
                }
                Ok(TargetCount {
                    min: t.min,
                    max: t.max,
                    x_scaled: t.x_scaled,
                    sacrifice_scaled: t.sacrifice_scaled,
                    strive_scaled: t.strive_scaled,
                })
            }
        }

        d.deserialize_any(CountVisitor)
    }
}

/// The zone a `per_permanent` count ranges over: `"battlefield"` (default) or `"graveyard"`.
impl<'de> Deserialize<'de> for AmountZone {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match String::deserialize(d)?.as_str() {
            "battlefield" => AmountZone::Battlefield,
            "graveyard" => AmountZone::Graveyard,
            other => {
                return Err(de::Error::unknown_variant(
                    other,
                    &["battlefield", "graveyard"],
                ));
            }
        })
    }
}

/// The [`TypeSet`] bits a single card-type name spells, or `None` for an unknown name.
/// `"nonland"` is sugar for the four nonland permanent types; the `"_or_"` names are two-type
/// union shorthands (Steelbane Hydra's "artifact or enchantment", Quandrix Command's "creature or
/// planeswalker", Ozolith's "artifact or creature").
fn type_bits(name: &str) -> Option<TypeSet> {
    Some(match name {
        "creature" => TypeSet::CREATURE,
        "artifact" => TypeSet::ARTIFACT,
        "enchantment" => TypeSet::ENCHANTMENT,
        "planeswalker" => TypeSet::PLANESWALKER,
        "land" => TypeSet::LAND,
        "nonland" => TypeSet::NONLAND,
        "artifact_or_enchantment" => TypeSet::ARTIFACT.union(TypeSet::ENCHANTMENT),
        "creature_or_planeswalker" => TypeSet::CREATURE.union(TypeSet::PLANESWALKER),
        "artifact_or_creature" => TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        _ => return None,
    })
}

const TYPE_NAMES: &[&str] = &[
    "creature",
    "artifact",
    "enchantment",
    "planeswalker",
    "land",
    "nonland",
    "artifact_or_enchantment",
    "creature_or_planeswalker",
    "artifact_or_creature",
];

/// A [`TypeSet`] in TOML: one type name (`"artifact"`) or a list of them
/// (`["creature", "artifact"]`, their union). An empty list is the empty set.
impl<'de> Deserialize<'de> for TypeSet {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct TypeSetVisitor;

        impl<'de> Visitor<'de> for TypeSetVisitor {
            type Value = TypeSet;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a card-type name or a list of card-type names")
            }

            fn visit_str<E: de::Error>(self, name: &str) -> Result<TypeSet, E> {
                type_bits(name).ok_or_else(|| E::unknown_variant(name, TYPE_NAMES))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<TypeSet, A::Error> {
                let mut set = TypeSet::NONE;
                while let Some(name) = seq.next_element::<String>()? {
                    let bits = type_bits(&name)
                        .ok_or_else(|| de::Error::unknown_variant(&name, TYPE_NAMES))?;
                    set = set.union(bits);
                }
                Ok(set)
            }
        }

        d.deserialize_any(TypeSetVisitor)
    }
}

/// A [`PermanentFilter`] in TOML: either a bare-string shorthand for a common type set
/// (`"creatures"`, `"nonland"`, `"artifact"`, `"creature_or_planeswalker"`, …) — which keeps
/// the old `destroy_all`/edict spellings working — or a full `{ … }` table with any of the
/// composable axes (`types`, `controller`, `token`, `other`, `enchanted`, `attached_to_creature`,
/// `enchanted_by_you`, `mv_max`, `mv_min`, `mv_eq_x`, `mv_max_x`, `power_max`, `power_parity`,
/// `noncreature`, `exclude`, `color`, `not_color`, `modified`, `attacking`, `attacking_you`,
/// `power_less_than_source`, `entered_this_turn`, `nonbasic`, `nonlegendary`, `nonlair`,
/// `without_flying`). `noncreature` is sugar for `exclude = "creature"`; `not_color` is sugar for
/// `color`'s negated-color arm — both fold into the same [`PermanentFilter`] fields as their
/// general spelling (see below).
impl<'de> Deserialize<'de> for PermanentFilter {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct FilterVisitor;

        impl<'de> Visitor<'de> for FilterVisitor {
            type Value = PermanentFilter;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a permanent-filter shorthand string or a filter table")
            }

            fn visit_str<E: de::Error>(self, shorthand: &str) -> Result<PermanentFilter, E> {
                // Martyr's Bond's dynamic edict filter ("shares a card type with it") isn't a
                // static type set — its `types` is filled at `contextualize_effect` time from the
                // triggering dying permanent's own last-known card types, not from this shorthand.
                if shorthand == "shares_type_with_dying_permanent" {
                    return Ok(PermanentFilter {
                        shares_type_with_dying_permanent: true,
                        ..PermanentFilter::of(TypeSet::NONE)
                    });
                }
                let types = match shorthand {
                    // Plurals kept as sugar for the old mass-effect / edict spellings.
                    "creatures" | "creature" => TypeSet::CREATURE,
                    "nonland_permanents" | "nonland" => TypeSet::NONLAND,
                    "creature_or_planeswalker" => TypeSet::CREATURE.union(TypeSet::PLANESWALKER),
                    name => type_bits(name).ok_or_else(|| {
                        E::custom(format!("unknown permanent-filter shorthand {name:?}"))
                    })?,
                };
                Ok(PermanentFilter::of(types))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<PermanentFilter, A::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields, rename_all = "snake_case")]
                struct Table {
                    #[serde(default)]
                    types: TypeSet,
                    /// Subtype restriction (Goldspan Dragon's "Treasures you control").
                    #[serde(default)]
                    subtypes: Vec<String>,
                    #[serde(default)]
                    controller: FilterController,
                    #[serde(default)]
                    token: TokenFilter,
                    #[serde(default)]
                    other: bool,
                    #[serde(default)]
                    enchanted: Option<bool>,
                    #[serde(default)]
                    attached_to_creature: Option<bool>,
                    #[serde(default)]
                    enchanted_by_you: bool,
                    #[serde(default)]
                    mv_max: Option<u8>,
                    #[serde(default)]
                    mv_min: Option<u8>,
                    #[serde(default)]
                    mv_eq_x: bool,
                    #[serde(default)]
                    mv_max_x: bool,
                    #[serde(default)]
                    tapped: Option<bool>,
                    #[serde(default)]
                    power_max: Option<u8>,
                    #[serde(default)]
                    power_parity: Option<Parity>,
                    /// Sugar for `exclude = "creature"` (kept for the pool's existing spelling).
                    #[serde(default)]
                    noncreature: bool,
                    /// General type exclusion (Terror/Shriekmaw's "nonartifact") — a union with
                    /// `noncreature`'s implied `TypeSet::CREATURE`, not a replacement for it.
                    #[serde(default)]
                    exclude: TypeSet,
                    #[serde(default)]
                    color: Option<ColorFilter>,
                    /// Sugar for `color`'s negated arm (Terror/Shriekmaw's "nonblack").
                    #[serde(default)]
                    not_color: Option<Color>,
                    #[serde(default)]
                    modified: bool,
                    #[serde(default)]
                    attacking: bool,
                    #[serde(default)]
                    attacking_you: bool,
                    #[serde(default)]
                    power_less_than_source: bool,
                    #[serde(default)]
                    entered_this_turn: bool,
                    #[serde(default)]
                    nonbasic: bool,
                    /// Printed-name restriction (Leitmotif Composer's "creatures named Leitmotif
                    /// Composer").
                    #[serde(default)]
                    name: Option<String>,
                    #[serde(default)]
                    nonlegendary: bool,
                    #[serde(default)]
                    nonlair: bool,
                    #[serde(default)]
                    without_flying: bool,
                    /// Martyr's Bond's dynamic "shares a card type with it" edict gate — see the
                    /// bare-string shorthand of the same name above.
                    #[serde(default)]
                    shares_type_with_dying_permanent: bool,
                }

                let t = Table::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(PermanentFilter {
                    types: t.types,
                    subtypes: intern_strs(t.subtypes),
                    controller: t.controller,
                    token: t.token,
                    other: t.other,
                    enchanted: t.enchanted,
                    attached_to_creature: t.attached_to_creature,
                    enchanted_by_you: t.enchanted_by_you,
                    mv_max: t.mv_max,
                    mv_min: t.mv_min,
                    mv_eq_x: t.mv_eq_x,
                    mv_max_x: t.mv_max_x,
                    tapped: t.tapped,
                    power_max: t.power_max,
                    power_parity: t.power_parity,
                    exclude: t.exclude.union(if t.noncreature {
                        TypeSet::CREATURE
                    } else {
                        TypeSet::NONE
                    }),
                    color: t
                        .not_color
                        .map(ColorFilter::NotColor)
                        .unwrap_or(t.color.unwrap_or_default()),
                    modified: t.modified,
                    attacking: t.attacking,
                    attacking_you: t.attacking_you,
                    power_less_than_source: t.power_less_than_source,
                    entered_this_turn: t.entered_this_turn,
                    nonbasic: t.nonbasic,
                    name: t.name.map(|s| &*Box::leak(s.into_boxed_str())),
                    nonlegendary: t.nonlegendary,
                    nonlair: t.nonlair,
                    without_flying: t.without_flying,
                    shares_type_with_dying_permanent: t.shares_type_with_dying_permanent,
                })
            }
        }

        d.deserialize_any(FilterVisitor)
    }
}

/// A [`SacrificeCost`] in TOML: `"none"` / `"this"` / `"creature"` (bare-string sugar —
/// `"creature"` is "a creature you control", no self-exclusion, count 1), a
/// `{ creature = { … }, count = N }` table naming [`PermanentFilter`] overrides (Izoni's
/// "Sacrifice *another* creature" is `sacrifice = { creature = { other = true } }`) and/or a
/// sacrifice count above 1 (Priest of Forgotten Gods's "Sacrifice two other creatures" is
/// `sacrifice = { creature = { other = true }, count = 2 }`), or a `{ permanent = { … }, count =
/// N }` table for a non-creature sacrifice (Gyome, Master Chef's "Sacrifice a Food" is
/// `sacrifice = { permanent = { subtypes = ["Food"] } }`). `count` defaults to 1 when omitted.
/// The `creature` key's table forces its `types` axis to creature; `permanent`'s leaves `types`
/// unforced, so the filter's own `types`/`subtypes` decide what qualifies.
impl<'de> Deserialize<'de> for SacrificeCost {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct SacrificeCostVisitor;

        impl<'de> Visitor<'de> for SacrificeCostVisitor {
            type Value = SacrificeCost;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    r#""none", "this", "creature", or a `{ creature = { ... }, count = N }` table"#,
                )
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<SacrificeCost, E> {
                match s {
                    "none" => Ok(SacrificeCost::None),
                    "this" => Ok(SacrificeCost::This),
                    "creature" => Ok(SacrificeCost::Creature {
                        filter: PermanentFilter::of(TypeSet::CREATURE),
                        count: 1,
                    }),
                    other => Err(E::custom(format!("unknown sacrifice cost {other:?}"))),
                }
            }

            fn visit_map<A: de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<SacrificeCost, A::Error> {
                let mut filter: Option<PermanentFilter> = None;
                let mut count: u8 = 1;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "creature" => {
                            let mut f: PermanentFilter = map.next_value()?;
                            f.types = TypeSet::CREATURE;
                            filter = Some(f);
                        }
                        // "Sacrifice a Food" (Gyome, Master Chef; Gilded Goose): a non-creature
                        // sacrifice cost — `filter`'s own `types`/`subtypes` axes decide what
                        // qualifies, unforced (unlike the `creature` key above).
                        "permanent" => filter = Some(map.next_value()?),
                        "count" => count = map.next_value()?,
                        other => {
                            return Err(de::Error::custom(format!(
                                "unknown sacrifice cost key {other:?}"
                            )));
                        }
                    }
                }
                let filter =
                    filter.ok_or_else(|| de::Error::custom("expected a sacrifice-cost key"))?;
                Ok(SacrificeCost::Creature { filter, count })
            }
        }

        d.deserialize_any(SacrificeCostVisitor)
    }
}

/// The `timing` tag for a triggered ability. Mirrors [`Trigger`]'s variants one-for-one, but stays
/// fieldless: two of them ([`YouSacrifice`](TriggerTag::YouSacrifice)/
/// [`AnyPlayerSacrifices`](TriggerTag::AnyPlayerSacrifices)) carry a [`PermanentFilter`] on the
/// real `Trigger`, and a third ([`DealsCombatDamageToPlayer`](TriggerTag::DealsCombatDamageToPlayer))
/// carries a [`CombatDamageScope`], a fourth ([`CastSpell`](TriggerTag::CastSpell)) carries a
/// [`SpellFilter`]/[`CasterScope`]/`nth_each_turn`, a fifth ([`PlayerDraws`](TriggerTag::PlayerDraws))
/// carries a [`CasterScope`]/`nth_each_turn` (the draw-side twin of `CastSpell`, no filter), and a
/// sixth and seventh ([`PermanentEnters`](TriggerTag::PermanentEnters)/
/// [`PermanentEntersIncludingThis`](TriggerTag::PermanentEntersIncludingThis)) carry a
/// [`PermanentFilter`]/[`EnterController`], none of which can come from a bare `timing = "…"`
/// string —
/// [`Ability::deserialize`] pairs the tag with sibling fields (`filter`, `who`,
/// `spell_filter`/`caster`/`drawer`/`nth_each_turn`, `controller`) to build those by hand. An
/// eighth pair ([`YouAttackWithCreatures`](TriggerTag::YouAttackWithCreatures)/
/// [`OpponentAttacksYouWithCreatures`](TriggerTag::OpponentAttacksYouWithCreatures)) carries a
/// sibling `at_least` count the same way, and
/// [`CreatureEnchantedByYourAuraAttacks`](TriggerTag::CreatureEnchantedByYourAuraAttacks) and
/// [`AnotherPlayerAttacksWithCreatures`](TriggerTag::AnotherPlayerAttacksWithCreatures) reuse
/// that same `at_least` sibling. A ninth ([`SpellTargetsThisOnly`](TriggerTag::SpellTargetsThisOnly),
/// `timing = "spell_targets_this"`) reuses `CastSpell`'s `spell_filter` sibling.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TriggerTag {
    #[serde(alias = "etb_triggered")]
    Etb,
    TurnedFaceUp,
    Attacks,
    BlocksOrBecomesBlocked,
    Dies,
    CreatureDies,
    CreatureYouControlDies,
    CreatureDiesIncludingThis,
    CreatureYouControlDiesIncludingThis,
    CreatureYouControlDiesNontoken,
    CreatureYouControlDiesIncludingThisNontoken,
    CreatureAnOpponentControlsDies,
    EnchantmentYouControlDies,
    NonlandPermanentYouControlDiesIncludingThis,
    Upkeep,
    EachUpkeep,
    FirstMainPhase,
    BeginCombat,
    EndStep,
    EachEndStep,
    EachDrawStep,
    EachOtherPlayerUntapStep,
    YouGainLife,
    OpponentGainsLife,
    Magecraft,
    PlayerAttacksYourOpponent,
    YouAttackWithCreatures,
    OpponentAttacksYouWithCreatures,
    AnotherPlayerAttacksWithCreatures,
    /// Equipment's own name for the same "whenever the permanent this is attached to attacks"
    /// firing path (CR 508.1) — Fractal Harness's "whenever equipped creature attacks". The
    /// underlying [`Trigger::EnchantedCreatureAttacks`] already fires off any attached permanent,
    /// Aura or Equipment (see [`Game::queue_enchanted_creature_attacks_triggers`], which reads
    /// [`Game::attachments`] rather than filtering to Auras); this is a card-authoring alias only,
    /// not a distinct engine trigger.
    #[serde(alias = "equipped_creature_attacks")]
    EnchantedCreatureAttacks,
    EnchantedCreatureDies,
    /// Whenever the enchanted host deals damage, combat or noncombat (Armadillo Cloak's "you gain
    /// that much life"). See [`Trigger::EnchantedCreatureDealsDamage`].
    EnchantedCreatureDealsDamage,
    AnEnchantedCreatureDies,
    CreatureEnchantedByYourAuraAttacks,
    YouSacrifice,
    AnyPlayerSacrifices,
    YouDiscard,
    DealsCombatDamageToPlayer,
    DealsCombatDamageToCreature,
    CreatureDealtDamageByThisDies,
    DealsDamageToOpponent,
    CastSpell,
    PlayerDraws,
    ActivateAbility,
    PermanentEnters,
    PermanentEntersIncludingThis,
    CardsLeaveYourGraveyard,
    CardsExiledFromYourLibraryOrGraveyard,
    YouCreateToken,
    #[serde(alias = "becomes_the_target")]
    BecomesTargeted,
    #[serde(rename = "spell_targets_this")]
    SpellTargetsThisOnly,
    #[serde(rename = "when_you_cast_this")]
    YouCastThis,
    #[serde(rename = "this_put_into_graveyard")]
    ThisAuraLeaves,
    #[serde(rename = "this_leaves_battlefield")]
    ThisPermanentLeavesBattlefield,
    #[serde(rename = "zero_base_power_creatures_deal_combat_damage")]
    ZeroBasePowerCreaturesYouControlDealCombatDamage,
    SpendManaToCast,
    YouLoseLifeFirstTimeEachTurn,
    Cycled,
}

/// An `[[abilities]]` table is flat in TOML: the timing is a string, and an activated
/// ability's cost pieces (`taps_self`, `activation_cost`, `sacrifice`, `pay_life`,
/// `loyalty`, `once_each_turn`) sit beside it rather than nested inside [`Timing::Activated`].
impl<'de> Deserialize<'de> for Ability {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        /// The three timings that aren't a [`Trigger`].
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum SpecialTiming {
            Spell,
            Static,
            Activated,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TimingName {
            Special(SpecialTiming),
            Trigger(TriggerTag),
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Flat {
            timing: TimingName,
            #[serde(default)]
            taps_self: bool,
            #[serde(default)]
            activation_cost: Cost,
            #[serde(default)]
            sacrifice: SacrificeCost,
            #[serde(default)]
            pay_life: Amount,
            /// +1/+1 counters removed from the source as part of the activation cost (CR 118
            /// "remove a counter" cost — Steelbane Hydra's "Remove a +1/+1 counter from this
            /// creature").
            #[serde(default)]
            remove_counters: u8,
            /// Which counter kind `remove_counters` removes; unset (the default) is the +1/+1
            /// path above (staff_of_the_storyteller's "remove a story counter" sets this).
            #[serde(default)]
            remove_counters_kind: Option<CounterKind>,
            #[serde(default)]
            self_damage: u8,
            #[serde(default)]
            loyalty: Option<i32>,
            /// "Activate only once each turn" (CR 602.2b) on an activated ability, or "this
            /// ability triggers only once each turn" (CR) on a triggered one — one TOML key
            /// feeding whichever struct `timing` resolves to below (`ActivationCost` or
            /// `Ability` itself).
            #[serde(default)]
            once_each_turn: bool,
            /// "Activate only as a sorcery" (CR 602.5b): restricts activation to a legal
            /// sorcery-speed moment (Ozolith, the Shattered Spire's counter ability).
            #[serde(default)]
            sorcery_speed: bool,
            /// "Return this to its owner's hand" as part of the cost (Rootha, Mercurial
            /// Artist's "Return Rootha to its owner's hand").
            #[serde(default)]
            return_self: bool,
            /// "Mill a card" as part of the cost (Millikin's "Mill a card").
            #[serde(default)]
            mill_self: u8,
            /// "Discard a card" as part of the cost (Wild Mongrel's "Discard a card").
            #[serde(default)]
            discard_cost: u8,
            /// "Exile this artifact"/"exile this permanent" as part of the cost (Perpetual
            /// Timepiece's "Exile this artifact").
            #[serde(default)]
            exile_self: bool,
            /// "Exile N target cards from an opponent's graveyard" as an additional cost
            /// (Spurnmage Advocate's "Exile two target cards from an opponent's graveyard").
            #[serde(default)]
            graveyard_exile_target_count: u8,
            #[serde(default)]
            condition: Option<Condition>,
            #[serde(default)]
            optional: bool,
            /// The minimum Class level this ability requires to function (CR 717.5 — a Class's
            /// level-gated abilities). `0` (the default, omitted in TOML) is unconditional.
            #[serde(default)]
            min_level: u8,
            /// The cost to accept an `optional` trigger (CR 603.2c "you may pay …"), e.g. Trudge
            /// Garden's "you may pay {2}." Ignored for a non-optional ability. Same `[cost]`-table
            /// shape as a spell's own top-level cost (§2); omitted = a plain free "may".
            #[serde(default)]
            cost: Cost,
            /// The permanent filter for a `you_sacrifice`/`any_player_sacrifices`/
            /// `permanent_enters` trigger (Smothering Abomination's "a creature", Mazirek's
            /// "another permanent", Ajani's Chosen's "an enchantment"). Ignored for every other
            /// trigger/timing.
            #[serde(default)]
            filter: PermanentFilter,
            /// Whose permanent a `permanent_enters` trigger watches — `you` (default,
            /// constellation's "an enchantment you control"), `opponent` (Archaeomancer's Map's
            /// "a land an opponent controls"), or `any_player`. Ignored for every other
            /// trigger/timing.
            #[serde(default)]
            controller: EnterController,
            /// Who a `deals_combat_damage_to_player` trigger watches (Leitmotif Composer's
            /// `this`, Ohran Frostfang's `your_creatures`, Curiosity Crafter's `your_tokens`).
            /// Ignored for every other trigger/timing.
            #[serde(default)]
            who: CombatDamageScope,
            /// The spell filter for a `cast_spell` trigger (Monologue Tax's "a spell", Sram
            /// Senior Edificer's "an Aura, Equipment, or Vehicle spell"). Named distinctly from
            /// `filter` (a [`PermanentFilter`], taken by the sacrifice triggers above). Ignored
            /// for every other trigger/timing.
            #[serde(default)]
            spell_filter: SpellFilter,
            /// Whose cast a `cast_spell` trigger watches — `you` (default), `opponent`
            /// (Monologue Tax, Mangara), or `any_player`. Ignored for every other trigger/timing.
            #[serde(default)]
            caster: CasterScope,
            /// Whose draw a `player_draws` trigger watches — `you` (default), `opponent`
            /// (Faerie Mastermind), or `any_player`. Ignored for every other trigger/timing.
            #[serde(default)]
            drawer: CasterScope,
            /// Restricts a `cast_spell`/`player_draws` trigger to exactly the watched player's
            /// Nth spell/draw that turn (Monologue Tax/Mangara's "their second spell each turn",
            /// Faerie Mastermind's "their second card each turn" — `2`). `None` (the default,
            /// omitted in TOML) fires on every matching cast/draw. Ignored for every other
            /// trigger/timing.
            #[serde(default)]
            nth_each_turn: Option<u8>,
            /// Restricts a `cast_spell` trigger to a spell cast from its controller's hand (CR
            /// 601's default cast zone) — Dirgur Focusmage's "you cast … from your hand". `false`
            /// (the default, omitted in TOML) fires on a cast from any zone (flashback/escape,
            /// the command zone, an impulse-play permission). Ignored for every other
            /// trigger/timing.
            #[serde(default)]
            from_hand: bool,
            /// The attacker-count threshold for a `you_attack_with_creatures`/
            /// `opponent_attacks_you_with_creatures`/`creature_enchanted_by_your_aura_attacks`
            /// trigger (Firemane Commando/Mangara/Tomik's "two or more creatures" — `2`; Killian,
            /// Decisive Mentor's "one or more" — `1`). Ignored for every other trigger/timing.
            #[serde(default)]
            at_least: u8,
            /// Which cast a `spend_mana_to_cast` trigger accepts (Study Hall/Opal Palace's
            /// `commander`, Path of Ancestry's `creature_sharing_type_with_commander`). Ignored for
            /// every other trigger/timing; the field is required only when `timing =
            /// "spend_mana_to_cast"`, defaulting to `commander` otherwise (unread).
            #[serde(default = "default_spend_predicate")]
            spend_predicate: SpendToCastPredicate,
            /// The ability's effect(s), always the array-of-tables `[[abilities.effects]]` form
            /// (even a single-effect ability uses a one-element list). An ordered list runs as one
            /// resolution, sharing the ability's target/`{X}` (Faithless Looting's "draw two cards,
            /// then discard two cards"); a one-element list is just that effect (no Sequence
            /// wrapper).
            #[serde(default)]
            effects: Vec<Effect>,
        }

        let flat = Flat::deserialize(d)?;
        let effect = match flat.effects.as_slice() {
            [] => {
                return Err(de::Error::custom(
                    "an ability needs a non-empty `effects` list; write at least one \
                     [[abilities.effects]] block",
                ));
            }
            [only] => *only, // one-element `effects` is just that effect (no Sequence wrapper).
            _ => Effect::Sequence {
                steps: intern(flat.effects),
            },
        };
        let timing = match flat.timing {
            TimingName::Trigger(tag) => Timing::Triggered(match tag {
                TriggerTag::Etb => Trigger::Etb,
                TriggerTag::TurnedFaceUp => Trigger::TurnedFaceUp,
                TriggerTag::Attacks => Trigger::Attacks,
                TriggerTag::BlocksOrBecomesBlocked => Trigger::BlocksOrBecomesBlocked,
                TriggerTag::Dies => Trigger::Dies,
                TriggerTag::CreatureDies => Trigger::CreatureDies,
                TriggerTag::CreatureYouControlDies => Trigger::CreatureYouControlDies,
                TriggerTag::CreatureDiesIncludingThis => Trigger::CreatureDiesIncludingThis,
                TriggerTag::CreatureYouControlDiesIncludingThis => {
                    Trigger::CreatureYouControlDiesIncludingThis
                }
                TriggerTag::CreatureYouControlDiesNontoken => {
                    Trigger::CreatureYouControlDiesNontoken
                }
                TriggerTag::CreatureYouControlDiesIncludingThisNontoken => {
                    Trigger::CreatureYouControlDiesIncludingThisNontoken
                }
                TriggerTag::CreatureAnOpponentControlsDies => {
                    Trigger::CreatureAnOpponentControlsDies
                }
                TriggerTag::EnchantmentYouControlDies => Trigger::EnchantmentYouControlDies,
                TriggerTag::NonlandPermanentYouControlDiesIncludingThis => {
                    Trigger::NonlandPermanentYouControlDiesIncludingThis
                }
                TriggerTag::Upkeep => Trigger::Upkeep,
                TriggerTag::EachUpkeep => Trigger::EachUpkeep,
                TriggerTag::FirstMainPhase => Trigger::FirstMainPhase,
                TriggerTag::BeginCombat => Trigger::BeginCombat,
                TriggerTag::EndStep => Trigger::EndStep,
                TriggerTag::EachEndStep => Trigger::EachEndStep,
                TriggerTag::EachDrawStep => Trigger::EachDrawStep,
                TriggerTag::EachOtherPlayerUntapStep => Trigger::EachOtherPlayerUntapStep,
                TriggerTag::YouGainLife => Trigger::YouGainLife,
                TriggerTag::OpponentGainsLife => Trigger::OpponentGainsLife,
                TriggerTag::Magecraft => Trigger::Magecraft,
                TriggerTag::PlayerAttacksYourOpponent => Trigger::PlayerAttacksYourOpponent,
                TriggerTag::YouAttackWithCreatures => Trigger::YouAttackWithCreatures {
                    at_least: flat.at_least,
                },
                TriggerTag::OpponentAttacksYouWithCreatures => {
                    Trigger::OpponentAttacksYouWithCreatures {
                        at_least: flat.at_least,
                    }
                }
                TriggerTag::AnotherPlayerAttacksWithCreatures => {
                    Trigger::AnotherPlayerAttacksWithCreatures {
                        at_least: flat.at_least,
                    }
                }
                TriggerTag::EnchantedCreatureAttacks => Trigger::EnchantedCreatureAttacks,
                TriggerTag::EnchantedCreatureDies => Trigger::EnchantedCreatureDies,
                TriggerTag::EnchantedCreatureDealsDamage => Trigger::EnchantedCreatureDealsDamage,
                TriggerTag::AnEnchantedCreatureDies => Trigger::AnEnchantedCreatureDies,
                TriggerTag::CreatureEnchantedByYourAuraAttacks => {
                    Trigger::CreatureEnchantedByYourAuraAttacks {
                        at_least: flat.at_least,
                    }
                }
                TriggerTag::YouSacrifice => Trigger::YouSacrifice {
                    filter: flat.filter,
                },
                TriggerTag::AnyPlayerSacrifices => Trigger::AnyPlayerSacrifices {
                    filter: flat.filter,
                },
                TriggerTag::YouDiscard => Trigger::YouDiscard,
                TriggerTag::DealsCombatDamageToPlayer => {
                    Trigger::DealsCombatDamageToPlayer { who: flat.who }
                }
                TriggerTag::DealsCombatDamageToCreature => Trigger::DealsCombatDamageToCreature,
                TriggerTag::CreatureDealtDamageByThisDies => Trigger::CreatureDealtDamageByThisDies,
                TriggerTag::DealsDamageToOpponent => Trigger::DealsDamageToOpponent,
                TriggerTag::CastSpell => Trigger::CastSpell {
                    filter: flat.spell_filter,
                    caster: flat.caster,
                    nth_each_turn: flat.nth_each_turn,
                    from_hand: flat.from_hand,
                },
                TriggerTag::PlayerDraws => Trigger::PlayerDraws {
                    drawer: flat.drawer,
                    nth_each_turn: flat.nth_each_turn,
                },
                // Reuses `CastSpell`'s `caster` sibling — Unbound Flourishing's ability half is
                // `caster = "you"`.
                TriggerTag::ActivateAbility => Trigger::ActivateAbility {
                    caster: flat.caster,
                },
                TriggerTag::PermanentEnters => Trigger::PermanentEnters {
                    filter: flat.filter,
                    controller: flat.controller,
                },
                TriggerTag::PermanentEntersIncludingThis => Trigger::PermanentEntersIncludingThis {
                    filter: flat.filter,
                    controller: flat.controller,
                },
                TriggerTag::CardsLeaveYourGraveyard => Trigger::CardsLeaveYourGraveyard,
                TriggerTag::CardsExiledFromYourLibraryOrGraveyard => {
                    Trigger::CardsExiledFromYourLibraryOrGraveyard
                }
                TriggerTag::YouCreateToken => Trigger::YouCreateToken,
                TriggerTag::BecomesTargeted => Trigger::BecomesTargeted,
                TriggerTag::SpellTargetsThisOnly => Trigger::SpellTargetsThisOnly {
                    filter: flat.spell_filter,
                },
                TriggerTag::YouCastThis => Trigger::YouCastThis,
                TriggerTag::ThisAuraLeaves => Trigger::ThisAuraLeaves,
                TriggerTag::ThisPermanentLeavesBattlefield => {
                    Trigger::ThisPermanentLeavesBattlefield
                }
                TriggerTag::ZeroBasePowerCreaturesYouControlDealCombatDamage => {
                    Trigger::ZeroBasePowerCreaturesYouControlDealCombatDamage
                }
                TriggerTag::SpendManaToCast => Trigger::SpendManaToCast {
                    predicate: flat.spend_predicate,
                },
                TriggerTag::YouLoseLifeFirstTimeEachTurn => Trigger::YouLoseLifeFirstTimeEachTurn,
                TriggerTag::Cycled => Trigger::Cycled,
            }),
            TimingName::Special(SpecialTiming::Spell) => Timing::Spell,
            TimingName::Special(SpecialTiming::Static) => Timing::Static,
            TimingName::Special(SpecialTiming::Activated) => Timing::Activated(ActivationCost {
                taps_self: flat.taps_self,
                mana: flat.activation_cost,
                sacrifice: flat.sacrifice,
                pay_life: flat.pay_life,
                remove_counters: flat.remove_counters,
                remove_counters_kind: flat.remove_counters_kind,
                self_damage: flat.self_damage,
                loyalty: flat.loyalty,
                once_each_turn: flat.once_each_turn,
                sorcery_speed: flat.sorcery_speed,
                return_self: flat.return_self,
                mill_self: flat.mill_self,
                discard_cost: flat.discard_cost,
                exile_self: flat.exile_self,
                graveyard_exile_target_count: flat.graveyard_exile_target_count,
            }),
        };
        Ok(Ability {
            timing,
            effect,
            optional: flat.optional,
            cost: flat.cost,
            condition: flat.condition,
            once_each_turn: flat.once_each_turn,
            min_level: flat.min_level,
        })
    }
}
