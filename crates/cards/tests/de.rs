//! Card DSL `Deserialize` coverage for `cards::de` (requires `card-dsl` feature).

use cards::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct ManaRow {
    mana: Mana,
}

#[derive(Deserialize)]
struct TypeRow {
    types: TypeSet,
}

#[derive(Deserialize)]
struct FilterRow {
    filter: PermanentFilter,
}

#[derive(Deserialize)]
struct CountRow {
    count: TargetCount,
}

#[test]
fn cost_toml_schema_lists_pip_keys() {
    let schema = schemars::schema_for!(cards::toml_surface::CostToml);
    let json = serde_json::to_value(schema).unwrap();
    let props = json["properties"].as_object().expect("properties");
    assert!(props.contains_key("generic"));
    assert!(props.contains_key("red"));
    assert!(props.contains_key("hybrid"));
}

#[test]
fn kind_toml_schema_includes_instant_and_creature() {
    let schema = schemars::schema_for!(cards::toml_surface::KindToml);
    let json = serde_json::to_value(schema).unwrap();
    let text = json.to_string();
    assert!(text.contains("instant"), "{text}");
    assert!(text.contains("creature"), "{text}");
}

#[test]
fn card_toml_schema_includes_name_and_damage_effect() {
    let schema = schemars::schema_for!(cards::toml_surface::CardToml);
    let text = serde_json::to_string(&schema).unwrap();
    assert!(text.contains("\"name\""));
    assert!(text.contains("damage"));
}

#[test]
fn card_toml_round_trips_abrade_name_and_effect_type() {
    let raw = r#"
name = "Abrade"
id = "f9db72dc-9a5b-48a4-a86e-7464d9a2166a"
default_print = "1add1757-c1f8-448a-b279-c6940fb7ad5f"
oracle = "Choose one —"

[cost]
generic = 1
red = 1

[kind]
type = "instant"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "damage"
mode = "target"
amount = 3
target = "creature"
"#;
    let toml_card: cards::toml_surface::CardToml = toml::from_str(raw).expect("CardToml parse");
    assert_eq!(toml_card.name, "Abrade");
    let def: cards::CardDef = toml_card.into();
    assert_eq!(def.name, "Abrade");
    assert!(matches!(def.abilities[0].effect, cards::Effect::Damage(_)));
}

#[test]
fn card_toml_accepts_late_keys_inside_kind_table() {
    let raw = r#"
name = "Late Keywords"
id = "00000000-0000-0000-0000-000000000003"
default_print = "00000000-0000-0000-0000-000000000004"

[kind]
type = "creature"
power = 2
toughness = 2

keywords = ["flying"]
"#;
    let def: cards::CardDef = toml::from_str(raw).expect("CardDef parse");
    assert_eq!(def.name, "Late Keywords");
    assert!(matches!(
        def.kind,
        CardKind::Creature {
            power: 2,
            toughness: 2,
            ..
        }
    ));
}

