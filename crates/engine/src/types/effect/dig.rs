use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)
)]
pub enum DigEffect {
    Cascade {
        mana_value: u32,
    },

    CashOutExiledWithThis,

    CastExiledWithThisFree,

    Clash,

    DistributeTop {
        count: u32,
        to_hand: u32,
        to_bottom: u32,
        to_exile_may_play: u32,
    },

    EachPlayerExilesUntilNonlandOpponentPicks,

    ExileRandomFromGraveyardMayPlay,

    ExileTargetGraveyardCardRecordManaValue {
        filter: CardFilter,
    },

    ExileTargetGraveyardSpellCastFree {
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
    },

    ExileTopCastMatchingFree {
        count: u32,
        filter: CardFilter,
    },

    ExileTopUntilStopCastFreeUnderBudget {
        budget: u32,
    },

    LookAtTop {
        count: u32,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::any_card_filter"))]
        filter: CardFilter,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_u32"))]
        up_to: u32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        min: u32,
        dest: TopDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        dest_tapped: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        rest: RestDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        mv_budget: Option<u32>,
    },

    LookAtTargetPlayersHand,

    /// "You may have that player shuffle" (Natural Selection). `owner` is the targeted player,
    /// baked in when the yes/no is raised so the answer knows whose library to shuffle.
    MayShuffleTargetPlayersLibrary {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        owner: Option<PlayerId>,
    },

    OpponentSplitsExilePiles,

    /// "Look at the top `count` cards of target player's library, then put them back in any
    /// order" (Natural Selection). Unlike a scry there is no bottom pile — every card goes back
    /// on top, so the only choice is the order.
    RearrangeTargetPlayersTop {
        count: u32,
    },

    RevealTopOpponentPicksOneToGraveyard {
        count: u8,
    },

    RevealTopSplitPiles,

    RevealUntilExileCastFree {
        filter: CardFilter,
    },

    RevealUntilMayDeploy {
        filter: CardFilter,
    },

    Scry {
        count: Amount,
    },

    SearchLibrary {
        filter: CardFilter,
        to_zone: SearchDest,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        searcher: SearchScope,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default = "de::one_u8", deserialize_with = "de::count_or_any")
        )]
        count: u8,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        overflow: Option<SearchDest>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count_amount: Option<Amount>,
        /// Printed "may search" (White Orchid Phantom): pause on a yes/no *before* searching.
        /// Declining skips the search entirely — no fail-to-find, no shuffle (CR 701.19c only
        /// applies once a search actually begins). Accepting runs the ordinary search (including
        /// fail-to-find + shuffle). Distinct from ability-level `optional` (Fierce Empath), which
        /// gates the whole ability before it goes on the stack.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        optional: bool,
    },

    ShuffleLibrary,

    ShuffleTargetCardsFromGraveyardIntoLibrary {
        #[cfg_attr(feature = "card-dsl", serde(default))]
        max: u32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
    },

    Surveil {
        count: u32,
    },
}
