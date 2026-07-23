use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
pub enum ExileEffect {
    All {
        filter: PermanentFilter,
    },

    AllGraveyards,

    Graveyard,

    Object {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },

    Target {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    TargetMintingIllusionOnLeave {
        target: TargetSpec,
    },

    UntilSourceLeaves {
        target: TargetSpec,
    },
}
