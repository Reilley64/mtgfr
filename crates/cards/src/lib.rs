//! The card pool as data: one TOML file per card under `data/`, loaded once into a
//! registry of `engine::CardDef`. The engine's `card-dsl` feature deserializes a card's
//! TOML directly into `CardDef` (interning owned strings and slices to `&'static` at
//! load, so `CardDef` stays `Copy` — a bounded, load-once pool that lives for the
//! program's lifetime anyway); this crate is just the file I/O and the name-keyed
//! registry, keeping the engine free of I/O (`CLAUDE.md`).

use std::collections::HashMap;
use std::sync::OnceLock;

use engine::CardDef;

/// The loaded card registry, keyed by card name. Reads `data/*.toml` on first access.
pub fn registry() -> &'static HashMap<String, CardDef> {
    static REGISTRY: OnceLock<HashMap<String, CardDef>> = OnceLock::new();
    REGISTRY.get_or_init(load_from_data_dir)
}

/// The card with the given name, if it exists in the pool.
pub fn get(name: &str) -> Option<CardDef> {
    registry().get(name).copied()
}

fn load_from_data_dir() -> HashMap<String, CardDef> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("reading card data dir {dir}: {e}"));

    let mut cards = HashMap::new();
    for entry in entries {
        let path = entry.expect("card data dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let def: CardDef =
            toml::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
        cards.insert(def.name.to_string(), def);
    }
    cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{
        Amount, CardFilter, CardKind, Color, Condition, Cost, Effect, EnterController, Keyword,
        LandProduces, Mana, PermanentFilter, ProtectionScope, SacrificeCost, SearchDest,
        SpellFilter, SpellSpeed, TargetSpec, Timing, Trigger, TypeSet,
    };

    #[test]
    fn every_pool_toml_loads_into_the_registry() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        let toml_files = std::fs::read_dir(dir)
            .expect("card data dir")
            .filter(|entry| {
                let path = entry.as_ref().expect("card data dir entry").path();
                path.extension().and_then(|e| e.to_str()) == Some("toml")
            })
            .count();
        assert!(toml_files > 400, "the soc pool is ~430 files");
        // registry() parses every file (panicking with the file's path on a bad one);
        // the count match also proves no two files define the same card name.
        assert_eq!(registry().len(), toml_files);
    }

    #[test]
    fn set_and_subtypes_parse_and_default_empty() {
        // Catalog metadata for deck-builder search: a set code and printed subtypes, both
        // optional. Present:
        let card = r#"name = "Goblin Test"
set = "soc"
subtypes = ["Goblin", "Wizard"]

[kind]
type = "creature"
power = 1
toughness = 1
"#;
        let def: CardDef = toml::from_str(card).expect("set + subtypes parse");
        assert_eq!(def.set, "soc");
        assert_eq!(def.subtypes, ["Goblin", "Wizard"]);

        // Omitted: both default empty, so every not-yet-backfilled card still loads.
        let bare = "name = \"Bare\"\n\n[kind]\ntype = \"creature\"\npower = 1\ntoughness = 1\n";
        let def: CardDef = toml::from_str(bare).expect("a card without the keys still parses");
        assert_eq!(def.set, "");
        assert!(def.subtypes.is_empty());
    }

    #[test]
    fn misspelled_toml_keys_are_load_errors() {
        // deny_unknown_fields: a typo'd key fails the parse instead of silently defaulting
        // (e.g. `legendery` would otherwise load as a non-legendary card).
        let card = "name = \"Typo\"\nlegendery = true\n\n[kind]\ntype = \"creature\"\npower = 1\ntoughness = 1\n";
        assert!(toml::from_str::<CardDef>(card).is_err());

        // The same guard inside an ability table: `tap_self` (missing s) must not
        // quietly produce a cost-free activated ability.
        let card = "name = \"Typo\"\n\n[kind]\ntype = \"creature\"\npower = 1\ntoughness = 1\n\n[[abilities]]\ntiming = \"activated\"\ntap_self = true\n\n[[abilities.effects]]\ntype = \"gain_life\"\namount = 1\n";
        assert!(toml::from_str::<CardDef>(card).is_err());
    }

    #[test]
    fn dual_mana_spellings_parse_and_bad_ones_are_load_errors() {
        // A dual in an `add_mana` batch is a nested two-color array (one credit).
        let card = "name = \"Test Talisman\"\n\n[kind]\ntype = \"artifact\"\n\n[[abilities]]\ntiming = \"activated\"\ntaps_self = true\n\n[[abilities.effects]]\ntype = \"add_mana\"\nmana = [[\"black\", \"green\"]]\n";
        let def: CardDef = toml::from_str(card).expect("a dual add_mana batch parses");
        let Effect::AddMana { mana: produced, .. } = def.abilities[0].effect else {
            panic!("expected an add_mana effect");
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
        let triome = "name = \"Test Triome\"\n\n[kind]\ntype = \"artifact\"\n\n[[abilities]]\ntiming = \"activated\"\ntaps_self = true\n\n[[abilities.effects]]\ntype = \"add_mana\"\nmana = [[\"blue\", \"white\", \"green\"]]\n";
        let def: CardDef = toml::from_str(triome).expect("a 3-color add_mana batch parses");
        let Effect::AddMana { mana: produced, .. } = def.abilities[0].effect else {
            panic!("expected an add_mana effect");
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
                "name = \"Test Bad Dual\"\n\n[kind]\ntype = \"land\"\nproduces = {produces}\n"
            );
            assert!(
                toml::from_str::<CardDef>(&card).is_err(),
                "{produces} must not parse"
            );
        }
    }

    #[test]
    fn a_token_profile_can_be_a_full_card_with_abilities() {
        // A token table is either the creature sugar (name + P/T + keywords) or a full inline
        // card (its own `kind` and `[[abilities]]`), which is what lets a token be an artifact
        // or carry a death/sac ability.

        // Pest: a 1/1 creature token with "When this token dies, you gain 1 life."
        let pest = r#"name = "Make Pest (test)"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "create_token"
count = 1

[abilities.effects.token]
name = "Pest"

[abilities.effects.token.kind]
type = "creature"
power = 1
toughness = 1

[[abilities.effects.token.abilities]]
timing = "dies"

[[abilities.effects.token.abilities.effects]]
type = "gain_life"
amount = 1
"#;
        let def: CardDef = toml::from_str(pest).expect("a full-card creature token parses");
        let Effect::CreateToken { token, .. } = def.abilities[0].effect else {
            panic!("expected a create_token effect");
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
            Effect::GainLife {
                amount: Amount::Fixed(1)
            }
        ));

        // Food: an *artifact* token with "{2}, {T}, Sacrifice this token: You gain 3 life."
        let food = r#"name = "Make Food (test)"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "create_token"
