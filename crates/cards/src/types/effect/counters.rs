use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
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

    /// "Each opponent loses all counters" (Final Act) — CR 122.1/121.2: every counter of every
    /// kind on each player in `scope` is removed, not just poison.
    RemoveAllPlayerCounters {
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

    /// "Remove all counters from target nonland permanent you control" (Nexus Mentality) /
    /// "remove all but one +1/+1 counter from it" (Lily Bowen, Raging Grandma). Tallies how many
    /// counters came off into [`ResolutionFrame::counters_removed_this_way`] for a following step
    /// to read back through [`Amount::CountersRemovedThisWay`] — the "for each counter removed
    /// this way" payoff is an ordinary next effect (draw, gain life), not a rider on this one.
    ///
    /// `all_kinds` also sweeps every named [`CounterKind`], not just +1/+1 counters. `keep` leaves
    /// that many +1/+1 counters behind, and is a no-op when the permanent already has that few
    /// ("all but one" of nothing or one is nothing).
    ///
    /// Resolves via [`crate::Effect`] dispatch on the `Game::run` path rather than through event
    /// minting, because minting is `&self` and this writes the resolution frame.
    RemoveCounters {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_kinds: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        keep: u32,
    },

    RemoveCounterFromSelf,

    /// "Put a loyalty counter on each Garruk you control" (the Wolf token minted by Garruk,
    /// Cursed Huntsman's `0`) — a permanent-type filter walk, same shape as
    /// [`Self::PutCountersEach`], but loyalty is the scalar `Permanent::loyalty`
    /// ([`crate::Event::LoyaltyChanged`]), not a [`CounterKind`], so it can't reuse that variant's
    /// counter-placement events.
    PutLoyaltyCounterEach {
        filter: PermanentFilter,
    },
}
