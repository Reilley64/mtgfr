//! The card catalog: pool cards in browse form for the deck builder, plus color-identity
//! derivation (CR 903.4).

use serde::{Deserialize, Serialize};

use crate::dto::{WireCost, WireKind};

/// One pool card, for the deck builder to browse. Stats/keywords/summary are engine truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogCard {
    /// Card id (Scryfall oracle id).
    pub id: String,
    /// Default Printing UUID (Scryfall card id) for art when a deck line hasn't chosen otherwise.
    pub default_print: String,
    pub name: String,
    pub cost: WireCost,
    pub kind: WireKind,
    pub keywords: Vec<String>,
    /// Plain-English summary of the card's keywords + abilities (the engine's simplified behavior).
    pub summary: String,
    pub legendary: bool,
    /// Color identity as WUBRG indices (see `engine::Color::index`).
    pub color_identity: Vec<u8>,
    /// A one-line note on how this card's modeled behavior diverges from its printed rules
    /// text, when it does (`engine::CardDef::approximates`). `None` for a faithful card.
    pub approximates: Option<String>,
    /// The card's printed (oracle) rules text, for the deck builder's read-the-text hover
    /// (`engine::CardDef::oracle`). `None` for a card whose text isn't recorded, or a vanilla.
    pub oracle: Option<String>,
    /// Set/edition code (Scryfall's lowercase code, e.g. `"soc"`); empty when unrecorded. A
    /// deck-builder search dimension.
    pub set: String,
    /// Printed subtypes for search (creature types like "Goblin"/"Wizard", plus a land's printed
    /// types). The union of `engine::CardDef::subtypes` (creature/artifact/enchantment types) and,
    /// for a land, its `CardKind::Land::subtypes`.
    pub subtypes: Vec<String>,
    /// Scryfall Tagger oracle-tag slugs for thematic deck-builder search (e.g. `"typal-spirit"`,
    /// `"cost-reducer-enchantment"`). A deck-builder search dimension.
    pub otags: Vec<String>,
    /// Other face of a prepare DFC (`CardDef::back`). Embedded because back faces are not pool
    /// catalog entries. Absent for ordinary single-faced cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<CatalogBackFace>,
}

/// The back face of a prepare DFC, for card inspect flip (name + rules text).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogBackFace {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oracle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximates: Option<String>,
}

/// The five colors in WUBRG order, for building a mana-pool row.
pub(crate) const COLORS: [engine::Color; 5] = [
    engine::Color::White,
    engine::Color::Blue,
    engine::Color::Black,
    engine::Color::Red,
    engine::Color::Green,
];

/// Stable snake_case id for a keyword on the wire (`flying`, `first_strike`, `ward:2`,
/// `protection:red`). Used by battlefield `ObjectView` badges and catalog `keywords`.
pub(crate) fn wire_keyword(keyword: engine::Keyword) -> String {
    use engine::{Color, Keyword, ProtectionScope};
    match keyword {
        Keyword::Flying => "flying".into(),
        Keyword::FirstStrike => "first_strike".into(),
        Keyword::Vigilance => "vigilance".into(),
        Keyword::Haste => "haste".into(),
        Keyword::Trample => "trample".into(),
        Keyword::Deathtouch => "deathtouch".into(),
        Keyword::Reach => "reach".into(),
        Keyword::Menace => "menace".into(),
        Keyword::DoubleStrike => "double_strike".into(),
        Keyword::Lifelink => "lifelink".into(),
        Keyword::Defender => "defender".into(),
        Keyword::Unblockable => "unblockable".into(),
        Keyword::Indestructible => "indestructible".into(),
        Keyword::Flash => "flash".into(),
        Keyword::Hexproof => "hexproof".into(),
        Keyword::Shroud => "shroud".into(),
        Keyword::Prowess => "prowess".into(),
        Keyword::Skulk => "skulk".into(),
        Keyword::Shadow => "shadow".into(),
        Keyword::Fear => "fear".into(),
        Keyword::LesserPowerCantBlock => "lesser_power_cant_block".into(),
        Keyword::CantBlock => "cant_block".into(),
        Keyword::CanBlockOnlyFlyers => "can_block_only_flyers".into(),
        Keyword::Decayed => "decayed".into(),
        Keyword::Myriad => "myriad".into(),
        Keyword::Ward(n) => format!("ward:{n}"),
        Keyword::ProtectionFrom(scope) => {
            let name = match scope {
                ProtectionScope::Color(Color::White) => "white",
                ProtectionScope::Color(Color::Blue) => "blue",
                ProtectionScope::Color(Color::Black) => "black",
                ProtectionScope::Color(Color::Red) => "red",
                ProtectionScope::Color(Color::Green) => "green",
                ProtectionScope::Creatures => "creatures",
                ProtectionScope::Multicolored => "multicolored",
            };
            format!("protection:{name}")
        }
    }
}

