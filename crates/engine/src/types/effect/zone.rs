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
