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