/// Human-readable keyword for catalog `summary` (deck-builder hover text).
pub(crate) fn keyword_label(keyword: engine::Keyword) -> String {
    use engine::{Color, Keyword, ProtectionScope};
    match keyword {
        Keyword::Flying => "Flying".into(),
        Keyword::FirstStrike => "First strike".into(),
        Keyword::Vigilance => "Vigilance".into(),
        Keyword::Haste => "Haste".into(),
        Keyword::Trample => "Trample".into(),
        Keyword::Deathtouch => "Deathtouch".into(),
        Keyword::Reach => "Reach".into(),
        Keyword::Menace => "Menace".into(),
        Keyword::DoubleStrike => "Double strike".into(),
        Keyword::Lifelink => "Lifelink".into(),
        Keyword::Defender => "Defender".into(),
        Keyword::Unblockable => "Unblockable".into(),
        Keyword::Indestructible => "Indestructible".into(),
        Keyword::Flash => "Flash".into(),
        Keyword::Hexproof => "Hexproof".into(),
        Keyword::Shroud => "Shroud".into(),
        Keyword::Prowess => "Prowess".into(),
        Keyword::Skulk => "Skulk".into(),
        Keyword::Shadow => "Shadow".into(),
        Keyword::Fear => "Fear".into(),
        Keyword::LesserPowerCantBlock => "Lesser-power creatures can't block it".into(),
        Keyword::CantBlock => "Can't block".into(),
        Keyword::CanBlockOnlyFlyers => "Can block only creatures with flying".into(),
        Keyword::Decayed => "Decayed".into(),
        Keyword::Myriad => "Myriad".into(),
        Keyword::Ward(n) => format!("Ward {{{n}}}"),
        Keyword::ProtectionFrom(scope) => {
            let name = match scope {
                ProtectionScope::Color(Color::White) => "white",
                ProtectionScope::Color(Color::Blue) => "blue",
                ProtectionScope::Color(Color::Black) => "black",
                ProtectionScope::Color(Color::Red) => "red",
                ProtectionScope::Color(Color::Green) => "green",
                ProtectionScope::Creatures => "creatures",
                ProtectionScope::Multicolored => "multicolored",
            };
            format!("Protection from {name}")
        }
    }
}

/// Wire form of a card's kind.
pub(crate) fn wire_kind(def: engine::CardDef) -> WireKind {
    use engine::{CardKind, SpellSpeed};
    match def.kind {
        CardKind::Creature {
            power, toughness, ..
        } => WireKind::Creature { power, toughness },
        CardKind::Spell {
            speed: SpellSpeed::Instant,
        } => WireKind::Instant,
        CardKind::Spell {
            speed: SpellSpeed::Sorcery,
        } => WireKind::Sorcery,
        CardKind::Enchantment => WireKind::Enchantment,
        // ponytail: an Aura is an enchantment subtype; report it as Enchantment on the wire so
        // the client needs no new kind. Add a WireKind::Aura when the UI must distinguish them.
        CardKind::Aura => WireKind::Enchantment,
        CardKind::Artifact => WireKind::Artifact,
        CardKind::Planeswalker { loyalty } => WireKind::Planeswalker { loyalty },
        CardKind::Land { .. } => WireKind::Land {
            colors: land_colors(def),
        },
    }
}

