use super::*;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)
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

    EachPlayerLoses {
        amount: Amount,
    },

    Gain {
        amount: Amount,
    },

    GainTargetController {
        amount: Amount,
    },

    Lose {
        amount: Amount,
    },

    /// "Its owner loses half their life, rounded up" (Personal Incarnation's dies trigger). The
    /// only life loss in the pool billed to the source's *owner* rather than the ability's
    /// controller, and the only one that reads a life total to size itself — so both live in the
    /// variant rather than in an [`Amount`] and a player selector nothing else would use.
    SourceOwnerLosesHalfTheirLife,

    OpponentGains {
        amount: Amount,
    },

    TargetPlayerGains {
        amount: Amount,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        opponent: bool,
    },

    TargetPlayerLoses {
        amount: i32,
    },
}
