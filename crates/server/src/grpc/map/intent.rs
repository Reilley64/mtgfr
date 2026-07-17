//! Boundary mappers for `intent.proto`: the client → server intent surface. Exhaustive both
//! directions — every `WireIntent` variant must round-trip, so a new variant fails to compile
//! here until it's wired up. `to_pb` currently has no caller (the server only ever receives an
//! intent, never sends one) — kept for symmetry and the round-trip tests below.
#![allow(dead_code)]

use schema::{IntentEnvelope, WireIntent};

use crate::grpc::map::common::{
    opt_wire_target_from_pb, wire_attack_from_pb, wire_attack_to_pb, wire_block_from_pb,
    wire_block_to_pb, wire_damage_from_pb, wire_damage_to_pb, wire_mode_choice_from_pb,
    wire_mode_choice_to_pb, wire_spell_damage_from_pb, wire_spell_damage_to_pb,
    wire_target_from_pb, wire_target_to_pb,
};
use crate::grpc::pb;

fn u8_trunc(v: u32) -> u8 {
    v.min(255) as u8
}

fn try_modes(modes: Vec<pb::WireModeChoice>) -> Result<Vec<schema::WireModeChoice>, String> {
    modes.into_iter().map(wire_mode_choice_from_pb).collect()
}

fn try_assignment_damage(assignment: Vec<pb::WireDamage>) -> Vec<schema::WireDamage> {
    assignment.into_iter().map(wire_damage_from_pb).collect()
}

fn try_assignment_spell_damage(
    assignment: Vec<pb::WireSpellDamage>,
) -> Result<Vec<schema::WireSpellDamage>, String> {
    assignment
        .into_iter()
        .map(wire_spell_damage_from_pb)
        .collect()
}

pub fn intent_envelope_to_pb(envelope: IntentEnvelope) -> pb::IntentEnvelope {
    pb::IntentEnvelope {
        table_id: envelope.table_id,
        client_seq: envelope.client_seq,
        intent: Some(wire_intent_to_pb(envelope.intent)),
    }
}

pub fn intent_envelope_from_pb(envelope: pb::IntentEnvelope) -> Result<IntentEnvelope, String> {
    let intent = envelope
        .intent
        .ok_or_else(|| "missing intent".to_string())?;
    Ok(IntentEnvelope {
        table_id: envelope.table_id,
        client_seq: envelope.client_seq,
        intent: wire_intent_from_pb(intent)?,
    })
}

