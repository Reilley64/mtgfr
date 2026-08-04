use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum ExileEffect {
    All {
        filter: PermanentFilter,
    },

    AllGraveyards,

    Graveyard,

    /// "Exile Stangg Twin when Stangg leaves the battlefield" — the exile half of
    /// [`SacrificeEffect::LinkedTwin`](crate::SacrificeEffect::LinkedTwin): exile the permanent
    /// paired with the source (`token.link_as_twin`), found by scanning the battlefield for the
    /// permanent that points back at it. A token partner ceases to exist instead (CR 111.7).
    LinkedTwin,

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
