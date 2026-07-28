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
pub enum DamageEffect {
    EachCreature {
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponents_only: bool,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        filter: Option<PermanentFilter>,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        include_planeswalkers: bool,
    },

    EachOtherOpponent {
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        damaged: Option<PlayerId>,
    },

    /// The old "Radiance" keyword action (Cleansing Beam): "deals `amount` damage to target
    /// creature and each other creature that shares a color with it." One real target (`target`,
    /// a single creature — CR 608.2b legality/protection/hexproof gate only that choice); the
    /// rest of the batch is [`Game::radiance_batch`], swept in untargeted at resolution.
    Radiance {
        amount: Amount,
        target: TargetSpec,
    },

    EachPlayer {
        amount: Amount,
    },

    /// Damage to each living opponent of the ability's controller (CR 102.3) — Advanced
    /// Reconstruction / Fateful Tempest. Same per-player events as [`Self::EachPlayer`], but
    /// the controller is carved out.
    EachOpponent {
        amount: Amount,
    },

    Target {
        amount: Amount,
        target: TargetSpec,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        count: TargetCount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        divided: bool,
    },

    ToEnteringPermanent {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
        amount: i32,
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_str_slice")
        )]
        then_if_subtype: &'static [&'static str],
        #[cfg_attr(
            feature = "card-dsl",
            serde(default, deserialize_with = "de::static_slice")
        )]
        then: &'static [Effect],
    },

    ToSelf {
        amount: Amount,
    },

    ToTargetController {
        amount: Amount,
    },
}
