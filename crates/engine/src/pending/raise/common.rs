//! Identity `ChoiceRequest` → `PendingChoice` mappings (isomorphic fields, always pause).

use crate::PendingChoice;

use super::ChoiceRequest;

/// Map isomorphic request variants; `None` means a family handler must build the choice.
pub(super) fn map_identical(request: &ChoiceRequest) -> Option<PendingChoice> {
    match request {
        ChoiceRequest::ChooseTarget {
            player,
            source,
            effect,
            legal,
            count,
            x,
            activated,
        } => Some(PendingChoice::ChooseTarget {
            player: *player,
            source: *source,
            effect: Some(effect.clone()),
            legal: legal.clone(),
            count: *count,
            clause: 0,
            target: None,
            x: *x,
            spent_mana: [0; 6],
            activated: *activated,
        }),
        ChoiceRequest::PayOrCounter {
            player,
            cost,
            spell,
            strips_mana_on_decline,
        } => Some(PendingChoice::PayOrCounter {
            player: *player,
            cost: *cost,
            spell: *spell,
            strips_mana_on_decline: *strips_mana_on_decline,
        }),
        ChoiceRequest::ChooseCreatureType {
            player,
            source,
            options,
        } => Some(PendingChoice::ChooseCreatureType {
            player: *player,
            source: *source,
            options,
        }),
        ChoiceRequest::ChooseColor {
            player,
            source,
            until_end_of_turn,
        } => Some(PendingChoice::ChooseColor {
            player: *player,
            source: *source,
            until_end_of_turn: *until_end_of_turn,
        }),
        ChoiceRequest::ChooseMode {
            player,
            source,
            target,
            x,
            modes,
            at_placement,
            activated,
        } => Some(PendingChoice::ChooseMode {
            player: *player,
            source: *source,
            target: *target,
            x: *x,
            modes: modes.clone(),
            at_placement: *at_placement,
            activated: *activated,
        }),
        ChoiceRequest::MayYesNo {
            player,
            source,
            effect,
            resume,
        } => Some(PendingChoice::MayYesNo {
            player: *player,
            source: *source,
            effect: effect.clone(),
            resume: resume.clone(),
        }),
        ChoiceRequest::DivideSpellDamage {
            player,
            spell,
            targets,
            total,
        } => Some(PendingChoice::DivideSpellDamage {
            player: *player,
            spell: *spell,
            targets: targets.clone(),
            total: *total,
        }),
        ChoiceRequest::DivideCounters {
            player,
            spell,
            targets,
            total,
        } => Some(PendingChoice::DivideCounters {
            player: *player,
            spell: *spell,
            targets: targets.clone(),
            total: *total,
        }),
        ChoiceRequest::ChooseManaColor {
            player,
            source,
            amount,
        } => Some(PendingChoice::ChooseManaColor {
            player: *player,
            source: *source,
            amount: *amount,
        }),
        ChoiceRequest::PayOrElse {
            player,
            source,
            cost,
            otherwise,
        } => Some(PendingChoice::PayOrElse {
            player: *player,
            source: *source,
            cost: *cost,
            otherwise,
        }),
        ChoiceRequest::ChooseTargetPlayers {
            player,
            source,
            max,
            legal,
            min,
            keep_one,
            filter,
            life_loss,
            count,
            then,
        } => Some(PendingChoice::ChooseTargetPlayers {
            player: *player,
            source: *source,
            legal: legal.clone(),
            min: *min,
            max: *max,
            keep_one: *keep_one,
            filter: *filter,
            life_loss: *life_loss,
            count: *count,
            then,
        }),
        ChoiceRequest::DanceExileMore {
            player,
            source,
            exiled,
            total_mv,
            budget,
        } => Some(PendingChoice::DanceExileMore {
            player: *player,
            source: *source,
            exiled: exiled.clone(),
            total_mv: *total_mv,
            budget: *budget,
        }),
        ChoiceRequest::OpponentChoosesPile {
            player,
            controller,
            source,
            pile_a,
            pile_b,
        } => Some(PendingChoice::OpponentChoosesPile {
            player: *player,
            controller: *controller,
            source: *source,
            pile_a: pile_a.clone(),
            pile_b: pile_b.clone(),
        }),
        ChoiceRequest::PartitionRevealed {
            player,
            controller,
            source,
            revealed,
        } => Some(PendingChoice::PartitionRevealed {
            player: *player,
            controller: *controller,
            source: *source,
            revealed: revealed.clone(),
        }),
        ChoiceRequest::OpponentChoosesRevealedToGraveyard {
            player,
            controller,
            source,
            revealed,
        } => Some(PendingChoice::OpponentChoosesRevealedToGraveyard {
            player: *player,
            controller: *controller,
            source: *source,
            revealed: revealed.clone(),
        }),
        ChoiceRequest::ChoosePileForHand {
            player,
            source,
            pile_a,
            pile_b,
        } => Some(PendingChoice::ChoosePileForHand {
            player: *player,
            source: *source,
            pile_a: pile_a.clone(),
            pile_b: pile_b.clone(),
        }),
        ChoiceRequest::RevealedCardToBattlefieldOrHand { player, card } => {
            Some(PendingChoice::RevealedCardToBattlefieldOrHand {
                player: *player,
                card: *card,
            })
        }
        _ => None,
    }
}
