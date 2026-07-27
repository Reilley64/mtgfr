use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
pub enum ManaEffect {
    Add {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::mana_batch")
        )]
        mana: ManaPool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        identity: u8,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent_colors: u8,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        repeat: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        restriction: Option<SpendRestriction>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        single_color: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        track_provenance: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        persist_until_end_of_turn: bool,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        recipient: Option<PlayerId>,
    },
}
