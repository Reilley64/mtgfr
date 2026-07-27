#[test]
fn rejects_unknown_effect_type() {
    let card = r#"
name = "Bad"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[kind]
type = "instant"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "damge"
"#;

    let errors = cards::validate_toml_str(card).expect_err("unknown effect type must fail");
    let rendered = errors.join("\n");
    assert!(
        rendered.contains("/abilities/0/effects/0/type"),
        "{rendered}"
    );
    assert!(rendered.contains("damge"), "{rendered}");
}

#[test]
fn accepts_abrade_pool_card() {
    let abrade =
        std::fs::read_to_string("data/abrade.toml").expect("Abrade is in the cards data pool");

    cards::validate_toml_str(&abrade).expect("Abrade TOML validates against the card schema");
}

#[test]
fn rejects_misspelled_permanent_filter_shorthand() {
    let card = r#"
name = "Bad Filter"
id = "00000000-0000-0000-0000-000000000005"
default_print = "00000000-0000-0000-0000-000000000006"

[kind]
type = "creature"
power = 1
toughness = 1

[[abilities]]
timing = "you_sacrifice"
filter = "creaturs"

[[abilities.effects]]
type = "draw"
mode = "cards"
count = 1
"#;

    let errors = cards::validate_toml_str(card).expect_err("misspelled filter shorthand must fail");
    assert!(
        errors.join("\n").contains("/abilities/0/filter"),
        "{errors:?}"
    );
}

#[test]
fn rejects_sacrifice_cost_without_filter_key() {
    let card = r#"
name = "Bad Sacrifice Cost"
id = "00000000-0000-0000-0000-000000000007"
default_print = "00000000-0000-0000-0000-000000000008"

[kind]
type = "creature"
power = 1
toughness = 1

[[abilities]]
timing = "activated"
sacrifice = { count = 2 }

[[abilities.effects]]
type = "life"
mode = "gain"
amount = 1
"#;

    let errors = cards::validate_toml_str(card)
        .expect_err("sacrifice cost needs creature or permanent filter key");
    assert!(
        errors.join("\n").contains("/abilities/0/sacrifice"),
        "{errors:?}"
    );
}

/// A promoted opaque surface (Wave C) now carries a real typed schema, so a wrong-typed value is
/// rejected — where the former `any` escape silently accepted anything. `cumulative_upkeep` is an
/// object with an integer `graveyard_cards`, so a bare string no longer validates.
#[test]
fn rejects_wrong_typed_promoted_cumulative_upkeep() {
    let card = r#"
name = "Bad Upkeep"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"
cumulative_upkeep = "nonsense"

[kind]
type = "creature"
power = 1
toughness = 1
"#;

    let errors =
        cards::validate_toml_str(card).expect_err("wrong-typed cumulative_upkeep must fail");
    assert!(
        errors.join("\n").contains("/cumulative_upkeep"),
        "{errors:?}"
    );
}

/// The promoted `enter_as_copy` surface likewise rejects an unknown key rather than accepting any
/// object, proving the schema tightened beyond the former `any` escape.
#[test]
fn rejects_unknown_key_in_promoted_enter_as_copy() {
    let card = r#"
name = "Bad Copy"
id = "00000000-0000-0000-0000-000000000003"
default_print = "00000000-0000-0000-0000-000000000004"

[kind]
type = "creature"
power = 1
toughness = 1

[enter_as_copy]
gains_haste = true
bogus = true
"#;

    let errors = cards::validate_toml_str(card).expect_err("unknown enter_as_copy key must fail");
    assert!(errors.join("\n").contains("/enter_as_copy"), "{errors:?}");
}

#[test]
fn rejects_untyped_non_effect_schema_escape_fields() {
    let cases = [
        (
            "cost.additional.pay_life",
            r#"
name = "Bad Additional Cost"
id = "00000000-0000-0000-0000-000000000009"
default_print = "00000000-0000-0000-0000-000000000010"

[cost.additional]
pay_life = "not_x"

[kind]
type = "instant"
"#,
            "/cost/additional/pay_life",
        ),
        (
            "cost.reduce_own_generic",
            r#"
name = "Bad Own Reducer"
id = "00000000-0000-0000-0000-000000000011"
default_print = "00000000-0000-0000-0000-000000000012"

[cost]
reduce_own_generic = "not_an_amount"

[kind]
type = "instant"
"#,
            "/cost/reduce_own_generic",
        ),
        (
            "kind.also",
            r#"
name = "Bad Also"
id = "00000000-0000-0000-0000-000000000013"
default_print = "00000000-0000-0000-0000-000000000014"

[kind]
type = "creature"
power = 1
toughness = 1
also = 7
"#,
            "/kind",
        ),
        (
            "kind.produces",
            r#"
name = "Bad Produces"
id = "00000000-0000-0000-0000-000000000015"
default_print = "00000000-0000-0000-0000-000000000016"

[kind]
type = "land"
produces = "not_mana"
"#,
            "/kind",
        ),
    ];

    for (name, card, pointer) in cases {
        let Err(errors) = cards::validate_toml_str(card) else {
            panic!("{name} must fail schema");
        };
        assert!(
            errors.join("\n").contains(pointer),
            "{name} errors did not include {pointer}: {errors:?}"
        );
    }
}
