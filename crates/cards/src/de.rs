//! Deserialization of card definitions from the TOML card DSL (the `card-dsl` feature).
//!
//! Most types deserialize via derives on their definitions in `lib.rs`; this module holds
//! the handful whose TOML spelling differs structurally from their Rust shape (a flat
//! `[cost]` table of color names, the `instant`/`sorcery` split of [`CardKind::Spell`],
//! the flat ability table that folds into [`Timing::Activated`]), plus the load helpers
//! for the remaining `'static` payloads and the `Arc`-backed slice deserializers used by
//! `CardDef` and runtime-rebuilt effect lists.
//! See [`Effect`]'s doc comment for the invariant these helpers exist to satisfy.
//!
//! CR citations appear on individual fields where the DSL encodes a rules concept
//! (e.g. commander identity mana, target counts); see `docs/CR_INDEX.md`.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use serde::Deserialize;
use serde::de::{self, Deserializer, IntoDeserializer, Visitor};

use crate::{
    Ability, ActivationCost, AdditionalCost, Amount, AmountZone, ArithOp, CardDef, CardFilter,
    CardKind, Color, ColorFilter, CombatDamageScope, Condition, Cost, CounterAxis, CounterKind,
    Effect, FilterController, GrantedAbility, LandProduces, Mana, ManaPool, Parity,
    PermanentFilter, ProtectionScope, ReanimateBecomes, SacrificeAdditionalCost,
    SacrificeAdditionalCostCount, SacrificeCost, SpendToCastPredicate, TargetCount, Timing,
    TokenFilter, Trigger, TypeSet,
    toml_surface::{AbilityToml, CardToml, CostToml, KindToml},
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
    TOKEN_DEFS.get().and_then(|m| m.get(id).cloned())
}

// ── Interning + serde defaults (referenced by the derives in lib.rs) ────────────────

/// Leak an owned `Vec<T>` into the `&'static [T]` a `Copy` [`CardDef`]/[`Effect`] field needs.
/// The one place that actually calls `Box::leak` on a plain vec-to-slice; every other site in
/// this module (and [`static_slice`] below) should go through this rather than leaking directly.
pub fn intern<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

pub fn static_slice<'de, D, T>(d: D) -> Result<&'static [T], D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + 'static,
{
    Ok(intern(Vec::<T>::deserialize(d)?))
}

/// Deserialize an owned list into shared `Arc<[T]>` storage — used by effect payloads that may
/// be rebuilt at runtime without leaking.
pub fn arc_slice<'de, D, T>(d: D) -> Result<Arc<[T]>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Arc::from(Vec::<T>::deserialize(d)?))
}

/// Leak one owned `Effect` into the `&'static Effect` a nested `Copy` field needs (a single-value
/// sibling of [`static_slice`] — `Effect` can't hold itself by value, so
/// [`Effect::Misc(MiscEffect::ScheduleAtNextUpkeep)`]'s `then` is the one-element leaked case instead).
pub fn static_effect<'de, D>(d: D) -> Result<&'static Effect, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(&*Box::leak(Box::new(Effect::deserialize(d)?)))
}

/// Leak one owned [`Amount`] into the `&'static Amount` a `Copy` field needs (the [`Amount`]
/// sibling of [`static_effect`]). [`Condition::Compare`](crate::Condition::Compare)'s operands need
/// it because [`Amount::IfCondition`](crate::Amount::IfCondition) already holds a [`Condition`] by
/// value, so a `Condition` holding an `Amount` by value would be an infinitely sized cycle.
pub fn static_amount<'de, D>(d: D) -> Result<&'static Amount, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(&*Box::leak(Box::new(Amount::deserialize(d)?)))
}

/// Leak one owned [`Cost`] into the `&'static Cost` a `Copy` field needs (the `Cost` sibling of
/// [`static_effect`] — [`Suspend::cost`] can't hold a `Cost` by value without bloating a `Copy`
/// [`CardDef`], since `Cost` embeds an [`AdditionalCost`]).
pub fn leaked_cost<'de, D>(d: D) -> Result<&'static Cost, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(&*Box::leak(Box::new(Cost::deserialize(d)?)))
}

/// `deserialize_with` for [`Effect::Static(StaticEffect::GrantToAttached)`]'s `granted_ability`: leak the one owned
/// [`GrantedAbility`] the sub-table spells into the `&'static` a `Copy` [`Effect`] needs. Only
/// called when the key is present (a `#[serde(default)]` absent key stays `None`), so it always
/// yields `Some`.
pub fn opt_static_granted_ability<'de, D>(d: D) -> Result<Option<&'static GrantedAbility>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(&*Box::leak(Box::new(GrantedAbility::deserialize(d)?))))
}

/// The one [`Trigger`] flavor [`GrantedAbility`]'s `trigger` can spell in TOML today (Power
/// Fist's "Whenever this creature deals combat damage to a player, …"), externally tagged like a
/// plain Rust enum (`trigger = { deals_combat_damage_to_player = { who = "this" } }`) — unlike
/// [`Timing`]'s `TriggerTag`, which pairs a `timing` tag with sibling fields on the *ability's own
/// table* because an [`Ability`] already has a flat `timing` column to piggyback on; a
/// [`GrantedAbility`] has no such column, so its `trigger` nests instead.
/// ponytail: only the flavors below are wired — extend this tag (mirroring `TriggerTag`) the
/// moment a granted-trigger card needs a different one.
#[derive(Deserialize)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum GrantedTriggerTag {
    DealsCombatDamageToPlayer {
        #[serde(default)]
        who: CombatDamageScope,
    },
    /// Farmstead's "Enchanted land has \"At the beginning of your upkeep, …\"" — fieldless, so
    /// it is spelled `trigger = { upkeep = {} }`. "Your" is the *host's* controller, which is
    /// who a granted ability belongs to.
    Upkeep {},
}

