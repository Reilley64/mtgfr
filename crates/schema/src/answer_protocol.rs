//! Answer protocol: PendingChoiceView + raw answer → WireIntent.
//!
//! Structural packing only — engine validates legality (ADR 0004).
//! Client `choiceIntent` is the TypeScript adapter of the same mapping.

use crate::ObjectId;
use crate::dto::PendingChoiceView;
use crate::intent::{WireDamage, WireIntent, WireModeChoice, WireTarget};

/// Raw answer shapes collected by dumb prompt forms (mirrors client `AnswerInput`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Order {
        order: Vec<u32>,
    },
    Target {
        id: ObjectId,
        player: Option<u8>,
    },
    Targets {
        ids: Vec<ObjectId>,
    },
    May {
        yes: bool,
    },
    Pay {
        pay: bool,
    },
    Assign {
        assignment: Vec<WireDamage>,
    },
    Arrange {
        top: Vec<ObjectId>,
        bottom: Vec<ObjectId>,
    },
    Search {
        choice: Option<ObjectId>,
    },
    Sacrifice {
        ids: Vec<ObjectId>,
    },
    Discard {
        cards: Vec<ObjectId>,
    },
    DeclineUntap {
        keep_tapped: Vec<ObjectId>,
    },
    Dredge {
        /// `Some(dredger)` dredges that graveyard card; `None` declines and draws (CR 702.52).
        dredger: Option<ObjectId>,
    },
    PutLand {
        choice: Option<ObjectId>,
    },
    PutCreature {
        choice: Option<ObjectId>,
    },
    ReturnLand {
        choice: Option<ObjectId>,
    },
    ChooseExiled {
        choice: Option<ObjectId>,
    },
    SelectTop {
        cards: Vec<ObjectId>,
    },
    Mode {
        mode: usize,
    },
    TargetPlayers {
        players: Vec<u8>,
    },
    Distribute {
        to_hand: Vec<ObjectId>,
        to_bottom: Vec<ObjectId>,
        to_exile_may_play: Vec<ObjectId>,
    },
    ShuffleGy {
        cards: Vec<ObjectId>,
    },
    ChooseExiledCast {
        choice: Option<ObjectId>,
    },
    ChooseExiledDig {
        choice: Option<ObjectId>,
    },
    TriggerModes {
        modes: Vec<WireModeChoice>,
    },
    ManaColor {
        color: u8,
    },
    CreatureType {
        subtype: String,
    },
    Color {
        color: u8,
    },
    OpponentPile {
        pile: u8,
    },
    Revealed {
        choice: Option<ObjectId>,
    },
    AttachHost {
        host: Option<ObjectId>,
    },
    CopyTarget {
        copy: Option<ObjectId>,
    },
    TopOrBottom {
        top: bool,
    },
}

