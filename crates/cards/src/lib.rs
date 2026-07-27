//! The card pool as data: one TOML file per card under `data/`, plus token profiles under
//! `data/tokens/`. Loaded once into registries of `engine::CardDef`. The engine's `card-dsl`
//! feature deserializes a card's TOML directly into `CardDef` (interning owned strings and
//! load-once data to `'static` where useful, then cloning small handles from the bounded pool
//! as needed); this crate is just the file I/O and the id-keyed registry, keeping the engine
//! free of I/O (`CLAUDE.md`).
//!
//! Token profiles load first and are installed via [`engine::install_token_defs`] so
//! `create_token`'s `token = "<oracle-id>"` resolves during deckable-card deserialize. Tokens
//! are **not** in [`registry`] — the catalog/deck builder only sees castable cards.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use engine::CardDef;

struct Pool {
    /// Primary key: Scryfall oracle id ([`CardDef::id`]).
    by_id: HashMap<String, CardDef>,
    /// Secondary: printed name → CardDef (authoring, tests, fuzzy display).
    by_name: HashMap<String, CardDef>,
}

struct TokenPool {
    by_id: HashMap<String, CardDef>,
}

static POOL: OnceLock<Pool> = OnceLock::new();
static TOKEN_POOL: OnceLock<TokenPool> = OnceLock::new();

fn pool() -> &'static Pool {
    POOL.get_or_init(load_from_data_dir)
}

fn token_pool() -> &'static TokenPool {
    let _ = pool();
    TOKEN_POOL
        .get()
        .expect("token pool installed during card load")
}

/// The loaded card registry, keyed by Card id (Scryfall oracle id). Deckable cards only.
pub fn registry() -> &'static HashMap<String, CardDef> {
    &pool().by_id
}

/// The card with the given Card id, if it exists in the pool.
pub fn get(id: &str) -> Option<CardDef> {
    pool().by_id.get(id).cloned()
}

/// The card with the given printed name, if it exists in the pool.
pub fn get_by_name(name: &str) -> Option<CardDef> {
    pool().by_name.get(name).cloned()
}

/// Token profiles from `data/tokens/`, keyed by Scryfall oracle id.
pub fn token_registry() -> &'static HashMap<String, CardDef> {
    &token_pool().by_id
}

/// The token profile with the given Scryfall oracle id, if it exists.
pub fn get_token(id: &str) -> Option<CardDef> {
    token_pool().by_id.get(id).cloned()
}

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))
}

