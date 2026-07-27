use serde::Deserialize;

use crate::de::intern_strs;
use crate::{CardKind, LandProduces, SpellSpeed, TypeSet};

/// A `[kind]` table spells instants and sorceries as their own `type` tags
/// (`type = "instant"`) rather than as [`CardKind::Spell`]'s `speed` field.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KindToml {
    Creature {
        power: i32,
        toughness: i32,
        /// Additional card types (Artifact Creature, Enchantment Creature) — a list of
        /// type names. Empty for a plain creature.
        #[serde(default)]
        #[cfg_attr(feature = "card-schema", schemars(with = "serde_json::Value"))]
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
    Battle {
        defense: i32,
    },
    Land {
        /// Optional sugar for a free "{T}: Add one mana" base tap; omitted for a
        /// fetch-only land or a land whose mana is all explicit `add_mana` abilities.
        #[serde(default)]
        #[cfg_attr(feature = "card-schema", schemars(with = "Option<serde_json::Value>"))]
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

impl From<KindToml> for CardKind {
    fn from(kind: KindToml) -> Self {
        match kind {
            KindToml::Creature {
                power,
                toughness,
                also,
            } => CardKind::Creature {
                power,
                toughness,
                also,
            },
            KindToml::Instant => CardKind::Spell {
                speed: SpellSpeed::Instant,
            },
            KindToml::Sorcery => CardKind::Spell {
                speed: SpellSpeed::Sorcery,
            },
            KindToml::Enchantment => CardKind::Enchantment,
            KindToml::Aura => CardKind::Aura,
            KindToml::Artifact => CardKind::Artifact,
            KindToml::Planeswalker { loyalty } => CardKind::Planeswalker { loyalty },
            KindToml::Battle { defense } => CardKind::Battle { defense },
            KindToml::Land {
                produces,
                subtypes,
                basic,
            } => CardKind::Land {
                produces,
                subtypes: intern_strs(subtypes),
                basic,
            },
        }
    }
}
