//! Author-facing JSON Schema for the card-DSL types whose TOML spelling differs structurally
//! from their Rust shape (see [`crate::de`]). schemars can derive a faithful schema for the
//! plain externally-/internally-tagged enums directly; the handful here have hand-written
//! `Deserialize` impls (a bare-string shorthand, a scalar-or-table union, a keyword vocabulary),
//! so their `JsonSchema` is written by hand too, mirroring exactly what the visitor accepts.
//!
//! Everything in this module is compiled only under the `card-schema` feature and used purely by
//! the schema/reference generators; it never participates in deserialization.

use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use serde_json::Value as JsonValue;
use std::num::NonZeroU8;

use crate::de::{
    AMOUNT_KEYWORDS, PERMANENT_FILTER_SHORTHANDS, SACRIFICE_COST_SHORTHANDS, TYPE_NAMES,
};
use crate::toml_surface::CostToml;
use crate::{
    AdditionalCost, Amount, AmountZone, ArithOp, Color, ColorFilter, Condition, Cost, CounterAxis,
    CounterKind, Division, FilterController, Keyword, LandProduces, Mana, Parity, PermanentFilter,
    ProtectionScope, SacrificeCost, TargetCount, TokenFilter, TypeSet,
};

// ── schema-building helpers ─────────────────────────────────────────────────────────

fn string_enum(values: &[&str]) -> Schema {
    Schema::Object(SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        enum_values: Some(
            values
                .iter()
                .map(|v| JsonValue::String((*v).to_owned()))
                .collect(),
        ),
        ..Default::default()
    })
}

fn array_of(items: Schema) -> Schema {
    let mut object = SchemaObject {
        instance_type: Some(InstanceType::Array.into()),
        ..Default::default()
    };
    object.array().items = Some(SingleOrVec::Single(Box::new(items)));
    Schema::Object(object)
}

fn color_choice_array() -> Schema {
    let mut object = SchemaObject {
        instance_type: Some(InstanceType::Array.into()),
        ..Default::default()
    };
    let array = object.array();
    array.items = Some(SingleOrVec::Single(Box::new(string_enum(&[
        "white", "blue", "black", "red", "green",
    ]))));
    array.min_items = Some(2);
    array.max_items = Some(4);
    Schema::Object(object)
}

fn one_of(schemas: Vec<Schema>) -> Schema {
    let mut object = SchemaObject::default();
    object.subschemas().one_of = Some(schemas);
    Schema::Object(object)
}

fn required_key(name: &str) -> Schema {
    let mut object = SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        ..Default::default()
    };
    object.object().required.insert(name.to_owned());
    Schema::Object(object)
}

// ── TypeSet: a card-type name or a list of them ──────────────────────────────────────

/// The card-type names a [`TypeSet`] accepts (see [`crate::de`]'s `type_bits`).
impl JsonSchema for TypeSet {
    fn schema_name() -> String {
        "TypeSet".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            string_enum(TYPE_NAMES),
            array_of(string_enum(TYPE_NAMES)),
        ])
    }
}

// ── LandProduces: mana symbol sugar or computed land-producer keywords ───────────────

impl JsonSchema for LandProduces {
    fn schema_name() -> String {
        "LandProduces".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            string_enum(&[
                "white",
                "blue",
                "black",
                "red",
                "green",
                "colorless",
                "any",
                "commander_identity",
                "opponent_colors",
            ]),
            color_choice_array(),
        ])
    }
}

// ── ProtectionScope: a color name or a non-color quality ─────────────────────────────

impl JsonSchema for ProtectionScope {
    fn schema_name() -> String {
        "ProtectionScope".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        string_enum(&[
            "white",
            "blue",
            "black",
            "red",
            "green",
            "creatures",
            "multicolored",
        ])
    }
}

// ── Cost: the flat `[cost]` color-name table (mirrors CostToml) ───────────────────────

impl JsonSchema for Cost {
    fn schema_name() -> String {
        "Cost".to_owned()
    }

    fn is_referenceable() -> bool {
        false
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<CostToml>()
    }
}

// ── AdditionalCost: the closed `[cost.additional]` rider table ───────────────────────

impl JsonSchema for AdditionalCost {
    fn schema_name() -> String {
        "AdditionalCost".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<AdditionalCostTableSchema>()
    }
}

