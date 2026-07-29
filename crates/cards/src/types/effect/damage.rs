use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

/// How a damage effect's total is split among its chosen targets (CR 601.2d).
///
/// Spelled in TOML as `divided`: absent or `false` for [`Self::None`], `true` for
/// [`Self::AsYouChoose`], `"evenly"` for [`Self::Evenly`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Division {
    /// Not divided — every chosen target takes the full amount (the overwhelming majority).
    #[default]
    None,
    /// "divided as you choose" (Magma Opus) — the caster assigns the split, at least 1 each,
    /// right after targets are chosen.
    AsYouChoose,
    /// "divided evenly, rounded down" (Fireball) — a computed split with no choice at all, so a
    /// remainder simply isn't dealt (7 among 3 is 2 apiece) and 2 among 3 is nothing at all.
    Evenly,
}

#[cfg(feature = "card-dsl")]
impl<'de> serde::Deserialize<'de> for Division {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        /// Untagged so TOML's own scalar type picks the arm, like `[cost]`'s `x` key.
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Spelling {
            Chosen(bool),
            Named(String),
        }

        match Spelling::deserialize(d)? {
            Spelling::Chosen(false) => Ok(Division::None),
            Spelling::Chosen(true) => Ok(Division::AsYouChoose),
            Spelling::Named(name) if name == "evenly" => Ok(Division::Evenly),
            Spelling::Named(name) => Err(serde::de::Error::custom(format!(
                "unknown division {name:?} (expected true, false, or \"evenly\")"
            ))),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum DamageEffect {
    EachCreature {
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponents_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: Option<PermanentFilter>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        include_planeswalkers: bool,
    },

    /// "deals `amount` damage to `who`" — every damage the pool aims at seats rather than at
    /// permanents, from Psionic Blast's "2 damage to you" to Pestilence's "each player" to Ankh of
    /// Mishra's "that land's controller". Who takes it and how much are independent axes
    /// ([`PlayerSet`] and [`Amount`]), so no variant names a recipient.
    ///
    /// A player-relative `amount` counts the seat being damaged, not the ability's controller —
    /// Karma's "damage equal to the number of Swamps **they** control", Black Vise's "cards in
    /// **their** hand". Resolved once per recipient, so a fan-out bills each seat for its own
    /// things.
    ToPlayers {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        amount: Amount,
    },

    /// The old "Radiance" keyword action (Cleansing Beam): "deals `amount` damage to target
    /// creature and each other creature that shares a color with it." One real target (`target`,
    /// a single creature — CR 608.2b legality/protection/hexproof gate only that choice); the
    /// rest of the batch is [`Game::radiance_batch`], swept in untargeted at resolution.
    Radiance { amount: Amount, target: TargetSpec },

    Target {
        amount: Amount,
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        divided: Division,
        /// Disintegrate's "If it's a creature, it can't be regenerated this turn" (CR 701.15d) —
        /// a rider on the damaged creature rather than on a destruction, since the damage is what
        /// marks it and the state-based action that later kills it carries nothing.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_regenerated: bool,
        /// Disintegrate's "and if it would die this turn, exile it instead" — the same dies
        /// replacement a finality counter applies (CR 614.12), but nameless and turn-scoped.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exile_instead_of_dying: bool,
        /// Drain Life's "You gain life equal to the damage dealt, but not more life than the
        /// player's life total before the damage was dealt, the planeswalker's loyalty before the
        /// damage was dealt, or the creature's toughness" — the controller gains what this effect
        /// actually landed, so a prevented or shielded hit feeds nothing.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        gain_life_equal_to_damage: bool,
    },

    ToEnteringPermanent {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
        amount: i32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        then_if_subtype: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },
}
