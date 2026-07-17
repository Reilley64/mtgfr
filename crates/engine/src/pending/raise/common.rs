//! Identity `ChoiceRequest` → `PendingChoice` mappings (isomorphic fields, always pause).

use crate::PendingChoice;

use super::ChoiceRequest;

/// Generate [`map_identical`] for isomorphic request → pending variants.
macro_rules! define_map_identical {
    ($($variant:ident { $($field:ident),+ $(,)? }),+ $(,)?) => {
        /// Map isomorphic request variants, or return the request unchanged for family handlers.
        pub(super) fn map_identical(
            request: ChoiceRequest,
        ) -> Result<PendingChoice, ChoiceRequest> {
            match request {
                $(
                    ChoiceRequest::$variant { $($field),+ } => {
                        Ok(PendingChoice::$variant { $($field),+ })
                    }
                )+
                other => Err(other),
            }
        }
    };
}

define_map_identical! {
    ChooseTarget {
        player,
        source,
        effect,
        legal,
        optional,
    },
    PayOrCounter {
        player,
        cost,
        spell,
    },
    ChooseCreatureType {
        player,
        source,
        options,
    },
    ChooseColor { player, source },
    ChooseMode {
        player,
        source,
        target,
        x,
        modes,
    },
    MayYesNo {
        player,
        source,
        effect,
    },
    DivideSpellDamage {
        player,
        spell,
        targets,
        total,
    },
    DivideCounters {
        player,
        spell,
        targets,
        total,
    },
    ChooseManaColor {
        player,
        source,
        amount,
    },
    SacrificeUnlessPay {
        player,
        source,
        cost,
    },
    ChooseTargetPlayers {
        player,
        source,
        max,
        legal,
        min,
        keep_one,
        filter,
        life_loss,
        then,
    },
    DanceExileMore {
        player,
        source,
        exiled,
        total_mv,
        budget,
    },
    OpponentChoosesPile {
        player,
        controller,
        source,
        pile_a,
        pile_b,
    },
    PartitionRevealed {
        player,
        controller,
        source,
        revealed,
    },
    ChoosePileForHand {
        player,
        source,
        pile_a,
        pile_b,
    },
    RevealedCardToBattlefieldOrHand { player, card },
}