pub fn wire_intent_to_pb(intent: WireIntent) -> pb::WireIntent {
    use pb::wire_intent::Intent;
    let intent = match intent {
        WireIntent::Cast {
            player,
            object,
            target,
            x,
            modes,
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            strive_count,
            replicate_count,
        } => Intent::Cast(pb::WireIntentCast {
            player: u32::from(player),
            object,
            target: target.map(wire_target_to_pb),
            x,
            modes: modes.into_iter().map(wire_mode_choice_to_pb).collect(),
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            strive_count: u32::from(strive_count),
            replicate_count: u32::from(replicate_count),
        }),
        WireIntent::PlayLand { player, object } => Intent::PlayLand(pb::WireIntentPlayLand {
            player: u32::from(player),
            object,
        }),
        WireIntent::TapForMana { player, object } => Intent::TapForMana(pb::WireIntentTapForMana {
            player: u32::from(player),
            object,
        }),
        WireIntent::ActivateAbility {
            player,
            object,
            ability_index,
            target,
            sacrifice,
        } => Intent::ActivateAbility(pb::WireIntentActivateAbility {
            player: u32::from(player),
            object,
            ability_index,
            target: target.map(wire_target_to_pb),
            sacrifice,
        }),
        WireIntent::DeclareAttackers { player, attackers } => {
            Intent::DeclareAttackers(pb::WireIntentDeclareAttackers {
                player: u32::from(player),
                attackers: attackers.into_iter().map(wire_attack_to_pb).collect(),
            })
        }
        WireIntent::DeclareBlockers { player, blocks } => {
            Intent::DeclareBlockers(pb::WireIntentDeclareBlockers {
                player: u32::from(player),
                blocks: blocks.into_iter().map(wire_block_to_pb).collect(),
            })
        }
        WireIntent::ChooseOrder { player, order } => {
            Intent::ChooseOrder(pb::WireIntentChooseOrder {
                player: u32::from(player),
                order,
            })
        }
        WireIntent::ChooseTargets { player, targets } => {
            Intent::ChooseTargets(pb::WireIntentChooseTargets {
                player: u32::from(player),
                targets: targets.into_iter().map(wire_target_to_pb).collect(),
            })
        }
        WireIntent::ChooseTargetPlayers { player, players } => {
            Intent::ChooseTargetPlayers(pb::WireIntentChooseTargetPlayers {
                player: u32::from(player),
                players: players.into_iter().map(u32::from).collect(),
            })
        }
        WireIntent::AnswerMay { player, yes } => Intent::AnswerMay(pb::WireIntentAnswerMay {
            player: u32::from(player),
            yes,
        }),
        WireIntent::PayOptionalCost { player, pay } => {
            Intent::PayOptionalCost(pb::WireIntentPayOptionalCost {
                player: u32::from(player),
                pay,
            })
        }
        WireIntent::AssignDamage { player, assignment } => {
            Intent::AssignDamage(pb::WireIntentAssignDamage {
                player: u32::from(player),
                assignment: assignment.into_iter().map(wire_damage_to_pb).collect(),
            })
        }
        WireIntent::DivideSpellDamage { player, assignment } => {
            Intent::DivideSpellDamage(pb::WireIntentDivideSpellDamage {
                player: u32::from(player),
                assignment: assignment
                    .into_iter()
                    .map(wire_spell_damage_to_pb)
                    .collect(),
            })
        }
        WireIntent::ArrangeTop {
            player,
            top,
            bottom,
        } => Intent::ArrangeTop(pb::WireIntentArrangeTop {
            player: u32::from(player),
            top,
            bottom,
        }),
        WireIntent::SelectFromTop { player, cards } => {
            Intent::SelectFromTop(pb::WireIntentSelectFromTop {
                player: u32::from(player),
                cards,
            })
        }
        WireIntent::DistributeTop {
            player,
            to_hand,
            to_bottom,
            to_exile_may_play,
        } => Intent::DistributeTop(pb::WireIntentDistributeTop {
            player: u32::from(player),
            to_hand,
            to_bottom,
            to_exile_may_play,
        }),
        WireIntent::ShuffleFromGraveyard { player, cards } => {
            Intent::ShuffleFromGraveyard(pb::WireIntentShuffleFromGraveyard {
                player: u32::from(player),
                cards,
            })
        }
        WireIntent::SearchLibrary { player, choice } => {
            Intent::SearchLibrary(pb::WireIntentSearchLibrary {
                player: u32::from(player),
                choice,
            })
        }
        WireIntent::ChooseSacrifices { player, sacrifices } => {
            Intent::ChooseSacrifices(pb::WireIntentChooseSacrifices {
                player: u32::from(player),
                sacrifices,
            })
        }
        WireIntent::Discard { player, cards } => Intent::Discard(pb::WireIntentDiscard {
            player: u32::from(player),
            cards,
        }),
        WireIntent::PutLandFromHand { player, choice } => {
            Intent::PutLandFromHand(pb::WireIntentPutLandFromHand {
                player: u32::from(player),
                choice,
            })
        }
        WireIntent::ChooseExiledWithCard { player, choice } => {
            Intent::ChooseExiledWithCard(pb::WireIntentChooseExiledWithCard {
                player: u32::from(player),
                choice,
            })
        }
        WireIntent::ChooseExiledWithCardToCast { player, choice } => {
            Intent::ChooseExiledWithCardToCast(pb::WireIntentChooseExiledWithCardToCast {
                player: u32::from(player),
                choice,
            })
        }
        WireIntent::ChooseExiledDigToCastFree { player, choice } => {
            Intent::ChooseExiledDigToCastFree(pb::WireIntentChooseExiledDigToCastFree {
                player: u32::from(player),
                choice,
            })
        }
        WireIntent::ChooseOpponentPile { player, pile } => {
            Intent::ChooseOpponentPile(pb::WireIntentChooseOpponentPile {
                player: u32::from(player),
                pile: u32::from(pile),
            })
        }
        WireIntent::RevealedCardToBattlefieldOrHand { player, choice } => {
            Intent::RevealedCardToBattlefieldOrHand(pb::WireIntentRevealedCardToBattlefieldOrHand {
                player: u32::from(player),
                choice,
            })
        }
        WireIntent::ChooseMode { player, mode } => Intent::ChooseMode(pb::WireIntentChooseMode {
            player: u32::from(player),
            mode: mode as u64,
        }),
        WireIntent::ChooseTriggerModes { player, modes } => {
            Intent::ChooseTriggerModes(pb::WireIntentChooseTriggerModes {
                player: u32::from(player),
                modes: modes.into_iter().map(wire_mode_choice_to_pb).collect(),
            })
        }
        WireIntent::ChooseManaColor { player, color } => {
            Intent::ChooseManaColor(pb::WireIntentChooseManaColor {
                player: u32::from(player),
                color: u32::from(color),
            })
        }
        WireIntent::ChooseCreatureType { player, subtype } => {
            Intent::ChooseCreatureType(pb::WireIntentChooseCreatureType {
                player: u32::from(player),
                subtype,
            })
        }
        WireIntent::ChooseColor { player, color } => {
            Intent::ChooseColor(pb::WireIntentChooseColor {
                player: u32::from(player),
                color: u32::from(color),
            })
        }
        WireIntent::ChooseAttachHost { player, host } => {
            Intent::ChooseAttachHost(pb::WireIntentChooseAttachHost {
                player: u32::from(player),
                host,
            })
        }
        WireIntent::ChooseCopyTarget { player, copy } => {
            Intent::ChooseCopyTarget(pb::WireIntentChooseCopyTarget {
                player: u32::from(player),
                copy,
            })
        }
        WireIntent::Cycle { player, card } => Intent::Cycle(pb::WireIntentCycle {
            player: u32::from(player),
            card,
        }),
        WireIntent::ActivateHandAbility { player, card } => {
            Intent::ActivateHandAbility(pb::WireIntentActivateHandAbility {
                player: u32::from(player),
                card,
            })
        }
        WireIntent::Suspend { player, card } => Intent::Suspend(pb::WireIntentSuspend {
            player: u32::from(player),
            card,
        }),
        WireIntent::Encore { player, card } => Intent::Encore(pb::WireIntentEncore {
            player: u32::from(player),
            card,
        }),
        WireIntent::TurnFaceUp { player, permanent } => {
            Intent::TurnFaceUp(pb::WireIntentTurnFaceUp {
                player: u32::from(player),
                permanent,
            })
        }
        WireIntent::CastPrepared {
            player,
            source,
            target,
            x,
        } => Intent::CastPrepared(pb::WireIntentCastPrepared {
            player: u32::from(player),
            source,
            target: target.map(wire_target_to_pb),
            x,
        }),
        WireIntent::CastAdventure {
            player,
            source,
            target,
            x,
        } => Intent::CastAdventure(pb::WireIntentCastAdventure {
            player: u32::from(player),
            source,
            target: target.map(wire_target_to_pb),
            x,
        }),
        WireIntent::CastBestow {
            player,
            object,
            target,
        } => Intent::CastBestow(pb::WireIntentCastBestow {
            player: u32::from(player),
            object,
            target: target.map(wire_target_to_pb),
        }),
        WireIntent::PassPriority { player } => Intent::PassPriority(pb::WireIntentPassPriority {
            player: u32::from(player),
        }),
        WireIntent::Concede { player } => Intent::Concede(pb::WireIntentConcede {
            player: u32::from(player),
        }),
        WireIntent::TakeAction {
            player,
            id,
            target,
            x,
            modes,
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers,
            blocks,
        } => Intent::TakeAction(pb::WireIntentTakeAction {
            player: u32::from(player),
            id,
            target: target.map(wire_target_to_pb),
            x,
            modes: modes.into_iter().map(wire_mode_choice_to_pb).collect(),
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers: attackers.into_iter().map(wire_attack_to_pb).collect(),
            blocks: blocks.into_iter().map(wire_block_to_pb).collect(),
        }),
    };
    pb::WireIntent {
        intent: Some(intent),
    }
}

