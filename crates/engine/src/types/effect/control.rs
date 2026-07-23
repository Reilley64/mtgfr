use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
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