/// Encode a form answer into the WireIntent that answers `view`.
///
/// Player is taken from the view. Does not check legality of the chosen objects.
pub fn encode_answer(view: &PendingChoiceView, answer: Answer) -> WireIntent {
    let player = view_player(view);
    match answer {
        Answer::Order { order } => WireIntent::ChooseOrder { player, order },
        Answer::Target { id, player: seat } => WireIntent::ChooseTargets {
            player,
            targets: vec![match seat {
                Some(p) => WireTarget::Player { player: p },
                None => WireTarget::Object { id },
            }],
        },
        Answer::Targets { ids } => WireIntent::ChooseTargets {
            player,
            targets: ids
                .into_iter()
                .map(|id| WireTarget::Object { id })
                .collect(),
        },
        Answer::May { yes } => WireIntent::AnswerMay { player, yes },
        Answer::Pay { pay } => WireIntent::PayOptionalCost { player, pay },
        Answer::Assign { assignment } => WireIntent::AssignDamage { player, assignment },
        Answer::Arrange { top, bottom } => WireIntent::ArrangeTop {
            player,
            top,
            bottom,
        },
        Answer::Search { choice } => WireIntent::SearchLibrary { player, choice },
        Answer::Sacrifice { ids } => WireIntent::ChooseSacrifices {
            player,
            sacrifices: ids,
        },
        Answer::Discard { cards } => WireIntent::Discard { player, cards },
        Answer::DeclineUntap { keep_tapped } => WireIntent::DeclineUntap {
            player,
            keep_tapped,
        },
        Answer::Dredge { dredger } => WireIntent::ChooseDredge { player, dredger },
        Answer::PutLand { choice } => WireIntent::PutLandFromHand { player, choice },
        Answer::PutCreature { choice } => WireIntent::PutCreatureFromHand { player, choice },
        Answer::ReturnLand { choice } => WireIntent::ReturnLandOrSacrifice {
            player,
            land: choice,
        },
        Answer::ChooseExiled { choice } => WireIntent::ChooseExiledWithCard { player, choice },
        Answer::SelectTop { cards } => WireIntent::SelectFromTop { player, cards },
        Answer::Mode { mode } => WireIntent::ChooseMode { player, mode },
        Answer::TargetPlayers { players } => WireIntent::ChooseTargetPlayers { player, players },
        Answer::Distribute {
            to_hand,
            to_bottom,
            to_exile_may_play,
        } => WireIntent::DistributeTop {
            player,
            to_hand,
            to_bottom,
            to_exile_may_play,
        },
        Answer::ShuffleGy { cards } => WireIntent::ShuffleFromGraveyard { player, cards },
        Answer::ChooseExiledCast { choice } => {
            WireIntent::ChooseExiledWithCardToCast { player, choice }
        }
        Answer::ChooseExiledDig { choice } => {
            WireIntent::ChooseExiledDigToCastFree { player, choice }
        }
        Answer::TriggerModes { modes } => WireIntent::ChooseTriggerModes { player, modes },
        Answer::ManaColor { color } => WireIntent::ChooseManaColor { player, color },
        Answer::CreatureType { subtype } => WireIntent::ChooseCreatureType { player, subtype },
        Answer::Color { color } => WireIntent::ChooseColor { player, color },
        Answer::OpponentPile { pile } => WireIntent::ChooseOpponentPile { player, pile },
        Answer::Revealed { choice } => {
            WireIntent::RevealedCardToBattlefieldOrHand { player, choice }
        }
        Answer::AttachHost { host } => WireIntent::ChooseAttachHost { player, host },
        Answer::CopyTarget { copy } => WireIntent::ChooseCopyTarget { player, copy },
        Answer::TopOrBottom { top } => WireIntent::ChooseTopOrBottom { player, top },
    }
}

