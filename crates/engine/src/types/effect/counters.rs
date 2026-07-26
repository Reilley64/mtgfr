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
        /// `None` puts +1/+1 counters (the historical, still-default spelling); `Some(kind)`
        /// (Contagion Engine's "-1/-1 counter on each creature target player controls") puts
        /// that named kind instead, mirroring [`Self::PutCounters`]'s own `kind` axis.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        kind: Option<CounterKind>,
    },

    /// "Each opponent gets a poison counter" (Infectious Inquiry, Vraska's Fall) / "each player
    /// gets a poison counter" (Ichor Rats) — CR 122.1. Places `count` counters of `kind` on every
    /// living player in `scope`, and targets nothing.
    PutCountersOnPlayer {
        kind: PlayerCounterKind,
        count: Amount,
        scope: EdictScope,
    },

    RemoveAllCountersThenDraw {
        target: TargetSpec,
    },

    RemoveCounterFromSelf,
}
