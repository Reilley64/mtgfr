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
pub enum TokenEffect {
    BecomeCopyOfTargetCreatureGainingMyriad {
        target: TargetSpec,
    },

    CopyEachEnteredThisTurnTokenTappedAttacking {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacking_context: Option<(PlayerId, PlayerId)>,
    },

    Create {
        #[cfg_attr(feature = "card-dsl", serde(deserialize_with = "de::token_profile"))]
        token: CardDef,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        controller: TokenController,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::zero_amount"))]
        enters_with: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        set_base_pt: Option<Amount>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exile_at_next_end_step: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        enters_tapped_and_attacking: bool,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacking_context: Option<(PlayerId, PlayerId)>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        must_attack_defender: bool,
    },

    CreateCopy {
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        targets: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        sacrifice_at_next_end_step: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exile_at_next_end_step: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        haste: bool,
    },

    CreateTreasure {
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        target_player: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },

    MyriadTokenCopies {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacking_context: Option<(PlayerId, PlayerId)>,
    },
}