/// `deserialize_with` for [`GrantedAbility`]'s `trigger`. Only called when the key is present (a
/// `#[serde(default)]` absent key stays `None`), so it always yields `Some`.
pub fn opt_granted_trigger<'de, D>(d: D) -> Result<Option<Trigger>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(match GrantedTriggerTag::deserialize(d)? {
        GrantedTriggerTag::DealsCombatDamageToPlayer { who } => {
            Trigger::DealsCombatDamageToPlayer { who }
        }
        GrantedTriggerTag::Upkeep {} => Trigger::Upkeep,
    }))
}

/// `deserialize_with` for [`Effect::Zone(ZoneEffect::ReanimateToBattlefield)`]'s `becomes`: leak the one owned
/// [`ReanimateBecomes`] the sub-table spells into the `&'static` a `Copy` [`Effect`] needs. Only
/// called when the key is present (an absent `#[serde(default)]` key stays `None`).
pub fn opt_static_reanimate_becomes<'de, D>(
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
pub fn intern_strs(strings: Vec<String>) -> &'static [&'static str] {
    let leaked: Vec<&'static str> = strings
        .into_iter()
        .map(|s| &*Box::leak(s.into_boxed_str()))
        .collect();
    intern(leaked)
}

/// Convert owned strings into shared `Arc<[&'static str]>` storage for `CardDef` fields while
/// still leaking the individual string data once at load.
pub fn arc_strs(strings: Vec<String>) -> Arc<[&'static str]> {
    Arc::from(
        strings
            .into_iter()
            .map(|s| &*Box::leak(s.into_boxed_str()))
            .collect::<Vec<_>>(),
    )
}

/// `deserialize_with` for a `&'static [&'static str]` field (land subtypes, and the card-filter /
/// [`Condition`] arms that filter or gate on them) — TOML spells it as a plain array of strings.
pub fn static_str_slice<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<&'static [&'static str], D::Error> {
    Ok(intern_strs(Vec::<String>::deserialize(d)?))
}

/// serde default for a `CounterReplacement`'s `times` (the multiplicative identity).
pub fn one() -> i32 {
    1
}

/// serde default for `modal_choose`: a modal spell chooses one mode unless it says more.
pub fn one_u8() -> u8 {
    1
}

/// `deserialize_with` for [`Effect::Dig(DigEffect::SearchLibrary)`]'s `count`: either a fixed integer (the
/// common "up to N") or the `"any"` marker (CR 701.19's "any number of" — Trench Gorger),
/// untagged so TOML's own scalar type picks the arm, mirroring `AdditionalCost::pay_life`'s
/// `PayLife` marker-or-fixed shape. `"any"` becomes `u8::MAX` — no real library holds anywhere
/// close to that many cards, so the search re-pauses until the searcher fails to find or the
/// matches run out, same as a genuinely unbounded count.
pub fn count_or_any<'de, D: Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum CountOrAny {
        Marker(String),
        Fixed(u8),
    }
    match CountOrAny::deserialize(d)? {
        CountOrAny::Fixed(n) => Ok(n),
        CountOrAny::Marker(marker) if marker == "any" => Ok(u8::MAX),
        CountOrAny::Marker(other) => Err(serde::de::Error::custom(format!(
            "unknown SearchLibrary count marker {other:?}, expected \"any\""
        ))),
    }
}

/// serde default for [`Effect::Dig(DigEffect::LookAtTop)`]'s `up_to`: the printed "put *that card*" ⇒ one.
pub fn one_u32() -> u32 {
    1
}

/// serde default for [`Effect::Dig(DigEffect::LookAtTop)`]'s `filter`: a filterless look sees any card.
pub fn any_card_filter() -> CardFilter {
    CardFilter::AnyCard
}

/// serde default for an edict's `filter`: a creature is the common sacrifice.
pub fn creature_edict() -> PermanentFilter {
    PermanentFilter::of(TypeSet::CREATURE)
}

/// A token profile reference on `create_token` (and siblings): a Scryfall oracle id string
/// (`token = "37c4adc8-…"`) resolved against the registry installed by [`install_token_defs`].
/// Token characteristics live in `cards/data/tokens/*.toml`; after resolve the effect embeds a
/// full [`CardDef`] so mint paths stay pool-agnostic.
pub fn token_profile<'de, D: Deserializer<'de>>(d: D) -> Result<CardDef, D::Error> {
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
/// A `deserialize_with` on the [`Effect::Mana(ManaEffect::Add)`] `mana` field rather than a `Deserialize`
/// on [`ManaPool`] itself — the pool is runtime game state (events, replays), and its
/// canonical serde shape shouldn't be a card-DSL spelling.
pub fn mana_batch<'de, D: Deserializer<'de>>(d: D) -> Result<ManaPool, D::Error> {
    let mut pool = ManaPool::default();
    for symbol in Vec::<Mana>::deserialize(d)? {
        pool.add(symbol, 1);
    }
    Ok(pool)
}

/// The default `repeat`/`count` for an amount-bearing field that omits one — a single copy.
pub fn one_amount() -> Amount {
    Amount::Fixed(1)
}

/// The default target of a `must_attack_target` effect — Basandra, Battle Seraph's unqualified
/// "target creature", which every card printing this clause narrows rather than replaces.
pub(crate) fn target_creature() -> crate::TargetSpec {
    crate::TargetSpec::Creature
}

/// The default for an amount-bearing field that omits one and means "none" rather than "one" —
/// `create_token`'s `enters_with` (no counters unless a card says otherwise).
pub fn zero_amount() -> Amount {
    Amount::Fixed(0)
}

/// The default `spend_predicate` for an ability that isn't a `spend_mana_to_cast` trigger (the
/// field is unread there) — an arbitrary variant so the derive has a default.
pub fn default_spend_predicate() -> SpendToCastPredicate {
    SpendToCastPredicate::Commander
}

// ── Types whose TOML spelling differs structurally from their Rust shape ────────────

impl<'de> Deserialize<'de> for CardDef {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(CardToml::deserialize(d)?.into())
    }
}

