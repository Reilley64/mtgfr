use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
pub enum CountersEffect {
    AttackerDrawsControllerCounters {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacker: Option<PlayerId>,
        counters: u32,
    },

    CommanderEntersWithBonusCounters {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        triggering_spell: Option<ObjectId>,
        count: Amount,
    },

    DoubleCounters {
        target: TargetSpec,
    },

    DoubleCountersOnAttachedCreature,

    DoubleCountersOnTargetCreatures {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    LevelUp {
        level: u8,
    },

    MoveCounters {
        target: TargetSpec,
        to_filter: PermanentFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_kinds: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        distributed: bool,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        from: Option<Target>,
    },

    PlaceVowCounters {
        filter: PermanentFilter,
    },

    PutCounters {
        count: Amount,
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        targets: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        kind: Option<CounterKind>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        divided: bool,
    },

    PutCountersEach {
        filter: PermanentFilter,
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
    },

    RemoveAllCountersThenDraw {
        target: TargetSpec,
    },

    RemoveCounterFromSelf,
}