#[test]
fn cost_rejects_a_mono_hybrid_pair() {
    let err = toml::from_str::<Cost>(
        r#"
hybrid = [["red", "red"]]
"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("must differ"));
}

#[test]
fn card_toml_rejects_a_mono_hybrid_pair() {
    let raw = r#"
name = "Bad Hybrid"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[cost]
hybrid = [["red", "red"]]

[kind]
type = "artifact"
"#;
    let err = toml::from_str::<CardDef>(raw).unwrap_err();
    assert!(err.to_string().contains("must differ"));
}

#[test]
fn additional_cost_parses_pay_life_x_marker() {
    let extra: AdditionalCost = toml::from_str(r#"pay_life = "x""#).unwrap();
    assert!(extra.pay_life_x);
    assert_eq!(extra.pay_life, 0);
}

#[test]
fn additional_cost_parses_fixed_pay_life() {
    let extra: AdditionalCost = toml::from_str("pay_life = 3").unwrap();
    assert!(!extra.pay_life_x);
    assert_eq!(extra.pay_life, 3);
}

#[test]
fn mana_normalizes_dual_symbol_order() {
    let row: ManaRow = toml::from_str(
        r#"
mana = ["green", "blue"]
"#,
    )
    .unwrap();
    assert_eq!(row.mana, Mana::Either(Color::Blue, Color::Green));
}

#[test]
fn target_count_parses_scalar_and_table_forms() {
    let exact: CountRow = toml::from_str("count = 2").unwrap();
    assert_eq!(
        exact.count,
        TargetCount {
            min: 2,
            max: 2,
            x_scaled: false,
            sacrifice_scaled: false,
            strive_scaled: false,
            total_mv_max: None,
            multikicker_scaled: false,
            kicked_scaled: false,
            main_phase_scaled: false,
        }
    );

    let range: CountRow = toml::from_str(
        r#"
[count]
min = 1
max = 3
"#,
    )
    .unwrap();
    assert_eq!(
        range.count,
        TargetCount {
            min: 1,
            max: 3,
            x_scaled: false,
            sacrifice_scaled: false,
            strive_scaled: false,
            total_mv_max: None,
            multikicker_scaled: false,
            kicked_scaled: false,
            main_phase_scaled: false,
        }
    );

    // Rampaging Yao Guai's "any number of target artifacts and/or enchantments with total mana
    // value X or less" (CR 601.2c): the budget rides on the count table, and a dropped
    // `total_mv_max` would silently deserialize to `None` — no budget at all.
    let budgeted: CountRow = toml::from_str(
        r#"
[count]
max = 255
total_mv_max = "x"
"#,
    )
    .unwrap();
    assert_eq!(
        budgeted.count,
        TargetCount {
            min: 0,
            max: 255,
            x_scaled: false,
            sacrifice_scaled: false,
            strive_scaled: false,
            total_mv_max: Some(Amount::X),
            multikicker_scaled: false,
            kicked_scaled: false,
            main_phase_scaled: false,
        }
    );

    match toml::from_str::<CountRow>(
        r#"
[count]
min = 3
max = 1
"#,
    ) {
        Ok(_) => panic!("expected min > max to fail"),
        Err(err) => assert!(err.to_string().contains("min exceeds max")),
    }
}

#[test]
fn type_set_parses_planeswalker_and_unions() {
    let pw: TypeRow = toml::from_str(r#"types = "planeswalker""#).unwrap();
    assert_eq!(pw.types, TypeSet::PLANESWALKER);

    let both: TypeRow = toml::from_str(
        r#"
types = ["creature", "planeswalker"]
"#,
    )
    .unwrap();
    assert!(both.types.intersects(TypeSet::CREATURE));
    assert!(both.types.intersects(TypeSet::PLANESWALKER));
}

#[test]
fn permanent_filter_parses_creature_shorthand() {
    let row: FilterRow = toml::from_str(r#"filter = "creatures""#).unwrap();
    assert_eq!(row.filter.types, TypeSet::CREATURE);
}

#[test]
fn additional_cost_rejects_unknown_pay_life_marker() {
    let err = toml::from_str::<AdditionalCost>(r#"pay_life = "life""#);
    match err {
        Ok(_) => panic!("expected unsupported pay_life string to fail"),
        Err(message) => assert!(message.to_string().contains("unsupported string")),
    }
}

#[test]
fn cost_orders_hybrid_pair_when_first_color_is_after_second() {
    let cost: Cost = toml::from_str(
        r#"
hybrid = [["green", "blue"]]
"#,
    )
    .unwrap();
    assert_eq!(cost.hybrid, &[(Color::Blue, Color::Green)]);

    let reversed: Cost = toml::from_str(
        r#"
hybrid = [["blue", "green"]]
"#,
    )
    .unwrap();
    assert_eq!(reversed.hybrid, cost.hybrid);
}

#[test]
fn look_at_top_defaults_up_to_one() {
    let effect: Effect = toml::from_str(
        r#"
type = "dig"
mode = "look_at_top"
count = 3
filter = "land"
dest = "hand"
"#,
    )
    .unwrap();
    match effect {
        Effect::Dig(DigEffect::LookAtTop { up_to, .. }) => assert_eq!(up_to, 1),
        other => panic!("expected LookAtTop, got {other:?}"),
    }
}

#[test]
fn each_player_sacrifices_defaults_to_creature_filter() {
    let effect: Effect = toml::from_str(
        r#"
type = "choice"
mode = "each_player_sacrifices"
"#,
    )
    .unwrap();
    match effect {
        Effect::Choice(ChoiceEffect::EachPlayerSacrifices { filter, .. }) => {
            assert_eq!(filter.types, TypeSet::CREATURE);
        }
        other => panic!("expected EachPlayerSacrifices, got {other:?}"),
    }
}

#[test]
fn ability_unwraps_a_singleton_effects_list() {
    #[derive(serde::Deserialize)]
    struct Row {
        #[serde(flatten)]
        ability: Ability,
    }
    let row: Row = toml::from_str(
        r#"
timing = "spell"
effects = [{ type = "draw", mode = "cards", count = 1 }]
"#,
    )
    .unwrap();
    assert!(matches!(
        row.ability.effect,
        Effect::Draw(DrawEffect::Cards {
            count: Amount::Fixed(1)
        })
    ));
}