/// The colors a land can produce, as WUBRG indices, for the client's mana-dot display: from its
/// optional `produces` base tap and every `add_mana` ability it carries (painlands, filter lands,
/// and the `{1},{T}` karoos have no `produces` — their colors live in the abilities). `{C}`/`{T}`
/// (colorless, "any") contribute no *color*; "any" mana (the commander-identity credit and the
/// opponent-producible-colors credit, both of whose actual colors depend on table state) is
/// shown as all five, like Command Tower. A restricted opponent-producible-colors credit that's
/// already resolved to a concrete [`engine::Mana::OfColors`] bitmask (never authored in TOML —
/// only produced at resolution) shows exactly its set.
fn land_colors(def: engine::CardDef) -> Vec<u8> {
    let mut colors = [false; engine::Color::COUNT];
    if let engine::CardKind::Land {
        produces: Some(produces),
        ..
    } = def.kind
    {
        match produces {
            engine::LandProduces::Mana(engine::Mana::Color(c)) => colors[c.index()] = true,
            engine::LandProduces::Mana(engine::Mana::Either(a, b)) => {
                colors[a.index()] = true;
                colors[b.index()] = true;
            }
            engine::LandProduces::Mana(engine::Mana::Any)
            | engine::LandProduces::CommanderIdentity
            | engine::LandProduces::OpponentColors => colors.iter_mut().for_each(|c| *c = true),
            engine::LandProduces::Mana(engine::Mana::OfColors(mask)) => {
                for (i, on) in colors.iter_mut().enumerate() {
                    *on |= mask & (1 << i) != 0;
                }
            }
            engine::LandProduces::Mana(engine::Mana::Colorless) => {}
            // No land in the pool has a spend-restricted `produces` (restriction is
            // authored only on `add_mana`/`grant_mana_ability` abilities) — handled for
            // exhaustiveness, same as its unrestricted `base` kind.
            engine::LandProduces::Mana(engine::Mana::Restricted { base, .. }) => match base {
                engine::RestrictedManaBase::Color(c) => colors[c.index()] = true,
                engine::RestrictedManaBase::Colorless => {}
                engine::RestrictedManaBase::Any => colors.iter_mut().for_each(|c| *c = true),
            },
        }
    }
    for ability in def.abilities {
        let engine::Effect::AddMana { mana: produced, .. } = ability.effect else {
            continue;
        };
        for (c, on) in colors.iter_mut().enumerate() {
            *on |= produced.colored[c] > 0;
        }
        for (&(a, b), &count) in engine::COLOR_PAIRS.iter().zip(produced.either.iter()) {
            if count > 0 {
                colors[a.index()] = true;
                colors[b.index()] = true;
            }
        }
        // A fixed 3-4 color choice (Treva's Ruins' "{T}: Add {G}, {W}, or {U}") — same bit order
        // as `Mana::OfColors`'s own doc.
        for (mask, &count) in produced.of_colors.iter().enumerate() {
            if count == 0 {
                continue;
            }
            for (i, on) in colors.iter_mut().enumerate() {
                *on |= mask & (1 << i) != 0;
            }
        }
        if produced.any > 0 {
            colors.iter_mut().for_each(|c| *c = true);
        }
    }
    colors
        .iter()
        .enumerate()
        .filter_map(|(i, &on)| on.then_some(i as u8))
        .collect()
}

/// Wire form of a mana cost.
/// ponytail: colorless `{C}` cost pips aren't surfaced on the wire yet (no pool card has one);
/// add a field to `WireCost` when a `{C}`-costed card enters the pool.
pub(crate) fn wire_cost(cost: engine::Cost) -> WireCost {
    WireCost {
        generic: cost.generic,
        colored: cost.colored,
        has_x: cost.x > 0,
    }
}