/// A `[cost]` table spells each color by name (`white = 1`) rather than as the
/// [`Cost::colored`] WUBRG array; every field is optional.
impl<'de> Deserialize<'de> for Cost {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let cost = CostToml::deserialize(d)?;
        cost.validate_hybrid()?;
        Ok(cost.into())
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
/// table shape, the per-payment cost. `multikicker = { white = 1 }` spells Multikicker (CR
/// 702.33c) — same table shape, the per-payment cost (a kicker cost payable any number of
/// times). `reveal_creature_from_hand = true` spells "reveal a creature card from your hand"
/// (CR 601.2g — Disaster Radius).
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
            /// "Reveal a creature card from your hand" (CR 601.2g — Disaster Radius) —
            /// `reveal_creature_from_hand = true`.
            reveal_creature_from_hand: bool,
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
            /// `[cost.additional.multikicker]` — Multikicker (CR 702.33c), the same table shape
            /// as `[cost]`.
            multikicker: Option<Cost>,
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
            reveal_creature_from_hand: raw.reveal_creature_from_hand,
            pay_life_x,
            pay_life,
            sacrifice,
            kicker: raw.kicker.map(|c| &*Box::leak(Box::new(c))),
            buyback: raw.buyback.map(|c| &*Box::leak(Box::new(c))),
            strive: raw.strive.map(|c| &*Box::leak(Box::new(c))),
            replicate: raw.replicate.map(|c| &*Box::leak(Box::new(c))),
            multikicker: raw.multikicker.map(|c| &*Box::leak(Box::new(c))),
        })
    }
}

/// A `[kind]` table spells instants and sorceries as their own `type` tags
/// (`type = "instant"`) rather than as [`CardKind::Spell`]'s `speed` field.
impl<'de> Deserialize<'de> for CardKind {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(KindToml::deserialize(d)?.into())
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
/// value (`"x"`, `"per_creature_you_control"`, `"source_power"`,
/// `"source_toughness"`, `"source_mana_value"`, `"target_power"`, `"target_mana_value"`, `"per_counter_on_source"`, `"your_life_total"`, `"life_gained_this_turn"`,
/// `"spells_cast_this_turn"`, `"damage_taken_this_turn"`, `"untapped_lands_at_turn_start"`,
/// `"commander_casts_from_command_zone"`, `"creatures_died_this_turn"`,
/// `"creatures_died_this_turn_any_controller"`,
/// `"nontoken_creatures_entered_this_turn"`,
/// `"sacrificed_creature_power"`, `"commander_color_count"`, `"total_power_you_control"`,
/// `"greatest_power_among_creatures_you_control"`,
/// `"triggering_spell_mana_value"`, `"spell_sacrifice_count"`, `"spell_sacrificed_mana_value"`,
/// `"spell_multikicker_count"`,
/// `"revealed_creature_mana_value"`,
/// `"permanents_died_this_turn"`,
/// `"mana_paid_this_way"`, `"past_votes"`, `"present_votes"`, `"total_mana_value_milled_this_way"`,
/// `"exiled_card_mana_value_this_way"`, `"combat_damage_dealt"`, `"spells_cast_before_this_this_turn"`)
/// or a table for a filtered count
/// (`{ per_permanent = <filter>, zone = "graveyard" }`), a per-kind counter count
/// (`{ per_counter_of_kind = "charge" }`), a conditional amount
/// (`{ condition = <Condition>, then = <Amount>, else = <Amount> }` — both arms default to 0, and
/// `condition = { type = "spell_was_kicked" }` is CR 702.33d's kicked branch), an arithmetic
/// amount (`{ left = <Amount>, op = "multiply", right = <Amount> }` — see [`ArithOp`]; both sides
/// are full amounts, so these nest), a
/// "destroyed this way" count (`{ permanents_destroyed_this_way = <filter> }`, filter optional
/// — defaults to matching every destroyed permanent), or a count of Auras attached to the
/// effect's source (`{ auras_attached_to_source = {} }`).
impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct AmountVisitor;

