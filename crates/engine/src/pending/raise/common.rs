//! Identity `ChoiceRequest` → `PendingChoice` mappings (isomorphic fields, always pause).

use crate::PendingChoice;

use super::ChoiceRequest;

/// Generate [`map_identical`] for isomorphic request → pending variants.
macro_rules! define_map_identical {
    ($($variant:ident { $($field:ident),+ $(,)? }),+ $(,)?) => {
        /// Map isomorphic request variants; `None` means a family handler must build the choice.
        pub(super) fn map_identical(request: &ChoiceRequest) -> Option<PendingChoice> {
            match request {
                $(
                    ChoiceRequest::$variant { $($field),+ } => {
                        Some(PendingChoice::$variant {
                            $($field: $field.clone()),+
                        })
                    }
                )+
                _ => None,
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
    ChooseColor {
        player,
        source,
        until_end_of_turn,
    },
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
