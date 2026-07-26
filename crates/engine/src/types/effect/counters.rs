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

    /// "Monstrosity N" (CR 701.28a — Alpha Deathclaw's "{5}{B}{G}: Monstrosity 4"): a no-op if
    /// the source is already monstrous (CR 701.28c); otherwise puts `count` +1/+1 counters on it
    /// (through the replacement pipeline) and sets [`Permanent::monstrous`], mirroring
    /// [`Self::LevelUp`]'s self-targeting, source-mutating shape.
    Monstrosity {
        count: u8,
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
    /// living player in `scope`. Targets nothing except under
    /// [`EdictScope::TargetedOpponent`] ("target opponent gets a poison counter", Venerated
    /// Rotpriest), which names exactly one chosen opponent.
    PutCountersOnPlayer {
        kind: PlayerCounterKind,
        count: Amount,
        scope: EdictScope,
    },

    /// "If target player has fewer than nine poison counters, they get a number of poison counters
    /// equal to the difference" (Vraska, Betrayal's Sting's −9) — a *top-up* to `to`, not a fixed
    /// add: the count placed is `to - current`, and a target already at or above `to` gets nothing
    /// at all (no counters, no event). Targets a player (CR "target player").
    TopUpCountersOnPlayer {
        kind: PlayerCounterKind,
        to: u8,
    },

    RemoveAllCountersThenDraw {
        target: TargetSpec,
    },

    /// "remove all but one +1/+1 counter from it, then you gain 1 life for each +1/+1 counter
    /// removed this way" (Lily Bowen, Raging Grandma) — the cull-and-gain sibling of
    /// [`Self::RemoveAllCountersThenDraw`]: keeps exactly one +1/+1 counter (a no-op with zero or
    /// one already present — "all but one" of nothing or one is nothing) and the life gained is
    /// the number actually removed, not a flat amount.
    /// ponytail: +1/+1-only and always "keep one, gain life" — Lily Bowen is the only consumer;
    /// grow a `keep`/`gain_life` rider (or a `kind` axis) on `RemoveAllCountersThenDraw` instead of
    /// a new sibling if a future card needs a different keep-count or payoff.
    RemoveAllButOnePlusOneCounterThenGainLife {
        target: TargetSpec,
    },

    RemoveCounterFromSelf,
}
