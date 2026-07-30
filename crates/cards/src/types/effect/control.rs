use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
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
        /// "Creatures it was blocking that had become blocked by only that creature this combat
        /// become unblocked" (False Orders) — the one printed exception to CR 509.1h's sticky
        /// blocked-ness. Drops this blocker's pairs from `CombatState::blocked_ever`, so an
        /// attacker it was the only blocker of goes back to unblocked and one that a second
        /// creature is also blocking does not. `false` (Spurnmage Advocate) leaves 509.1h alone:
        /// the creature stops blocking, and everything it blocked stays blocked.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        release_solely_blocked: bool,
    },

    RevertAllCreaturesToOwners,

    /// A tap sweep over whatever `filter` matches — Dread Cacodemon's "tap all other creatures you
    /// control" (`controller = "you"`) or Arena of the Ancients' table-wide "tap all legendary
    /// creatures" (the default [`FilterController::Any`](crate::FilterController)). The seat
    /// restriction lives entirely in the filter, unlike
    /// [`TapAllTargetPlayerControls`](Self::TapAllTargetPlayerControls), which reads the seat off a
    /// chosen target instead.
    TapAll {
        filter: PermanentFilter,
    },

    /// "Tap all lands target player controls" (Mana Short) — [`TapAll`](Self::TapAll)'s
    /// other-seat twin. `TapAll` is a "you control" sweep with no target at all; this one taps the
    /// chosen player's board and leaves yours alone. A plain tap, not a tap *for mana* (CR 106.11)
    /// — nothing is produced and no land-tap watch fires, which is exactly what the card wants,
    /// since the next step takes their pool away.
    TapAllTargetPlayerControls {
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