        const KEYWORDS: &[&str] = AMOUNT_KEYWORDS;

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
                    "per_creature_you_control" => Amount::PerCreatureYouControl,
                    "per_creature_on_battlefield" => Amount::PerCreatureOnBattlefield,
                    "source_power" => Amount::SourcePower,
                    "source_toughness" => Amount::SourceToughness,
                    "source_mana_value" => Amount::SourceManaValue,
                    "target_power" => Amount::TargetPower,
                    "target_toughness" => Amount::TargetToughness,
                    "target_mana_value" => Amount::TargetManaValue,
                    "per_counter_on_source" => Amount::PerCounterOnSource,
                    "opponents_poison_counters" => Amount::OpponentsPoisonCounters,
                    "controllers_poison_counters" => Amount::ControllersPoisonCounters,
                    "your_life_total" => Amount::YourLifeTotal,
                    "life_gained_this_turn" => Amount::LifeGainedThisTurn,
                    "spells_cast_this_turn" => Amount::SpellsCastThisTurn,
                    "damage_taken_this_turn" => Amount::DamageTakenThisTurn,
                    "untapped_lands_at_turn_start" => Amount::UntappedLandsAtTurnStart,
                    "cards_in_target_player_hand" => Amount::CardsInTargetPlayerHand,
                    "cards_in_your_hand" => Amount::CardsInYourHand,
                    "commander_casts_from_command_zone" => Amount::CommanderCastsFromCommandZone,
                    "creatures_died_this_turn" => Amount::CreaturesDiedThisTurn,
                    "creatures_died_this_turn_any_controller" => {
                        Amount::CreaturesDiedThisTurnAnyController
                    }
                    "nontoken_creatures_entered_this_turn" => {
                        Amount::NontokenCreaturesEnteredThisTurn
                    }
                    "sacrificed_creature_power" => Amount::SacrificedCreaturePower,
                    "sacrificed_creature_toughness" => Amount::SacrificedCreatureToughness,
                    "dying_enchanted_creature_toughness" => Amount::DyingEnchantedCreatureToughness,
                    "commander_color_count" => Amount::CommanderColorCount,
                    "total_power_you_control" => Amount::TotalPowerYouControl,
                    "greatest_power_among_creatures_you_control" => {
                        Amount::GreatestPowerAmongCreaturesYouControl
                    }
                    "permanents_you_own_opponents_control" => {
                        Amount::PermanentsYouOwnOpponentsControl
                    }
                    "triggering_spell_mana_value" => Amount::TriggeringSpellManaValue,
                    "triggering_spell_mana_spent" => Amount::TriggeringSpellManaSpent,
                    "spell_sacrifice_count" => Amount::SpellSacrificeCount,
                    "spell_sacrificed_mana_value" => Amount::SpellSacrificedManaValue,
                    "spell_multikicker_count" => Amount::SpellMultikickerCount,
                    "revealed_creature_mana_value" => Amount::RevealedCreatureManaValue,
                    "permanents_died_this_turn" => Amount::PermanentsDiedThisTurn,
                    "nonland_cards_exiled_this_way" => Amount::NonlandCardsExiledThisWay,
                    "cards_exiled_by_search_this_way" => Amount::CardsExiledBySearchThisWay,
                    "mana_paid_this_way" => Amount::ManaPaidThisWay,
                    "past_votes" => Amount::PastVotes,
                    "present_votes" => Amount::PresentVotes,
                    "total_mana_value_milled_this_way" => Amount::TotalManaValueMilledThisWay,
                    "exiled_card_mana_value_this_way" => Amount::ExiledCardManaValueThisWay,
                    "returned_nonland_card_mana_value" => Amount::ReturnedNonlandCardManaValue,
                    "auras_you_controlled_attached_to_dying_creature" => {
                        Amount::AurasYouControlledAttachedToDyingCreature
                    }
                    "greatest_instant_or_sorcery_mana_value_cast_this_turn" => {
                        Amount::GreatestInstantOrSorceryManaValueCastThisTurn
                    }
                    "instants_and_sorceries_cast_this_turn" => {
                        Amount::InstantsAndSorceriesCastThisTurn
                    }
                    "instant_or_sorcery_cards_in_your_graveyard" => {
                        Amount::InstantOrSorceryCardsInYourGraveyard
                    }
                    "combat_damage_dealt" => Amount::CombatDamageDealt,
                    "triggering_damage_dealt" => Amount::TriggeringDamageDealt,
                    "spells_cast_before_this_this_turn" => Amount::SpellsCastBeforeThisThisTurn,
                    "cards_discarded_this_way" => Amount::CardsDiscardedThisWay,
                    "creatures_sacrificed_this_way" => Amount::CreaturesSacrificedThisWay,
                    "spell_first_target_mana_value" => Amount::SpellFirstTargetManaValue,
                    "counters_removed_this_way" => Amount::CountersRemovedThisWay,
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
                    /// `else` defaults to 0 — a conditional cost reduction that doesn't apply
                    /// reduces nothing (Mortality Spear).
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
                    /// `{ left = 2, op = "multiply", right = "per_creature_on_battlefield" }` —
                    /// [`Amount::Combine`] (Congregate's "2 life for each creature on the
                    /// battlefield"). All three keys go together; both sides are full amounts, so
                    /// combines nest.
                    #[serde(default)]
                    left: Option<Amount>,
                    #[serde(default)]
                    op: Option<ArithOp>,
                    #[serde(default)]
                    right: Option<Amount>,
                }
                let t = Table::deserialize(de::value::MapAccessDeserializer::new(map))?;
                // `left`/`op`/`right` wrap other amounts rather than naming a count of their own,
                // so they are answered here instead of joining the exactly-one-of table below.
                if t.left.is_some() || t.op.is_some() || t.right.is_some() {
                    let (Some(left), Some(op), Some(right)) = (t.left, t.op, t.right) else {
                        return Err(de::Error::custom(
                            "an arithmetic amount needs all three of `left`, `op`, and `right`",
                        ));
                    };
                    return Ok(Amount::Combine {
                        left: &*Box::leak(Box::new(left)),
                        op,
                        right: &*Box::leak(Box::new(right)),
                    });
                }
                match (
                    t.per_permanent,
                    t.per_counter_of_kind,
                    t.condition,
                    t.permanents_destroyed_this_way,
                    t.auras_attached_to_source,
                ) {
                    (Some(filter), None, None, None, None) => Ok(Amount::PerPermanentMatching {
                        filter,
                        zone: t.zone,
                    }),
                    (None, Some(kind), None, None, None) => {
                        Ok(Amount::PerCounterOfKindOnSource { kind })
                    }
                    (None, None, Some(condition), None, None) => Ok(Amount::IfCondition {
                        condition,
                        then: &*Box::leak(Box::new(t.then.unwrap_or(Amount::Fixed(0)))),
                        else_: &*Box::leak(Box::new(t.otherwise.unwrap_or(Amount::Fixed(0)))),
                    }),
                    (None, None, None, Some(filter), None) => {
                        Ok(Amount::PermanentsDestroyedThisWay { filter })
                    }
                    (None, None, None, None, Some(_)) => Ok(Amount::AurasAttachedToSource),
                    _ => Err(de::Error::custom(
                        "an amount table needs exactly one of `per_permanent`, `per_counter_of_kind`, \
                         `condition` (with `then`/`else`), `permanents_destroyed_this_way`, \
                         `auras_attached_to_source`, or `left`+`op`+`right`",
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
///   target count (Twinflame). `total_mv_max` (an [`Amount`], default `None`) is a set-level cap
///   on the chosen targets' *summed* mana value (see [`TargetCount::total_mv_max`]'s own doc):
///   `{ min = 0, max = 255, total_mv_max = "x" }` is "any number of target artifacts and/or
///   enchantments with total mana value X or less" (Rampaging Yao Guai).
///   target count (Twinflame). `multikicker_scaled` (default `false`) is Multikicker's own
///   sibling (see [`TargetCount::multikicker_scaled`]'s own doc): `{ multikicker_scaled = true }`
///   is "one target, then one more for each time this spell was kicked" (Comet Storm) — unlike
///   the others, "exactly `1 + N`," not "exactly N." `kicked_scaled` (default `false`) is Kicker's
///   own sibling (see [`TargetCount::kicked_scaled`]'s own doc): `{ min = 1, max = 1,
///   kicked_scaled = true }` is a whole second target clause present only if kicked (Orim's
///   Thunder), forced to `(0, 0)` otherwise (CR 702.33g) — unlike the others above, not a
///   substituted count. `main_phase_scaled` (default `false`) is its cast-timing sibling (see
///   [`TargetCount::main_phase_scaled`]'s own doc): `{ min = 1, max = 2, main_phase_scaled = true
///   }` is "one mandatory target, plus one more only if cast during your main phase" (Return to
///   Dust) — `max` caps down to `min` outside the caster's main phase; `min` is untouched.
///
/// ponytail: no pool card needs a *fixed* range yet (Aether Gale is exactly six); the table form
/// is here so "up to N"/"one or two" cards don't need a new deserializer when they land.
/// The `{ min, max, …scaled }` table form of a [`TargetCount`] (the other form is a bare
/// integer). Lives at module level so the JSON Schema can point at the same fields the visitor
/// below reads.
#[derive(Deserialize)]
#[cfg_attr(
    feature = "card-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(deny_unknown_fields)]
pub(crate) struct TargetCountToml {
    #[serde(default)]
    pub(crate) min: u8,
    #[serde(default)]
    pub(crate) max: u8,
    #[serde(default)]
    pub(crate) x_scaled: bool,
    #[serde(default)]
    pub(crate) sacrifice_scaled: bool,
    #[serde(default)]
    pub(crate) strive_scaled: bool,
    #[serde(default)]
    pub(crate) total_mv_max: Option<Amount>,
    #[serde(default)]
    pub(crate) multikicker_scaled: bool,
    #[serde(default)]
    pub(crate) kicked_scaled: bool,
    #[serde(default)]
    pub(crate) main_phase_scaled: bool,
}

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
                    total_mv_max: None,
                    multikicker_scaled: false,
                    kicked_scaled: false,
                    main_phase_scaled: false,
                })
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<TargetCount, E> {
                let n = u64::try_from(n).map_err(|_| {
                    E::invalid_value(de::Unexpected::Signed(n), &"a non-negative target count")
                })?;
                self.visit_u64(n)
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<TargetCount, A::Error> {
                let t = TargetCountToml::deserialize(de::value::MapAccessDeserializer::new(map))?;
                if t.min > t.max {
                    return Err(de::Error::custom("target count min exceeds max"));
                }
                Ok(TargetCount {
                    min: t.min,
                    max: t.max,
                    x_scaled: t.x_scaled,
                    sacrifice_scaled: t.sacrifice_scaled,
                    strive_scaled: t.strive_scaled,
                    total_mv_max: t.total_mv_max,
                    multikicker_scaled: t.multikicker_scaled,
                    kicked_scaled: t.kicked_scaled,
                    main_phase_scaled: t.main_phase_scaled,
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
        "battle" => TypeSet::BATTLE,
        "land" => TypeSet::LAND,
        "nonland" => TypeSet::NONLAND,
        "artifact_or_enchantment" => TypeSet::ARTIFACT.union(TypeSet::ENCHANTMENT),
        "creature_or_planeswalker" => TypeSet::CREATURE.union(TypeSet::PLANESWALKER),
        "artifact_or_creature" => TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        _ => return None,
    })
}

pub const TYPE_NAMES: &[&str] = &[
    "creature",
    "artifact",
    "enchantment",
    "planeswalker",
    "battle",
    "land",
    "nonland",
    "artifact_or_enchantment",
    "creature_or_planeswalker",
    "artifact_or_creature",
];

/// Every bare-string [`Amount`] keyword the visitor below accepts — also the candidate list
/// the generated JSON Schema offers, so the two can't drift.
pub const AMOUNT_KEYWORDS: &[&str] = &[
    "x",
    "per_creature_you_control",
    "per_creature_on_battlefield",
    "source_power",
    "source_toughness",
    "source_mana_value",
    "target_power",
    "target_toughness",
    "target_mana_value",
    "per_counter_on_source",
    "opponents_poison_counters",
    "controllers_poison_counters",
    "your_life_total",
    "life_gained_this_turn",
    "spells_cast_this_turn",
    "damage_taken_this_turn",
    "untapped_lands_at_turn_start",
    "cards_in_target_player_hand",
    "cards_in_your_hand",
    "commander_casts_from_command_zone",
    "creatures_died_this_turn",
    "creatures_died_this_turn_any_controller",
    "nontoken_creatures_entered_this_turn",
    "sacrificed_creature_power",
    "sacrificed_creature_toughness",
    "dying_enchanted_creature_toughness",
    "commander_color_count",
    "total_power_you_control",
    "greatest_power_among_creatures_you_control",
    "permanents_you_own_opponents_control",
    "triggering_spell_mana_value",
    "triggering_spell_mana_spent",
    "spell_sacrifice_count",
    "spell_sacrificed_mana_value",
    "spell_multikicker_count",
    "revealed_creature_mana_value",
    "permanents_died_this_turn",
    "nonland_cards_exiled_this_way",
    "cards_exiled_by_search_this_way",
    "mana_paid_this_way",
    "past_votes",
    "present_votes",
    "total_mana_value_milled_this_way",
    "exiled_card_mana_value_this_way",
    "returned_nonland_card_mana_value",
    "auras_you_controlled_attached_to_dying_creature",
    "greatest_instant_or_sorcery_mana_value_cast_this_turn",
    "instants_and_sorceries_cast_this_turn",
    "instant_or_sorcery_cards_in_your_graveyard",
    "combat_damage_dealt",
    "triggering_damage_dealt",
    "spells_cast_before_this_this_turn",
    "cards_discarded_this_way",
    "creatures_sacrificed_this_way",
    "spell_first_target_mana_value",
    "counters_removed_this_way",
];

pub const PERMANENT_FILTER_SHORTHANDS: &[&str] = &[
    "shares_type_with_dying_permanent",
    "creatures",
    "creature",
    "battles",
    "battle",
    "nonland_permanents",
    "nonland",
    "artifact",
    "enchantment",
    "planeswalker",
    "land",
    "artifact_or_enchantment",
    "creature_or_planeswalker",
    "artifact_or_creature",
];

pub const SACRIFICE_COST_SHORTHANDS: &[&str] = &["none", "this", "creature"];

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
/// `enchanted_by_you`, `mv_max`, `mv_min`, `mv_eq_x`, `mv_max_x`, `power_max`, `power_min`, `power_parity`,
/// `noncreature`, `exclude`, `color`, `not_color`, `modified`, `attacking`, `not_attacking`, `attacking_you`,
/// `blocking`, `attacking_or_blocking`, `tapped_or_blocking`, `unblocked`, `power_less_than_source`,
/// `toughness_less_than_source_power`, `entered_this_turn`,
/// `has_mana_ability`,
/// `controlled_since_turn_start`, `did_not_attack_this_turn`,
/// `nonbasic`, `basic`, `nonlegendary`, `legendary`, `nonlair`, `exclude_subtypes`,
/// `without_flying`, `without_keyword`, `with_flying`, `with_counter`). `noncreature` is sugar for `exclude = "creature"`;
/// `not_color` is sugar for `color`'s negated-color arm — both fold into the same
/// [`PermanentFilter`] fields as their general spelling (see below).
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
                    "battles" | "battle" => TypeSet::BATTLE,
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
                    /// "Lands with mana abilities they control" (Power Sink).
                    #[serde(default)]
                    has_mana_ability: bool,
                    #[serde(default)]
                    power_max: Option<u8>,
                    #[serde(default)]
                    power_min: Option<u8>,
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
                    /// Arcades Sabboth's "as long as it's not attacking" — `attacking`'s negation.
                    #[serde(default)]
                    not_attacking: bool,
                    #[serde(default)]
                    attacking_you: bool,
                    #[serde(default)]
                    blocking: bool,
                    #[serde(default)]
                    attacking_or_blocking: bool,
                    #[serde(default)]
                    tapped_or_blocking: bool,
                    #[serde(default)]
                    unblocked: bool,
                    #[serde(default)]
                    power_less_than_source: bool,
                    #[serde(default)]
                    toughness_less_than_source_power: bool,
                    #[serde(default)]
                    entered_this_turn: bool,
                    #[serde(default)]
                    controlled_since_turn_start: bool,
                    #[serde(default)]
                    did_not_attack_this_turn: bool,
                    #[serde(default)]
                    nonbasic: bool,
                    #[serde(default)]
                    basic: bool,
                    /// Printed-name restriction (Leitmotif Composer's "creatures named Leitmotif
                    /// Composer").
                    #[serde(default)]
                    name: Option<String>,
                    #[serde(default)]
                    nonlegendary: bool,
                    /// Karakas' "target legendary creature" — `nonlegendary`'s positive twin.
                    #[serde(default)]
                    legendary: bool,
                    #[serde(default)]
                    nonlair: bool,
                    #[serde(default)]
                    without_flying: bool,
                    /// Island Sanctuary's second keyword exclusion — `{ landwalk = "island" }`,
                    /// the same spelling a printed keyword takes.
                    #[serde(default)]
                    without_keyword: Option<crate::Keyword>,
                    #[serde(default)]
                    with_flying: bool,
                    /// Martyr's Bond's dynamic "shares a card type with it" edict gate — see the
                    /// bare-string shorthand of the same name above.
                    #[serde(default)]
                    shares_type_with_dying_permanent: bool,
                    #[serde(default)]
                    with_counter: Option<CounterAxis>,
                    /// Ao, the Dawn Sky mode 2: creature OR Vehicle subtype (not a card type).
                    #[serde(default)]
                    creature_or_vehicle: bool,
                    /// Snow permanents (CR 205.4g).
                    #[serde(default)]
                    snow: bool,
                    /// Subtype exclusion (Keldon Warlord's "non-Wall creatures you control").
                    #[serde(default)]
                    exclude_subtypes: Vec<String>,
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
                    has_mana_ability: t.has_mana_ability,
                    power_max: t.power_max,
                    power_min: t.power_min,
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
                    not_attacking: t.not_attacking,
                    attacking_you: t.attacking_you,
                    blocking: t.blocking,
                    attacking_or_blocking: t.attacking_or_blocking,
                    tapped_or_blocking: t.tapped_or_blocking,
                    unblocked: t.unblocked,
                    power_less_than_source: t.power_less_than_source,
                    toughness_less_than_source_power: t.toughness_less_than_source_power,
                    entered_this_turn: t.entered_this_turn,
                    controlled_since_turn_start: t.controlled_since_turn_start,
                    did_not_attack_this_turn: t.did_not_attack_this_turn,
                    nonbasic: t.nonbasic,
                    basic: t.basic,
                    name: t.name.map(|s| &*Box::leak(s.into_boxed_str())),
                    nonlegendary: t.nonlegendary,
                    legendary: t.legendary,
                    nonlair: t.nonlair,
                    without_flying: t.without_flying,
                    without_keyword: t.without_keyword,
                    with_flying: t.with_flying,
                    shares_type_with_dying_permanent: t.shares_type_with_dying_permanent,
                    with_counter: t.with_counter,
                    creature_or_vehicle: t.creature_or_vehicle,
                    snow: t.snow,
                    exclude_subtypes: intern_strs(t.exclude_subtypes),
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
                    other => Err(E::unknown_variant(other, SACRIFICE_COST_SHORTHANDS)),
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
/// [`SpellFilter`]/[`WatchedPlayer`]/`nth_each_turn`, a fifth ([`PlayerDraws`](TriggerTag::PlayerDraws))
/// carries a [`WatchedPlayer`]/`nth_each_turn` (the draw-side twin of `CastSpell`, no filter), and a
/// sixth and seventh ([`PermanentEnters`](TriggerTag::PermanentEnters)/
/// [`PermanentEntersIncludingThis`](TriggerTag::PermanentEntersIncludingThis)) carry a
/// [`PermanentFilter`]/[`WatchedPlayer`], none of which can come from a bare `timing = "…"`
/// string —
/// [`Ability::deserialize`] pairs the tag with sibling fields (`filter`, `who`,
/// `spell_filter`/`caster`/`drawer`/`nth_each_turn`, `controller`) to build those by hand. An
/// eighth pair ([`YouAttackWithCreatures`](TriggerTag::YouAttackWithCreatures)/
/// [`OpponentAttacksYouWithCreatures`](TriggerTag::OpponentAttacksYouWithCreatures)) carries a
/// sibling `at_least` count the same way, and
/// [`CreatureEnchantedByYourAuraAttacks`](TriggerTag::CreatureEnchantedByYourAuraAttacks) and
/// [`AnotherPlayerAttacksWithCreatures`](TriggerTag::AnotherPlayerAttacksWithCreatures) reuse
/// that same `at_least` sibling. A ninth ([`SpellTargetsThisOnly`](TriggerTag::SpellTargetsThisOnly),
/// `timing = "spell_targets_this"`) reuses `CastSpell`'s `spell_filter` sibling. A tenth
/// ([`BlocksOrBecomesBlockedBy`](TriggerTag::BlocksOrBecomesBlockedBy)) reuses `YouSacrifice`'s
/// `filter` sibling for the creature on the other side of the block.
/// The three timings that aren't a [`Trigger`].
#[derive(Deserialize)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpecialTiming {
    Spell,
    Static,
    Activated,
}

/// An ability's `timing` string: one of the three non-trigger timings, or a trigger tag.
#[derive(Deserialize)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub(crate) enum TimingName {
    Special(SpecialTiming),
    Trigger(TriggerTag),
}