fn load_toml_file(path: &Path) -> CardDef {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    toml::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

fn load_token_defs(dir: &Path) {
    let tokens_dir = dir.join("tokens");
    let entries = std::fs::read_dir(&tokens_dir)
        .unwrap_or_else(|e| panic!("reading token data dir {}: {e}", tokens_dir.display()));

    let mut by_id_owned: HashMap<String, CardDef> = HashMap::new();
    let mut engine_map: HashMap<&'static str, CardDef> = HashMap::new();

    for entry in entries {
        let path = entry.expect("token data dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let def = load_toml_file(&path);
        if def.id.is_empty() {
            panic!(
                "{}: token CardDef.id (Scryfall oracle id) is required",
                path.display()
            );
        }
        if def.default_print.is_empty() {
            panic!(
                "{}: token CardDef.default_print (Scryfall card UUID) is required",
                path.display()
            );
        }
        if by_id_owned
            .insert(def.id.to_string(), def.clone())
            .is_some()
        {
            panic!("{}: duplicate token id {}", path.display(), def.id);
        }
        // `def.id` is already leaked/`'static` from CardDef deserialize.
        engine_map.insert(def.id, def);
    }

    TOKEN_POOL
        .set(TokenPool { by_id: by_id_owned })
        .unwrap_or_else(|_| panic!("token pool installed twice"));
    engine::install_token_defs(engine_map);
}

fn load_from_data_dir() -> Pool {
    let dir = data_dir();
    // Tokens first so `token = "<id>"` resolves while parsing deckable cards.
    load_token_defs(&dir);

    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("reading card data dir {dir:?}: {e}"));

    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    for entry in entries {
        let path = entry.expect("card data dir entry").path();
        // Non-recursive: `data/tokens/` is loaded separately above.
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let def = load_toml_file(&path);
        if def.id.is_empty() {
            panic!(
                "{}: CardDef.id (Scryfall oracle id) is required",
                path.display()
            );
        }
        if def.default_print.is_empty() {
            panic!(
                "{}: CardDef.default_print (Scryfall card UUID) is required",
                path.display()
            );
        }
        if by_id.insert(def.id.to_string(), def.clone()).is_some() {
            panic!("{}: duplicate Card id {}", path.display(), def.id);
        }
        if by_name.insert(def.name.to_string(), def.clone()).is_some() {
            panic!("{}: duplicate card name {}", path.display(), def.name);
        }
    }
    Pool { by_id, by_name }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{
        Amount, CardFilter, CardKind, CasterScope, ChoiceEffect, Color, ColorFilter, Condition,
        ControlEffect, CopyEffect, Cost, CountersEffect, DamageEffect, DestroyEffect, DigEffect,
        DrawEffect, Effect, EnterController, ExileEffect, GraveyardScope, Keyword, LandProduces,
        LifeEffect, Mana, ManaEffect, MillEffect, MiscEffect, PermanentFilter, ProtectionScope,
        PumpEffect, SacrificeCost, SacrificeEffect, SearchDest, SpellFilter, SpellSpeed,
        StaticEffect, TargetCount, TargetSpec, Timing, TokenEffect, Trigger, TypeSet, ZoneEffect,
    };

    #[test]
    fn every_pool_toml_loads_into_the_registry() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        let toml_files = std::fs::read_dir(dir)
            .expect("card data dir")
            .filter(|entry| {
                let path = entry.as_ref().expect("card data dir entry").path();
                path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("toml")
            })
            .count();
        assert!(toml_files > 400, "the soc pool is ~430 files");
        // registry() parses every file (panicking with the file's path on a bad one);
        // the count match also proves no two files define the same card name.
        assert_eq!(registry().len(), toml_files);
    }

    #[test]
    fn every_token_toml_loads_into_the_token_registry() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data/tokens");
        let toml_files = std::fs::read_dir(dir)
            .expect("token data dir")
            .filter(|entry| {
                let path = entry.as_ref().expect("token data dir entry").path();
                path.extension().and_then(|e| e.to_str()) == Some("toml")
            })
            .count();
        assert!(
            toml_files >= 30,
            "expected ~35 token profiles, got {toml_files}"
        );
        assert_eq!(token_registry().len(), toml_files);
        for id in token_registry().keys() {
            assert!(
                get(id).is_none(),
                "token {id} must not be in the deckable registry"
            );
        }
    }

    #[test]
    fn treasure_token_resolves_from_token_registry_after_load() {
        let _ = registry();
        let from_registry =
            get_token(engine::TREASURE_ORACLE_ID).expect("treasure.toml in token_registry");
        let from_helper = engine::treasure_token();
        assert_eq!(from_helper.id, from_registry.id);
        assert_eq!(from_helper.default_print, from_registry.default_print);
        assert_eq!(from_helper.name, "Treasure");
        assert_eq!(from_helper.kind, CardKind::Artifact);
        assert_eq!(from_helper.subtypes.as_ref(), &["Treasure"]);
        // Same ability shape: {T}, sac → add {any}.
        assert_eq!(from_helper.abilities.len(), from_registry.abilities.len());
        assert_eq!(
            from_helper.abilities[0].timing,
            from_registry.abilities[0].timing
        );
    }

    #[test]
    fn nested_damage_target_deserializes() {
        let toml = r#"
name = "Fixture Bolt"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"
oracle = "Fixture deals 3 damage to any target."

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
"#;
        let def: CardDef = toml::from_str(toml).expect("nested damage parses");
        assert!(matches!(
            def.abilities[0].effect,
            Effect::Damage(DamageEffect::Target {
                amount: Amount::Fixed(3),
                ..
            })
        ));
    }

    #[test]
    fn split_destroy_related_families_deserialize() {
        let toml = r#"
name = "Fixture Split"
id = "00000000-0000-0000-0000-000000000011"
default_print = "00000000-0000-0000-0000-000000000012"
oracle = "Fixture exiles a creature, destroys an artifact, then sacrifices itself."

[cost]
generic = 1

[kind]
type = "artifact"

[[abilities]]
timing = "activated"

[[abilities.effects]]
type = "exile"
mode = "target"
target = "creature"

[[abilities.effects]]
type = "destroy"
mode = "target"
target = { permanent = { types = "artifact" } }

[[abilities.effects]]
type = "sacrifice"
mode = "source"
"#;
        let def: CardDef = toml::from_str(toml).expect("split destroy-related families parse");
        let Effect::Sequence { steps } = &def.abilities[0].effect else {
            panic!("multiple effects should parse as a sequence");
        };
        assert!(matches!(
            steps[0],
            Effect::Exile(ExileEffect::Target {
                target: TargetSpec::Creature,
                ..
            })
        ));
        assert!(matches!(
            steps[1],
            Effect::Destroy(DestroyEffect::Target {
                target: TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::ARTIFACT,
                    ..
                }),
                ..
            })
        ));
        assert!(matches!(
            steps[2],
            Effect::Sacrifice(SacrificeEffect::Source)
        ));
    }

    #[test]
    fn sets_and_subtypes_parse_and_default_empty() {
        let card = r#"name = "Goblin Test"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"
sets = ["soc", "c16"]
subtypes = ["Goblin", "Wizard"]

[kind]
type = "creature"
power = 1
toughness = 1
"#;
        let def: CardDef = toml::from_str(card).expect("sets + subtypes parse");
        assert_eq!(def.sets.as_ref(), &["soc", "c16"]);
        assert_eq!(def.subtypes.as_ref(), &["Goblin", "Wizard"]);

        let legacy = r#"name = "Legacy Set"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"
set = "cmd"

[kind]
type = "creature"
power = 1
toughness = 1
"#;
        assert!(
            toml::from_str::<CardDef>(legacy).is_err(),
            "singular set metadata was removed; use sets = [...]"
        );

        let bare = "name = \"Bare\"\nid = \"00000000-0000-0000-0000-000000000001\"\ndefault_print = \"00000000-0000-0000-0000-000000000002\"\n\n[kind]\ntype = \"creature\"\npower = 1\ntoughness = 1\n";
        let def: CardDef = toml::from_str(bare).expect("omitted sets defaults empty");
        assert!(def.sets.is_empty());
    }

    #[test]
    fn misspelled_toml_keys_are_load_errors() {
        // deny_unknown_fields: a typo'd key fails the parse instead of silently defaulting
        // (e.g. `legendery` would otherwise load as a non-legendary card).
        let card = "name = \"Typo\"
id = \"00000000-0000-0000-0000-000000000001\"
default_print = \"00000000-0000-0000-0000-000000000002\"
legendery = true\n\n[kind]\ntype = \"creature\"\npower = 1\ntoughness = 1\n";
        assert!(toml::from_str::<CardDef>(card).is_err());

        // The same guard inside an ability table: `tap_self` (missing s) must not
        // quietly produce a cost-free activated ability.
        let card = "name = \"Typo\"\nid = \"00000000-0000-0000-0000-000000000001\"\ndefault_print = \"00000000-0000-0000-0000-000000000002\"\n\n[kind]\ntype = \"creature\"\npower = 1\ntoughness = 1\n\n[[abilities]]\ntiming = \"activated\"\ntap_self = true\n\n[[abilities.effects]]\ntype = \"gain_life\"\namount = 1\n";
        assert!(toml::from_str::<CardDef>(card).is_err());

        // …and inside an effect table, the deepest and highest-churn surface of the DSL. An
        // effect block is the last table in most card files, so a key appended one line too far
        // lands here rather than at the top level — `toughness` misspelled on an anthem must not
        // load as a +1/+0 lord.
        let card = "name = \"Typo\"\nid = \"00000000-0000-0000-0000-000000000001\"\ndefault_print = \"00000000-0000-0000-0000-000000000002\"\n\n[kind]\ntype = \"enchantment\"\n\n[[abilities]]\ntiming = \"static\"\n\n[[abilities.effects]]\ntype = \"static\"\nmode = \"anthem\"\npower = 1\ntoughnes = 1\n";
        assert!(toml::from_str::<CardDef>(card).is_err());

        // The structural composers are tagged by `type` alone, with no `mode` leaf — they need
        // the guard on their own arm, not on a family enum.
        let card = "name = \"Typo\"\nid = \"00000000-0000-0000-0000-000000000001\"\ndefault_print = \"00000000-0000-0000-0000-000000000002\"\n\n[kind]\ntype = \"sorcery\"\n\n[[abilities]]\ntiming = \"spell\"\n\n[[abilities.effects]]\ntype = \"sequence\"\nsteps = []\nstep = []\n";
        assert!(toml::from_str::<CardDef>(card).is_err());
    }

    #[test]
    fn dual_mana_spellings_parse_and_bad_ones_are_load_errors() {
        // A dual in a nested mana-add batch is a nested two-color array (one credit).
        let card = r#"name = "Test Talisman"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[kind]
type = "artifact"

[[abilities]]
timing = "activated"
taps_self = true

[[abilities.effects]]
type = "mana"
mode = "add"
mana = [["black", "green"]]
"#;
        let def: CardDef = toml::from_str(card).expect("a dual nested mana-add batch parses");
        let Effect::Mana(ManaEffect::Add { mana: produced, .. }) = def.abilities[0].effect else {
            panic!("expected a nested mana-add effect");
        };
        assert_eq!(
            produced,
            {
                let mut pool = engine::ManaPool::default();
                pool.add(Mana::Either(Color::Black, Color::Green), 1);
                pool
            },
            "one credit of either black or green"
        );

        // A 3-color array (a triome's fixed choice — Treva's Ruins) normalizes to `Mana::OfColors`.
        let triome = "name = \"Test Triome\"\n\n[kind]\ntype = \"artifact\"\n\n[[abilities]]\ntiming = \"activated\"\ntaps_self = true\n\n[[abilities.effects]]\ntype = \"mana\"\nmode = \"add\"\nmana = [[\"blue\", \"white\", \"green\"]]\n";
        let def: CardDef = toml::from_str(triome).expect("a 3-color nested mana-add batch parses");
        let Effect::Mana(ManaEffect::Add { mana: produced, .. }) = def.abilities[0].effect else {
            panic!("expected a nested mana-add effect");
        };
        assert_eq!(
            produced,
            {
                let mut pool = engine::ManaPool::default();
                let mask = 1 << Color::Blue.index()
                    | 1 << Color::White.index()
                    | 1 << Color::Green.index();
                pool.add(Mana::OfColors(mask), 1);
                pool
            },
            "one credit of blue, white, or green"
        );

        // A same-color "dual", a duplicate-color triome, and an out-of-range (1 or 5 color)
        // array are all load errors.
        for produces in [
            "[\"green\", \"green\"]",
            "[\"white\", \"blue\", \"white\"]",
            "[\"green\"]",
            "[\"white\", \"blue\", \"black\", \"red\", \"green\"]",
        ] {
            let card = format!(
                "name = \"Test Bad Dual\"\nid = \"00000000-0000-0000-0000-000000000001\"\ndefault_print = \"00000000-0000-0000-0000-000000000002\"\n\n[kind]\ntype = \"land\"\nproduces = {produces}\n"
            );
            assert!(
                toml::from_str::<CardDef>(&card).is_err(),
                "{produces} must not parse"
            );
        }
    }

    #[test]
    fn create_token_resolves_oracle_id_from_token_registry() {
        // Install tokens via the normal load path, then parse a card that only names an id.
        let _ = registry();
        let pest_id = "37c4adc8-7455-4725-95fb-169a8b0254e5";
        let food_id = "a468338f-635e-4206-89d6-72d723071d45";
        let inkling_id = "fbdbff76-c1ea-47ea-bfcc-7c64c23dad70";

        let pest = format!(
            r#"name = "Make Pest (test)"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "token"
mode = "create"
count = 1
token = "{pest_id}"
"#
        );
        let def: CardDef = toml::from_str(&pest).expect("token id resolves");
        let Effect::Token(TokenEffect::Create { token, .. }) = &def.abilities[0].effect else {
            panic!("expected a nested token-create effect");
        };
        assert_eq!(token.name, "Pest");
        assert_eq!(token.cost, Cost::FREE, "a token has no mana cost");
        assert_eq!(
            token.kind,
            CardKind::Creature {
                power: 1,
                toughness: 1,
                also: TypeSet::NONE,
            }
        );
        assert_eq!(token.abilities[0].timing, Timing::Triggered(Trigger::Dies));
        assert!(matches!(
            token.abilities[0].effect,
            Effect::Life(LifeEffect::Gain {
                amount: Amount::Fixed(1)
            })
        ));

        let food = format!(
            r#"name = "Make Food (test)"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "token"
mode = "create"
count = 1
token = "{food_id}"
"#
        );
        let def: CardDef = toml::from_str(&food).expect("food token id resolves");
        let Effect::Token(TokenEffect::Create { token, .. }) = &def.abilities[0].effect else {
            panic!("expected a nested token-create effect");
        };
        assert_eq!(
            token.kind,
            CardKind::Artifact,
            "a Food is an artifact token"
        );
        let Timing::Activated(ref cost) = token.abilities[0].timing else {
            panic!("Food has an activated ability");
        };
        assert!(cost.taps_self);
        assert_eq!(cost.sacrifice, SacrificeCost::This);
        assert_eq!(cost.mana.generic, 2);

        let inkling = format!(
            r#"name = "Make Inkling (test)"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "token"
mode = "create"
count = 1
token = "{inkling_id}"
"#
        );
        let def: CardDef = toml::from_str(&inkling).expect("inkling token id resolves");
        let Effect::Token(TokenEffect::Create { token, .. }) = &def.abilities[0].effect else {
            panic!("expected a nested token-create effect");
        };
        assert_eq!(
            token.kind,
            CardKind::Creature {
                power: 2,
                toughness: 1,
                also: TypeSet::NONE,
            }
        );
        assert!(token.keywords.contains(&Keyword::Flying));
    }

    #[test]
    fn create_token_rejects_unknown_and_inline_profiles() {
        let _ = registry();
        let unknown = r#"name = "Make Unknown (test)"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "token"
mode = "create"
token = "00000000-0000-0000-0000-000000000099"
"#;
        assert!(
            toml::from_str::<CardDef>(unknown).is_err(),
            "unknown token id must fail at load"
        );

        let inline = r#"name = "Make Inline (test)"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "token"
mode = "create"
token = { name = "Inkling", power = 2, toughness = 1 }
"#;
        assert!(
            toml::from_str::<CardDef>(inline).is_err(),
            "inline token tables are no longer accepted"
        );
    }

    /// Battlefield art is print-UUID-only (accounts-decks-and-catalog spec). Every resolved `create_token` profile
    /// must stamp `id` + `default_print` from `data/tokens/`.
    #[test]
    fn pool_token_profiles_carry_scryfall_art_ids() {
        fn collect_steps(steps: &[Effect], out: &mut Vec<(&'static str, CardDef)>) {
            for step in steps {
                collect(step, out);
            }
        }

        fn collect(effect: &Effect, out: &mut Vec<(&'static str, CardDef)>) {
            match effect {
                Effect::Token(TokenEffect::Create { token, .. })
                | Effect::Misc(MiscEffect::PreventCombatDamageToYouCreatingTokens { token })
                | Effect::Choice(ChoiceEffect::EachPlayerCreatesFractalFromExiledPower { token }) =>
                {
                    out.push((token.name, token.clone()));
                }
                Effect::Sequence { steps } => {
                    collect_steps(steps.as_ref(), out);
                }
                // Both branches: a token minted only in the else branch is still a pool token
                // that needs its art id, and nothing else in the build would catch it.
                Effect::Conditional {
                    then, otherwise, ..
                } => {
                    collect_steps(then.as_ref(), out);
                    collect_steps(otherwise, out);
                }
                Effect::Zone(ZoneEffect::ExileTargetGraveyardCardThenIfCreature { then }) => {
                    collect_steps(then, out);
                }
                Effect::Zone(ZoneEffect::ReflexiveTrigger { then }) => collect_steps(then, out),
                Effect::Damage(DamageEffect::ToEnteringPermanent { then, .. }) => {
                    collect_steps(then, out);
                }
                Effect::Misc(MiscEffect::ScheduleNextCastTrigger { then, .. }) => {
                    collect_steps(then, out);
                }
                Effect::Choice(ChoiceEffect::EachPlayerSacrifices { then, .. }) => {
                    collect_steps(then, out);
                }
                Effect::Choice(ChoiceEffect::MaySacrifice { then, .. }) => collect_steps(then, out),
                Effect::Choice(ChoiceEffect::MayDiscard { then }) => collect_steps(then, out),
                _ => {}
            }
        }

        fn scan_card(def: &CardDef, out: &mut Vec<(String, &'static str, CardDef)>) {
            let mut tokens = Vec::new();
            for ability in def.abilities.iter() {
                collect(&ability.effect, &mut tokens);
            }
            for hand in def.hand_ability.iter() {
                for effect in hand.effects.iter() {
                    collect(effect, &mut tokens);
                }
            }
            if let Some(forecast) = def.forecast.clone() {
                for effect in forecast.effects.iter() {
                    collect(effect, &mut tokens);
                }
            }
            if let Some(back) = def.back {
                let back = engine::card_def(back);
                scan_card(back.as_ref(), out);
            }
            if let Some(adventure) = def.adventure {
                let adventure = engine::card_def(adventure);
                scan_card(adventure.as_ref(), out);
            }
            for (name, token) in tokens {
                out.push((def.name.to_string(), name, token));
            }
        }

        let mut tokens = Vec::new();
        for def in registry().values() {
            scan_card(def, &mut tokens);
        }
        assert!(
            !tokens.is_empty(),
            "pool should mint at least one authored token profile"
        );
        let missing: Vec<_> = tokens
            .iter()
            .filter(|(_, _, t)| t.id.is_empty() || t.default_print.is_empty())
            .map(|(card, name, _)| format!("{card} → {name}"))
            .collect();
        assert!(
            missing.is_empty(),
            "token profiles need Scryfall id + default_print for battlefield art: {missing:?}"
        );
        for (_, _, token) in &tokens {
            let reg = get_token(token.id).unwrap_or_else(|| {
                panic!(
                    "create_token embeds id {} missing from token_registry",
                    token.id
                )
            });
            assert_eq!(reg.default_print, token.default_print);
        }

        let beast = get_by_name("Beast Within").expect("Beast Within in pool");
        let Effect::Sequence { steps } = &beast.abilities[0].effect else {
            panic!("Beast Within spell body should be a Sequence");
        };
        let Effect::Token(TokenEffect::Create { token, .. }) = &steps[1] else {
            panic!("expected create_token step");
        };
        assert_eq!(token.name, "Beast");
        assert_eq!(token.id, "6bb61f34-5d57-4eaa-a02c-f5d08c1ee920");
        assert_eq!(token.default_print, "5871be0a-0fd6-441d-8f9e-76c66b5bd8bc");
    }

    /// Regression: Rubinia shipped with a hallucinated frame — {2}{W}{U}{U}, 2/4, flying — which
    /// no ability-level test caught but the deck-legality identity check rejected live (every green
    /// card read as off-identity). Pin the printed frame (CR 903.4 identity flows from these pips).
    #[test]
    fn rubinia_soulsingers_printed_frame() {
        let rubinia = get_by_name("Rubinia Soulsinger").expect("Rubinia Soulsinger is in the pool");
        assert_eq!(
            rubinia.kind,
            CardKind::Creature {
                power: 2,
                toughness: 3,
                also: TypeSet::NONE
            }
        );
        assert_eq!(rubinia.cost.generic, 2);
        assert_eq!(rubinia.cost.colored[Color::Green.index()], 1);
        assert_eq!(rubinia.cost.colored[Color::White.index()], 1);
        assert_eq!(rubinia.cost.colored[Color::Blue.index()], 1);
        assert!(rubinia.legendary);
        assert!(
            rubinia.keywords.is_empty(),
            "the printed card has no keywords (no flying)"
        );
    }

    /// Agent Frank Horrigan's "has indestructible as long as it attacked this turn" is wired
    /// through `conditional_keywords`, not a plain printed keyword (increment
    /// `attacked-this-turn-condition`).
    #[test]
    fn agent_frank_horrigans_indestructible_is_conditional_on_having_attacked() {
        let frank =
            get_by_name("Agent Frank Horrigan").expect("Agent Frank Horrigan is in the pool");
        assert_eq!(
            frank.kind,
            CardKind::Creature {
                power: 8,
                toughness: 6,
                also: TypeSet::NONE
            }
        );
        assert!(frank.legendary);
        assert_eq!(*frank.keywords, [Keyword::Trample]);
        assert_eq!(
            *frank.conditional_keywords,
            [(Condition::SourceAttackedThisTurn, Keyword::Indestructible)]
        );
        assert_eq!(
            frank
                .abilities
                .iter()
                .filter(|a| a.timing == Timing::Triggered(Trigger::Etb))
                .count(),
            1,
            "one etb ability"
        );
        assert_eq!(
            frank
                .abilities
                .iter()
                .filter(|a| a.timing == Timing::Triggered(Trigger::Attacks))
                .count(),
            1,
            "one attacks ability"
        );
        // "proliferate twice" is one proliferate with `times = 2` (CR 701.27b repeats the
        // process, each repetition its own choice) — not two `times = 1` effects, which would
        // label as "Proliferate 1 times" twice and split one oracle clause across two blocks.
        for ability in frank.abilities.iter() {
            assert_eq!(
                ability.effect,
                Effect::Choice(ChoiceEffect::Proliferate {
                    times: Amount::Fixed(2)
                }),
                "each trigger proliferates twice in a single effect"
            );
        }
    }

    #[test]
    fn the_pool_loads_with_expected_card_shapes() {
        let bear = get_by_name("Grizzly Bears").expect("Grizzly Bears is in the pool");
        assert_eq!(
            bear.kind,
            CardKind::Creature {
                power: 2,
                toughness: 2,
                also: TypeSet::NONE
            }
        );
        assert_eq!(bear.cost.generic, 1);
        assert_eq!(bear.cost.colored[Color::Green.index()], 1);

        let shock = get_by_name("Shock").expect("Shock is in the pool");
        assert!(matches!(
            shock.abilities[0].effect,
            Effect::Damage(DamageEffect::Target {
                amount: Amount::Fixed(2),
                ..
            })
        ));

        // Catalog metadata backfilled from Scryfall: set codes for printing-aware coverage,
        // and creature subtypes for search.
        assert!(
            !bear.sets.is_empty(),
            "every backfilled card carries at least one set code"
        );
        let viper = get_by_name("Ambush Viper").expect("Ambush Viper is in the pool");
        assert!(
            viper.sets.contains(&"inr"),
            "Ambush Viper printings include inr: {:?}",
            viper.sets
        );
        assert!(
            viper.sets.len() > 1,
            "backfill lists every printing set, not only the old singular default: {:?}",
            viper.sets
        );
        assert_eq!(viper.subtypes.as_ref(), &["Snake"]);

        let starfield = get_by_name("Starfield Mystic").expect("Starfield Mystic is in the pool");
        assert!(
            starfield.otags.contains(&"cost-reducer-enchantment"),
            "otags backfilled from Scryfall: {:?}",
            starfield.otags
        );

        let elf = get_by_name("Llanowar Elves").expect("Llanowar Elves is in the pool");
        assert!(matches!(elf.abilities[0].timing, Timing::Activated(_)));
        let Effect::Mana(ManaEffect::Add { mana: produced, .. }) = elf.abilities[0].effect else {
            panic!("Llanowar Elves has a mana ability");
        };
        assert_eq!(produced.colored[Color::Green.index()], 1);

        // Sol Ring's {T}: Add {C}{C} — colorless (not a color) and a multi-mana batch.
        let sol_ring = get_by_name("Sol Ring").expect("Sol Ring is in the pool");
        let Effect::Mana(ManaEffect::Add { mana: sol, .. }) = sol_ring.abilities[0].effect else {
            panic!("Sol Ring taps for mana");
        };
        assert_eq!(sol.colorless, 2, "Sol Ring adds two colorless");
        assert_eq!(sol.colored, [0; Color::COUNT], "colorless is not a color");

        // Command Tower is a land that taps for one mana of the commander's color identity.
        let tower = get_by_name("Command Tower").expect("Command Tower is in the pool");
        assert_eq!(
            tower.kind,
            CardKind::Land {
                produces: Some(LandProduces::CommanderIdentity),
                subtypes: &[],
                basic: false,
            }
        );

        // Tangled Islet: "{T}: Add {G} or {U}" — a dual, spelled `produces = ["green",
        // "blue"]` in oracle order and normalized to WUBRG order at load. Land — Forest Island,
        // but nonbasic: it does not carry the "Basic" supertype despite sharing both basic
        // land types with Forest and Island.
        let islet = get_by_name("Tangled Islet").expect("Tangled Islet is in the pool");
        assert_eq!(
            islet.kind,
            CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Either(Color::Blue, Color::Green))),
                subtypes: &["Forest", "Island"],
                basic: false,
            }
        );
        assert!(islet.enters_tapped, "Tangled Islet enters tapped");

        let serra = get_by_name("Serra Angel").expect("Serra Angel is in the pool");
        assert!(serra.keywords.contains(&Keyword::Flying));
        assert!(serra.keywords.contains(&Keyword::Vigilance));

        let forest = get_by_name("Forest").expect("Forest is in the pool");
        assert_eq!(
            forest.kind,
            CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            }
        );
        assert!(!forest.legendary, "a basic land is not legendary");

        let tajic = get_by_name("Tajic, Legion's Edge").expect("Tajic is in the pool");
        assert!(
            tajic.legendary,
            "Tajic is a legendary creature (a commander)"
        );

        // Lightning Bolt: "3 damage to any target" — the modern any-target spec.
        let bolt = get_by_name("Lightning Bolt").expect("Lightning Bolt is in the pool");
        assert!(matches!(
            bolt.abilities[0].effect,
            Effect::Damage(DamageEffect::Target {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
                ..
            })
        ));

        // Laelia: an attack trigger that impulse-exiles the top card (play it until end of turn).
        let laelia = get_by_name("Laelia, the Blade Reforged").expect("Laelia is in the pool");
        assert!(laelia.keywords.contains(&Keyword::Haste));
        assert_eq!(
            laelia.abilities[0].timing,
            Timing::Triggered(Trigger::Attacks)
        );
        assert!(matches!(
            laelia.abilities[0].effect,
            Effect::Mill(MillEffect::ExileTopMayPlay {
                count: Amount::Fixed(1),
                until_next_turn: false,
                face_down: false,
                free_while_source: false,
            })
        ));

        // Expressive Iteration: look at the top three, route one each to hand/bottom/exile.
        let iteration =
            get_by_name("Expressive Iteration").expect("Expressive Iteration is in the pool");
        assert!(matches!(
            iteration.abilities[0].effect,
            Effect::Dig(DigEffect::DistributeTop {
                count: 3,
                to_hand: 1,
                to_bottom: 1,
                to_exile_may_play: 1,
            })
        ));

        // Containment Construct: a body-only 2/1 (its discard trigger is dropped).
        let construct =
            get_by_name("Containment Construct").expect("Containment Construct is in the pool");
        assert_eq!(
            construct.kind,
            CardKind::Creature {
                power: 2,
                toughness: 1,
                also: TypeSet::NONE
            }
        );

        // Ancestral Recall: "target player draws three cards" — a targeted-player draw.
        let recall = get_by_name("Ancestral Recall").expect("Ancestral Recall is in the pool");
        assert!(matches!(
            recall.abilities[0].effect,
            Effect::Draw(DrawEffect::TargetPlayer {
                count: Amount::Fixed(3),
                opponent: false,
            })
        ));

        // Sentinel's Eyes: an Aura granting +1/+1 and vigilance to the enchanted creature.
        let eyes = get_by_name("Sentinel's Eyes").expect("Sentinel's Eyes is in the pool");
        assert_eq!(eyes.kind, CardKind::Aura);
        let Effect::Static(StaticEffect::GrantToAttached {
            power,
            toughness,
            keywords,
            ..
        }) = eyes.abilities[0].effect
        else {
            panic!("Sentinel's Eyes grants a static buff to its host");
        };
        assert_eq!((power, toughness), (Amount::Fixed(1), Amount::Fixed(1)));
        assert_eq!(keywords, &[Keyword::Vigilance]);

        // Bonesplitter: an Equipment (+2/+0) with an Equip {1} activated ability.
        let bonesplitter = get_by_name("Bonesplitter").expect("Bonesplitter is in the pool");
        assert_eq!(bonesplitter.kind, CardKind::Artifact);
        assert!(matches!(
            bonesplitter.abilities[0].effect,
            Effect::Static(StaticEffect::GrantToAttached {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(0),
                ..
            })
        ));
        let equip = &bonesplitter.abilities[1];
        assert!(matches!(
            equip.effect,
            Effect::Control(ControlEffect::Equip)
        ));
        let Timing::Activated(cost) = equip.timing else {
            panic!("Equip is an activated ability");
        };
        assert_eq!(cost.mana.generic, 1, "Equip {{1}}");

        // Swords to Plowshares: "Exile target creature. Its controller gains life equal to its
        // power." — a life-gain rider then a zone-change removal.
        let swords =
            get_by_name("Swords to Plowshares").expect("Swords to Plowshares is in the pool");
        let Effect::Sequence { steps } = &swords.abilities[0].effect else {
            panic!("expected a two-step sequence");
        };
        assert!(matches!(
            steps[0],
            Effect::Life(LifeEffect::GainTargetController {
                amount: Amount::TargetPower
            })
        ));
        assert!(matches!(
            steps[1],
            Effect::Exile(ExileEffect::Target {
                target: TargetSpec::Creature,
                ..
            })
        ));

        // Unsummon: "Return target creature to its owner's hand" — a bounce.
        let unsummon = get_by_name("Unsummon").expect("Unsummon is in the pool");
        assert!(matches!(
            unsummon.abilities[0].effect,
            Effect::Zone(ZoneEffect::ReturnToHand {
                target: TargetSpec::Creature,
                ..
            })
        ));

        // Tome Scour: "Target player mills five cards" — a targeted mill.
        let tome = get_by_name("Tome Scour").expect("Tome Scour is in the pool");
        assert!(matches!(
            tome.abilities[0].effect,
            Effect::Mill(MillEffect::Mill {
                count: Amount::Fixed(5),
                target: TargetSpec::Player
            })
        ));

        // Blood Artist: "Whenever this creature or another creature dies, target player loses
        // 1 / you gain 1."
        let blood_artist = get_by_name("Blood Artist").expect("Blood Artist is in the pool");
        assert_eq!(
            blood_artist.abilities[0].timing,
            Timing::Triggered(Trigger::CreatureDiesIncludingThis),
        );
        assert!(matches!(
            blood_artist.abilities[0].effect,
            Effect::Life(LifeEffect::DrainTarget {
                amount: 1,
                opponent: false,
            })
        ));

        // Zulaport Cutthroat: "Whenever this creature or another creature you control dies,
        // each opponent loses 1 / you gain 1."
        let zulaport =
            get_by_name("Zulaport Cutthroat").expect("Zulaport Cutthroat is in the pool");
        assert_eq!(
            zulaport.abilities[0].timing,
            Timing::Triggered(Trigger::CreatureYouControlDiesIncludingThis),
        );
        assert!(matches!(
            zulaport.abilities[0].effect,
            Effect::Life(LifeEffect::EachOpponentDrain {
                amount: Amount::Fixed(1),
                sum_gain: false
            })
        ));

        // High Market: "{T}, Sacrifice a creature: You gain 1 life" — a sac-a-creature outlet
        // whose activation cost carries a `SacrificeCost::Creature`.
        let high_market = get_by_name("High Market").expect("High Market is in the pool");
        let Timing::Activated(sac_outlet) = high_market.abilities[1].timing else {
            panic!("High Market's second ability is activated");
        };
        assert!(matches!(
            sac_outlet.sacrifice,
            SacrificeCost::Creature { .. }
        ));
        assert!(sac_outlet.taps_self);
        assert!(matches!(
            high_market.abilities[1].effect,
            Effect::Life(LifeEffect::Gain {
                amount: Amount::Fixed(1)
            })
        ));

        // Mogg Fanatic: "Sacrifice this creature: It deals 1 damage to any target" — a
        // self-sacrifice cost (`SacrificeCost::This`).
        let mogg = get_by_name("Mogg Fanatic").expect("Mogg Fanatic is in the pool");
        let Timing::Activated(self_sac) = mogg.abilities[0].timing else {
            panic!("Mogg Fanatic's ability is activated");
        };
        assert_eq!(self_sac.sacrifice, SacrificeCost::This);
        assert!(matches!(
            mogg.abilities[0].effect,
            Effect::Damage(DamageEffect::Target {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
                ..
            })
        ));

        // Blaze: "{X}{R}. Blaze deals X damage to any target." — a variable-cost X burn.
        let blaze = get_by_name("Blaze").expect("Blaze is in the pool");
        assert!(blaze.cost.x > 0, "Blaze's cost includes {{X}}");
        assert_eq!(blaze.cost.colored[Color::Red.index()], 1, "…and one red");
        assert!(matches!(
            blaze.abilities[0].effect,
            Effect::Damage(DamageEffect::Target {
                amount: Amount::X,
                target: TargetSpec::AnyTarget,
                ..
            })
        ));

        // Raise Dead: "Return target creature card from your graveyard to your hand."
        let raise_dead = get_by_name("Raise Dead").expect("Raise Dead is in the pool");
        assert_eq!(raise_dead.cost.colored[Color::Black.index()], 1);
        assert!(matches!(
            raise_dead.abilities[0].effect,
            Effect::Zone(ZoneEffect::ReturnFromGraveyardToHand {
                target: TargetSpec::CreatureCardInYourGraveyard,
                ..
            })
        ));

        // Reanimate: "Put target creature card from a graveyard onto the battlefield under your
        // control. You lose life equal to that card's mana value." — reanimation from any
        // graveyard, then the mana-value life-loss rider.
        let reanimate = get_by_name("Reanimate").expect("Reanimate is in the pool");
        assert_eq!(reanimate.cost.colored[Color::Black.index()], 1);
        let Effect::Sequence { steps } = &reanimate.abilities[0].effect else {
            panic!("expected a two-step sequence");
        };
        assert!(matches!(
            steps[0],
            Effect::Zone(ZoneEffect::ReanimateToBattlefield {
                target: TargetSpec::CreatureCardInAnyGraveyard,
                ..
            })
        ));
        assert!(matches!(
            steps[1],
            Effect::Life(LifeEffect::Lose {
                amount: Amount::TargetManaValue
            })
        ));

        // Stroke of Genius: "{X}{2}{U}. Target player draws X cards." — a variable-cost draw.
        let stroke = get_by_name("Stroke of Genius").expect("Stroke of Genius is in the pool");
        assert!(stroke.cost.x > 0, "Stroke of Genius's cost includes {{X}}");
        assert_eq!(stroke.cost.generic, 2);
        assert_eq!(stroke.cost.colored[Color::Blue.index()], 1);
        assert!(matches!(
            stroke.abilities[0].effect,
            Effect::Draw(DrawEffect::TargetPlayer {
                count: Amount::X,
                opponent: false,
            })
        ));

        // Augury Owl: "When this creature enters, scry 3." — an ETB scry.
        let owl = get_by_name("Augury Owl").expect("Augury Owl is in the pool");
        assert_eq!(owl.abilities[0].timing, Timing::Triggered(Trigger::Etb));
        assert!(matches!(
            owl.abilities[0].effect,
            Effect::Dig(DigEffect::Scry {
                count: Amount::Fixed(3)
            })
        ));

        // Dimir Informant: "When this creature enters, surveil 2." — an ETB surveil.
        let informant = get_by_name("Dimir Informant").expect("Dimir Informant is in the pool");
        assert_eq!(
            informant.abilities[0].timing,
            Timing::Triggered(Trigger::Etb)
        );
        assert!(matches!(
            informant.abilities[0].effect,
            Effect::Dig(DigEffect::Surveil { count: 2 })
        ));

        // Marauding Raptor: "Creature spells you cast cost {1} less to cast." — a static,
        // color-agnostic creature-spell reducer.
        let raptor = get_by_name("Marauding Raptor").expect("Marauding Raptor is in the pool");
        assert_eq!(raptor.abilities[0].timing, Timing::Static);
        assert!(matches!(
            raptor.abilities[0].effect,
            Effect::Static(StaticEffect::ReduceSpellCost {
                amount: Amount::Fixed(1),
                filter: SpellFilter::CreatureSpells,
                ..
            })
        ));

        // Killian, Ink Duelist: "Spells you cast that target a creature cost {2} less to cast."
        let killian = get_by_name("Killian, Ink Duelist").expect("Killian is in the pool");
        assert!(killian.legendary);
        assert!(killian.keywords.contains(&Keyword::Lifelink));
        assert!(killian.keywords.contains(&Keyword::Menace));
        assert!(matches!(
            killian.abilities[0].effect,
            Effect::Static(StaticEffect::ReduceSpellCost {
                amount: Amount::Fixed(2),
                filter: SpellFilter::SpellsThatTargetACreature,
                ..
            })
        ));

        // Temple of Malady: a scry land whose ETB scries 1 (its enters-tapped / dual-mana
        // clauses are simplified — see the card's TOML).
        let temple = get_by_name("Temple of Malady").expect("Temple of Malady is in the pool");
        assert!(matches!(temple.kind, CardKind::Land { .. }));
        assert_eq!(temple.abilities[0].timing, Timing::Triggered(Trigger::Etb));
        assert!(matches!(
            temple.abilities[0].effect,
            Effect::Dig(DigEffect::Scry {
                count: Amount::Fixed(1)
            })
        ));

        // Besmirch: a sorcery that steals target creature until end of turn (with haste),
        // untaps it, and goads it.
        let besmirch = get_by_name("Besmirch").expect("Besmirch is in the pool");
        assert!(matches!(
            besmirch.kind,
            CardKind::Spell {
                speed: SpellSpeed::Sorcery
            }
        ));
        assert_eq!(besmirch.abilities[0].timing, Timing::Spell);
        let Effect::Sequence { steps } = &besmirch.abilities[0].effect else {
            panic!("Besmirch should resolve as a four-step sequence");
        };
        assert_eq!(steps.len(), 4);
        assert!(matches!(
            &steps[0],
            Effect::Control(ControlEffect::GainControlUntilEndOfTurn {
                target: TargetSpec::Creature
            })
        ));
        assert!(matches!(
            &steps[1],
            Effect::Pump(PumpEffect::PumpUntilEndOfTurn {
                target: TargetSpec::Creature,
                ..
            })
        ));
        assert!(matches!(
            &steps[2],
            Effect::Control(ControlEffect::UntapTarget {
                target: TargetSpec::Creature,
                ..
            })
        ));
        assert!(matches!(
            &steps[3],
            Effect::Control(ControlEffect::GoadTarget {
                target: TargetSpec::Creature
            })
        ));

        // Silverquill Charm: a modal "choose one" instant (CR 700.2). Its three spell-timed
        // abilities are its modes — two target a creature, one takes no target.
        let charm = get_by_name("Silverquill Charm").expect("Silverquill Charm is in the pool");
        assert!(charm.modal, "Silverquill Charm is a modal choose-one spell");
        assert!(matches!(
            charm.kind,
            CardKind::Spell {
                speed: SpellSpeed::Instant
            }
        ));
        assert_eq!(charm.abilities.len(), 3, "three modes");
        assert!(charm.abilities.iter().all(|a| a.timing == Timing::Spell));
        // Mode 0: put two +1/+1 counters on target creature.
        assert!(matches!(
            charm.abilities[0].effect,
            Effect::Counters(CountersEffect::PutCounters {
                count: Amount::Fixed(2),
                target: TargetSpec::Creature,
                ..
            })
        ));
        // Mode 1: exile target creature with power 2 or less.
        assert!(matches!(
            charm.abilities[1].effect,
            Effect::Exile(ExileEffect::Target {
                target: TargetSpec::Permanent(PermanentFilter {
                    power_max: Some(2),
                    ..
                }),
                ..
            })
        ));
        // Mode 2: each opponent loses 3 / you gain 3 — no target.
        assert!(matches!(
            charm.abilities[2].effect,
            Effect::Life(LifeEffect::EachOpponentDrain {
                amount: Amount::Fixed(3),
                sum_gain: false
            })
        ));

        // Quandrix Charm: a modal "choose one" instant — counter, destroy-enchantment, and
        // set-base-P/T-5/5 modes.
        let qcharm = get_by_name("Quandrix Charm").expect("Quandrix Charm is in the pool");
        assert!(qcharm.modal && qcharm.modal_choose == 1);
        assert_eq!(qcharm.abilities.len(), 3, "three modeled modes");
        assert!(matches!(
            qcharm.abilities[0].effect,
            Effect::Misc(MiscEffect::CounterTargetSpell {
                unless_pays: Some(Amount::Fixed(2)),
                ..
            })
        ));
        assert!(matches!(
            qcharm.abilities[1].effect,
            Effect::Destroy(DestroyEffect::Target {
                target: TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::ENCHANTMENT,
                    ..
                }),
                ..
            })
        ));
        assert!(matches!(
            qcharm.abilities[2].effect,
            Effect::Pump(PumpEffect::SetBasePtTargetUntilEndOfTurn {
                power: Amount::Fixed(5),
                toughness: Amount::Fixed(5),
                target: TargetSpec::Creature,
            })
        ));

        // Prismari Command: a modal "choose two" instant — four modes, pick two distinct.
        let prismari = get_by_name("Prismari Command").expect("Prismari Command is in the pool");
        assert!(prismari.modal && prismari.modal_choose == 2);
        assert_eq!(prismari.abilities.len(), 4, "four modes");
        assert!(prismari.abilities.iter().all(|a| a.timing == Timing::Spell));
        assert!(matches!(
            prismari.abilities[0].effect,
            Effect::Damage(DamageEffect::Target {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
                ..
            })
        ));
        assert_eq!(
            &prismari.abilities[1].effect,
            &Effect::Sequence {
                steps: std::sync::Arc::from([
                    Effect::Draw(DrawEffect::TargetPlayer {
                        count: Amount::Fixed(2),
                        opponent: false,
                    }),
                    Effect::Choice(ChoiceEffect::Discard {
                        count: 2,
                        target_player: true,
                        or_one_matching: None,
                    }),
                ]),
            }
        );
        assert!(matches!(
            prismari.abilities[2].effect,
            Effect::Token(TokenEffect::CreateTreasure {
                count: Amount::Fixed(1),
                target_player: true,
                ..
            })
        ));
        assert!(matches!(
            prismari.abilities[3].effect,
            Effect::Destroy(DestroyEffect::Target {
                target: TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::ARTIFACT,
                    ..
                }),
                ..
            })
        ));

        // Witherbloom Command: a modal "choose two" sorcery — four modes, pick two distinct.
        let wither =
            get_by_name("Witherbloom Command").expect("Witherbloom Command is in the pool");
        assert!(wither.modal && wither.modal_choose == 2);
        assert_eq!(wither.abilities.len(), 4, "four modes");
        let Effect::Sequence { steps } = &wither.abilities[0].effect else {
            panic!("Witherbloom Command's first mode should be a two-step sequence");
        };
        assert_eq!(steps.len(), 2);
        assert!(matches!(
            &steps[0],
            Effect::Mill(MillEffect::Mill {
                count: Amount::Fixed(3),
                target: TargetSpec::Player,
            })
        ));
        assert!(matches!(
            &steps[1],
            Effect::Choice(ChoiceEffect::MayReturnFromGraveyard {
                filter: CardFilter::Land,
                ..
            })
        ));
        assert!(matches!(
            wither.abilities[1].effect,
            Effect::Destroy(DestroyEffect::Target {
                target: TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::NONLAND,
                    exclude: TypeSet::CREATURE,
                    mv_max: Some(2),
                    ..
                }),
                ..
            })
        ));
        assert!(matches!(
            wither.abilities[2].effect,
            Effect::Pump(PumpEffect::PumpUntilEndOfTurn {
                power: Amount::Fixed(-3),
                toughness: Amount::Fixed(-1),
                target: TargetSpec::Creature,
                ..
            })
        ));
        assert!(matches!(
            wither.abilities[3].effect,
            Effect::Life(LifeEffect::DrainTarget {
                amount: 2,
                opponent: true,
            })
        ));

        // Quandrix Command: a modal "choose two" instant, all four printed modes modeled.
        let quandrix = get_by_name("Quandrix Command").expect("Quandrix Command is in the pool");
        assert!(quandrix.modal && quandrix.modal_choose == 2);
        assert_eq!(quandrix.abilities.len(), 4, "four modeled modes");
        match &quandrix.abilities[0].effect {
            Effect::Zone(ZoneEffect::ReturnToHand {
                target: TargetSpec::Permanent(filter),
                ..
            }) => {
                assert_eq!(filter.types, TypeSet::CREATURE.union(TypeSet::PLANESWALKER));
            }
            other => panic!("mode 0 should bounce a creature or planeswalker, got {other:?}"),
        }
        assert!(matches!(
            quandrix.abilities[1].effect,
            Effect::Misc(MiscEffect::CounterTargetSpell {
                unless_pays: None,
                filter: SpellFilter::ArtifactOrEnchantment,
                countered_dest: None,
            })
        ));
        assert!(matches!(
            quandrix.abilities[2].effect,
            Effect::Counters(CountersEffect::PutCounters {
                count: Amount::Fixed(2),
                target: TargetSpec::Creature,
                ..
            })
        ));
        assert!(matches!(
            quandrix.abilities[3].effect,
            Effect::Dig(DigEffect::ShuffleTargetCardsFromGraveyardIntoLibrary {
                max: 3,
                target_player: true,
            })
        ));

        // Killian, Decisive Mentor: the tap-and-goad half of the commander, on a watch for an
        // enchantment you control entering.
        let killian = get_by_name("Killian, Decisive Mentor").expect("Killian is in the pool");
        assert!(killian.legendary);
        assert!(matches!(
            killian.abilities[0].timing,
            Timing::Triggered(Trigger::PermanentEnters {
                filter: PermanentFilter {
                    types: TypeSet::ENCHANTMENT,
                    ..
                },
                controller: EnterController::You,
            })
        ));
        let Effect::Sequence { steps } = &killian.abilities[0].effect else {
            panic!("Killian's trigger should tap, then goad");
        };
        assert_eq!(steps.len(), 2);
        assert!(matches!(
            &steps[0],
            Effect::Control(ControlEffect::TapTarget {
                target: TargetSpec::Creature,
                ..
            })
        ));
        assert!(matches!(
            &steps[1],
            Effect::Control(ControlEffect::GoadTarget {
                target: TargetSpec::Creature
            })
        ));

        // Leonin Vanguard: an intervening-if trigger — "if you control three or more creatures"
        // gates a begin-combat self-pump + life gain.
        let leonin = get_by_name("Leonin Vanguard").expect("Leonin Vanguard is in the pool");
        assert_eq!(
            leonin.abilities[0].timing,
            Timing::Triggered(Trigger::BeginCombat)
        );
        assert_eq!(
            leonin.abilities[0].condition,
            Some(Condition::YouControlAtLeastCreatures { count: 3 })
        );
        let Effect::Sequence { steps } = &leonin.abilities[0].effect else {
            panic!("Leonin Vanguard should resolve as pump, then life gain");
        };
        assert_eq!(steps.len(), 2);
        assert!(matches!(
            &steps[0],
            Effect::Pump(PumpEffect::PumpSelfUntilEndOfTurn {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                ..
            })
        ));
        assert!(matches!(
            &steps[1],
            Effect::Life(LifeEffect::Gain {
                amount: Amount::Fixed(1)
            })
        ));

        // Breena, the Demagogue: a watch-others attack trigger with an intervening-if condition
        // and the composite "attacking player draws / you put two counters" effect.
        let breena = get_by_name("Breena, the Demagogue").expect("Breena is in the pool");
        assert!(breena.legendary);
        assert!(breena.keywords.contains(&Keyword::Flying));
        assert_eq!(
            breena.abilities[0].timing,
            Timing::Triggered(Trigger::PlayerAttacksYourOpponent)
        );
        assert_eq!(
            breena.abilities[0].condition,
            Some(Condition::AttackedOpponentHasMoreLifeThanAnotherOpponent)
        );
        assert!(matches!(
            breena.abilities[0].effect,
            Effect::Counters(CountersEffect::AttackerDrawsControllerCounters {
                attacker: None,
                counters: 2,
            })
        ));

        // Quintorius, History Chaser: a Lorehold planeswalker commander — starting loyalty 5, with
        // a +1 loyalty ability that may discard a card to draw two and mill one.
        let quintorius =
            get_by_name("Quintorius, History Chaser").expect("Quintorius is in the pool");
        assert!(quintorius.legendary);
        assert_eq!(quintorius.kind, CardKind::Planeswalker { loyalty: 5 });
        let Timing::Activated(plus_one) = quintorius.abilities[0].timing else {
            panic!("Quintorius's +1 is an activated (loyalty) ability");
        };
        assert_eq!(
            plus_one.loyalty,
            Some(1),
            "the ability's loyalty cost is +1"
        );
        let Effect::Choice(ChoiceEffect::MayDiscard { then }) = &quintorius.abilities[0].effect
        else {
            panic!("Quintorius's +1 is a may-discard rider");
        };
        assert_eq!(
            then.len(),
            2,
            "the discard rider has draw-then-mill follow-ups"
        );
        assert!(matches!(
            then[0],
            Effect::Draw(DrawEffect::Cards {
                count: Amount::Fixed(2)
            })
        ));
        assert!(matches!(
            then[1],
            Effect::Mill(MillEffect::MillSelf {
                count: Amount::Fixed(1)
            })
        ));

        // Rite of Replication: "Kicker {5} ... Create a token that's a copy of target creature.
        // If this spell was kicked, create five of those tokens instead." {2}{U}{U} sorcery.
        let rite = get_by_name("Rite of Replication").expect("Rite of Replication is in the pool");
        assert_eq!(rite.cost.generic, 2);
        assert_eq!(rite.cost.colored[Color::Blue.index()], 2);
        assert!(matches!(rite.cost.additional.kicker, Some(k) if k.generic == 5));
        assert!(matches!(
            rite.abilities[0].effect,
            Effect::Token(TokenEffect::CreateCopy {
                count: Amount::IfSpellKicked { then, else_ },
                target: TargetSpec::Creature,
                sacrifice_at_next_end_step: false,
                exile_at_next_end_step: false,
                haste: false,
                ..
            }) if *then == Amount::Fixed(5) && *else_ == Amount::Fixed(1)
        ));

        // Twincast: "Copy target instant or sorcery spell." — {U}{U} instant, targets a spell
        // on the stack (the "choose new targets" clause is simplified to same-targets).
        let twincast = get_by_name("Twincast").expect("Twincast is in the pool");
        assert_eq!(twincast.cost.colored[Color::Blue.index()], 2);
        assert!(matches!(
            twincast.kind,
            CardKind::Spell {
                speed: SpellSpeed::Instant
            }
        ));
        assert_eq!(twincast.abilities[0].timing, Timing::Spell);
        assert!(matches!(
            twincast.abilities[0].effect,
            Effect::Copy(CopyEffect::TargetSpell)
        ));

        // Hardened Scales: "…that many plus one." — a static +1 counter-replacement.
        let scales = get_by_name("Hardened Scales").expect("Hardened Scales is in the pool");
        assert_eq!(scales.kind, CardKind::Enchantment);
        assert_eq!(scales.abilities[0].timing, Timing::Static);
        assert!(matches!(
            scales.abilities[0].effect,
            Effect::Static(StaticEffect::CounterReplacement {
                add: 1,
                times: 1,
                ..
            })
        ));

        // Doubling Season: "…twice that many." — a static x2 token-creation replacement plus a
        // static x2 counter-replacement (times defaults to 1, so an adder can omit it; the doubler
        // sets it).
        let doubling = get_by_name("Doubling Season").expect("Doubling Season is in the pool");
        assert!(matches!(
            doubling.abilities[0].effect,
            Effect::Static(StaticEffect::TokenReplacement { times: 2 })
        ));
        assert!(matches!(
            doubling.abilities[1].effect,
            Effect::Static(StaticEffect::CounterReplacement {
                add: 0,
                times: 2,
                ..
            })
        ));

        // Diabolic Tutor: "Search your library for a card, put it into your hand, then shuffle."
        let tutor = get_by_name("Diabolic Tutor").expect("Diabolic Tutor is in the pool");
        assert_eq!(tutor.cost.generic, 2);
        assert_eq!(tutor.cost.colored[Color::Black.index()], 2);
        assert!(matches!(
            tutor.abilities[0].effect,
            Effect::Dig(DigEffect::SearchLibrary {
                filter: CardFilter::AnyCard,
                to_zone: SearchDest::Hand,
                tapped: false,
                ..
            })
        ));

        // Rampant Growth: "Search your library for a basic land card, put it onto the battlefield
        // tapped, then shuffle." — basic-land ramp.
        let ramp = get_by_name("Rampant Growth").expect("Rampant Growth is in the pool");
        assert!(matches!(
            ramp.abilities[0].effect,
            Effect::Dig(DigEffect::SearchLibrary {
                filter: CardFilter::BasicLand,
                to_zone: SearchDest::Battlefield,
                tapped: true,
                ..
            })
        ));

        // Terramorphic Expanse: "{T}, Sacrifice this land: search a basic land onto the
        // battlefield tapped, then shuffle." — a fetchland (no life cost).
        let terramorphic =
            get_by_name("Terramorphic Expanse").expect("Terramorphic Expanse is in the pool");
        assert!(matches!(terramorphic.kind, CardKind::Land { .. }));
        let Timing::Activated(fetch) = terramorphic.abilities[0].timing else {
            panic!("Terramorphic Expanse's fetch is an activated ability");
        };
        assert!(fetch.taps_self);
        assert_eq!(fetch.sacrifice, SacrificeCost::This);
        assert_eq!(
            fetch.pay_life,
            Amount::Fixed(0),
            "Terramorphic pays no life"
        );
        assert!(matches!(
            terramorphic.abilities[0].effect,
            Effect::Dig(DigEffect::SearchLibrary {
                filter: CardFilter::BasicLand,
                to_zone: SearchDest::Battlefield,
                tapped: true,
                ..
            })
        ));

        // Fabled Passage: same as Terramorphic (its "untap that land" rider is deferred).
        let fabled = get_by_name("Fabled Passage").expect("Fabled Passage is in the pool");
        let Timing::Activated(fabled_fetch) = fabled.abilities[0].timing else {
            panic!("Fabled Passage's fetch is an activated ability");
        };
        assert_eq!(fabled_fetch.sacrifice, SacrificeCost::This);
        assert_eq!(fabled_fetch.pay_life, Amount::Fixed(0));

        // Prismatic Vista: "{T}, Pay 1 life, Sacrifice this land: search a basic land onto the
        // battlefield (untapped), then shuffle." — the pay-life fetchland.
        let vista = get_by_name("Prismatic Vista").expect("Prismatic Vista is in the pool");
        let Timing::Activated(vista_fetch) = vista.abilities[0].timing else {
            panic!("Prismatic Vista's fetch is an activated ability");
        };
        assert!(vista_fetch.taps_self);
        assert_eq!(vista_fetch.sacrifice, SacrificeCost::This);
        assert_eq!(
            vista_fetch.pay_life,
            Amount::Fixed(1),
            "Prismatic Vista pays 1 life"
        );
        assert!(matches!(
            vista.abilities[0].effect,
            Effect::Dig(DigEffect::SearchLibrary {
                filter: CardFilter::BasicLand,
                to_zone: SearchDest::Battlefield,
                tapped: false,
                ..
            })
        ));

        // Goldvein Hydra: {X}{G} 0/0 that "enters with X +1/+1 counters", with vigilance/trample/
        // haste (its death -> Treasure rider is deferred).
        let hydra = get_by_name("Goldvein Hydra").expect("Goldvein Hydra is in the pool");
        assert!(hydra.cost.x > 0, "the hydra's cost includes {{X}}");
        assert_eq!(
            hydra.kind,
            CardKind::Creature {
                power: 0,
                toughness: 0,
                also: TypeSet::NONE
            }
        );
        assert!(hydra.keywords.contains(&Keyword::Trample));
        assert_eq!(hydra.abilities[0].timing, Timing::Static);
        assert!(matches!(
            hydra.abilities[0].effect,
            Effect::Static(StaticEffect::EntersWithCounters {
                amount: Amount::X,
                kind: None
            })
        ));

        // Blasphemous Act: "13 damage to each creature." — a fixed mass-damage wipe.
        let blasphemous = get_by_name("Blasphemous Act").expect("Blasphemous Act is in the pool");
        assert!(matches!(
            blasphemous.abilities[0].effect,
            Effect::Damage(DamageEffect::EachCreature {
                amount: Amount::Fixed(13),
                ..
            })
        ));

        // Chain Reaction: "X damage to each creature, X = creatures on the battlefield." — a
        // board-derived mass-damage wipe.
        let chain = get_by_name("Chain Reaction").expect("Chain Reaction is in the pool");
        assert!(matches!(
            chain.abilities[0].effect,
            Effect::Damage(DamageEffect::EachCreature {
                amount: Amount::PerCreatureOnBattlefield,
                ..
            })
        ));

        // Toxic Deluge: "pay X life, all creatures get -X/-X." — {X} models the life (see TOML).
        let deluge = get_by_name("Toxic Deluge").expect("Toxic Deluge is in the pool");
        assert!(deluge.cost.x > 0, "Toxic Deluge's X is the pay-X source");
        assert!(matches!(
            deluge.abilities[0].effect,
            Effect::Pump(PumpEffect::WeakenEachCreature {
                power: Amount::X,
                toughness: Amount::X,
                opponents_only: false,
            })
        ));

        // Winds of Rath: "destroy all creatures that aren't enchanted."
        let winds = get_by_name("Winds of Rath").expect("Winds of Rath is in the pool");
        assert!(matches!(
            winds.abilities[0].effect,
            Effect::Destroy(DestroyEffect::All {
                filter: PermanentFilter {
                    types: TypeSet::CREATURE,
                    enchanted: Some(false),
                    ..
                },
                cant_be_regenerated: true,
            })
        ));

        // Culling Ritual: "destroy each nonland permanent with mana value 2 or less. Add {B} or
        // {G} for each permanent destroyed this way." — a `Sequence` of the wipe, then the
        // count-derived mana rider.
        let culling = get_by_name("Culling Ritual").expect("Culling Ritual is in the pool");
        let Effect::Sequence { steps } = &culling.abilities[0].effect else {
            panic!("Culling Ritual's ability is a two-step Sequence (wipe, then mana rider)");
        };
        let [wipe, rider] = steps.as_ref() else {
            panic!("Culling Ritual's sequence should have exactly two steps");
        };
        assert!(matches!(
            wipe,
            Effect::Destroy(DestroyEffect::All {
                filter: PermanentFilter {
                    types: TypeSet::NONLAND,
                    mv_max: Some(2),
                    ..
                },
                ..
            })
        ));
        assert!(matches!(
            rider,
            Effect::Mana(ManaEffect::Add {
                repeat: Amount::PermanentsDestroyedThisWay { .. },
                ..
            })
        ));

        // Fracture: "destroy target artifact, enchantment, or planeswalker." — noncreature removal.
        let fracture = get_by_name("Fracture").expect("Fracture is in the pool");
        assert!(matches!(
            fracture.abilities[0].effect,
            Effect::Destroy(DestroyEffect::Target {
                target: TargetSpec::ArtifactEnchantmentOrPlaneswalker,
                ..
            })
        ));

        // Storm-Kiln Artist: "This creature gets +1/+0 for each artifact you control. Magecraft —
        // Whenever you cast or copy an instant or sorcery, create a Treasure token."
        let storm_kiln =
            get_by_name("Storm-Kiln Artist").expect("Storm-Kiln Artist is in the pool");
        assert_eq!(storm_kiln.abilities[0].timing, Timing::Static);
        assert!(matches!(
            storm_kiln.abilities[0].effect,
            Effect::Static(StaticEffect::Anthem {
                self_only: true,
                ..
            })
        ));
        assert_eq!(
            storm_kiln.abilities[1].timing,
            Timing::Triggered(Trigger::Magecraft)
        );
        assert!(matches!(
            storm_kiln.abilities[1].effect,
            Effect::Token(TokenEffect::CreateTreasure {
                count: Amount::Fixed(1),
                target_player: false,
                ..
            })
        ));

        // Big Score: "Draw two cards and create two Treasure tokens." — a non-modal instant with two
        // spell halves (its "discard a card" additional cost is deferred — see its TOML).
        let big_score = get_by_name("Big Score").expect("Big Score is in the pool");
        assert!(matches!(
            big_score.kind,
            CardKind::Spell {
                speed: SpellSpeed::Instant
            }
        ));
        assert!(matches!(
            big_score.abilities[0].effect,
            Effect::Draw(DrawEffect::Cards {
                count: Amount::Fixed(2)
            })
        ));
        assert!(matches!(
            big_score.abilities[1].effect,
            Effect::Token(TokenEffect::CreateTreasure {
                count: Amount::Fixed(2),
                target_player: false,
                ..
            })
        ));

        // Darksteel Myr: a {3} 0/1 artifact creature with intrinsic indestructible.
        let myr = get_by_name("Darksteel Myr").expect("Darksteel Myr is in the pool");
        assert!(myr.keywords.contains(&Keyword::Indestructible));

        // Ambush Viper: {1}{G} 2/1 with flash and deathtouch.
        let viper = get_by_name("Ambush Viper").expect("Ambush Viper is in the pool");
        assert!(viper.keywords.contains(&Keyword::Flash));
        assert!(viper.keywords.contains(&Keyword::Deathtouch));

        // Tomakul Honor Guard: {1}{G} 3/1 with Ward {2} (a parametrized keyword from a table).
        let guard = get_by_name("Tomakul Honor Guard").expect("Tomakul Honor Guard is in the pool");
        assert!(guard.keywords.contains(&Keyword::Ward(2)));

        // White Knight: {W}{W} 2/2 with first strike and protection from black.
        let knight = get_by_name("White Knight").expect("White Knight is in the pool");
        assert!(knight.keywords.contains(&Keyword::FirstStrike));
        assert!(
            knight
                .keywords
                .contains(&Keyword::ProtectionFrom(ProtectionScope::Color(
                    Color::Black
                )))
        );

        // Shielded by Faith: an Aura granting indestructible to the enchanted creature.
        let shielded = get_by_name("Shielded by Faith").expect("Shielded by Faith is in the pool");
        assert_eq!(shielded.kind, CardKind::Aura);
        let Effect::Static(StaticEffect::GrantToAttached { keywords, .. }) =
            shielded.abilities[0].effect
        else {
            panic!("Shielded by Faith grants a static keyword to its host");
        };
        assert_eq!(keywords, &[Keyword::Indestructible]);

        // Blight Mound makes a Pest token that carries its own death trigger ("When this token
        // dies, you gain 1 life") — a token profile that's a full inline card, not just P/T.
        // abilities[0] is the "attacking Pests get +1/+0 and menace" anthem; abilities[1] is the
        // death-trigger token maker.
        let blight = get_by_name("Blight Mound").expect("Blight Mound is in the pool");
        let Effect::Token(TokenEffect::Create { token: pest, .. }) = &blight.abilities[1].effect
        else {
            panic!("Blight Mound creates a Pest token");
        };
        assert_eq!(pest.name, "Pest");
        assert_eq!(pest.abilities[0].timing, Timing::Triggered(Trigger::Dies));
        assert!(matches!(
            pest.abilities[0].effect,
            Effect::Life(LifeEffect::Gain {
                amount: Amount::Fixed(1)
            })
        ));

        // Gilded Goose's ETB makes a Food — an *artifact* token whose own activated ability
        // sacrifices it ("{2}, {T}, Sacrifice this token: You gain 3 life").
        let goose = get_by_name("Gilded Goose").expect("Gilded Goose is in the pool");
        let Effect::Token(TokenEffect::Create { token: food, .. }) = &goose.abilities[0].effect
        else {
            panic!("Gilded Goose's ETB creates a Food token");
        };
        assert_eq!(food.name, "Food");
        assert_eq!(food.kind, CardKind::Artifact);
        let Timing::Activated(ref sac) = food.abilities[0].timing else {
            panic!("a Food has an activated sacrifice ability");
        };
        assert_eq!(sac.sacrifice, SacrificeCost::This);
        assert_eq!(sac.mana.generic, 2);
    }

    /// End-to-end through a migrated card: Skyclave Apparition's ETB exile targets "a nonland,
    /// nontoken permanent an opponent controls with mana value 4 or less" (#2 + #3). Drives the
    /// real target-legality pipeline (pool `CardDef` → `TargetSpec::Permanent` → `permanent_matches`)
    /// and checks the controller and mana-value axes gate the legal targets together.
    #[test]
    fn skyclave_apparitions_exile_gates_targets_by_controller_and_mana_value() {
        use engine::{Game, PlayerId, Target};

        const P0: PlayerId = PlayerId(0);
        const P1: PlayerId = PlayerId(1);

        let mut game = Game::with_players(2, 0);

        // Skyclave Apparition is a {1}{W}{W} 2/2 (mana value 3); its only ability (index 0) is the
        // ETB exile with the composable permanent filter.
        let skyclave =
            get_by_name("Skyclave Apparition").expect("Skyclave Apparition is in the pool");
        let source = game.spawn_on_battlefield(P0, skyclave.clone());

        // An opponent's mana-value-3 nontoken permanent — inside the gate, a legal target.
        let in_gate = game.spawn_on_battlefield(P1, skyclave);
        // An opponent's Sun Titan (mana value 6) — filtered out by the "4 or less" gate.
        let over_gate = game.spawn_on_battlefield(
            P1,
            get_by_name("Sun Titan").expect("Sun Titan is in the pool"),
        );

        let targets = game.legal_targets(source, Some(0));

        assert!(
            targets.contains(&Target::Object(in_gate)),
            "an opponent's mana-value-3 nontoken permanent is a legal target"
        );
        assert!(
            !targets.contains(&Target::Object(over_gate)),
            "Sun Titan (mana value 6) is filtered out by the mana-value-4-or-less gate"
        );
        assert!(
            !targets.contains(&Target::Object(source)),
            "Skyclave exiles an opponent's permanent, never one you control"
        );
    }

    #[test]
    fn an_effects_list_parses_into_an_ordered_sequence() {
        // Faithless Looting: "Draw two cards, then discard two cards" is one ability whose
        // `effects = [..]` list becomes an ordered Effect::Sequence.
        let loot = get_by_name("Faithless Looting").expect("Faithless Looting is in the pool");
        let Effect::Sequence { steps } = &loot.abilities[0].effect else {
            panic!("an `effects` list is an Effect::Sequence");
        };
        assert_eq!(
            steps.as_ref(),
            &[
                Effect::Draw(DrawEffect::Cards {
                    count: Amount::Fixed(2)
                }),
                Effect::Choice(ChoiceEffect::Discard {
                    count: 2,
                    target_player: false,
                    or_one_matching: None,
                }),
            ],
            "draw two, then discard two — in order"
        );

        // A one-element `effects` list stays a bare effect (not wrapped in a Sequence): Shock's
        // lone ability stays a bare DealDamage.
        let shock = get_by_name("Shock").expect("Shock is in the pool");
        assert!(matches!(
            shock.abilities[0].effect,
            Effect::Damage(DamageEffect::Target { .. })
        ));

        // The singular `effect` sugar was removed: only `effects` is accepted, so a lone `effect`
        // key is now an unknown-field load error.
        let bad = "name = \"Singular\"
id = \"00000000-0000-0000-0000-000000000001\"
default_print = \"00000000-0000-0000-0000-000000000002\"\nid = \"00000000-0000-0000-0000-000000000001\"\ndefault_print = \"00000000-0000-0000-0000-000000000002\"\n\n[kind]\ntype = \"sorcery\"\n\n[[abilities]]\ntiming = \"spell\"\neffect = { type = \"draw_cards\", count = 1 }\n";
        assert!(toml::from_str::<CardDef>(bad).is_err());

        // An ability with no effects at all is likewise a load error.
        let empty =
            "name = \"Empty\"
id = \"00000000-0000-0000-0000-000000000001\"
default_print = \"00000000-0000-0000-0000-000000000002\"\nid = \"00000000-0000-0000-0000-000000000001\"\ndefault_print = \"00000000-0000-0000-0000-000000000002\"\n\n[kind]\ntype = \"sorcery\"\n\n[[abilities]]\ntiming = \"spell\"\n";
        assert!(toml::from_str::<CardDef>(empty).is_err());
    }

    /// Unlimited Edition's creatures whose whole rules text is nothing, or nothing but bare
    /// keywords. Their fidelity is entirely frame fidelity, so the assertion is the frame.
    #[test]
    fn unlimited_vanilla_and_keyword_only_creatures_have_their_printed_frames() {
        let cases: &[(&str, i32, i32, &[Keyword])] = &[
            ("Air Elemental", 4, 4, &[Keyword::Flying]),
            ("Craw Wurm", 6, 4, &[]),
            ("Earth Elemental", 4, 5, &[]),
            ("Elvish Archers", 2, 1, &[Keyword::FirstStrike]),
            ("Fire Elemental", 5, 4, &[]),
            ("Giant Spider", 2, 4, &[Keyword::Reach]),
            ("Gray Ogre", 2, 2, &[]),
            ("Hill Giant", 3, 3, &[]),
            ("Hurloon Minotaur", 2, 3, &[]),
            ("Ironroot Treefolk", 3, 5, &[]),
            ("Mahamoti Djinn", 5, 6, &[Keyword::Flying]),
            ("Merfolk of the Pearl Trident", 1, 1, &[]),
            ("Mons's Goblin Raiders", 1, 1, &[]),
            ("Pearled Unicorn", 2, 2, &[]),
            ("Phantom Monster", 3, 3, &[Keyword::Flying]),
            ("Roc of Kher Ridges", 3, 3, &[Keyword::Flying]),
            ("Scathe Zombies", 2, 2, &[]),
            ("Scryb Sprites", 1, 1, &[Keyword::Flying]),
            ("Wall of Air", 1, 5, &[Keyword::Defender, Keyword::Flying]),
            ("Wall of Ice", 0, 7, &[Keyword::Defender]),
            ("Wall of Stone", 0, 8, &[Keyword::Defender]),
            (
                "Wall of Swords",
                3,
                5,
                &[Keyword::Defender, Keyword::Flying],
            ),
            ("Wall of Wood", 0, 3, &[Keyword::Defender]),
            ("War Mammoth", 3, 3, &[Keyword::Trample]),
            ("Water Elemental", 5, 4, &[]),
        ];
        for &(name, power, toughness, keywords) in cases {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert_eq!(
                card.kind,
                CardKind::Creature {
                    power,
                    toughness,
                    also: TypeSet::NONE
                },
                "{name} P/T"
            );
            let mut sorted = card.keywords.to_vec();
            sorted.sort_by_key(|k| format!("{k:?}"));
            let mut want = keywords.to_vec();
            want.sort_by_key(|k| format!("{k:?}"));
            assert_eq!(sorted, want, "{name} keywords");
            assert!(card.abilities.is_empty(), "{name} has no rules text");
            assert!(card.sets.contains(&"2ed"), "{name} was printed in 2ed");
        }

        // Obsianus Golem is the one artifact creature in the group — same shape, extra type.
        let golem = get_by_name("Obsianus Golem").expect("Obsianus Golem is in the pool");
        assert_eq!(
            golem.kind,
            CardKind::Creature {
                power: 4,
                toughness: 6,
                also: TypeSet::ARTIFACT
            }
        );
    }

    /// The original ten dual lands: no rules text at all, just two basic land types. The mana
    /// comes from the types (CR 305.6), which the DSL spells as a two-color `produces`.
    #[test]
    fn unlimited_dual_lands_tap_for_either_of_their_two_basic_types() {
        // Subtypes are in printed order; the color pair is in WUBRG order, which is how
        // `Mana::Either` normalizes an unordered pair.
        let cases: &[(&str, Color, Color, [&str; 2])] = &[
            ("Badlands", Color::Black, Color::Red, ["Swamp", "Mountain"]),
            ("Bayou", Color::Black, Color::Green, ["Swamp", "Forest"]),
            ("Plateau", Color::White, Color::Red, ["Mountain", "Plains"]),
            ("Savannah", Color::White, Color::Green, ["Forest", "Plains"]),
            ("Scrubland", Color::White, Color::Black, ["Plains", "Swamp"]),
            ("Taiga", Color::Red, Color::Green, ["Mountain", "Forest"]),
            (
                "Tropical Island",
                Color::Blue,
                Color::Green,
                ["Forest", "Island"],
            ),
            ("Tundra", Color::White, Color::Blue, ["Plains", "Island"]),
            (
                "Underground Sea",
                Color::Blue,
                Color::Black,
                ["Island", "Swamp"],
            ),
            (
                "Volcanic Island",
                Color::Blue,
                Color::Red,
                ["Island", "Mountain"],
            ),
        ];
        for &(name, a, b, subtypes) in cases {
            let land = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            let CardKind::Land {
                produces,
                subtypes: printed,
                basic,
            } = land.kind
            else {
                panic!("{name} is a land");
            };
            assert_eq!(
                produces,
                Some(LandProduces::Mana(Mana::Either(a, b))),
                "{name} taps for either of its two colors"
            );
            assert_eq!(printed, subtypes, "{name} printed land types");
            assert!(!basic, "{name} is nonbasic");
            assert!(land.abilities.is_empty(), "{name} has no rules text");
        }
    }

    /// Unlimited's mana artifacts. The Moxen tap for one colored mana; Black Lotus sacrifices for
    /// three of one color (CR 106.4, the `single_color` lock); Celestial Prism filters {2} into one
    /// mana of any color.
    #[test]
    fn unlimited_mana_artifacts_add_their_printed_mana() {
        for (name, color) in [
            ("Mox Pearl", Color::White),
            ("Mox Sapphire", Color::Blue),
            ("Mox Jet", Color::Black),
            ("Mox Ruby", Color::Red),
            ("Mox Emerald", Color::Green),
        ] {
            let mox = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert_eq!(mox.kind, CardKind::Artifact, "{name} is an artifact");
            assert_eq!(mox.cost, Cost::FREE, "{name} costs {{0}}");
            let ability = &mox.abilities[0];
            let Timing::Activated(activation) = ability.timing else {
                panic!("{name} has an activated ability");
            };
            assert!(activation.taps_self, "{name} taps for its mana");
            let Effect::Mana(ManaEffect::Add { mana, .. }) = ability.effect else {
                panic!("{name} has a mana ability");
            };
            assert_eq!(mana.colored[color.index()], 1, "{name} adds one {color:?}");
        }

        let lotus = get_by_name("Black Lotus").expect("Black Lotus is in the pool");
        let ability = &lotus.abilities[0];
        let Timing::Activated(activation) = ability.timing else {
            panic!("Black Lotus has an activated ability");
        };
        assert!(activation.taps_self);
        assert_eq!(activation.sacrifice, SacrificeCost::This);
        let Effect::Mana(ManaEffect::Add {
            mana, single_color, ..
        }) = ability.effect
        else {
            panic!("Black Lotus has a mana ability");
        };
        assert_eq!(mana.any, 3, "three mana");
        assert!(single_color, "…of any one color, not three different ones");

        let prism = get_by_name("Celestial Prism").expect("Celestial Prism is in the pool");
        let ability = &prism.abilities[0];
        let Timing::Activated(activation) = ability.timing else {
            panic!("Celestial Prism has an activated ability");
        };
        assert!(activation.taps_self);
        assert_eq!(activation.mana.generic, 2, "{{2}} in the activation cost");
        let Effect::Mana(ManaEffect::Add { mana, .. }) = ability.effect else {
            panic!("Celestial Prism has a mana ability");
        };
        assert_eq!(mana.any, 1, "one mana of any color");
    }

    /// Unlimited's Auras whose whole text is a static grant to the host: the five Wards, the
    /// strength cycle, and the keyword-granters.
    #[test]
    fn unlimited_auras_grant_their_printed_statics_to_the_enchanted_creature() {
        let cases: &[(&str, i32, i32, &[Keyword])] = &[
            (
                "White Ward",
                0,
                0,
                &[Keyword::ProtectionFrom(ProtectionScope::Color(
                    Color::White,
                ))],
            ),
            (
                "Blue Ward",
                0,
                0,
                &[Keyword::ProtectionFrom(ProtectionScope::Color(Color::Blue))],
            ),
            (
                "Black Ward",
                0,
                0,
                &[Keyword::ProtectionFrom(ProtectionScope::Color(
                    Color::Black,
                ))],
            ),
            (
                "Red Ward",
                0,
                0,
                &[Keyword::ProtectionFrom(ProtectionScope::Color(Color::Red))],
            ),
            (
                "Green Ward",
                0,
                0,
                &[Keyword::ProtectionFrom(ProtectionScope::Color(
                    Color::Green,
                ))],
            ),
            ("Fear", 0, 0, &[Keyword::Fear]),
            ("Flight", 0, 0, &[Keyword::Flying]),
            ("Lance", 0, 0, &[Keyword::FirstStrike]),
            ("Web", 0, 2, &[Keyword::Reach]),
            ("Holy Strength", 1, 2, &[]),
            ("Unholy Strength", 2, 1, &[]),
            ("Weakness", -2, -1, &[]),
            ("Holy Armor", 0, 2, &[]),
        ];
        for &(name, power, toughness, keywords) in cases {
            let aura = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert_eq!(aura.kind, CardKind::Aura, "{name} is an Aura");
            let grant = aura
                .abilities
                .iter()
                .find_map(|a| match a.effect {
                    Effect::Static(StaticEffect::GrantToAttached {
                        power: p,
                        toughness: t,
                        keywords: k,
                        ..
                    }) => Some((p, t, k)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name} grants something to its host"));
            assert_eq!(grant.0, Amount::Fixed(power), "{name} power grant");
            assert_eq!(grant.1, Amount::Fixed(toughness), "{name} toughness grant");
            assert_eq!(grant.2, keywords, "{name} keyword grant");
        }

        // A Ward grants protection from its own color, and the printed "This effect doesn't
        // remove this Aura" holds: the CR 704.5m sweep checks the `enchant` filter, not
        // protection, so the Aura stays attached to the creature it just made protected.
        let white_ward = get_by_name("White Ward").expect("White Ward is in the pool");
        assert_eq!(white_ward.cost.colored[Color::White.index()], 1);
    }

    /// The three Auras that hand their host a repeatable pump — the ability lives on the Aura and
    /// affects the enchanted creature, so it is activated, not granted.
    #[test]
    fn unlimited_pump_auras_activate_off_the_aura_onto_its_host() {
        for (name, activation_color, power, toughness) in [
            ("Blessing", Color::White, 1, 1),
            ("Holy Armor", Color::White, 0, 1),
            ("Firebreathing", Color::Red, 1, 0),
        ] {
            let aura = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            let ability = aura
                .abilities
                .iter()
                .find(|a| matches!(a.timing, Timing::Activated(_)))
                .unwrap_or_else(|| panic!("{name} has an activated ability"));
            let Timing::Activated(activation) = ability.timing else {
                unreachable!();
            };
            assert_eq!(
                activation.mana.colored[activation_color.index()],
                1,
                "{name} activation cost"
            );
            assert_eq!(
                ability.effect,
                Effect::Pump(PumpEffect::PumpUntilEndOfTurn {
                    power: Amount::Fixed(power),
                    toughness: Amount::Fixed(toughness),
                    // The pump lands on the creature this Aura enchants, not on a fresh target.
                    target: TargetSpec::EnchantedCreature,
                    keywords: &[],
                }),
                "{name} pumps its host"
            );
        }
    }

    /// Control Magic and Steal Artifact: the Aura's controller controls the enchanted permanent.
    /// Steal Artifact is the pool's proof that `enchant` restricts to a non-creature type.
    #[test]
    fn unlimited_control_auras_take_control_of_their_host() {
        for name in ["Control Magic", "Steal Artifact"] {
            let aura = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert!(
                aura.abilities
                    .iter()
                    .any(|a| matches!(a.effect, Effect::Static(StaticEffect::ControlAttached))),
                "{name} controls its host"
            );
        }

        let steal = get_by_name("Steal Artifact").expect("Steal Artifact is in the pool");
        assert_eq!(
            steal.enchant.map(|f| f.types),
            Some(TypeSet::ARTIFACT),
            "Steal Artifact enchants an artifact, not a creature"
        );
        assert_eq!(
            get_by_name("Control Magic").expect("in pool").enchant,
            None,
            "Control Magic's plain \"Enchant creature\" is the default"
        );
    }

    /// The one-shot removal spells: each destroys exactly the permanent its text names, and only
    /// Tunnel and Wrath of God deny regeneration.
    #[test]
    fn unlimited_removal_spells_destroy_exactly_what_they_name() {
        let spot: &[(&str, TargetSpec, bool)] = &[
            (
                "Ice Storm",
                TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::LAND,
                    ..PermanentFilter::default()
                }),
                false,
            ),
            (
                "Sinkhole",
                TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::LAND,
                    ..PermanentFilter::default()
                }),
                false,
            ),
            (
                "Stone Rain",
                TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::LAND,
                    ..PermanentFilter::default()
                }),
                false,
            ),
            (
                "Shatter",
                TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::ARTIFACT,
                    ..PermanentFilter::default()
                }),
                false,
            ),
            (
                "Disenchant",
                TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::ARTIFACT.union(TypeSet::ENCHANTMENT),
                    ..PermanentFilter::default()
                }),
                false,
            ),
            (
                "Tunnel",
                TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::CREATURE,
                    subtypes: &["Wall"],
                    ..PermanentFilter::default()
                }),
                true,
            ),
        ];
        for (name, target, cant_be_regenerated) in spot {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert_eq!(
                card.abilities[0].effect,
                Effect::Destroy(DestroyEffect::Target {
                    target: *target,
                    count: TargetCount::default(),
                    cant_be_regenerated: *cant_be_regenerated,
                }),
                "{name} destroys what it names"
            );
        }

        let sweepers: &[(&str, TypeSet, bool)] = &[
            ("Armageddon", TypeSet::LAND, false),
            ("Tranquility", TypeSet::ENCHANTMENT, false),
            ("Wrath of God", TypeSet::CREATURE, true),
        ];
        for (name, types, cant_be_regenerated) in sweepers {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert_eq!(
                card.abilities[0].effect,
                Effect::Destroy(DestroyEffect::All {
                    filter: PermanentFilter {
                        types: *types,
                        ..PermanentFilter::default()
                    },
                    cant_be_regenerated: *cant_be_regenerated,
                }),
                "{name} sweeps what it names"
            );
        }
    }

    /// Giant Growth, Jump and Howl from Beyond: the pool's plain until-end-of-turn combat tricks.
    /// Howl is the pool's first `+X/+0`, so its power reads off the spell's own `{X}`.
    #[test]
    fn unlimited_combat_tricks_pump_their_target_until_end_of_turn() {
        let cases: &[(&str, Amount, Amount, &[Keyword])] = &[
            ("Giant Growth", Amount::Fixed(3), Amount::Fixed(3), &[]),
            (
                "Jump",
                Amount::Fixed(0),
                Amount::Fixed(0),
                &[Keyword::Flying],
            ),
            ("Howl from Beyond", Amount::X, Amount::Fixed(0), &[]),
        ];
        for (name, power, toughness, keywords) in cases {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert_eq!(
                card.abilities[0].effect,
                Effect::Pump(PumpEffect::PumpUntilEndOfTurn {
                    power: *power,
                    toughness: *toughness,
                    target: TargetSpec::Creature,
                    keywords,
                }),
                "{name} pumps its target"
            );
        }
        assert!(
            get_by_name("Howl from Beyond").expect("in pool").cost.x > 0,
            "Howl from Beyond's {{X}} is what its power reads"
        );
    }

    /// Psionic Blast and Hurricane: the set's two damage spells that hit more than one thing.
    #[test]
    fn unlimited_burn_spells_deal_their_printed_damage() {
        let blast = get_by_name("Psionic Blast").expect("Psionic Blast is in the pool");
        let Effect::Sequence { steps } = &blast.abilities[0].effect else {
            panic!("Psionic Blast is two damage steps");
        };
        assert_eq!(
            steps.as_ref(),
            &[
                Effect::Damage(DamageEffect::Target {
                    amount: Amount::Fixed(4),
                    target: TargetSpec::AnyTarget,
                    count: TargetCount::default(),
                    divided: false,
                }),
                // The 2 to its own caster is damage, not life loss — Psionic Blast can be
                // prevented, redirected, or seen by a damage watcher like any other 2 damage.
                Effect::Damage(DamageEffect::ToSelf {
                    amount: Amount::Fixed(2)
                }),
            ],
            "4 to any target, then 2 to you"
        );

        let hurricane = get_by_name("Hurricane").expect("Hurricane is in the pool");
        let Effect::Sequence { steps } = &hurricane.abilities[0].effect else {
            panic!("Hurricane is a creature sweep plus a player sweep");
        };
        assert_eq!(
            steps.as_ref(),
            &[
                Effect::Damage(DamageEffect::EachCreature {
                    amount: Amount::X,
                    opponents_only: false,
                    filter: Some(PermanentFilter {
                        with_flying: true,
                        ..PermanentFilter::default()
                    }),
                    include_planeswalkers: false,
                }),
                Effect::Damage(DamageEffect::EachPlayer { amount: Amount::X }),
            ],
            "X to each flier and X to each player — the caster included"
        );
    }

    /// The set's remaining one-shots, each a single non-damage effect.
    #[test]
    fn unlimited_utility_spells_carry_their_printed_effects() {
        let expected: &[(&str, Effect)] = &[
            (
                "Counterspell",
                Effect::Misc(MiscEffect::CounterTargetSpell {
                    unless_pays: None,
                    filter: SpellFilter::default(),
                    countered_dest: None,
                }),
            ),
            (
                "Death Ward",
                Effect::Control(ControlEffect::RegenerateShield {
                    target: TargetSpec::Creature,
                }),
            ),
            (
                "Fog",
                Effect::Misc(MiscEffect::PreventAllCombatDamageThisTurn),
            ),
            (
                "Regrowth",
                Effect::Zone(ZoneEffect::ReturnFromGraveyardToHand {
                    target: TargetSpec::CardInGraveyard {
                        whose: GraveyardScope::Yours,
                        filter: CardFilter::AnyCard,
                        other: false,
                    },
                    count: TargetCount::default(),
                }),
            ),
            (
                "Stream of Life",
                Effect::Life(LifeEffect::TargetPlayerGains {
                    amount: Amount::X,
                    opponent: false,
                }),
            ),
        ];
        for (name, effect) in expected {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            assert_eq!(&card.abilities[0].effect, effect, "{name}'s printed effect");
        }

        // Demonic Tutor's unrestricted search: any card, straight to hand.
        let tutor = get_by_name("Demonic Tutor").expect("Demonic Tutor is in the pool");
        let Effect::Dig(DigEffect::SearchLibrary {
            filter,
            to_zone,
            count,
            ..
        }) = tutor.abilities[0].effect
        else {
            panic!("Demonic Tutor searches the library");
        };
        assert_eq!(
            (filter, to_zone, count),
            (CardFilter::AnyCard, SearchDest::Hand, 1),
            "one card of any kind, to hand"
        );

        // Twiddle's "tap or untap" is a two-mode choice, each mode carrying its own target.
        let twiddle = get_by_name("Twiddle").expect("Twiddle is in the pool");
        let artifact_creature_or_land = TargetSpec::Permanent(PermanentFilter {
            types: TypeSet::ARTIFACT
                .union(TypeSet::CREATURE)
                .union(TypeSet::LAND),
            ..PermanentFilter::default()
        });
        let Effect::ChooseOne { options } = &twiddle.abilities[0].effect else {
            panic!("Twiddle is a choose-one");
        };
        assert_eq!(
            options.as_ref(),
            &[
                Effect::Control(ControlEffect::TapTarget {
                    target: artifact_creature_or_land,
                    count: TargetCount::default(),
                }),
                Effect::Control(ControlEffect::UntapTarget {
                    target: artifact_creature_or_land,
                    count: TargetCount::default(),
                }),
            ],
            "tap or untap, same legal targets either way"
        );
    }

    /// The set's mana-sink permanents: a regeneration cycle and a self-pump cycle, each paying one
    /// pip of its own color for a repeatable effect on its own source.
    #[test]
    fn unlimited_mana_sinks_pay_one_pip_to_shield_or_pump_themselves() {
        let regenerators: &[(&str, Color, u8)] = &[
            ("Drudge Skeletons", Color::Black, 0),
            ("Uthden Troll", Color::Red, 0),
            ("Wall of Bone", Color::Black, 0),
            ("Wall of Brambles", Color::Green, 0),
            ("Will-o'-the-Wisp", Color::Black, 0),
        ];
        for (name, color, generic) in regenerators {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            let ability = card
                .abilities
                .iter()
                .find(|a| matches!(a.timing, Timing::Activated(_)))
                .unwrap_or_else(|| panic!("{name} has an activated ability"));
            let Timing::Activated(activation) = ability.timing else {
                unreachable!()
            };
            assert_eq!(activation.mana.generic, *generic, "{name}'s generic cost");
            assert_eq!(
                activation.mana.colored[color.index()],
                1,
                "{name} pays one pip of its own color"
            );
            assert!(!activation.taps_self, "{name} regenerates without tapping");
            assert_eq!(
                ability.effect,
                Effect::Control(ControlEffect::RegenerateShield {
                    target: TargetSpec::ThisPermanent,
                }),
                "{name} shields itself"
            );
        }

        // Living Wall's regeneration is the cycle's odd one out — a generic {1}, no colored pip.
        let living_wall = get_by_name("Living Wall").expect("Living Wall is in the pool");
        let Timing::Activated(activation) = living_wall.abilities[0].timing else {
            panic!("Living Wall regenerates on an activated ability");
        };
        assert_eq!(
            (activation.mana.generic, activation.mana.colored),
            (1, [0; Color::COUNT]),
            "Living Wall regenerates for a colorless {{1}}"
        );

        let pumpers: &[(&str, Color, i32, i32, &[Keyword])] = &[
            ("Frozen Shade", Color::Black, 1, 1, &[]),
            ("Wall of Fire", Color::Red, 1, 0, &[]),
            ("Wall of Water", Color::Blue, 1, 0, &[]),
            ("Granite Gargoyle", Color::Red, 0, 1, &[]),
            ("Shivan Dragon", Color::Red, 1, 0, &[]),
            (
                "Goblin Balloon Brigade",
                Color::Red,
                0,
                0,
                &[Keyword::Flying],
            ),
        ];
        for (name, color, power, toughness, keywords) in pumpers {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            let ability = &card.abilities[0];
            let Timing::Activated(activation) = ability.timing else {
                panic!("{name} pumps on an activated ability");
            };
            assert_eq!(
                activation.mana.colored[color.index()],
                1,
                "{name} pays one pip of its own color"
            );
            assert_eq!(
                ability.effect,
                Effect::Pump(PumpEffect::PumpSelfUntilEndOfTurn {
                    power: Amount::Fixed(*power),
                    toughness: Amount::Fixed(*toughness),
                    keywords,
                }),
                "{name} pumps itself"
            );
        }
    }

    /// Sedge Troll carries both halves at once — a conditional self-anthem and a regeneration.
    #[test]
    fn sedge_troll_grows_only_while_its_controller_holds_a_swamp() {
        let troll = get_by_name("Sedge Troll").expect("Sedge Troll is in the pool");
        let Effect::Static(StaticEffect::Anthem {
            power,
            toughness,
            self_only,
            condition,
            ..
        }) = troll.abilities[0].effect
        else {
            panic!("Sedge Troll's first ability is its conditional self-anthem");
        };
        assert_eq!(
            (power, toughness, self_only),
            (Amount::Fixed(1), Amount::Fixed(1), true),
            "+1/+1, and only to itself"
        );
        assert_eq!(
            condition,
            Some(Condition::ControlsLandsWithSubtype {
                subtypes: &["Swamp"],
                count: 1,
            }),
            "the +1/+1 is live only while you control a Swamp"
        );
        assert!(
            troll.abilities.iter().any(|a| a.effect
                == Effect::Control(ControlEffect::RegenerateShield {
                    target: TargetSpec::ThisPermanent
                })),
            "Sedge Troll also regenerates"
        );
    }

    /// The tap-to-do-something permanents: pingers, a card drawer, a tapper, and the removal.
    #[test]
    fn unlimited_tap_abilities_carry_their_printed_effects() {
        let expected: &[(&str, Effect)] = &[
            (
                "Prodigal Sorcerer",
                Effect::Damage(DamageEffect::Target {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::AnyTarget,
                    count: TargetCount::default(),
                    divided: false,
                }),
            ),
            (
                "Rod of Ruin",
                Effect::Damage(DamageEffect::Target {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::AnyTarget,
                    count: TargetCount::default(),
                    divided: false,
                }),
            ),
            (
                "Jayemdae Tome",
                Effect::Draw(DrawEffect::Cards {
                    count: Amount::Fixed(1),
                }),
            ),
            (
                "Royal Assassin",
                Effect::Destroy(DestroyEffect::Target {
                    target: TargetSpec::Permanent(PermanentFilter {
                        types: TypeSet::CREATURE,
                        tapped: Some(true),
                        ..PermanentFilter::default()
                    }),
                    count: TargetCount::default(),
                    cant_be_regenerated: false,
                }),
            ),
            (
                "Northern Paladin",
                Effect::Destroy(DestroyEffect::Target {
                    target: TargetSpec::Permanent(PermanentFilter {
                        color: ColorFilter::Black,
                        ..PermanentFilter::default()
                    }),
                    count: TargetCount::default(),
                    cant_be_regenerated: false,
                }),
            ),
            (
                "Dwarven Demolition Team",
                Effect::Destroy(DestroyEffect::Target {
                    target: TargetSpec::Permanent(PermanentFilter {
                        types: TypeSet::CREATURE,
                        subtypes: &["Wall"],
                        ..PermanentFilter::default()
                    }),
                    count: TargetCount::default(),
                    cant_be_regenerated: false,
                }),
            ),
            (
                "Ley Druid",
                Effect::Control(ControlEffect::UntapTarget {
                    target: TargetSpec::Permanent(PermanentFilter {
                        types: TypeSet::LAND,
                        ..PermanentFilter::default()
                    }),
                    count: TargetCount::default(),
                }),
            ),
            (
                "Dwarven Warriors",
                // "Can't be blocked this turn" is the `unblockable` keyword with no stat change.
                Effect::Pump(PumpEffect::PumpUntilEndOfTurn {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    target: TargetSpec::Permanent(PermanentFilter {
                        types: TypeSet::CREATURE,
                        power_max: Some(2),
                        ..PermanentFilter::default()
                    }),
                    keywords: &[Keyword::Unblockable],
                }),
            ),
        ];
        for (name, effect) in expected {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            let ability = &card.abilities[0];
            assert!(
                matches!(ability.timing, Timing::Activated(a) if a.taps_self),
                "{name} taps to activate"
            );
            assert_eq!(&ability.effect, effect, "{name}'s printed effect");
        }

        // Orcish Artillery's ping costs its own controller more life than it deals.
        let artillery = get_by_name("Orcish Artillery").expect("Orcish Artillery is in the pool");
        let Effect::Sequence { steps } = &artillery.abilities[0].effect else {
            panic!("Orcish Artillery deals damage twice");
        };
        assert_eq!(
            steps.as_ref(),
            &[
                Effect::Damage(DamageEffect::Target {
                    amount: Amount::Fixed(2),
                    target: TargetSpec::AnyTarget,
                    count: TargetCount::default(),
                    divided: false,
                }),
                Effect::Damage(DamageEffect::ToSelf {
                    amount: Amount::Fixed(3)
                }),
            ],
            "2 to any target, 3 to you"
        );
    }

    /// Nevinyrral's Disk and The Hive: the set's two artifacts whose frame carries a rider.
    #[test]
    fn unlimited_artifacts_enter_tapped_and_mint_their_named_token() {
        let disk = get_by_name("Nevinyrral's Disk").expect("Nevinyrral's Disk is in the pool");
        assert!(disk.enters_tapped, "the Disk enters tapped");
        assert_eq!(
            disk.abilities[0].effect,
            Effect::Destroy(DestroyEffect::All {
                filter: PermanentFilter {
                    types: TypeSet::ARTIFACT
                        .union(TypeSet::CREATURE)
                        .union(TypeSet::ENCHANTMENT),
                    ..PermanentFilter::default()
                },
                cant_be_regenerated: false,
            }),
            "the Disk sweeps artifacts, creatures and enchantments — itself included"
        );

        let hive = get_by_name("The Hive").expect("The Hive is in the pool");
        let Effect::Token(TokenEffect::Create { token: wasp, .. }) = &hive.abilities[0].effect
        else {
            panic!("The Hive mints a token");
        };
        assert_eq!(wasp.name, "Wasp");
        assert_eq!(wasp.keywords.as_ref(), &[Keyword::Flying]);
        assert_eq!(
            wasp.kind,
            CardKind::Creature {
                power: 1,
                toughness: 1,
                also: TypeSet::ARTIFACT,
            },
            "a 1/1 artifact creature"
        );
    }

    /// The set's three anthems differ only in who they reach: two colors, every battlefield;
    /// one attack lord, your side only.
    #[test]
    fn unlimited_anthems_buff_exactly_the_creatures_they_name() {
        for (
            name,
            want_colors,
            want_all_players,
            want_attacking_only,
            want_power,
            want_toughness,
        ) in [
            ("Bad Moon", &[Color::Black][..], true, false, 1, 1),
            ("Crusade", &[Color::White][..], true, false, 1, 1),
            ("Orcish Oriflamme", &[][..], false, true, 1, 0),
        ] {
            let card = get_by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"));
            let Effect::Static(StaticEffect::Anthem {
                power,
                toughness,
                colors,
                all_players,
                attacking_only,
                self_only,
                condition,
                subtypes,
                keywords,
                ..
            }) = card.abilities[0].effect
            else {
                panic!("{name} is an anthem");
            };
            assert_eq!(
                (power, toughness),
                (Amount::Fixed(want_power), Amount::Fixed(want_toughness)),
                "{name}"
            );
            assert_eq!(colors, want_colors, "{name} buffs only its named color");
            assert_eq!(
                all_players, want_all_players,
                "{name}: does the buff cross the table?"
            );
            assert_eq!(attacking_only, want_attacking_only, "{name}");
            assert!(
                !self_only && condition.is_none() && subtypes.is_empty() && keywords.is_empty(),
                "{name} carries no rider its oracle doesn't print"
            );
        }
    }

    /// The set's triggered abilities: each fires off a different watch, and the watch is the
    /// half a wrong port would silently get wrong.
    #[test]
    fn unlimited_triggers_fire_off_the_event_their_oracle_names() {
        // Sengir Vampire grows off kills it caused, not off every creature death.
        let vampire = get_by_name("Sengir Vampire").expect("Sengir Vampire is in the pool");
        assert_eq!(
            vampire.abilities[0].timing,
            Timing::Triggered(Trigger::CreatureDealtDamageByThisDies)
        );
        assert_eq!(
            vampire.abilities[0].effect,
            Effect::Counters(CountersEffect::PutCounters {
                count: Amount::Fixed(1),
                target: TargetSpec::ThisPermanent,
                targets: TargetCount::default(),
                kind: None,
                divided: false,
            })
        );

        // Phantasmal Forces is a 4/1 flier with rent due every upkeep.
        let forces = get_by_name("Phantasmal Forces").expect("Phantasmal Forces is in the pool");
        assert_eq!(
            forces.abilities[0].timing,
            Timing::Triggered(Trigger::Upkeep)
        );
        let Effect::Choice(ChoiceEffect::SacrificeSelfUnlessPay { cost }) =
            forces.abilities[0].effect
        else {
            panic!("Phantasmal Forces asks for rent");
        };
        assert_eq!(
            cost.colored[Color::Blue.index()],
            1,
            "{{U}}, not one generic"
        );
        assert_eq!(cost.generic, 0);

        // Verduran Enchantress watches enchantment casts, and the draw is optional.
        let enchantress =
            get_by_name("Verduran Enchantress").expect("Verduran Enchantress is in the pool");
        assert_eq!(
            enchantress.abilities[0].timing,
            Timing::Triggered(Trigger::CastSpell {
                filter: SpellFilter::Enchantment,
                caster: CasterScope::You,
                nth_each_turn: None,
                from_hand: false,
            })
        );
        assert!(enchantress.abilities[0].optional, "\"you may draw a card\"");
    }

    /// Pestilence: a sweeper you rent by the point, with a gate that removes it once the board
    /// is already empty.
    #[test]
    fn pestilence_sacrifices_itself_only_once_no_creatures_remain() {
        let pestilence = get_by_name("Pestilence").expect("Pestilence is in the pool");
        assert_eq!(
            pestilence.abilities[0].timing,
            Timing::Triggered(Trigger::EachEndStep),
            "every end step, not only yours"
        );
        assert_eq!(
            pestilence.abilities[0].condition,
            Some(Condition::NoCreaturesOnBattlefield),
            "an intervening-if, so a board that refills before resolution keeps it"
        );
        assert_eq!(
            pestilence.abilities[0].effect,
            Effect::Sacrifice(SacrificeEffect::Source)
        );

        let Effect::Sequence { steps } = &pestilence.abilities[1].effect else {
            panic!("Pestilence pings creatures and players");
        };
        assert_eq!(
            steps.as_ref(),
            &[
                Effect::Damage(DamageEffect::EachCreature {
                    amount: Amount::Fixed(1),
                    opponents_only: false,
                    filter: None,
                    include_planeswalkers: false,
                }),
                Effect::Damage(DamageEffect::EachPlayer {
                    amount: Amount::Fixed(1)
                }),
            ],
            "each creature and each player — its own controller included"
        );
        let Timing::Activated(activation) = pestilence.abilities[1].timing else {
            panic!("the ping is activated");
        };
        assert_eq!(activation.mana.colored[Color::Black.index()], 1);
        assert!(
            !activation.taps_self,
            "the ping is repeatable, not once a turn"
        );
    }

    /// Regeneration and Black Knight: the Aura that shields its host, and the knight whose
    /// protection is a parameterized keyword rather than an ability.
    #[test]
    fn unlimited_protection_is_a_keyword_and_regeneration_shields_its_host() {
        let regeneration = get_by_name("Regeneration").expect("Regeneration is in the pool");
        assert_eq!(
            regeneration.abilities[0].effect,
            Effect::Control(ControlEffect::RegenerateShield {
                target: TargetSpec::EnchantedCreature,
            }),
            "the shield lands on the host, not on the Aura"
        );

        let knight = get_by_name("Black Knight").expect("Black Knight is in the pool");
        assert_eq!(
            knight.keywords.as_ref(),
            &[
                Keyword::FirstStrike,
                Keyword::ProtectionFrom(ProtectionScope::Color(Color::White))
            ],
        );
        assert!(
            knight.abilities.is_empty(),
            "both halves are keywords — nothing to script"
        );
    }
}
