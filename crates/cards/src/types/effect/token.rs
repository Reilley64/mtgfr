use super::*;
#[cfg(feature = "card-dsl")]
use crate::de;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "card-dsl",
    derive(serde::Deserialize),
    serde(tag = "mode", rename_all = "snake_case")
)]
#[cfg_attr(feature = "card-schema", derive(schemars::JsonSchema))]
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
        #[cfg_attr(feature = "card-schema", schemars(with = "String"))]
        token: CardDef,
        #[cfg_attr(feature = "card-dsl", serde(default = "de::one_amount"))]
        count: Amount,
        /// Who the tokens enter under (CR 111.4) — the ability's controller by default,
        /// `targets_controller` for Beast Within's "its controller creates a 3/3 Beast",
        /// `target_player` / `target_opponent` for a chosen seat, `each_other_player` for Death
        /// by Dragons. A multi-seat set mints the whole `count` under *each* named seat.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        /// "**For each opponent**, you create …" (Eccentric Pestfinder, Furygale Flocking): the
        /// batch repeats once per opponent while `who` still names the recipient, so this is a
        /// repeat count rather than a recipient — spelling it as `who = "each_opponent"` would
        /// hand the tokens to the opponents instead. With `must_attack_defender`, each repeat's
        /// tokens are bound to *that* opponent rather than to the one flattened defender.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        per_opponent: bool,
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
        /// "Create Stangg Twin … Exile that token when Stangg leaves the battlefield. Sacrifice
        /// Stangg when that token leaves the battlefield." — pair the minted token with the
        /// creating permanent, so each half's leaves-the-battlefield ability can name the other
        /// ([`TargetSpec::LinkedTwin`](crate::TargetSpec), [`SacrificeEffect::LinkedTwin`]). The
        /// two abilities themselves are printed on the two cards; this only ties the knot.
        #[cfg_attr(feature = "card-dsl", serde(default))]
        link_as_twin: bool,
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
        /// Who the Treasures land under — the ability's controller by default, `target_player` for
        /// the chosen seat (Prismari Command's "target player creates a Treasure token").
        #[cfg_attr(feature = "card-dsl", serde(default))]
        who: PlayerSet,
        #[cfg_attr(feature = "card-dsl", serde(default))]
        tapped: bool,
    },

    MyriadTokenCopies {
        #[cfg_attr(feature = "card-dsl", serde(skip))]
        attacking_context: Option<(PlayerId, PlayerId)>,
    },
}
