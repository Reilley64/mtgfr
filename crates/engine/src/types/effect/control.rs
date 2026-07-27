use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)
)]
pub enum ControlEffect {
    AttachSelfToEntering {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
    },

    Equip,

    ExchangeAllCreaturesUntilEndOfTurn {
        target: TargetSpec,
    },

    ExchangeControl {
        first: TargetSpec,
        second: TargetSpec,
    },

    GainControl {
        target: TargetSpec,
    },

    GainControlAllUntilEndOfTurn {
        filter: PermanentFilter,
    },

    GainControlUntilEndOfTurn {
        target: TargetSpec,
    },

    GainControlWhile {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        while_source_tapped: bool,
    },

    GoadTarget {
        target: TargetSpec,
    },

    GrantSourceAbilitiesUntilEndOfTurn,

    RegenerateShield {
        target: TargetSpec,
    },

    RemoveFromCombat {
        target: TargetSpec,
    },

    RevertAllCreaturesToOwners,

    TapAll {
        filter: PermanentFilter,
    },

    /// "Tap this creature" as an *effect* (Demonic Hordes' unpaid-upkeep penalty), not as the
    /// `{T}` in an activation cost — the source taps itself on resolution, with nothing chosen and
    /// nothing targeted. A permanent that has already left the battlefield taps nothing.
    TapSource,

    TapTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    TargetOpponentGainsControl {
        target: TargetSpec,
        player: TargetSpec,
    },

    UntapAll {
        filter: PermanentFilter,
    },

    UntapTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },
}