#[derive(Deserialize)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub(crate) enum TriggerTag {
    Etb,
    AsEnters,
    TurnedFaceUp,
    BecomesMonstrous,
    Attacks,
    BlocksOrBecomesBlocked,
    BlocksOrBecomesBlockedBy,
    AttacksOrBlocks,
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
    LandPutIntoGraveyard,
    Upkeep,
    EachUpkeep,
    FirstMainPhase,
    EachPlayerFirstMainPhase,
    BeginCombat,
    EndOfCombat,
    EndStep,
    EachEndStep,
    EachDrawStep,
    DrawStep,
    EachOtherPlayerUntapStep,
    YouGainLife,
    OpponentGainsLife,
    Magecraft,
    PlayerAttacksYourOpponent,
    YouAttackWithCreatures,
    OpponentAttacksYouWithCreatures,
    AnotherPlayerAttacksWithCreatures,
    CreatureAttacks,
    /// "Whenever the permanent this is attached to attacks" (CR 508.1) — an Aura's "whenever
    /// enchanted creature attacks" *and* an Equipment's "whenever equipped creature attacks"
    /// (Fractal Harness), which is the same firing path: [`Trigger::EnchantedCreatureAttacks`]
    /// fires off any attached permanent (see [`Game::queue_enchanted_creature_attacks_triggers`],
    /// which reads [`Game::attachments`] rather than filtering to Auras).
    EnchantedCreatureAttacks,
    EnchantedCreatureDies,
    /// Whenever the enchanted host deals damage, combat or noncombat (Armadillo Cloak's "you gain
    /// that much life"). See [`Trigger::EnchantedCreatureDealsDamage`].
    EnchantedCreatureDealsDamage,
    /// Whenever this permanent's controller is dealt damage, combat or noncombat (Living
    /// Artifact's "put that many vitality counters"). See [`Trigger::YouAreDealtDamage`].
    YouAreDealtDamage,
    AnEnchantedCreatureDies,
    CreatureEnchantedByYourAuraAttacks,
    YouSacrifice,
    AnyPlayerSacrifices,
    /// Reuses `AnyPlayerSacrifices`' `filter` sibling for the tapped permanent, plus its own
    /// `for_mana` bool.
    PermanentBecomesTapped,
    EnchantedPermanentBecomesTapped,
    YouDiscard,
    YouDiscardNonland,
    YouPlayALand,
    DealsCombatDamageToPlayer,
    DealsCombatDamageToCreature,
    ThisIsDealtDamage,
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
    YouProliferate,
    /// Takes a `counter_kind` sibling naming the counter kind whose last one coming off fires the
    /// ability (Divine Intervention's `intervention`). See
    /// [`Trigger::YouRemoveLastCounterFromThis`].
    YouRemoveLastCounterFromThis,
}

