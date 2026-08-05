use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum SacrificeEffect {
    EnchantedCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
    },

    Object {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },

    /// "Sacrifice Stangg when that token leaves the battlefield" — sacrifice the permanent this
    /// one is paired with, the other half of the pair the token was minted into
    /// (`token.link_as_twin`). Untargeted: the partner is whichever battlefield permanent points
    /// back at the source, so it is found at resolution even though the source itself has already
    /// left. Nothing happens if the partner is already gone.
    LinkedTwin,

    Source,
}