count = 1

[abilities.effects.token]
name = "Food"

[abilities.effects.token.kind]
type = "artifact"

[[abilities.effects.token.abilities]]
timing = "activated"
taps_self = true
sacrifice = "this"

[abilities.effects.token.abilities.activation_cost]
generic = 2

[[abilities.effects.token.abilities.effects]]
type = "gain_life"
amount = 3
"#;
        let def: CardDef = toml::from_str(food).expect("a full-card artifact token parses");
        let Effect::CreateToken { token, .. } = def.abilities[0].effect else {
            panic!("expected a create_token effect");
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

        // The minimal creature sugar (no `kind` table) still parses as base P/T + keywords.
        let inkling = r#"name = "Make Inkling (test)"

[kind]
type = "sorcery"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "create_token"
count = 1

[abilities.effects.token]
name = "Inkling"
power = 1
toughness = 1
keywords = ["flying"]
"#;
        let def: CardDef = toml::from_str(inkling).expect("the minimal token sugar still parses");
        let Effect::CreateToken { token, .. } = def.abilities[0].effect else {
            panic!("expected a create_token effect");
        };
        assert_eq!(
            token.kind,
            CardKind::Creature {
                power: 1,
                toughness: 1,
                also: TypeSet::NONE,
            }
        );
        assert!(token.keywords.contains(&Keyword::Flying));
        assert!(token.abilities.is_empty());
    }

    #[test]
    fn the_pool_loads_with_expected_card_shapes() {
        let bear = get("Grizzly Bear").expect("Grizzly Bear is in the pool");
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

        let shock = get("Shock").expect("Shock is in the pool");
        assert!(matches!(
            shock.abilities[0].effect,
            Effect::DealDamage {
                amount: Amount::Fixed(2),
                ..
            }
        ));

        // Catalog metadata backfilled from Scryfall (tooling/backfill-card-meta.mjs): a set code
        // on every card, and creature subtypes for search.
        assert!(
            !bear.set.is_empty(),
            "every backfilled card carries a set code"
        );
        let viper = get("Ambush Viper").expect("Ambush Viper is in the pool");
        assert_eq!(viper.set, "inr");
        assert_eq!(viper.subtypes, ["Snake"]);

        let starfield = get("Starfield Mystic").expect("Starfield Mystic is in the pool");
        assert!(
            starfield.otags.contains(&"cost-reducer-enchantment"),
            "otags backfilled from Scryfall: {:?}",
            starfield.otags
        );

        let elf = get("Llanowar Elves").expect("Llanowar Elves is in the pool");
        assert!(matches!(elf.abilities[0].timing, Timing::Activated(_)));
        let Effect::AddMana { mana: produced, .. } = elf.abilities[0].effect else {
            panic!("Llanowar Elves has a mana ability");
        };
        assert_eq!(produced.colored[Color::Green.index()], 1);

        // Sol Ring's {T}: Add {C}{C} — colorless (not a color) and a multi-mana batch.
        let sol_ring = get("Sol Ring").expect("Sol Ring is in the pool");
        let Effect::AddMana { mana: sol, .. } = sol_ring.abilities[0].effect else {
            panic!("Sol Ring taps for mana");
        };
        assert_eq!(sol.colorless, 2, "Sol Ring adds two colorless");
        assert_eq!(sol.colored, [0; Color::COUNT], "colorless is not a color");

        // Command Tower is a land that taps for one mana of the commander's color identity.
        let tower = get("Command Tower").expect("Command Tower is in the pool");
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
        let islet = get("Tangled Islet").expect("Tangled Islet is in the pool");
        assert_eq!(
            islet.kind,
            CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Either(Color::Blue, Color::Green))),
                subtypes: &["Forest", "Island"],
                basic: false,
            }
        );
        assert!(islet.enters_tapped, "Tangled Islet enters tapped");

        let serra = get("Serra Angel").expect("Serra Angel is in the pool");
        assert!(serra.keywords.contains(&Keyword::Flying));
        assert!(serra.keywords.contains(&Keyword::Vigilance));

        let forest = get("Forest").expect("Forest is in the pool");
        assert_eq!(
            forest.kind,
            CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            }
        );
        assert!(!forest.legendary, "a basic land is not legendary");

        let tajic = get("Tajic, Legion's Edge").expect("Tajic is in the pool");
        assert!(
            tajic.legendary,
            "Tajic is a legendary creature (a commander)"
        );

        // Lightning Bolt: "3 damage to any target" — the modern any-target spec.
        let bolt = get("Lightning Bolt").expect("Lightning Bolt is in the pool");
        assert!(matches!(
            bolt.abilities[0].effect,
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
                ..
            }
        ));

        // Laelia: an attack trigger that impulse-exiles the top card (play it until end of turn).
        let laelia = get("Laelia, the Blade Reforged").expect("Laelia is in the pool");
        assert!(laelia.keywords.contains(&Keyword::Haste));
        assert_eq!(
            laelia.abilities[0].timing,
            Timing::Triggered(Trigger::Attacks)
        );
        assert!(matches!(
            laelia.abilities[0].effect,
            Effect::ExileTopMayPlay {
                count: Amount::Fixed(1),
                until_next_turn: false,
            }
        ));

        // Expressive Iteration: look at the top three, route one each to hand/bottom/exile.
        let iteration = get("Expressive Iteration").expect("Expressive Iteration is in the pool");
        assert!(matches!(
            iteration.abilities[0].effect,
            Effect::DistributeTop {
                count: 3,
                to_hand: 1,
                to_bottom: 1,
                to_exile_may_play: 1,
            }
        ));

        // Containment Construct: a body-only 2/1 (its discard trigger is dropped).
        let construct = get("Containment Construct").expect("Containment Construct is in the pool");
        assert_eq!(
            construct.kind,
            CardKind::Creature {
                power: 2,
                toughness: 1,
                also: TypeSet::NONE
            }
        );

        // Ancestral Recall: "target player draws three cards" — a targeted-player draw.
        let recall = get("Ancestral Recall").expect("Ancestral Recall is in the pool");
        assert!(matches!(
            recall.abilities[0].effect,
            Effect::TargetPlayerDraws {
                count: Amount::Fixed(3),
                opponent: false,
            }
        ));

        // Sentinel's Eyes: an Aura granting +1/+1 and vigilance to the enchanted creature.
        let eyes = get("Sentinel's Eyes").expect("Sentinel's Eyes is in the pool");
        assert_eq!(eyes.kind, CardKind::Aura);
        let Effect::GrantToAttached {
            power,
            toughness,
            keywords,
            ..
        } = eyes.abilities[0].effect
        else {
            panic!("Sentinel's Eyes grants a static buff to its host");
        };
        assert_eq!((power, toughness), (Amount::Fixed(1), Amount::Fixed(1)));
        assert_eq!(keywords, &[Keyword::Vigilance]);

        // Bonesplitter: an Equipment (+2/+0) with an Equip {1} activated ability.
        let bonesplitter = get("Bonesplitter").expect("Bonesplitter is in the pool");
        assert_eq!(bonesplitter.kind, CardKind::Artifact);
        assert!(matches!(
            bonesplitter.abilities[0].effect,
            Effect::GrantToAttached {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(0),
                ..
            }
        ));
        let equip = bonesplitter.abilities[1];
        assert!(matches!(equip.effect, Effect::Equip));
        let Timing::Activated(cost) = equip.timing else {
            panic!("Equip is an activated ability");
        };
        assert_eq!(cost.mana.generic, 1, "Equip {{1}}");

        // Swords to Plowshares: "Exile target creature. Its controller gains life equal to its
        // power." — a life-gain rider then a zone-change removal.
        let swords = get("Swords to Plowshares").expect("Swords to Plowshares is in the pool");
        let Effect::Sequence { steps } = swords.abilities[0].effect else {
            panic!("expected a two-step sequence");
        };
        assert!(matches!(
            steps[0],
            Effect::GainLifeTargetController {
                amount: Amount::TargetPower
            }
        ));
        assert!(matches!(
            steps[1],
            Effect::ExileTarget {
                target: TargetSpec::Creature,
                ..
            }
        ));

        // Unsummon: "Return target creature to its owner's hand" — a bounce.
        let unsummon = get("Unsummon").expect("Unsummon is in the pool");
        assert!(matches!(
            unsummon.abilities[0].effect,
            Effect::ReturnToHand {
                target: TargetSpec::Creature,
                ..
            }
        ));

        // Tome Scour: "Target player mills five cards" — a targeted mill.
        let tome = get("Tome Scour").expect("Tome Scour is in the pool");
        assert!(matches!(
            tome.abilities[0].effect,
            Effect::Mill {
                count: Amount::Fixed(5),
                target: TargetSpec::Player
            }
        ));

        // Blood Artist: "Whenever this creature or another creature dies, target player loses
        // 1 / you gain 1."
        let blood_artist = get("Blood Artist").expect("Blood Artist is in the pool");
        assert_eq!(
            blood_artist.abilities[0].timing,
            Timing::Triggered(Trigger::CreatureDiesIncludingThis),
        );
        assert!(matches!(
            blood_artist.abilities[0].effect,
            Effect::DrainTarget {
                amount: 1,
                opponent: false,
            }
        ));

        // Zulaport Cutthroat: "Whenever this creature or another creature you control dies,
        // each opponent loses 1 / you gain 1."
        let zulaport = get("Zulaport Cutthroat").expect("Zulaport Cutthroat is in the pool");
        assert_eq!(
            zulaport.abilities[0].timing,
            Timing::Triggered(Trigger::CreatureYouControlDiesIncludingThis),
        );
        assert!(matches!(
            zulaport.abilities[0].effect,
            Effect::EachOpponentDrain {
                amount: Amount::Fixed(1),
                sum_gain: false
            }
        ));

        // High Market: "{T}, Sacrifice a creature: You gain 1 life" — a sac-a-creature outlet
        // whose activation cost carries a `SacrificeCost::Creature`.
        let high_market = get("High Market").expect("High Market is in the pool");
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
            Effect::GainLife {
                amount: Amount::Fixed(1)
            }
        ));

        // Mogg Fanatic: "Sacrifice this creature: It deals 1 damage to any target" — a
        // self-sacrifice cost (`SacrificeCost::This`).
        let mogg = get("Mogg Fanatic").expect("Mogg Fanatic is in the pool");
        let Timing::Activated(self_sac) = mogg.abilities[0].timing else {
            panic!("Mogg Fanatic's ability is activated");
        };
        assert_eq!(self_sac.sacrifice, SacrificeCost::This);
        assert!(matches!(
            mogg.abilities[0].effect,
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
                ..
            }
        ));

        // Blaze: "{X}{R}. Blaze deals X damage to any target." — a variable-cost X burn.
        let blaze = get("Blaze").expect("Blaze is in the pool");
        assert!(blaze.cost.x > 0, "Blaze's cost includes {{X}}");
        assert_eq!(blaze.cost.colored[Color::Red.index()], 1, "…and one red");
        assert!(matches!(
            blaze.abilities[0].effect,
            Effect::DealDamage {
                amount: Amount::X,
                target: TargetSpec::AnyTarget,
                ..
            }
        ));

        // Raise Dead: "Return target creature card from your graveyard to your hand."
        let raise_dead = get("Raise Dead").expect("Raise Dead is in the pool");
        assert_eq!(raise_dead.cost.colored[Color::Black.index()], 1);
        assert!(matches!(
            raise_dead.abilities[0].effect,
            Effect::ReturnFromGraveyardToHand {
                target: TargetSpec::CreatureCardInYourGraveyard
            }
        ));

        // Reanimate: "Put target creature card from a graveyard onto the battlefield under your
        // control. You lose life equal to that card's mana value." — reanimation from any
        // graveyard, then the mana-value life-loss rider.
        let reanimate = get("Reanimate").expect("Reanimate is in the pool");
        assert_eq!(reanimate.cost.colored[Color::Black.index()], 1);
        let Effect::Sequence { steps } = reanimate.abilities[0].effect else {
            panic!("expected a two-step sequence");
        };
        assert!(matches!(
            steps[0],
            Effect::ReanimateToBattlefield {
                target: TargetSpec::CreatureCardInAnyGraveyard,
                ..
            }
        ));
        assert!(matches!(
            steps[1],
            Effect::LoseLife {
                amount: Amount::TargetManaValue
            }
        ));

        // Stroke of Genius: "{X}{2}{U}. Target player draws X cards." — a variable-cost draw.
        let stroke = get("Stroke of Genius").expect("Stroke of Genius is in the pool");
        assert!(stroke.cost.x > 0, "Stroke of Genius's cost includes {{X}}");
        assert_eq!(stroke.cost.generic, 2);
        assert_eq!(stroke.cost.colored[Color::Blue.index()], 1);
        assert!(matches!(
            stroke.abilities[0].effect,
            Effect::TargetPlayerDraws {
                count: Amount::X,
                opponent: false,
            }
        ));

        // Augury Owl: "When this creature enters, scry 3." — an ETB scry.
        let owl = get("Augury Owl").expect("Augury Owl is in the pool");
        assert_eq!(owl.abilities[0].timing, Timing::Triggered(Trigger::Etb));
        assert!(matches!(
            owl.abilities[0].effect,
            Effect::Scry {
                count: Amount::Fixed(3)
            }
        ));

        // Dimir Informant: "When this creature enters, surveil 2." — an ETB surveil.
        let informant = get("Dimir Informant").expect("Dimir Informant is in the pool");
        assert_eq!(
            informant.abilities[0].timing,
            Timing::Triggered(Trigger::Etb)
        );
        assert!(matches!(
            informant.abilities[0].effect,
            Effect::Surveil { count: 2 }
        ));

        // Marauding Raptor: "Creature spells you cast cost {1} less to cast." — a static,
        // color-agnostic creature-spell reducer.
        let raptor = get("Marauding Raptor").expect("Marauding Raptor is in the pool");
        assert_eq!(raptor.abilities[0].timing, Timing::Static);
        assert!(matches!(
            raptor.abilities[0].effect,
            Effect::ReduceSpellCost {
                amount: Amount::Fixed(1),
                filter: SpellFilter::CreatureSpells,
                ..
            }
        ));

        // Killian, Ink Duelist: "Spells you cast that target a creature cost {2} less to cast."
        let killian = get("Killian, Ink Duelist").expect("Killian is in the pool");
        assert!(killian.legendary);
        assert!(killian.keywords.contains(&Keyword::Lifelink));
        assert!(killian.keywords.contains(&Keyword::Menace));
        assert!(matches!(
            killian.abilities[0].effect,
            Effect::ReduceSpellCost {
                amount: Amount::Fixed(2),
                filter: SpellFilter::SpellsThatTargetACreature,
                ..
            }
        ));

        // Temple of Malady: a scry land whose ETB scries 1 (its enters-tapped / dual-mana
        // clauses are simplified — see the card's TOML).
        let temple = get("Temple of Malady").expect("Temple of Malady is in the pool");
        assert!(matches!(temple.kind, CardKind::Land { .. }));
        assert_eq!(temple.abilities[0].timing, Timing::Triggered(Trigger::Etb));
        assert!(matches!(
            temple.abilities[0].effect,
            Effect::Scry {
                count: Amount::Fixed(1)
            }
        ));

        // Besmirch: a sorcery that steals target creature until end of turn (with haste),
        // untaps it, and goads it.
        let besmirch = get("Besmirch").expect("Besmirch is in the pool");
        assert!(matches!(
            besmirch.kind,
            CardKind::Spell {
                speed: SpellSpeed::Sorcery
            }
        ));
        assert_eq!(besmirch.abilities[0].timing, Timing::Spell);
        assert!(matches!(
            besmirch.abilities[0].effect,
            Effect::Sequence {
                steps: [
                    Effect::GainControlUntilEndOfTurn {
                        target: TargetSpec::Creature
                    },
                    Effect::PumpUntilEndOfTurn {
                        target: TargetSpec::Creature,
                        ..
                    },
                    Effect::UntapTarget {
                        target: TargetSpec::Creature
                    },
                    Effect::GoadTarget {
                        target: TargetSpec::Creature
                    },
                ]
            }
        ));

        // Silverquill Charm: a modal "choose one" instant (CR 700.2). Its three spell-timed
        // abilities are its modes — two target a creature, one takes no target.
        let charm = get("Silverquill Charm").expect("Silverquill Charm is in the pool");
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
            Effect::PutCounters {
                count: Amount::Fixed(2),
                target: TargetSpec::Creature,
                ..
            }
        ));
        // Mode 1: exile target creature with power 2 or less.
        assert!(matches!(
            charm.abilities[1].effect,
            Effect::ExileTarget {
                target: TargetSpec::Permanent(PermanentFilter {
                    power_max: Some(2),
                    ..
                }),
                ..
            }
        ));
        // Mode 2: each opponent loses 3 / you gain 3 — no target.
        assert!(matches!(
            charm.abilities[2].effect,
            Effect::EachOpponentDrain {
                amount: Amount::Fixed(3),
                sum_gain: false
            }
        ));

        // Quandrix Charm: a modal "choose one" instant — counter, destroy-enchantment, and
        // set-base-P/T-5/5 modes.
        let qcharm = get("Quandrix Charm").expect("Quandrix Charm is in the pool");
        assert!(qcharm.modal && qcharm.modal_choose == 1);
        assert_eq!(qcharm.abilities.len(), 3, "three modeled modes");
        assert!(matches!(
            qcharm.abilities[0].effect,
            Effect::CounterTargetSpell {
                unless_pays: Some(Amount::Fixed(2)),
                ..
            }
        ));
        assert!(matches!(
            qcharm.abilities[1].effect,
            Effect::DestroyTarget {
                target: TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::ENCHANTMENT,
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            qcharm.abilities[2].effect,
            Effect::SetBasePtTargetUntilEndOfTurn {
                power: Amount::Fixed(5),
                toughness: Amount::Fixed(5),
                target: TargetSpec::Creature,
            }
        ));

        // Prismari Command: a modal "choose two" instant — four modes, pick two distinct.
        let prismari = get("Prismari Command").expect("Prismari Command is in the pool");
        assert!(prismari.modal && prismari.modal_choose == 2);
        assert_eq!(prismari.abilities.len(), 4, "four modes");
        assert!(prismari.abilities.iter().all(|a| a.timing == Timing::Spell));
        assert!(matches!(
            prismari.abilities[0].effect,
            Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::AnyTarget,
                ..
            }
        ));
        assert!(matches!(
            prismari.abilities[1].effect,
            Effect::Sequence {
                steps: &[
                    Effect::TargetPlayerDraws {
                        count: Amount::Fixed(2),
                        opponent: false,
                    },
                    Effect::Discard {
                        count: 2,
                        target_player: true,
                        or_one_matching: None,
                    },
                ],
            }
        ));
        assert!(matches!(
            prismari.abilities[2].effect,
            Effect::CreateTreasure {
                count: Amount::Fixed(1),
                target_player: true,
                ..
            }
        ));
        assert!(matches!(
            prismari.abilities[3].effect,
            Effect::DestroyTarget {
                target: TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::ARTIFACT,
                    ..
                }),
                ..
            }
        ));

        // Witherbloom Command: a modal "choose two" sorcery — four modes, pick two distinct.
        let wither = get("Witherbloom Command").expect("Witherbloom Command is in the pool");
        assert!(wither.modal && wither.modal_choose == 2);
        assert_eq!(wither.abilities.len(), 4, "four modes");
        assert!(matches!(
            wither.abilities[0].effect,
            Effect::Sequence {
                steps: [
                    Effect::Mill {
                        count: Amount::Fixed(3),
                        target: TargetSpec::Player,
                    },
                    Effect::MayReturnFromGraveyard {
                        filter: CardFilter::Land,
                        ..
                    },
                ],
            }
        ));
        assert!(matches!(
            wither.abilities[1].effect,
            Effect::DestroyTarget {
                target: TargetSpec::Permanent(PermanentFilter {
                    types: TypeSet::NONLAND,
                    noncreature: true,
                    mv_max: Some(2),
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            wither.abilities[2].effect,
            Effect::PumpUntilEndOfTurn {
                power: Amount::Fixed(-3),
                toughness: Amount::Fixed(-1),
                target: TargetSpec::Creature,
                ..
            }
        ));
        assert!(matches!(
            wither.abilities[3].effect,
            Effect::DrainTarget {
                amount: 2,
                opponent: true,
            }
        ));

        // Quandrix Command: a modal "choose two" instant, all four printed modes modeled.
        let quandrix = get("Quandrix Command").expect("Quandrix Command is in the pool");
        assert!(quandrix.modal && quandrix.modal_choose == 2);
        assert_eq!(quandrix.abilities.len(), 4, "four modeled modes");
        match quandrix.abilities[0].effect {
            Effect::ReturnToHand {
                target: TargetSpec::Permanent(filter),
                ..
            } => {
                assert_eq!(filter.types, TypeSet::CREATURE.union(TypeSet::PLANESWALKER));
            }
            other => panic!("mode 0 should bounce a creature or planeswalker, got {other:?}"),
        }
        assert!(matches!(
            quandrix.abilities[1].effect,
            Effect::CounterTargetSpell {
                unless_pays: None,
                filter: SpellFilter::ArtifactOrEnchantment,
                countered_dest: None,
            }
        ));
        assert!(matches!(
            quandrix.abilities[2].effect,
            Effect::PutCounters {
                count: Amount::Fixed(2),
                target: TargetSpec::Creature,
                ..
            }
        ));
        assert!(matches!(
            quandrix.abilities[3].effect,
            Effect::ShuffleTargetCardsFromGraveyardIntoLibrary {
                max: 3,
                target_player: true,
            }
        ));

        // Killian, Decisive Mentor: the tap-and-goad half of the commander, on a watch for an
        // enchantment you control entering.
        let killian = get("Killian, Decisive Mentor").expect("Killian is in the pool");
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
        assert!(matches!(
            killian.abilities[0].effect,
            Effect::Sequence {
                steps: [
                    Effect::TapTarget {
                        target: TargetSpec::Creature,
                        ..
                    },
                    Effect::GoadTarget {
                        target: TargetSpec::Creature
                    },
                ]
            }
        ));

        // Leonin Vanguard: an intervening-if trigger — "if you control three or more creatures"
        // gates a begin-combat self-pump + life gain.
        let leonin = get("Leonin Vanguard").expect("Leonin Vanguard is in the pool");
        assert_eq!(
            leonin.abilities[0].timing,
            Timing::Triggered(Trigger::BeginCombat)
        );
        assert_eq!(
            leonin.abilities[0].condition,
            Some(Condition::YouControlAtLeastCreatures { count: 3 })
        );
        assert!(matches!(
            leonin.abilities[0].effect,
            Effect::Sequence {
                steps: [
                    Effect::PumpSelfUntilEndOfTurn {
                        power: Amount::Fixed(1),
                        toughness: Amount::Fixed(1),
                        ..
                    },
                    Effect::GainLife {
                        amount: Amount::Fixed(1)
                    },
                ]
            }
        ));

        // Breena, the Demagogue: a watch-others attack trigger with an intervening-if condition
        // and the composite "attacking player draws / you put two counters" effect.
        let breena = get("Breena, the Demagogue").expect("Breena is in the pool");
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
            Effect::AttackerDrawsControllerCounters {
                attacker: None,
                counters: 2,
            }
        ));

        // Quintorius, History Chaser: a Lorehold planeswalker commander — starting loyalty 5, with
        // a +1 loyalty ability that may discard a card to draw two and mill one.
        let quintorius = get("Quintorius, History Chaser").expect("Quintorius is in the pool");
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
        assert!(matches!(
            quintorius.abilities[0].effect,
            Effect::MayDiscard {
                then: [
                    Effect::DrawCards {
                        count: Amount::Fixed(2)
                    },
                    Effect::MillSelf {
                        count: Amount::Fixed(1)
                    }
                ]
            }
        ));

        // Rite of Replication: "Kicker {5} ... Create a token that's a copy of target creature.
        // If this spell was kicked, create five of those tokens instead." {2}{U}{U} sorcery.
        let rite = get("Rite of Replication").expect("Rite of Replication is in the pool");
        assert_eq!(rite.cost.generic, 2);
        assert_eq!(rite.cost.colored[Color::Blue.index()], 2);
        assert!(matches!(rite.cost.additional.kicker, Some(k) if k.generic == 5));
        assert!(matches!(
            rite.abilities[0].effect,
            Effect::CreateTokenCopy {
                count: Amount::IfSpellKicked { then, else_ },
                target: TargetSpec::Creature,
                sacrifice_at_next_end_step: false,
                exile_at_next_end_step: false,
                haste: false,
                ..
            } if *then == Amount::Fixed(5) && *else_ == Amount::Fixed(1)
        ));

        // Twincast: "Copy target instant or sorcery spell." — {U}{U} instant, targets a spell
        // on the stack (the "choose new targets" clause is simplified to same-targets).
        let twincast = get("Twincast").expect("Twincast is in the pool");
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
            Effect::CopyTargetSpell
        ));

        // Hardened Scales: "…that many plus one." — a static +1 counter-replacement.
        let scales = get("Hardened Scales").expect("Hardened Scales is in the pool");
        assert_eq!(scales.kind, CardKind::Enchantment);
        assert_eq!(scales.abilities[0].timing, Timing::Static);
        assert!(matches!(
            scales.abilities[0].effect,
            Effect::CounterReplacement {
                add: 1,
                times: 1,
                ..
            }
        ));

        // Doubling Season: "…twice that many." — a static x2 token-creation replacement plus a
        // static x2 counter-replacement (times defaults to 1, so an adder can omit it; the doubler
        // sets it).
        let doubling = get("Doubling Season").expect("Doubling Season is in the pool");
        assert!(matches!(
            doubling.abilities[0].effect,
            Effect::TokenReplacement { times: 2 }
        ));
        assert!(matches!(
            doubling.abilities[1].effect,
            Effect::CounterReplacement {
                add: 0,
                times: 2,
                ..
            }
        ));

        // Diabolic Tutor: "Search your library for a card, put it into your hand, then shuffle."
        let tutor = get("Diabolic Tutor").expect("Diabolic Tutor is in the pool");
        assert_eq!(tutor.cost.generic, 2);
        assert_eq!(tutor.cost.colored[Color::Black.index()], 2);
        assert!(matches!(
            tutor.abilities[0].effect,
            Effect::SearchLibrary {
                filter: CardFilter::AnyCard,
                to_zone: SearchDest::Hand,
                tapped: false,
                ..
            }
        ));

        // Rampant Growth: "Search your library for a basic land card, put it onto the battlefield
        // tapped, then shuffle." — basic-land ramp.
        let ramp = get("Rampant Growth").expect("Rampant Growth is in the pool");
        assert!(matches!(
            ramp.abilities[0].effect,
            Effect::SearchLibrary {
                filter: CardFilter::BasicLand,
                to_zone: SearchDest::Battlefield,
                tapped: true,
                ..
            }
        ));

        // Terramorphic Expanse: "{T}, Sacrifice this land: search a basic land onto the
        // battlefield tapped, then shuffle." — a fetchland (no life cost).
        let terramorphic =
            get("Terramorphic Expanse").expect("Terramorphic Expanse is in the pool");
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
            Effect::SearchLibrary {
                filter: CardFilter::BasicLand,
                to_zone: SearchDest::Battlefield,
                tapped: true,
                ..
            }
        ));

        // Fabled Passage: same as Terramorphic (its "untap that land" rider is deferred).
        let fabled = get("Fabled Passage").expect("Fabled Passage is in the pool");
        let Timing::Activated(fabled_fetch) = fabled.abilities[0].timing else {
            panic!("Fabled Passage's fetch is an activated ability");
        };
        assert_eq!(fabled_fetch.sacrifice, SacrificeCost::This);
        assert_eq!(fabled_fetch.pay_life, Amount::Fixed(0));

        // Prismatic Vista: "{T}, Pay 1 life, Sacrifice this land: search a basic land onto the
        // battlefield (untapped), then shuffle." — the pay-life fetchland.
        let vista = get("Prismatic Vista").expect("Prismatic Vista is in the pool");
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
            Effect::SearchLibrary {
                filter: CardFilter::BasicLand,
                to_zone: SearchDest::Battlefield,
                tapped: false,
                ..
            }
        ));

        // Goldvein Hydra: {X}{G} 0/0 that "enters with X +1/+1 counters", with vigilance/trample/
        // haste (its death -> Treasure rider is deferred).
        let hydra = get("Goldvein Hydra").expect("Goldvein Hydra is in the pool");
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
            Effect::EntersWithCounters {
                amount: Amount::X,
                kind: None
            }
        ));

        // Blasphemous Act: "13 damage to each creature." — a fixed mass-damage wipe.
        let blasphemous = get("Blasphemous Act").expect("Blasphemous Act is in the pool");
        assert!(matches!(
            blasphemous.abilities[0].effect,
            Effect::DamageEachCreature {
                amount: Amount::Fixed(13),
                ..
            }
        ));

        // Chain Reaction: "X damage to each creature, X = creatures on the battlefield." — a
        // board-derived mass-damage wipe.
        let chain = get("Chain Reaction").expect("Chain Reaction is in the pool");
        assert!(matches!(
            chain.abilities[0].effect,
            Effect::DamageEachCreature {
                amount: Amount::PerCreatureOnBattlefield,
                ..
            }
        ));

        // Toxic Deluge: "pay X life, all creatures get -X/-X." — {X} models the life (see TOML).
        let deluge = get("Toxic Deluge").expect("Toxic Deluge is in the pool");
        assert!(deluge.cost.x > 0, "Toxic Deluge's X is the pay-X source");
        assert!(matches!(
            deluge.abilities[0].effect,
            Effect::WeakenEachCreature {
                power: Amount::X,
                toughness: Amount::X,
                opponents_only: false,
            }
        ));

        // Winds of Rath: "destroy all creatures that aren't enchanted."
        let winds = get("Winds of Rath").expect("Winds of Rath is in the pool");
        assert!(matches!(
            winds.abilities[0].effect,
            Effect::DestroyAll {
                filter: PermanentFilter {
                    types: TypeSet::CREATURE,
                    enchanted: Some(false),
                    ..
                }
            }
        ));

        // Culling Ritual: "destroy each nonland permanent with mana value 2 or less. Add {B} or
        // {G} for each permanent destroyed this way." — a `Sequence` of the wipe, then the
        // count-derived mana rider.
        let culling = get("Culling Ritual").expect("Culling Ritual is in the pool");
        let Effect::Sequence {
            steps: [wipe, rider],
        } = culling.abilities[0].effect
        else {
            panic!("Culling Ritual's ability is a two-step Sequence (wipe, then mana rider)");
        };
        assert!(matches!(
            wipe,
            Effect::DestroyAll {
                filter: PermanentFilter {
                    types: TypeSet::NONLAND,
                    mv_max: Some(2),
                    ..
                }
            }
        ));
        assert!(matches!(
            rider,
            Effect::AddMana {
                repeat: Amount::PermanentsDestroyedThisWay { .. },
                ..
            }
        ));

        // Fracture: "destroy target artifact, enchantment, or planeswalker." — noncreature removal.
        let fracture = get("Fracture").expect("Fracture is in the pool");
        assert!(matches!(
            fracture.abilities[0].effect,
            Effect::DestroyTarget {
                target: TargetSpec::ArtifactEnchantmentOrPlaneswalker,
                ..
            }
        ));

        // Storm-Kiln Artist: "This creature gets +1/+0 for each artifact you control. Magecraft —
        // Whenever you cast or copy an instant or sorcery, create a Treasure token."
        let storm_kiln = get("Storm-Kiln Artist").expect("Storm-Kiln Artist is in the pool");
        assert_eq!(storm_kiln.abilities[0].timing, Timing::Static);
        assert!(matches!(
            storm_kiln.abilities[0].effect,
            Effect::AnthemStatic {
                self_only: true,
                ..
            }
        ));
        assert_eq!(
            storm_kiln.abilities[1].timing,
            Timing::Triggered(Trigger::Magecraft)
        );
        assert!(matches!(
            storm_kiln.abilities[1].effect,
            Effect::CreateTreasure {
                count: Amount::Fixed(1),
                target_player: false,
                ..
            }
        ));

        // Big Score: "Draw two cards and create two Treasure tokens." — a non-modal instant with two
        // spell halves (its "discard a card" additional cost is deferred — see its TOML).
        let big_score = get("Big Score").expect("Big Score is in the pool");
        assert!(matches!(
            big_score.kind,
            CardKind::Spell {
                speed: SpellSpeed::Instant
            }
        ));
        assert!(matches!(
            big_score.abilities[0].effect,
            Effect::DrawCards {
                count: Amount::Fixed(2)
            }
        ));
        assert!(matches!(
            big_score.abilities[1].effect,
            Effect::CreateTreasure {
                count: Amount::Fixed(2),
                target_player: false,
                ..
            }
        ));

        // Darksteel Myr: a {3} 0/1 artifact creature with intrinsic indestructible.
        let myr = get("Darksteel Myr").expect("Darksteel Myr is in the pool");
        assert!(myr.keywords.contains(&Keyword::Indestructible));

        // Ambush Viper: {1}{G} 2/1 with flash and deathtouch.
        let viper = get("Ambush Viper").expect("Ambush Viper is in the pool");
        assert!(viper.keywords.contains(&Keyword::Flash));
        assert!(viper.keywords.contains(&Keyword::Deathtouch));

        // Tomakul Honor Guard: {1}{G} 3/1 with Ward {2} (a parametrized keyword from a table).
        let guard = get("Tomakul Honor Guard").expect("Tomakul Honor Guard is in the pool");
        assert!(guard.keywords.contains(&Keyword::Ward(2)));

        // White Knight: {W}{W} 2/2 with first strike and protection from black.
        let knight = get("White Knight").expect("White Knight is in the pool");
        assert!(knight.keywords.contains(&Keyword::FirstStrike));
        assert!(
            knight
                .keywords
                .contains(&Keyword::ProtectionFrom(ProtectionScope::Color(
                    Color::Black
                )))
        );

        // Shielded by Faith: an Aura granting indestructible to the enchanted creature.
        let shielded = get("Shielded by Faith").expect("Shielded by Faith is in the pool");
        assert_eq!(shielded.kind, CardKind::Aura);
        let Effect::GrantToAttached { keywords, .. } = shielded.abilities[0].effect else {
            panic!("Shielded by Faith grants a static keyword to its host");
        };
        assert_eq!(keywords, &[Keyword::Indestructible]);

        // Blight Mound makes a Pest token that carries its own death trigger ("When this token
        // dies, you gain 1 life") — a token profile that's a full inline card, not just P/T.
        // abilities[0] is the "attacking Pests get +1/+0 and menace" anthem; abilities[1] is the
        // death-trigger token maker.
        let blight = get("Blight Mound").expect("Blight Mound is in the pool");
        let Effect::CreateToken { token: pest, .. } = blight.abilities[1].effect else {
            panic!("Blight Mound creates a Pest token");
        };
        assert_eq!(pest.name, "Pest");
        assert_eq!(pest.abilities[0].timing, Timing::Triggered(Trigger::Dies));
        assert!(matches!(
            pest.abilities[0].effect,
            Effect::GainLife {
                amount: Amount::Fixed(1)
            }
        ));

        // Gilded Goose's ETB makes a Food — an *artifact* token whose own activated ability
        // sacrifices it ("{2}, {T}, Sacrifice this token: You gain 3 life").
        let goose = get("Gilded Goose").expect("Gilded Goose is in the pool");
        let Effect::CreateToken { token: food, .. } = goose.abilities[0].effect else {
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
        let skyclave = get("Skyclave Apparition").expect("Skyclave Apparition is in the pool");
        let source = game.spawn_on_battlefield(P0, skyclave);

        // An opponent's mana-value-3 nontoken permanent — inside the gate, a legal target.
        let in_gate = game.spawn_on_battlefield(P1, skyclave);
        // An opponent's Sun Titan (mana value 6) — filtered out by the "4 or less" gate.
        let over_gate =
            game.spawn_on_battlefield(P1, get("Sun Titan").expect("Sun Titan is in the pool"));

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
        let loot = get("Faithless Looting").expect("Faithless Looting is in the pool");
        let Effect::Sequence { steps } = loot.abilities[0].effect else {
            panic!("an `effects` list is an Effect::Sequence");
        };
        assert_eq!(
            steps,
            &[
                Effect::DrawCards {
                    count: Amount::Fixed(2)
                },
                Effect::Discard {
                    count: 2,
                    target_player: false,
                    or_one_matching: None,
                },
            ],
            "draw two, then discard two — in order"
        );

        // A one-element `effects` list stays a bare effect (not wrapped in a Sequence): Shock's
        // lone ability stays a bare DealDamage.
        let shock = get("Shock").expect("Shock is in the pool");
        assert!(matches!(
            shock.abilities[0].effect,
            Effect::DealDamage { .. }
        ));

        // The singular `effect` sugar was removed: only `effects` is accepted, so a lone `effect`
        // key is now an unknown-field load error.
        let bad = "name = \"Singular\"\n\n[kind]\ntype = \"sorcery\"\n\n[[abilities]]\ntiming = \"spell\"\neffect = { type = \"draw_cards\", count = 1 }\n";
        assert!(toml::from_str::<CardDef>(bad).is_err());

        // An ability with no effects at all is likewise a load error.
        let empty =
            "name = \"Empty\"\n\n[kind]\ntype = \"sorcery\"\n\n[[abilities]]\ntiming = \"spell\"\n";
        assert!(toml::from_str::<CardDef>(empty).is_err());
    }
}