/// The TOML shape accepted by [`crate::de`]'s [`AdditionalCost`] visitor.
#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
struct AdditionalCostTableSchema {
    discard: Option<u8>,
    discard_land: Option<bool>,
    reveal_creature_from_hand: Option<bool>,
    pay_life: Option<PayLifeAdditionalCostTomlSchema>,
    sacrifice: Option<SacrificeAdditionalCostTomlSchema>,
    kicker: Option<CostToml>,
    buyback: Option<CostToml>,
    strive: Option<CostToml>,
    replicate: Option<CostToml>,
    multikicker: Option<CostToml>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(untagged)]
enum PayLifeAdditionalCostTomlSchema {
    Marker(PayLifeAdditionalCostMarkerSchema),
    Fixed(u8),
}

#[derive(JsonSchema)]
#[schemars(rename = "PayLifeAdditionalCostMarker")]
#[allow(dead_code)]
enum PayLifeAdditionalCostMarkerSchema {
    #[schemars(rename = "x")]
    X,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
struct SacrificeAdditionalCostTomlSchema {
    count: SacrificeAdditionalCostCountTomlSchema,
    filter: Option<PermanentFilter>,
}

#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(untagged)]
enum SacrificeAdditionalCostCountTomlSchema {
    Marker(SacrificeAdditionalCostCountMarkerSchema),
    Fixed(NonZeroU8),
}

#[derive(JsonSchema)]
#[schemars(rename = "SacrificeAdditionalCostCountMarker")]
#[allow(dead_code)]
enum SacrificeAdditionalCostCountMarkerSchema {
    #[schemars(rename = "one_or_more")]
    OneOrMore,
}

// ── Amount: an integer, a derived-value keyword, or a table ──────────────────────────

/// A bare `{}` presence-flag table (an [`Amount`] arm that carries no fields of its own).
#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
struct EmptyTableSchema {}

/// The table form of an [`Amount`] — mirrors the visitor's `Table` in [`crate::de`]. Every key is
/// optional and mutually-exclusive combinations are enforced at load, not in the schema.
#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
struct AmountTableSchema {
    per_permanent: Option<PermanentFilter>,
    zone: Option<AmountZone>,
    per_counter_of_kind: Option<CounterKind>,
    condition: Option<Condition>,
    then: Option<Amount>,
    #[schemars(rename = "else")]
    otherwise: Option<Amount>,
    permanents_destroyed_this_way: Option<PermanentFilter>,
    auras_attached_to_source: Option<EmptyTableSchema>,
    discard_cost_was_land: Option<i32>,
    left: Option<Amount>,
    op: Option<ArithOp>,
    right: Option<Amount>,
}

impl JsonSchema for Amount {
    fn schema_name() -> String {
        "Amount".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            generator.subschema_for::<i32>(),
            string_enum(AMOUNT_KEYWORDS),
            generator.subschema_for::<AmountTableSchema>(),
        ])
    }
}

// ── AmountZone: a bare `"battlefield"` / `"graveyard"` string ─────────────────────────

impl JsonSchema for AmountZone {
    fn schema_name() -> String {
        "AmountZone".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        string_enum(&["battlefield", "graveyard"])
    }
}

// ── PermanentFilter: a shorthand string or a composable table ────────────────────────