fn view_player(view: &PendingChoiceView) -> u8 {
    match view {
        PendingChoiceView::OrderTriggers { player, .. }
        | PendingChoiceView::ChooseTarget { player, .. }
        | PendingChoiceView::ChooseSpellTargets { player, .. }
        | PendingChoiceView::ChooseTargetPlayers { player, .. }
        | PendingChoiceView::MayYesNo { player, .. }
        | PendingChoiceView::DeclineUntap { player, .. }
        | PendingChoiceView::ChooseDredge { player, .. }
        | PendingChoiceView::PayCost { player, .. }
        | PendingChoiceView::PayOrCounter { player, .. }
        | PendingChoiceView::PayOrControllerDraws { player, .. }
        | PendingChoiceView::ChooseCounteredSpellDestination { player, .. }
        | PendingChoiceView::PayEchoOrSacrifice { player, .. }
        | PendingChoiceView::PayRecoverOrExile { player, .. }
        | PendingChoiceView::SacrificeUnlessPay { player, .. }
        | PendingChoiceView::SacrificeUnlessReturnLand { player, .. }
        | PendingChoiceView::AssignCombatDamage { player, .. }
        | PendingChoiceView::DivideSpellDamage { player, .. }
        | PendingChoiceView::DivideCounters { player, .. }
        | PendingChoiceView::Scry { player, .. }
        | PendingChoiceView::Surveil { player, .. }
        | PendingChoiceView::SearchLibrary { player, .. }
        | PendingChoiceView::SelectFromTop { player, .. }
        | PendingChoiceView::DistributeTop { player, .. }
        | PendingChoiceView::ShuffleFromGraveyard { player, .. }
        | PendingChoiceView::SacrificeEdict { player, .. }
        | PendingChoiceView::Proliferate { player, .. }
        | PendingChoiceView::PhaseOut { player, .. }
        | PendingChoiceView::ChooseAbilityTargets { player, .. }
        | PendingChoiceView::MaySacrifice { player, .. }
        | PendingChoiceView::ChooseOwnSacrifices { player, .. }
        | PendingChoiceView::Devour { player, .. }
        | PendingChoiceView::ExileFromGraveyard { player, .. }
        | PendingChoiceView::CasterKeepPermanents { player, .. }
        | PendingChoiceView::ChooseCounterTargetForPlayer { player, .. }
        | PendingChoiceView::MayReturnFromGraveyard { player, .. }
        | PendingChoiceView::MayDiscard { player, .. }
        | PendingChoiceView::Discard { player, .. }
        | PendingChoiceView::PutLandFromHand { player, .. }
        | PendingChoiceView::PutCreatureFromHand { player, .. }
        | PendingChoiceView::CastCreatureFaceDown { player, .. }
        | PendingChoiceView::ChooseExiledWithCard { player, .. }
        | PendingChoiceView::ChooseExiledWithCardToCast { player, .. }
        | PendingChoiceView::ChooseExiledDigToCastFree { player, .. }
        | PendingChoiceView::DanceExileMore { player, .. }
        | PendingChoiceView::OpponentChoosesPile { player, .. }
        | PendingChoiceView::OpponentChoosesExiledNonland { player, .. }
        | PendingChoiceView::ChooseSplittingOpponent { player, .. }
        | PendingChoiceView::PartitionRevealed { player, .. }
        | PendingChoiceView::ChoosePileForHand { player, .. }
        | PendingChoiceView::ChooseExiledToCastFree { player, .. }
        | PendingChoiceView::RevealedCardToBattlefieldOrHand { player, .. }
        | PendingChoiceView::ChooseMode { player, .. }
        | PendingChoiceView::ChooseTriggerModes { player, .. }
        | PendingChoiceView::ChooseManaColor { player, .. }
        | PendingChoiceView::ChooseCreatureType { player, .. }
        | PendingChoiceView::ChooseColor { player, .. }
        | PendingChoiceView::ChooseAttachHost { player, .. }
        | PendingChoiceView::ChooseCopyTarget { player, .. } => *player,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::ChoiceItem;

    fn proliferate(player: u8) -> PendingChoiceView {
        PendingChoiceView::Proliferate {
            player,
            source: 1,
            items: vec![ChoiceItem {
                id: 7,
                label: "Beast".into(),
                print: String::new(),
                player: None,
            }],
        }
    }

    #[test]
    fn sacrifice_shape_encodes_choose_sacrifices_for_proliferate() {
        let intent = encode_answer(&proliferate(1), Answer::Sacrifice { ids: vec![7] });
        assert_eq!(
            intent,
            WireIntent::ChooseSacrifices {
                player: 1,
                sacrifices: vec![7],
            }
        );
    }

    #[test]
    fn may_encodes_answer_may() {
        let view = PendingChoiceView::MayYesNo {
            player: 0,
            source: 3,
            label: "Draw a card".into(),
        };
        assert_eq!(
            encode_answer(&view, Answer::May { yes: false }),
            WireIntent::AnswerMay {
                player: 0,
                yes: false
            }
        );
    }
}