/// A card's color identity as a WUBRG bitset (CR 903.4: every mana symbol printed on the
/// card, whether or not gameplay fully implements it). Derived from colored cost pips, a
/// land's single modeled producer, mana-adding/activated colored costs, plus each card's
/// declared `identity_pips` for symbols the simplified gameplay model drops (the dropped
/// half of a flattened dual/pain/filter land, or a colored activated ability cut entirely).
/// No color indicators in the pool yet — add one there if a future card needs it.
pub fn color_identity(def: &engine::CardDef) -> u8 {
    let mut id = 0u8;
    for c in COLORS {
        if def.cost.colored[c.index()] > 0 {
            id |= 1 << c.index();
        }
    }
    for c in def.identity_pips {
        id |= 1 << c.index();
    }
    // Colorless {C}, "any color", the commander-identity credit, and the opponent-producible-
    // colors credit all add nothing to identity (CR 903.4 counts printed mana *symbols*, and
    // none of these print one — Command Tower's/Exotic Orchard's own oracle text has no colored
    // pip); a dual ("either of two colors") adds both its colors.
    if let engine::CardKind::Land {
        produces: Some(produces),
        ..
    } = def.kind
    {
        match produces {
            engine::LandProduces::Mana(engine::Mana::Color(c)) => id |= 1 << c.index(),
            engine::LandProduces::Mana(engine::Mana::Either(a, b)) => {
                id |= 1 << a.index() | 1 << b.index()
            }
            // No land in the pool has a spend-restricted `produces` — handled for
            // exhaustiveness, same as its unrestricted `base` kind (a restricted color still
            // prints that color's symbol).
            engine::LandProduces::Mana(engine::Mana::Restricted {
                base: engine::RestrictedManaBase::Color(c),
                ..
            }) => id |= 1 << c.index(),
            engine::LandProduces::Mana(
                engine::Mana::Colorless
                | engine::Mana::Any
                | engine::Mana::OfColors(_)
                | engine::Mana::Restricted {
                    base: engine::RestrictedManaBase::Colorless | engine::RestrictedManaBase::Any,
                    ..
                },
            )
            | engine::LandProduces::CommanderIdentity
            | engine::LandProduces::OpponentColors => {}
        }
    }
    for ability in def.abilities {
        if let engine::Effect::AddMana { mana: produced, .. } = ability.effect {
            for c in COLORS {
                if produced.colored[c.index()] > 0 {
                    id |= 1 << c.index();
                }
            }
            for (&(a, b), &count) in engine::COLOR_PAIRS.iter().zip(produced.either.iter()) {
                if count > 0 {
                    id |= 1 << a.index() | 1 << b.index();
                }
            }
        }
        if let engine::Timing::Activated(cost) = ability.timing {
            for c in COLORS {
                if cost.mana.colored[c.index()] > 0 {
                    id |= 1 << c.index();
                }
            }
        }
    }
    id
}

/// The colors present in a color-identity bitset, as WUBRG indices.
fn identity_indices(id: u8) -> Vec<u8> {
    COLORS
        .iter()
        .map(|c| c.index() as u8)
        .filter(|&i| id & (1 << i) != 0)
        .collect()
}

/// A card's printed subtypes for search: the metadata `CardDef::subtypes` (creature/artifact/
/// enchantment types) unioned with a land's rules-side `CardKind::Land::subtypes`, so both surface
/// on the wire without duplicating land types into the metadata field.
fn all_subtypes(def: &engine::CardDef) -> Vec<String> {
    let mut out: Vec<String> = def.subtypes.iter().map(|s| s.to_string()).collect();
    if let engine::CardKind::Land { subtypes, .. } = def.kind {
        for s in subtypes {
            let s = s.to_string();
            if !out.contains(&s) {
                out.push(s);
            }
        }
    }
    out
}