/// The table form of a [`PermanentFilter`] — mirrors the visitor's `Table` in [`crate::de`].
#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(deny_unknown_fields)]
struct PermanentFilterTableSchema {
    types: Option<TypeSet>,
    subtypes: Option<Vec<String>>,
    controller: Option<FilterController>,
    token: Option<TokenFilter>,
    other: Option<bool>,
    enchanted: Option<bool>,
    attached_to_creature: Option<bool>,
    enchanted_by_you: Option<bool>,
    mv_max: Option<u8>,
    mv_min: Option<u8>,
    mv_eq_x: Option<bool>,
    mv_max_x: Option<bool>,
    tapped: Option<bool>,
    has_mana_ability: Option<bool>,
    power_max: Option<u8>,
    power_min: Option<u8>,
    power_parity: Option<Parity>,
    toughness_max: Option<u8>,
    toughness_min: Option<u8>,
    noncreature: Option<bool>,
    exclude: Option<TypeSet>,
    color: Option<ColorFilter>,
    not_color: Option<Color>,
    modified: Option<bool>,
    attacking: Option<bool>,
    not_attacking: Option<bool>,
    attacking_you: Option<bool>,
    blocking: Option<bool>,
    attacking_or_blocking: Option<bool>,
    tapped_or_blocking: Option<bool>,
    unblocked: Option<bool>,
    blocked_by_a_wall_this_turn: Option<bool>,
    power_less_than_source: Option<bool>,
    toughness_less_than_source_power: Option<bool>,
    entered_this_turn: Option<bool>,
    controlled_since_turn_start: Option<bool>,
    did_not_attack_this_turn: Option<bool>,
    nonbasic: Option<bool>,
    basic: Option<bool>,
    name: Option<String>,
    nonlegendary: Option<bool>,
    legendary: Option<bool>,
    nonlair: Option<bool>,
    without_flying: Option<bool>,
    without_keyword: Option<Keyword>,
    with_flying: Option<bool>,
    shares_type_with_dying_permanent: Option<bool>,
    with_counter: Option<CounterAxis>,
    creature_or_vehicle: Option<bool>,
    snow: Option<bool>,
    exclude_subtypes: Option<Vec<String>>,
    exclude_name: Option<String>,
}

impl JsonSchema for PermanentFilter {
    fn schema_name() -> String {
        "PermanentFilter".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            string_enum(PERMANENT_FILTER_SHORTHANDS),
            generator.subschema_for::<PermanentFilterTableSchema>(),
        ])
    }
}

// ── SacrificeCost: a shorthand string or a `{ creature|permanent, count }` table ──────

impl JsonSchema for SacrificeCost {
    fn schema_name() -> String {
        "SacrificeCost".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            string_enum(SACRIFICE_COST_SHORTHANDS),
            sacrifice_cost_table_schema(generator),
        ])
    }
}

fn sacrifice_cost_table_schema(generator: &mut SchemaGenerator) -> Schema {
    let mut object = SchemaObject {
        instance_type: Some(InstanceType::Object.into()),
        ..Default::default()
    };
    object.object().properties.insert(
        "creature".to_owned(),
        generator.subschema_for::<PermanentFilter>(),
    );
    object.object().properties.insert(
        "permanent".to_owned(),
        generator.subschema_for::<PermanentFilter>(),
    );
    object
        .object()
        .properties
        .insert("count".to_owned(), generator.subschema_for::<u8>());
    object.object().additional_properties = Some(Box::new(Schema::Bool(false)));
    object.subschemas().any_of = Some(vec![required_key("creature"), required_key("permanent")]);
    Schema::Object(object)
}

// ── Mana: a mana-symbol name or a 2-to-4-color choice array ──────────────────────────

impl JsonSchema for Mana {
    fn schema_name() -> String {
        "Mana".to_owned()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            string_enum(&["white", "blue", "black", "red", "green", "colorless", "any"]),
            color_choice_array(),
        ])
    }
}

// ── TargetCount: a bare count or a `{ min, max, …scaled }` range table ───────────────

impl JsonSchema for TargetCount {
    fn schema_name() -> String {
        "TargetCount".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            generator.subschema_for::<u8>(),
            generator.subschema_for::<crate::de::TargetCountToml>(),
        ])
    }
}

// ── Division: `divided = true` / `divided = "evenly"` ───────────────────────────────

/// `divided`'s TOML spelling (see [`Division`]'s `Deserialize`): the bool picks between no
/// division and "divided as you choose", and the one named split is a string.
impl JsonSchema for Division {
    fn schema_name() -> String {
        "Division".to_owned()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        one_of(vec![
            generator.subschema_for::<bool>(),
            string_enum(&["evenly"]),
        ])
    }
}

// ── `count_or_any`: a fixed count or the `"any"` keyword ─────────────────────────────

/// The TOML shape accepted by [`crate::de`]'s `count_or_any` — a fixed count, or `"any"` for
/// "any number" (a search that may take every match).
#[allow(dead_code)]
#[derive(JsonSchema)]
#[schemars(untagged)]
pub enum CountOrAnyToml {
    Any(String),
    Fixed(u8),
}
