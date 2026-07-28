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

    Source,
}
