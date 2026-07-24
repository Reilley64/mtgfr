use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
pub enum LifeEffect {
    AttackerLosesYouDraw {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacker: Option<PlayerId>,
        life_loss: u32,
    },

    AttackerLosesYouGain {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacker: Option<PlayerId>,
        amount: u32,
    },

    DrainTarget {
        amount: i32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },

    EachOpponentDrain {
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        sum_gain: bool,
    },

    EachOpponentLoses {
        amount: Amount,
    },

    EachPlayerBecomesHighest,

    Gain {
        amount: Amount,
    },

    GainTargetController {
        amount: Amount,
    },

    Lose {
        amount: Amount,
    },

    OpponentGains {
        amount: Amount,
    },

    TargetPlayerGains {
        amount: i32,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },

    TargetPlayerLoses {
        amount: i32,
    },
}