/// A pool card in browse form for the deck builder.
pub fn catalog_card(def: &engine::CardDef) -> CatalogCard {
    let keywords: Vec<String> = def.keywords.iter().copied().map(wire_keyword).collect();
    let mut parts: Vec<String> = def.keywords.iter().copied().map(keyword_label).collect();
    parts.extend(def.abilities.iter().map(|a| a.effect.label()));
    CatalogCard {
        id: def.id.to_string(),
        default_print: def.default_print.to_string(),
        name: def.name.to_string(),
        cost: wire_cost(def.cost),
        kind: wire_kind(*def),
        keywords,
        summary: parts.join(", "),
        legendary: def.legendary,
        color_identity: identity_indices(color_identity(def)),
        approximates: def.approximates.map(str::to_string),
        oracle: def.oracle.map(str::to_string),
        set: def.set.to_string(),
        subtypes: all_subtypes(def),
        otags: def.otags.iter().map(|s| s.to_string()).collect(),
        back: def.back.map(|b| CatalogBackFace {
            name: b.name.to_string(),
            oracle: b.oracle.map(str::to_string),
            approximates: b.approximates.map(str::to_string),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::def;

    #[test]
    fn catalog_card_carries_engine_truth_and_color_identity() {
        let green = engine::Color::Green.index() as u8;

        let serra = catalog_card(&def("Serra Angel"));
        assert!(serra.keywords.iter().any(|k| k == "flying"));
        assert!(serra.keywords.iter().any(|k| k == "vigilance"));
        assert!(serra.summary.contains("Flying"));
        assert!(serra.summary.contains("Vigilance"));

        let shock = catalog_card(&def("Shock"));
        assert!(
            shock.summary.contains("Deal 2 damage"),
            "got {:?}",
            shock.summary
        );

        let forest = catalog_card(&def("Forest"));
        assert_eq!(
            forest.color_identity,
            vec![green],
            "Forest is mono-green by its produced mana"
        );

        let tajic = catalog_card(&def("Tajic, Legion's Edge"));
        assert!(tajic.legendary);
        // Tajic is {1}{R}{W}: red + white, not green.
        assert!(!tajic.color_identity.contains(&green));

        // Tangled Islet ("{T}: Add {G} or {U}"): both produced colors are its identity
        // (CR 903.4), and the wire land kind carries both so auto-tap can plan with it.
        let blue = engine::Color::Blue.index() as u8;
        let islet = catalog_card(&def("Tangled Islet"));
        assert_eq!(islet.color_identity, vec![blue, green]);
        assert_eq!(
            islet.kind,
            WireKind::Land {
                colors: vec![blue, green]
            }
        );

        // Serra Angel is faithfully modeled — no approximation note.
        assert_eq!(serra.approximates, None);
    }

    #[test]
    fn catalog_card_surfaces_otags_from_card_def() {
        let starfield = catalog_card(&def("Starfield Mystic"));
        assert!(
            starfield
                .otags
                .contains(&"cost-reducer-enchantment".to_string()),
            "got {:?}",
            starfield.otags
        );

        let vanguard = catalog_card(&def("Vanguard of the Restless"));
        assert!(
            vanguard.otags.contains(&"typal-spirit".to_string()),
            "got {:?}",
            vanguard.otags
        );
    }

    #[test]
    fn catalog_card_surfaces_prepare_back_face() {
        let kirol = catalog_card(&def("Kirol, History Buff"));
        let back = kirol.back.expect("Kirol has a prepare back face");
        assert_eq!(back.name, "Pack a Punch");
        assert!(
            back.oracle
                .as_deref()
                .is_some_and(|o| o.contains("Mill a card")),
            "got {:?}",
            back.oracle
        );
    }

    #[test]
    fn catalog_card_surfaces_a_known_faithfulness_gap() {
        // Final Act drops three of its five modes (battles, mass-graveyard exile, counters on
        // players — none a modeled game object) — the gap is recorded as a datum, not just a TOML
        // comment, so the deck builder / audits can read it.
        let final_act = catalog_card(&def("Final Act"));
        let note = final_act
            .approximates
            .expect("Final Act's dropped modes are a known approximation");
        assert!(
            note.contains("dropped"),
            "expected the note to call out the dropped modes, got {note:?}"
        );
    }
}

#[cfg(test)]
mod color_identity_audit {
    use super::color_identity;
    use crate::test_support::def;
    use engine::Color::{Black, Blue, Green, Red, White};

    fn bitset(colors: &[engine::Color]) -> u8 {
        colors.iter().fold(0u8, |id, c| id | (1 << c.index()))
    }

    fn assert_identity(name: &str, expected: &[engine::Color]) {
        let got = color_identity(&def(name));
        assert_eq!(
            got,
            bitset(expected),
            "{name}: expected color identity {expected:?}, got bitset {got:#07b}"
        );
    }

    #[test]
    fn dual_and_pain_lands_flattened_to_a_single_producer_still_carry_both_colors() {
        // Second color dropped by the "flattened to one producer" simplification — the modeled
        // producer color is still correctly there, so the *full* identity is producer + the added
        // `identity_pips` color.
        for name in [
            "Flooded Grove",
            "Hinterland Harbor",
            "Overflowing Basin",
            "Quandrix Campus",
            "Rain-Slicked Copse",
            "Sodden Verdure",
            "Tangled Islet",
            "Temple of Mystery",
            "Turbulent Wilderness",
            "Vineglimmer Snarl",
        ] {
            assert_identity(name, &[Green, Blue]);
        }

        for name in [
            "Cascade Bluffs",
            "Coastal Peak",
            "Ferrous Lake",
            "Frostboil Snarl",
            "Molten Tributary",
            "Prismari Campus",
            "Restless Spire",
            "Scorched Geyser",
            "Sulfur Falls",
            "Talisman of Creativity",
            "Temple of Epiphany",
            "Turbulent Springs",
        ] {
            assert_identity(name, &[Blue, Red]);
        }

        for name in [
            "Caves of Koilos",
            "Desolate Mire",
            "Eclipsed Steppe",
            "Isolated Chapel",
            "Shineshadow Snarl",
            "Silverquill Campus",
        ] {
            assert_identity(name, &[White, Black]);
        }

        for name in [
            "Festering Thicket",
            "Haunted Mire",
            "Llanowar Wastes",
            "Necroblossom Snarl",
            "Temple of Malady",
            "Turbulent Fen",
            "Twilight Mire",
            "Vernal Fen",
            "Viridescent Bog",
            "Witherbloom Campus",
            "Woodland Cemetery",
        ] {
            assert_identity(name, &[Green, Black]);
        }

        for name in [
            "Sunlit Marsh",
            "Temple of Silence",
            "Turbulent Moor",
            "Umbral Expanse",
        ] {
            assert_identity(name, &[Black, White]);
        }
    }

    #[test]
    fn any_color_lands_that_are_really_two_symbol_duals_carry_both_colors() {
        // "Any color" is a genuine wildcard for these — but Battlefield Forge et al. are real
        // two-symbol duals flattened to "any", which on its own contributes no color at all.
        assert_identity("Alchemist's Refuge", &[Green, Blue]);
        assert_identity("Yavimaya Coast", &[Green, Blue]);
        assert_identity("Fetid Heath", &[White, Black]);
        assert_identity("Shivan Reef", &[Red, Blue]);

        for name in [
            "Sunscorched Divide",
            "Battlefield Forge",
            "Clifftop Retreat",
            "Rugged Prairie",
            "Furycalm Snarl",
            "Glittering Massif",
            "Lorehold Campus",
            "Radiant Summit",
            "Sacred Peaks",
            "Temple of Triumph",
            "Turbulent Steppe",
        ] {
            assert_identity(name, &[Red, White]);
        }
    }

    #[test]
    fn a_dropped_colored_activated_ability_still_carries_its_color() {
        // Haywire Mite's exile ability ({G}, sac) is dropped entirely by the simplified target
        // model, but its {G} still belongs to color identity.
        assert_identity("Haywire Mite", &[Green]);
    }

    #[test]
    fn genuine_any_color_wildcards_with_no_printed_symbol_stay_colorless() {
        for name in [
            "Command Tower",
            "Exotic Orchard",
            "Path of Ancestry",
            "Lotus Field",
            "Opal Palace",
            "Study Hall",
        ] {
            assert_identity(name, &[]);
        }
    }

    #[test]
    fn lands_already_correct_via_a_modeled_secondary_colored_ability_are_unchanged() {
        // These "flattened" lands still expose their color(s) through an exactly-modeled activated
        // ability elsewhere on the card, so no `identity_pips` addition is needed.
        assert_identity("Grim Backwoods", &[Black, Green]);
        assert_identity("Forum of Amity", &[White, Black]);
        assert_identity("Fields of Strife", &[Red, White]);
        assert_identity("Paradox Gardens", &[Green, Blue]);
        assert_identity("Spectacle Summit", &[Blue, Red]);
    }

    #[test]
    fn ordinary_single_color_creatures_are_sanity_controls() {
        assert_identity("Savannah Lions", &[White]);
        assert_identity("Llanowar Elves", &[Green]);
    }
}