pub fn wire_intent_from_pb(intent: pb::WireIntent) -> Result<WireIntent, String> {
    use pb::wire_intent::Intent;
    let intent = intent.intent.ok_or_else(|| "missing kind".to_string())?;
    let wire = match intent {
        Intent::Cast(pb::WireIntentCast {
            player,
            object,
            target,
            x,
            modes,
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            strive_count,
            replicate_count,
        }) => WireIntent::Cast {
            player: u8_trunc(player),
            object,
            target: opt_wire_target_from_pb(target)?,
            x,
            modes: try_modes(modes)?,
            discard_cost,
            graveyard_exile,
            sacrifice_cost,
            kicked,
            strive_count: u8_trunc(strive_count),
            replicate_count: u8_trunc(replicate_count),
        },
        Intent::PlayLand(pb::WireIntentPlayLand { player, object }) => WireIntent::PlayLand {
            player: u8_trunc(player),
            object,
        },
        Intent::TapForMana(pb::WireIntentTapForMana { player, object }) => WireIntent::TapForMana {
            player: u8_trunc(player),
            object,
        },
        Intent::ActivateAbility(pb::WireIntentActivateAbility {
            player,
            object,
            ability_index,
            target,
            sacrifice,
        }) => WireIntent::ActivateAbility {
            player: u8_trunc(player),
            object,
            ability_index,
            target: opt_wire_target_from_pb(target)?,
            sacrifice,
        },
        Intent::DeclareAttackers(pb::WireIntentDeclareAttackers { player, attackers }) => {
            WireIntent::DeclareAttackers {
                player: u8_trunc(player),
                attackers: attackers.into_iter().map(wire_attack_from_pb).collect(),
            }
        }
        Intent::DeclareBlockers(pb::WireIntentDeclareBlockers { player, blocks }) => {
            WireIntent::DeclareBlockers {
                player: u8_trunc(player),
                blocks: blocks.into_iter().map(wire_block_from_pb).collect(),
            }
        }
        Intent::ChooseOrder(pb::WireIntentChooseOrder { player, order }) => {
            WireIntent::ChooseOrder {
                player: u8_trunc(player),
                order,
            }
        }
        Intent::ChooseTargets(pb::WireIntentChooseTargets { player, targets }) => {
            WireIntent::ChooseTargets {
                player: u8_trunc(player),
                targets: targets
                    .into_iter()
                    .map(wire_target_from_pb)
                    .collect::<Result<_, _>>()?,
            }
        }
        Intent::ChooseTargetPlayers(pb::WireIntentChooseTargetPlayers { player, players }) => {
            WireIntent::ChooseTargetPlayers {
                player: u8_trunc(player),
                players: players.into_iter().map(u8_trunc).collect(),
            }
        }
        Intent::AnswerMay(pb::WireIntentAnswerMay { player, yes }) => WireIntent::AnswerMay {
            player: u8_trunc(player),
            yes,
        },
        Intent::PayOptionalCost(pb::WireIntentPayOptionalCost { player, pay }) => {
            WireIntent::PayOptionalCost {
                player: u8_trunc(player),
                pay,
            }
        }
        Intent::AssignDamage(pb::WireIntentAssignDamage { player, assignment }) => {
            WireIntent::AssignDamage {
                player: u8_trunc(player),
                assignment: try_assignment_damage(assignment),
            }
        }
        Intent::DivideSpellDamage(pb::WireIntentDivideSpellDamage { player, assignment }) => {
            WireIntent::DivideSpellDamage {
                player: u8_trunc(player),
                assignment: try_assignment_spell_damage(assignment)?,
            }
        }
        Intent::ArrangeTop(pb::WireIntentArrangeTop {
            player,
            top,
            bottom,
        }) => WireIntent::ArrangeTop {
            player: u8_trunc(player),
            top,
            bottom,
        },
        Intent::SelectFromTop(pb::WireIntentSelectFromTop { player, cards }) => {
            WireIntent::SelectFromTop {
                player: u8_trunc(player),
                cards,
            }
        }
        Intent::DistributeTop(pb::WireIntentDistributeTop {
            player,
            to_hand,
            to_bottom,
            to_exile_may_play,
        }) => WireIntent::DistributeTop {
            player: u8_trunc(player),
            to_hand,
            to_bottom,
            to_exile_may_play,
        },
        Intent::ShuffleFromGraveyard(pb::WireIntentShuffleFromGraveyard { player, cards }) => {
            WireIntent::ShuffleFromGraveyard {
                player: u8_trunc(player),
                cards,
            }
        }
        Intent::SearchLibrary(pb::WireIntentSearchLibrary { player, choice }) => {
            WireIntent::SearchLibrary {
                player: u8_trunc(player),
                choice,
            }
        }
        Intent::ChooseSacrifices(pb::WireIntentChooseSacrifices { player, sacrifices }) => {
            WireIntent::ChooseSacrifices {
                player: u8_trunc(player),
                sacrifices,
            }
        }
        Intent::Discard(pb::WireIntentDiscard { player, cards }) => WireIntent::Discard {
            player: u8_trunc(player),
            cards,
        },
        Intent::PutLandFromHand(pb::WireIntentPutLandFromHand { player, choice }) => {
            WireIntent::PutLandFromHand {
                player: u8_trunc(player),
                choice,
            }
        }
        Intent::ChooseExiledWithCard(pb::WireIntentChooseExiledWithCard { player, choice }) => {
            WireIntent::ChooseExiledWithCard {
                player: u8_trunc(player),
                choice,
            }
        }
        Intent::ChooseExiledWithCardToCast(pb::WireIntentChooseExiledWithCardToCast {
            player,
            choice,
        }) => WireIntent::ChooseExiledWithCardToCast {
            player: u8_trunc(player),
            choice,
        },
        Intent::ChooseExiledDigToCastFree(pb::WireIntentChooseExiledDigToCastFree {
            player,
            choice,
        }) => WireIntent::ChooseExiledDigToCastFree {
            player: u8_trunc(player),
            choice,
        },
        Intent::ChooseOpponentPile(pb::WireIntentChooseOpponentPile { player, pile }) => {
            WireIntent::ChooseOpponentPile {
                player: u8_trunc(player),
                pile: u8_trunc(pile),
            }
        }
        Intent::RevealedCardToBattlefieldOrHand(
            pb::WireIntentRevealedCardToBattlefieldOrHand { player, choice },
        ) => WireIntent::RevealedCardToBattlefieldOrHand {
            player: u8_trunc(player),
            choice,
        },
        Intent::ChooseMode(pb::WireIntentChooseMode { player, mode }) => WireIntent::ChooseMode {
            player: u8_trunc(player),
            mode: mode as usize,
        },
        Intent::ChooseTriggerModes(pb::WireIntentChooseTriggerModes { player, modes }) => {
            WireIntent::ChooseTriggerModes {
                player: u8_trunc(player),
                modes: try_modes(modes)?,
            }
        }
        Intent::ChooseManaColor(pb::WireIntentChooseManaColor { player, color }) => {
            WireIntent::ChooseManaColor {
                player: u8_trunc(player),
                color: u8_trunc(color),
            }
        }
        Intent::ChooseCreatureType(pb::WireIntentChooseCreatureType { player, subtype }) => {
            WireIntent::ChooseCreatureType {
                player: u8_trunc(player),
                subtype,
            }
        }
        Intent::ChooseColor(pb::WireIntentChooseColor { player, color }) => {
            WireIntent::ChooseColor {
                player: u8_trunc(player),
                color: u8_trunc(color),
            }
        }
        Intent::ChooseAttachHost(pb::WireIntentChooseAttachHost { player, host }) => {
            WireIntent::ChooseAttachHost {
                player: u8_trunc(player),
                host,
            }
        }
        Intent::ChooseCopyTarget(pb::WireIntentChooseCopyTarget { player, copy }) => {
            WireIntent::ChooseCopyTarget {
                player: u8_trunc(player),
                copy,
            }
        }
        Intent::Cycle(pb::WireIntentCycle { player, card }) => WireIntent::Cycle {
            player: u8_trunc(player),
            card,
        },
        Intent::ActivateHandAbility(pb::WireIntentActivateHandAbility { player, card }) => {
            WireIntent::ActivateHandAbility {
                player: u8_trunc(player),
                card,
            }
        }
        Intent::Suspend(pb::WireIntentSuspend { player, card }) => WireIntent::Suspend {
            player: u8_trunc(player),
            card,
        },
        Intent::Encore(pb::WireIntentEncore { player, card }) => WireIntent::Encore {
            player: u8_trunc(player),
            card,
        },
        Intent::TurnFaceUp(pb::WireIntentTurnFaceUp { player, permanent }) => {
            WireIntent::TurnFaceUp {
                player: u8_trunc(player),
                permanent,
            }
        }
        Intent::CastPrepared(pb::WireIntentCastPrepared {
            player,
            source,
            target,
            x,
        }) => WireIntent::CastPrepared {
            player: u8_trunc(player),
            source,
            target: opt_wire_target_from_pb(target)?,
            x,
        },
        Intent::CastAdventure(pb::WireIntentCastAdventure {
            player,
            source,
            target,
            x,
        }) => WireIntent::CastAdventure {
            player: u8_trunc(player),
            source,
            target: opt_wire_target_from_pb(target)?,
            x,
        },
        Intent::CastBestow(pb::WireIntentCastBestow {
            player,
            object,
            target,
        }) => WireIntent::CastBestow {
            player: u8_trunc(player),
            object,
            target: opt_wire_target_from_pb(target)?,
        },
        Intent::PassPriority(pb::WireIntentPassPriority { player }) => WireIntent::PassPriority {
            player: u8_trunc(player),
        },
        Intent::Concede(pb::WireIntentConcede { player }) => WireIntent::Concede {
            player: u8_trunc(player),
        },
        Intent::TakeAction(pb::WireIntentTakeAction {
            player,
            id,
            target,
            x,
            modes,
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers,
            blocks,
        }) => WireIntent::TakeAction {
            player: u8_trunc(player),
            id,
            target: opt_wire_target_from_pb(target)?,
            x,
            modes: try_modes(modes)?,
            sacrifice,
            discard_cost,
            graveyard_exile,
            attackers: attackers.into_iter().map(wire_attack_from_pb).collect(),
            blocks: blocks.into_iter().map(wire_block_from_pb).collect(),
        },
    };
    Ok(wire)
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::WireTarget;

    #[test]
    fn cast_round_trips_through_pb() {
        let cast = WireIntent::Cast {
            player: 1,
            object: 7,
            target: Some(WireTarget::Object { id: 9 }),
            x: 3,
            modes: vec![],
            discard_cost: vec![2],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: true,
            strive_count: 2,
            replicate_count: 0,
        };
        let pb = wire_intent_to_pb(cast.clone());
        assert_eq!(wire_intent_from_pb(pb).unwrap(), cast);
    }

    #[test]
    fn declare_blockers_round_trips_through_pb() {
        let intent = WireIntent::DeclareBlockers {
            player: 0,
            blocks: vec![schema::WireBlock {
                blocker: 3,
                attacker: 4,
            }],
        };
        let envelope = IntentEnvelope {
            table_id: "t1".to_string(),
            client_seq: 5,
            intent: intent.clone(),
        };
        let pb = intent_envelope_to_pb(envelope.clone());
        assert_eq!(intent_envelope_from_pb(pb).unwrap(), envelope);
    }

    #[test]
    fn missing_oneof_is_an_error() {
        assert!(wire_intent_from_pb(pb::WireIntent { intent: None }).is_err());
    }
}
