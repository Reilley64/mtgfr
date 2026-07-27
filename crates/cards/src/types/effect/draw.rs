use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
pub enum DrawEffect {
    AttackingPlayer {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        drawer: Option<PlayerId>,
        count: u32,
    },

    Cards {
        count: Amount,
    },

    EachDrawStepPlayer {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        drawer: Option<PlayerId>,
        count: u32,
    },

    EachPlayer {
        count: Amount,
    },

    TargetOwner {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        controller: bool,
    },

    TargetPlayer {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },
}
