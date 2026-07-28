use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
pub enum ZoneEffect {
    AttachMintedAuraToTarget {
        target: TargetSpec,
    },

    AttachSelfToMintedToken,

    AttachSelfToReanimated,

    AttachTriggeringAuraToMintedToken {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
    },

    ExileDeadCreatureCreateCopyWithSubtype {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        dead: Option<ObjectId>,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        add_subtypes: &'static [&'static str],
        #[cfg_attr(feature = "card-dsl", serde(default))]
        leaves_returns_exiled: bool,
    },

    ExileGraveyardObjectGainLife {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
        amount: i32,
    },

    ExileSelfOnResolve,

    ExileSelfWithTimeCounters {
        counters: u32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        on_expiry: &'static [Effect],
    },

    ExileTargetGraveyardCardThenIfCreature {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    FlickerTarget {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        return_at: Option<Step>,
    },

    Manifest,

    MassReturnFromGraveyard {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        all_players: bool,
    },

    ReanimateDyingEnchantedCreature {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        dying: Option<ObjectId>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        under_owner: bool,
    },

    /// Tariel, Reckoner of Souls: "Choose a creature card at random from target opponent's
    /// graveyard. Put that card onto the battlefield under your control." `target` is the
    /// targeted opponent (CR-real target, `TargetSpec::OpponentPlayer`); the creature card
    /// itself is picked by the injected RNG at resolution — needs `&mut self`, so this resolves
    /// via `Game::run_misc_choreo` like `ExileRandomFromGraveyardMayPlay`'s "at random" pick,
    /// not the pure `mint_zones` path `ReanimateToBattlefield` (a chosen, not random, card) uses.
    ReanimateRandomFromTargetOpponentGraveyard {
        target: TargetSpec,
    },

    ReanimateToBattlefield {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        finality: bool,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::opt_static_reanimate_becomes")
        )]
        becomes: Option<&'static ReanimateBecomes>,
    },

    ReflexiveTrigger {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    /// "When one or more nonland cards are exiled this way, …" (CR 603.3b — Augusta, Order
    /// Returned). The reflexive twin of [`ReflexiveTrigger`](Self::ReflexiveTrigger) gated on a
    /// count rather than a minted token: placed after the same resolution's
    /// [`EachPlayerExilesFromGraveyard`](crate::ChoiceEffect::EachPlayerExilesFromGraveyard) fan-out,
    /// it creates a reflexive triggered ability for each `then` effect **only when that fan-out
    /// exiled one or more nonland cards** — none at all when the count is zero. The count
    /// ([`ResolutionFrame::nonland_cards_exiled_this_way`](crate::resolution::ResolutionFrame)) is
    /// baked into each `then` effect's [`Amount::NonlandCardsExiledThisWay`](crate::Amount) at
    /// placement, so the follow-up reads the settled number even though it resolves in its own
    /// later frame; its target is chosen when it goes on the stack (CR 601.2c), after the fan-out.
    ReflexiveTriggerIfNonlandExiled {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    ReturnAllToHand {
        filter: PermanentFilter,
    },

    ReturnExiledCardToOwnersGraveyard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        exiled: ObjectId,
    },

    ReturnFlickeredCard {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        exiled: Option<ObjectId>,
    },

    ReturnFromGraveyardAttachedToToken {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        token: Option<ObjectId>,
    },

    ReturnFromGraveyardToHand {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    ReturnObjectToHand {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        object: Option<ObjectId>,
    },

    ReturnThisAuraAttachedTo {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        creature: Option<ObjectId>,
    },

    ReturnThisAuraFromGraveyardAttachedToChosenHost,

    ReturnThisFromGraveyardToBattlefield {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },

    ReturnThisToHand,

    ReturnToHand {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    ScheduleReturnReanimatedToHand,

    ScheduleReturnThisAuraAttachedToReanimated,

    ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost,

    ShuffleTargetPermanentIntoLibrary {
        target: TargetSpec,
    },

    ShuffleTargetPermanentIntoLibraryThenReveal {
        target: TargetSpec,
    },

    TuckFromGraveyard {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_top: bool,
    },

    TuckPermanentIntoLibrary {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        to_top: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        second_from_top: bool,
    },

    TuckSelfAndBlockedCreatures,

    TuckSelfToLibraryBottom,

    UntapSearchedLand,
}