/// An `[[abilities]]` table is flat in TOML: the timing is a string, and an activated
/// ability's cost pieces (`taps_self`, `activation_cost`, `sacrifice`, `pay_life`,
/// `loyalty`, `once_each_turn`) sit beside it rather than nested inside [`Timing::Activated`].
impl<'de> Deserialize<'de> for Ability {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let flat = AbilityToml::deserialize(d)?;
        let effect = match flat.effects.as_slice() {
            [] => {
                return Err(de::Error::custom(
                    "an ability needs a non-empty `effects` list; write at least one \
                     [[abilities.effects]] block",
                ));
            }
            [only] => only.clone(), // one-element `effects` is just that effect (no Sequence wrapper).
            _ => Effect::Sequence {
                steps: Arc::from(flat.effects),
            },
        };
        let timing = match flat.timing {
            TimingName::Trigger(tag) => Timing::Triggered(match tag {
                TriggerTag::Etb => Trigger::Etb,
                TriggerTag::AsEnters => Trigger::AsEnters,
                TriggerTag::TurnedFaceUp => Trigger::TurnedFaceUp,
                TriggerTag::BecomesMonstrous => Trigger::BecomesMonstrous,
                TriggerTag::Attacks => Trigger::Attacks,
                TriggerTag::BlocksOrBecomesBlocked => Trigger::BlocksOrBecomesBlocked,
                TriggerTag::BlocksOrBecomesBlockedBy => Trigger::BlocksOrBecomesBlockedBy {
                    filter: flat.filter,
                },
                TriggerTag::AttacksOrBlocks => Trigger::AttacksOrBlocks,
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
                TriggerTag::LandPutIntoGraveyard => Trigger::LandPutIntoGraveyard,
                TriggerTag::NonlandPermanentYouControlDiesIncludingThis => {
                    Trigger::NonlandPermanentYouControlDiesIncludingThis
                }
                TriggerTag::Upkeep => Trigger::Upkeep,
                TriggerTag::EachUpkeep => Trigger::EachUpkeep,
                TriggerTag::FirstMainPhase => Trigger::FirstMainPhase,
                TriggerTag::EachPlayerFirstMainPhase => Trigger::EachPlayerFirstMainPhase,
                TriggerTag::BeginCombat => Trigger::BeginCombat,
                TriggerTag::EndOfCombat => Trigger::EndOfCombat,
                TriggerTag::EndStep => Trigger::EndStep,
                TriggerTag::EachEndStep => Trigger::EachEndStep,
                TriggerTag::EachDrawStep => Trigger::EachDrawStep,
                TriggerTag::DrawStep => Trigger::DrawStep,
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
                TriggerTag::CreatureAttacks => Trigger::CreatureAttacks,
                TriggerTag::EnchantedCreatureAttacks => Trigger::EnchantedCreatureAttacks,
                TriggerTag::EnchantedCreatureDies => Trigger::EnchantedCreatureDies,
                TriggerTag::EnchantedCreatureDealsDamage => Trigger::EnchantedCreatureDealsDamage,
                TriggerTag::YouAreDealtDamage => Trigger::YouAreDealtDamage,
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
                TriggerTag::PermanentBecomesTapped => Trigger::PermanentBecomesTapped {
                    filter: flat.filter,
                    for_mana: flat.for_mana,
                },
                TriggerTag::EnchantedPermanentBecomesTapped => {
                    Trigger::EnchantedPermanentBecomesTapped
                }
                TriggerTag::YouDiscard => Trigger::YouDiscard,
                TriggerTag::YouDiscardNonland => Trigger::YouDiscardNonland,
                TriggerTag::YouPlayALand => Trigger::YouPlayALand,
                TriggerTag::DealsCombatDamageToPlayer => {
                    Trigger::DealsCombatDamageToPlayer { who: flat.who }
                }
                TriggerTag::DealsCombatDamageToCreature => Trigger::DealsCombatDamageToCreature,
                TriggerTag::ThisIsDealtDamage => Trigger::ThisIsDealtDamage,
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
                TriggerTag::BecomesTargeted => Trigger::BecomesTargeted { who: flat.targeted },
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
                TriggerTag::YouProliferate => Trigger::YouProliferate,
                TriggerTag::YouRemoveLastCounterFromThis => {
                    let Some(kind) = flat.counter_kind else {
                        return Err(de::Error::custom(
                            "`timing = \"you_remove_last_counter_from_this\"` needs a \
                             `counter_kind` sibling naming the counter kind it watches",
                        ));
                    };
                    Trigger::YouRemoveLastCounterFromThis { kind }
                }
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
                remove_counters_x: flat.remove_counters_x,
                self_damage: flat.self_damage,
                loyalty: flat.loyalty,
                once_each_turn: flat.once_each_turn,
                sorcery_speed: flat.sorcery_speed,
                only_during_opponents_turn: flat.only_during_opponents_turn,
                only_during_your_turn: flat.only_during_your_turn,
                only_before_attackers: flat.only_before_attackers,
                only_during_your_upkeep: flat.only_during_your_upkeep,
                only_owner_may_activate: flat.only_owner_may_activate,
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
