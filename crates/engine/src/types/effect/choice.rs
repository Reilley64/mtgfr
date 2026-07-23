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
pub enum ChoiceEffect {
    CastCreatureFaceDown,

    CasterKeepsOneOfEachTypePerPlayer,

    ChooseColor,

    ChooseCreatureType,

    CouncilsDilemmaVote {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::static_str_slice"))]
        options: &'static [&'static str],
    },

    DamagingCreatureControllerMayDraw {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        drawer: Option<PlayerId>,
        count: u32,
    },

    DefendingPlayerSacrifices {
        count: u8,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        defender: Option<PlayerId>,
    },

    Discard {
        count: u32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        or_one_matching: Option<CardFilter>,
    },

    EachOtherTokenBecomesCopyOfChosen,

    EachPlayerControllerChoosesCounterTarget,

    EachPlayerCreatesFractalFromExiledPower {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        token: CardDef,
    },

    EachPlayerDiscardsHandThenDraws {
        count: Amount,
    },

    EachPlayerExilesFromGraveyard,

    EachPlayerNamesCardThenRevealsTop,

    EachPlayerSacrifices {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::all_players"))]
        scope: EdictScope,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        keep_one: bool,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::creature_edict"))]
        filter: PermanentFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        life_loss: i32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    JoinForcesPayMana,

    MayDiscard {
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    MayDrawUnlessPays {
        cost: Amount,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        caster: Option<PlayerId>,
    },

    MayDrawUpTo {
        count: Amount,
    },

    MayDrawUpToThenOpponentMayRepeat {
        count: Amount,
    },

    MayReturnFromGraveyard {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        if_you_sacrificed_this_way: bool,
    },

    MaySacrifice {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: PermanentFilter,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    PhaseOut,

    Proliferate {
        times: Amount,
    },

    PutCounterThenMayBecomeCopyOfCardFromList {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        cards: &'static [ObjectId],
    },

    PutCreatureFromHand,

    PutFromHandOnTop {
        count: u32,
    },

    PutLandFromHand {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },

    SacrificeOwn {
        filter: PermanentFilter,
        count: u32,
    },

    SacrificeSelfUnlessPay {
        cost: Cost,
    },

    SacrificeSelfUnlessReturnLand {
        filter: PermanentFilter,
    },

    SetOwnColorUntilEndOfTurn,

    TargetPlayerExilesFromGraveyard {
        target: TargetSpec,
    },

    TargetPlayerMayDraw {
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },
}
