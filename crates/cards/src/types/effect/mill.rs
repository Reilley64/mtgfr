use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
pub enum MillEffect {
    ExileDiscardedWithThis {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },

    ExileFromGraveyardMayPlay {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        card: Option<ObjectId>,
    },

    ExileTargetFromGraveyardCreateTokenCopy {
        filter: CardFilter,
    },

    ExileTargetFromGraveyardWithThis,

    ExileTopMayPlay {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        until_next_turn: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        face_down: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        free_while_source: bool,
    },

    Mill {
        count: Amount,
        target: TargetSpec,
    },

    MillSelf {
        count: Amount,
    },
}
