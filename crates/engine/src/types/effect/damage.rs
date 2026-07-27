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
        /// Disintegrate's "If it's a creature, it can't be regenerated this turn" (CR 701.15d) —
        /// a rider on the damaged creature rather than on a destruction, since the damage is what
        /// marks it and the state-based action that later kills it carries nothing.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        cant_be_regenerated: bool,
        /// Disintegrate's "and if it would die this turn, exile it instead" — the same dies
        /// replacement a finality counter applies (CR 614.12), but nameless and turn-scoped.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        exile_instead_of_dying: bool,
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

    /// Ankh of Mishra's "deals 2 damage to **that land's** controller" — the player twin of
    /// [`ToEnteringPermanent`](Self::ToEnteringPermanent), filling the same `entering` slot from
    /// the same `PermanentEnters` trigger. Distinct from
    /// [`ToTargetController`](Self::ToTargetController), which reads an enclosing `Sequence`'s
    /// chosen target — a trigger that targets nothing never sets one.
    ToEnteringPermanentController {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        entering: Option<ObjectId>,
        amount: Amount,
    },

    /// Copper Tablet's "at the beginning of each player's upkeep … deals 1 damage to **that
    /// player**" — the player whose step this is, filled from [`TriggerContext::active_player`] at
    /// trigger placement. [`EachPlayer`](Self::EachPlayer) would bill the whole table on every
    /// upkeep, which is once per seat per round instead of once.
    ToTriggeringPlayer {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        player: Option<PlayerId>,
        amount: Amount,
    },

    ToSelf {
        amount: Amount,
    },

    ToTargetController {
        amount: Amount,
    },
}
