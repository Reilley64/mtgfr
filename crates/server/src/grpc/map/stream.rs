//! Boundary mappers for `stream.proto` (`to_pb` only — server-emit).

use schema::{
    ActionView, ChoiceItem, CombatView, ModalView, ModeView, ModifierSourceView, ObjectView,
    PendingChoiceView, PlayerView, StackObjectView, StreamFrame, VisibleEvent, VisibleState,
};

use crate::grpc::map::common::{
    commander_damage_view_to_pb, object_amount_to_pb, object_id_list_to_pb, player_amount_to_pb,
    wire_attack_to_pb, wire_block_to_pb, wire_cost_to_pb, wire_kind_to_pb, wire_mana_pool_to_pb,
    wire_target_to_pb,
};
use crate::grpc::pb;

pub fn choice_item_to_pb(item: ChoiceItem) -> pb::ChoiceItem {
    pb::ChoiceItem {
        id: item.id,
        label: item.label,
        player: item.player.map(u32::from),
        print: item.print,
    }
}

fn choice_items_to_pb(items: Vec<ChoiceItem>) -> Vec<pb::ChoiceItem> {
    items.into_iter().map(choice_item_to_pb).collect()
}

pub fn mode_view_to_pb(mode: ModeView) -> pb::ModeView {
    pb::ModeView {
        label: mode.label,
        targets: mode.targets.into_iter().map(wire_target_to_pb).collect(),
        needs_target: mode.needs_target,
    }
}

pub fn modal_view_to_pb(modal: ModalView) -> pb::ModalView {
    pb::ModalView {
        choose: u32::from(modal.choose),
        choose_max: u32::from(modal.choose_max),
        modes: modal.modes.into_iter().map(mode_view_to_pb).collect(),
    }
}

pub fn combat_view_to_pb(combat: CombatView) -> pb::CombatView {
    pb::CombatView {
        attackers: combat
            .attackers
            .into_iter()
            .map(wire_attack_to_pb)
            .collect(),
        blocks: combat.blocks.into_iter().map(wire_block_to_pb).collect(),
        attackers_declared: combat.attackers_declared,
        blockers_declared: combat
            .blockers_declared
            .into_iter()
            .map(u32::from)
            .collect(),
    }
}

pub fn stack_object_view_to_pb(entry: StackObjectView) -> pb::StackObjectView {
    pb::StackObjectView {
        kind: entry.kind,
        source: entry.source,
        controller: u32::from(entry.controller),
        label: entry.label,
        target: entry.target.map(wire_target_to_pb),
    }
}

pub fn modifier_source_view_to_pb(group: ModifierSourceView) -> pb::ModifierSourceView {
    pb::ModifierSourceView {
        source_name: group.source_name,
        source_card_id: group.source_card_id,
        contributions: group.contributions,
    }
}

pub fn object_view_to_pb(obj: ObjectView) -> pb::ObjectView {
    pb::ObjectView {
        id: obj.id,
        zone: u32::from(obj.zone),
        owner: u32::from(obj.owner),
        controller: u32::from(obj.controller),
        card_id: obj.card_id,
        name: obj.name,
        print: obj.print,
        kind: Some(wire_kind_to_pb(obj.kind)),
        mana_cost: Some(wire_cost_to_pb(obj.mana_cost)),
        needs_target: obj.needs_target,
        tapped: obj.tapped,
        summoning_sick: obj.summoning_sick,
        has_haste: obj.has_haste,
        keywords: obj.keywords,
        power: obj.power,
        toughness: obj.toughness,
        loyalty: obj.loyalty,
        plus_counters: obj.plus_counters,
        marked_damage: obj.marked_damage,
        is_commander: obj.is_commander,
        goaded: obj.goaded,
        taps_for_mana: obj.taps_for_mana,
        prepared: obj.prepared,
        phased_out: obj.phased_out,
        face_down: obj.face_down,
        attached_to: obj.attached_to,
        modifiers: obj
            .modifiers
            .into_iter()
            .map(modifier_source_view_to_pb)
            .collect(),
    }
}

pub fn player_view_to_pb(player: PlayerView) -> pb::PlayerView {
    pb::PlayerView {
        player: u32::from(player.player),
        username: player.username,
        life: player.life,
        commander_tax: u32::from(player.commander_tax),
        lost: player.lost,
        hand_count: player.hand_count,
        library_count: player.library_count,
        mulligans_taken: u32::from(player.mulligans_taken),
        hand_kept: player.hand_kept,
        can_mulligan: player.can_mulligan,
        mana_pool: Some(wire_mana_pool_to_pb(player.mana_pool)),
        commander_damage: player
            .commander_damage
            .into_iter()
            .map(commander_damage_view_to_pb)
            .collect(),
    }
}

pub fn action_view_to_pb(action: ActionView) -> pb::ActionView {
    pb::ActionView {
        id: action.id,
        kind: action.kind,
        object: action.object,
        ability_index: action.ability_index,
        section: action.section,
        label: action.label,
        needs_target: action.needs_target,
        targets: action.targets.into_iter().map(wire_target_to_pb).collect(),
        modal: action.modal.map(modal_view_to_pb),
        sacrifice_choices: object_id_list_to_pb(action.sacrifice_choices),
        discard_choices: object_id_list_to_pb(action.discard_choices),
        discard_count: u32::from(action.discard_count),
        graveyard_exile_choices: object_id_list_to_pb(action.graveyard_exile_choices),
        graveyard_exile_min: u32::from(action.graveyard_exile_min),
        graveyard_exile_max: u32::from(action.graveyard_exile_max),
        has_x: action.has_x,
        min_x: action.min_x,
        max_x: action.max_x,
        x_cost: action.x_cost.map(wire_cost_to_pb),
        auto_tap: action.auto_tap,
        required_attacks: action
            .required_attacks
            .into_iter()
            .map(wire_attack_to_pb)
            .collect(),
    }
}

pub fn pending_choice_view_to_pb(choice: PendingChoiceView) -> pb::PendingChoiceView {
    use pb::pending_choice_view::Choice;
    let choice = match choice {
        PendingChoiceView::OrderTriggers {
            player,
            source,
            count,
            labels,
        } => Choice::OrderTriggers(pb::PendingChoiceViewOrderTriggers {
            player: u32::from(player),
            source,
            count,
            labels,
        }),
        PendingChoiceView::ChooseTarget {
            player,
            source,
            label,
            items,
            optional,
            max,
        } => Choice::ChooseTarget(pb::PendingChoiceViewChooseTarget {
            player: u32::from(player),
            source,
            label,
            items: choice_items_to_pb(items),
            optional,
            max: u32::from(max),
        }),
        PendingChoiceView::ChooseSpellTargets {
            player,
            spell,
            label,
            min,
            max,
            items,
        } => Choice::ChooseSpellTargets(pb::PendingChoiceViewChooseSpellTargets {
            player: u32::from(player),
            spell,
            label,
            min: u32::from(min),
            max: u32::from(max),
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseTargetPlayers {
            player,
            source,
            label,
            min,
            max,
            items,
        } => Choice::ChooseTargetPlayers(pb::PendingChoiceViewChooseTargetPlayers {
            player: u32::from(player),
            source,
            label,
            min: u32::from(min),
            max: u32::from(max),
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::MayYesNo {
            player,
            source,
            label,
        } => Choice::MayYesNo(pb::PendingChoiceViewMayYesNo {
            player: u32::from(player),
            source,
            label,
        }),
        PendingChoiceView::MayDrawUpTo { player, max } => {
            Choice::MayDrawUpTo(pb::PendingChoiceViewMayDrawUpTo {
                player: u32::from(player),
                max: u32::from(max),
            })
        }
        PendingChoiceView::TradeSecretsCasterDraw {
            player,
            max,
            opponent,
        } => Choice::TradeSecretsCasterDraw(pb::PendingChoiceViewTradeSecretsCasterDraw {
            player: u32::from(player),
            max: u32::from(max),
            opponent: u32::from(opponent),
        }),
        PendingChoiceView::TradeSecretsRepeat { player, caster } => {
            Choice::TradeSecretsRepeat(pb::PendingChoiceViewTradeSecretsRepeat {
                player: u32::from(player),
                caster: u32::from(caster),
            })
        }
        PendingChoiceView::PayCost {
            player,
            source,
            cost,
            label,
        } => Choice::PayCost(pb::PendingChoiceViewPayCost {
            player: u32::from(player),
            source,
            cost: Some(wire_cost_to_pb(cost)),
            label,
        }),
        PendingChoiceView::PayOrCounter {
            player,
            spell,
            cost,
        } => Choice::PayOrCounter(pb::PendingChoiceViewPayOrCounter {
            player: u32::from(player),
            spell,
            cost: Some(wire_cost_to_pb(cost)),
        }),
        PendingChoiceView::PayEchoOrSacrifice {
            player,
            source,
            cost,
        } => Choice::PayEchoOrSacrifice(pb::PendingChoiceViewPayEchoOrSacrifice {
            player: u32::from(player),
            source,
            cost: Some(wire_cost_to_pb(cost)),
        }),
        PendingChoiceView::PayRecoverOrExile {
            player,
            source,
            cost,
        } => Choice::PayRecoverOrExile(pb::PendingChoiceViewPayRecoverOrExile {
            player: u32::from(player),
            source,
            cost: Some(wire_cost_to_pb(cost)),
        }),
        PendingChoiceView::PayCumulativeUpkeepOrSacrifice {
            player,
            source,
            items,
            count,
        } => Choice::PayCumulativeUpkeepOrSacrifice(
            pb::PendingChoiceViewPayCumulativeUpkeepOrSacrifice {
                player: u32::from(player),
                source,
                items: choice_items_to_pb(items),
                count,
            },
        ),
        PendingChoiceView::AssignCombatDamage {
            player,
            source,
            items,
        } => Choice::AssignCombatDamage(pb::PendingChoiceViewAssignCombatDamage {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::DivideSpellDamage {
            player,
            spell,
            items,
            total,
        } => Choice::DivideSpellDamage(pb::PendingChoiceViewDivideSpellDamage {
            player: u32::from(player),
            spell,
            items: choice_items_to_pb(items),
            total,
        }),
        PendingChoiceView::DivideCounters {
            player,
            spell,
            items,
            total,
        } => Choice::DivideCounters(pb::PendingChoiceViewDivideCounters {
            player: u32::from(player),
            spell,
            items: choice_items_to_pb(items),
            total,
        }),
        PendingChoiceView::Scry { player, items } => Choice::Scry(pb::PendingChoiceViewScry {
            player: u32::from(player),
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::Surveil { player, items } => {
            Choice::Surveil(pb::PendingChoiceViewSurveil {
                player: u32::from(player),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::SearchLibrary { player, items } => {
            Choice::SearchLibrary(pb::PendingChoiceViewSearchLibrary {
                player: u32::from(player),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::SelectFromTop {
            player,
            up_to,
            items,
        } => Choice::SelectFromTop(pb::PendingChoiceViewSelectFromTop {
            player: u32::from(player),
            up_to,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::DistributeTop {
            player,
            to_hand,
            to_bottom,
            to_exile_may_play,
            items,
        } => Choice::DistributeTop(pb::PendingChoiceViewDistributeTop {
            player: u32::from(player),
            to_hand,
            to_bottom,
            to_exile_may_play,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ShuffleFromGraveyard {
            player,
            owner,
            source,
            max,
            items,
        } => Choice::ShuffleFromGraveyard(pb::PendingChoiceViewShuffleFromGraveyard {
            player: u32::from(player),
            owner: u32::from(owner),
            source,
            max,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::SacrificeEdict {
            player,
            source,
            keep_one,
            items,
        } => Choice::SacrificeEdict(pb::PendingChoiceViewSacrificeEdict {
            player: u32::from(player),
            source,
            keep_one,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::Proliferate {
            player,
            source,
            items,
        } => Choice::Proliferate(pb::PendingChoiceViewProliferate {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::PhaseOut {
            player,
            source,
            items,
        } => Choice::PhaseOut(pb::PendingChoiceViewPhaseOut {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseAbilityTargets {
            player,
            source,
            label,
            min,
            max,
            items,
        } => Choice::ChooseAbilityTargets(pb::PendingChoiceViewChooseAbilityTargets {
            player: u32::from(player),
            source,
            label,
            min: u32::from(min),
            max: u32::from(max),
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseActivationCostTargets {
            player,
            source,
            count,
            items,
        } => {
            Choice::ChooseActivationCostTargets(pb::PendingChoiceViewChooseActivationCostTargets {
                player: u32::from(player),
                source,
                count: u32::from(count),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::MaySacrifice {
            player,
            source,
            items,
        } => Choice::MaySacrifice(pb::PendingChoiceViewMaySacrifice {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseOwnSacrifices {
            player,
            source,
            count,
            items,
        } => Choice::ChooseOwnSacrifices(pb::PendingChoiceViewChooseOwnSacrifices {
            player: u32::from(player),
            source,
            count,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::Devour {
            player,
            source,
            multiplier,
            items,
        } => Choice::Devour(pb::PendingChoiceViewDevour {
            player: u32::from(player),
            source,
            multiplier,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ExileFromGraveyard {
            player,
            source,
            items,
        } => Choice::ExileFromGraveyard(pb::PendingChoiceViewExileFromGraveyard {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::CasterKeepPermanents {
            player,
            source,
            target_player,
            items,
        } => Choice::CasterKeepPermanents(pb::PendingChoiceViewCasterKeepPermanents {
            player: u32::from(player),
            source,
            target_player: u32::from(target_player),
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseCounterTargetForPlayer {
            player,
            source,
            target_player,
            items,
        } => Choice::ChooseCounterTargetForPlayer(
            pb::PendingChoiceViewChooseCounterTargetForPlayer {
                player: u32::from(player),
                source,
                target_player: u32::from(target_player),
                items: choice_items_to_pb(items),
            },
        ),
        PendingChoiceView::MayReturnFromGraveyard {
            player,
            source,
            items,
        } => Choice::MayReturnFromGraveyard(pb::PendingChoiceViewMayReturnFromGraveyard {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::MayDiscard {
            player,
            source,
            items,
        } => Choice::MayDiscard(pb::PendingChoiceViewMayDiscard {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::Discard {
            player,
            count,
            items,
        } => Choice::Discard(pb::PendingChoiceViewDiscard {
            player: u32::from(player),
            count,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::PutFromHandOnTop {
            player,
            count,
            items,
        } => Choice::PutFromHandOnTop(pb::PendingChoiceViewPutFromHandOnTop {
            player: u32::from(player),
            count,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::PutLandFromHand { player, items } => {
            Choice::PutLandFromHand(pb::PendingChoiceViewPutLandFromHand {
                player: u32::from(player),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::PutCreatureFromHand { player, items } => {
            Choice::PutCreatureFromHand(pb::PendingChoiceViewPutCreatureFromHand {
                player: u32::from(player),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::ChooseDredge { player, items } => {
            Choice::ChooseDredge(pb::PendingChoiceViewChooseDredge {
                player: u32::from(player),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::ChooseExiledWithCard {
            player,
            source,
            items,
        } => Choice::ChooseExiledWithCard(pb::PendingChoiceViewChooseExiledWithCard {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseExiledWithCardToCast {
            player,
            source,
            items,
        } => Choice::ChooseExiledWithCardToCast(pb::PendingChoiceViewChooseExiledWithCardToCast {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseExiledDigToCastFree {
            player,
            source,
            items,
        } => Choice::ChooseExiledDigToCastFree(pb::PendingChoiceViewChooseExiledDigToCastFree {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::DanceExileMore {
            player,
            source,
            total_mv,
            budget,
            items,
        } => Choice::DanceExileMore(pb::PendingChoiceViewDanceExileMore {
            player: u32::from(player),
            source,
            total_mv,
            budget,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::OpponentChoosesPile {
            player,
            source,
            pile_a,
            pile_b,
        } => Choice::OpponentChoosesPile(pb::PendingChoiceViewOpponentChoosesPile {
            player: u32::from(player),
            source,
            pile_a: choice_items_to_pb(pile_a),
            pile_b: choice_items_to_pb(pile_b),
        }),
        PendingChoiceView::OpponentChoosesExiledNonland {
            player,
            source,
            items,
        } => Choice::OpponentChoosesExiledNonland(
            pb::PendingChoiceViewOpponentChoosesExiledNonland {
                player: u32::from(player),
                source,
                items: choice_items_to_pb(items),
            },
        ),
        PendingChoiceView::ChooseExiledToCastFree {
            player,
            source,
            count,
            items,
        } => Choice::ChooseExiledToCastFree(pb::PendingChoiceViewChooseExiledToCastFree {
            player: u32::from(player),
            source,
            count: u32::from(count),
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::RevealedCardToBattlefieldOrHand { player, item } => {
            Choice::RevealedCardToBattlefieldOrHand(
                pb::PendingChoiceViewRevealedCardToBattlefieldOrHand {
                    player: u32::from(player),
                    item: Some(choice_item_to_pb(item)),
                },
            )
        }
        PendingChoiceView::ChooseMode {
            player,
            source,
            labels,
        } => Choice::ChooseMode(pb::PendingChoiceViewChooseMode {
            player: u32::from(player),
            source,
            labels,
        }),
        PendingChoiceView::ChooseTriggerModes {
            player,
            source,
            choose,
            optional,
            modes,
        } => Choice::ChooseTriggerModes(pb::PendingChoiceViewChooseTriggerModes {
            player: u32::from(player),
            source,
            choose: u32::from(choose),
            optional,
            modes: modes.into_iter().map(mode_view_to_pb).collect(),
        }),
        PendingChoiceView::ChooseManaColor {
            player,
            source,
            amount,
        } => Choice::ChooseManaColor(pb::PendingChoiceViewChooseManaColor {
            player: u32::from(player),
            source,
            amount: u32::from(amount),
        }),
        PendingChoiceView::ChooseCreatureType {
            player,
            source,
            options,
        } => Choice::ChooseCreatureType(pb::PendingChoiceViewChooseCreatureType {
            player: u32::from(player),
            source,
            options,
        }),
        PendingChoiceView::ChooseColor { player, source } => {
            Choice::ChooseColor(pb::PendingChoiceViewChooseColor {
                player: u32::from(player),
                source,
            })
        }
        PendingChoiceView::ChooseCopyTarget {
            player,
            source,
            items,
        } => Choice::ChooseCopyTarget(pb::PendingChoiceViewChooseCopyTarget {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::ChooseAttachHost {
            player,
            attachment,
            items,
            optional,
        } => Choice::ChooseAttachHost(pb::PendingChoiceViewChooseAttachHost {
            player: u32::from(player),
            attachment,
            items: choice_items_to_pb(items),
            optional,
        }),
        PendingChoiceView::DeclineUntap { player, items } => {
            Choice::DeclineUntap(pb::PendingChoiceViewDeclineUntap {
                player: u32::from(player),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::PayOrControllerDraws {
            player,
            controller,
            cost,
        } => Choice::PayOrControllerDraws(pb::PendingChoiceViewPayOrControllerDraws {
            player: u32::from(player),
            controller: u32::from(controller),
            cost: Some(wire_cost_to_pb(cost)),
        }),
        PendingChoiceView::ChooseCounteredSpellDestination { player, spell } => {
            Choice::ChooseCounteredSpellDestination(
                pb::PendingChoiceViewChooseCounteredSpellDestination {
                    player: u32::from(player),
                    spell,
                },
            )
        }
        PendingChoiceView::SacrificeUnlessPay {
            player,
            source,
            cost,
        } => Choice::SacrificeUnlessPay(pb::PendingChoiceViewSacrificeUnlessPay {
            player: u32::from(player),
            source,
            cost: Some(wire_cost_to_pb(cost)),
        }),
        PendingChoiceView::SacrificeUnlessReturnLand {
            player,
            source,
            items,
        } => Choice::SacrificeUnlessReturnLand(pb::PendingChoiceViewSacrificeUnlessReturnLand {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::CastCreatureFaceDown { player, items } => {
            Choice::CastCreatureFaceDown(pb::PendingChoiceViewCastCreatureFaceDown {
                player: u32::from(player),
                items: choice_items_to_pb(items),
            })
        }
        PendingChoiceView::ChooseSplittingOpponent {
            player,
            source,
            label,
            items,
        } => Choice::ChooseSplittingOpponent(pb::PendingChoiceViewChooseSplittingOpponent {
            player: u32::from(player),
            source,
            label,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::PartitionRevealed {
            player,
            source,
            items,
        } => Choice::PartitionRevealed(pb::PendingChoiceViewPartitionRevealed {
            player: u32::from(player),
            source,
            items: choice_items_to_pb(items),
        }),
        PendingChoiceView::OpponentChoosesRevealedToGraveyard {
            player,
            source,
            items,
        } => Choice::OpponentChoosesRevealedToGraveyard(
            pb::PendingChoiceViewOpponentChoosesRevealedToGraveyard {
                player: u32::from(player),
                source,
                items: choice_items_to_pb(items),
            },
        ),
        PendingChoiceView::ChoosePileForHand {
            player,
            source,
            pile_a,
            pile_b,
        } => Choice::ChoosePileForHand(pb::PendingChoiceViewChoosePileForHand {
            player: u32::from(player),
            source,
            pile_a: choice_items_to_pb(pile_a),
            pile_b: choice_items_to_pb(pile_b),
        }),
    };
    pb::PendingChoiceView {
        choice: Some(choice),
    }
}

pub fn visible_event_to_pb(event: VisibleEvent) -> Option<pb::VisibleEvent> {
    use pb::visible_event::Event;
    let event = match event {
        VisibleEvent::SpellCast {
            spell,
            from,
            controller,
            target,
            flashback,
            escape,
        } => Event::SpellCast(pb::VisibleEventSpellCast {
            spell,
            from,
            controller: u32::from(controller),
            target: target.map(wire_target_to_pb),
            flashback,
            escape,
        }),
        VisibleEvent::SpellTargetsChosen { spell, targets } => {
            Event::SpellTargetsChosen(pb::VisibleEventSpellTargetsChosen {
                spell,
                targets: targets.into_iter().map(wire_target_to_pb).collect(),
            })
        }
        VisibleEvent::PreparedChanged { object, prepared } => {
            Event::PreparedChanged(pb::VisibleEventPreparedChanged { object, prepared })
        }
        VisibleEvent::LeveledUp { object, level } => Event::LeveledUp(pb::VisibleEventLeveledUp {
            object,
            level: u32::from(level),
        }),
        VisibleEvent::PhasedOut { object } => {
            Event::PhasedOut(pb::VisibleEventPhasedOut { object })
        }
        VisibleEvent::PhasedIn { object } => Event::PhasedIn(pb::VisibleEventPhasedIn { object }),
        VisibleEvent::CreatureTypeChosen { object, subtype } => {
            Event::CreatureTypeChosen(pb::VisibleEventCreatureTypeChosen { object, subtype })
        }
        VisibleEvent::ColorChosen { object, color } => {
            Event::ColorChosen(pb::VisibleEventColorChosen {
                object,
                color: u32::from(color),
            })
        }
        VisibleEvent::ColorSetUntilEndOfTurn { object, color } => {
            Event::ColorSetUntilEndOfTurn(pb::VisibleEventColorSetUntilEndOfTurn {
                object,
                color: u32::from(color),
            })
        }
        VisibleEvent::Flipped { object } => Event::Flipped(pb::VisibleEventFlipped { object }),
        VisibleEvent::PreparedSpellCast {
            spell,
            source,
            controller,
            target,
            x,
        } => Event::PreparedSpellCast(pb::VisibleEventPreparedSpellCast {
            spell,
            source,
            controller: u32::from(controller),
            target: target.map(wire_target_to_pb),
            x,
        }),
        VisibleEvent::AdventureSpellCast {
            spell,
            source,
            controller,
            target,
            x,
        } => Event::AdventureSpellCast(pb::VisibleEventAdventureSpellCast {
            spell,
            source,
            controller: u32::from(controller),
            target: target.map(wire_target_to_pb),
            x,
        }),
        VisibleEvent::StepBegan {
            step,
            active_player,
        } => Event::StepBegan(pb::VisibleEventStepBegan {
            step: u32::from(step),
            active_player: u32::from(active_player),
        }),
        VisibleEvent::TriggeredAbilityOnStack {
            controller,
            source,
            target,
        } => Event::TriggeredAbilityOnStack(pb::VisibleEventTriggeredAbilityOnStack {
            controller: u32::from(controller),
            source,
            target: target.map(wire_target_to_pb),
        }),
        VisibleEvent::AbilityResolved { source } => {
            Event::AbilityResolved(pb::VisibleEventAbilityResolved { source })
        }
        VisibleEvent::LandPlayed {
            permanent,
            from,
            player,
        } => Event::LandPlayed(pb::VisibleEventLandPlayed {
            permanent,
            from,
            player: u32::from(player),
        }),
        VisibleEvent::Tapped { object } => Event::Tapped(pb::VisibleEventTapped { object }),
        VisibleEvent::Untapped { object } => Event::Untapped(pb::VisibleEventUntapped { object }),
        VisibleEvent::RemovedFromCombat { object } => {
            Event::RemovedFromCombat(pb::VisibleEventRemovedFromCombat { object })
        }
        VisibleEvent::RegenerationShieldCreated { object } => {
            Event::RegenerationShieldCreated(pb::VisibleEventRegenerationShieldCreated { object })
        }
        VisibleEvent::Regenerated { object } => {
            Event::Regenerated(pb::VisibleEventRegenerated { object })
        }
        VisibleEvent::RegenerationShieldsExpired { object } => {
            Event::RegenerationShieldsExpired(pb::VisibleEventRegenerationShieldsExpired { object })
        }
        VisibleEvent::LostSummoningSickness { object } => {
            Event::LostSummoningSickness(pb::VisibleEventLostSummoningSickness { object })
        }
        VisibleEvent::CountersPlaced { object, count } => {
            Event::CountersPlaced(pb::VisibleEventCountersPlaced { object, count })
        }
        VisibleEvent::KindCountersPlaced {
            object,
            counter_kind,
            count,
        } => Event::KindCountersPlaced(pb::VisibleEventKindCountersPlaced {
            object,
            counter_kind: u32::from(counter_kind),
            count,
        }),
        VisibleEvent::LoyaltyChanged { object, amount } => {
            Event::LoyaltyChanged(pb::VisibleEventLoyaltyChanged { object, amount })
        }
        VisibleEvent::LoyaltyActivated { object, active } => {
            Event::LoyaltyActivated(pb::VisibleEventLoyaltyActivated { object, active })
        }
        VisibleEvent::AbilityActivatedThisTurn {
            object,
            ability_index,
        } => Event::AbilityActivatedThisTurn(pb::VisibleEventAbilityActivatedThisTurn {
            object,
            ability_index: ability_index as u64,
        }),
        VisibleEvent::TriggeredAbilityThisTurn { source } => {
            Event::TriggeredAbilityThisTurn(pb::VisibleEventTriggeredAbilityThisTurn { source })
        }
        VisibleEvent::AttachedTo { object, host } => {
            Event::AttachedTo(pb::VisibleEventAttachedTo { object, host })
        }
        VisibleEvent::TempBoost {
            object,
            power,
            toughness,
        } => Event::TempBoost(pb::VisibleEventTempBoost {
            object,
            power,
            toughness,
        }),
        VisibleEvent::TempBoostsEnded { object } => {
            Event::TempBoostsEnded(pb::VisibleEventTempBoostsEnded { object })
        }
        VisibleEvent::BasePtSetUntilEndOfTurn {
            object,
            power,
            toughness,
        } => Event::BasePtSetUntilEndOfTurn(pb::VisibleEventBasePtSetUntilEndOfTurn {
            object,
            power,
            toughness,
        }),
        VisibleEvent::TypesAddedUntilEndOfTurn { object } => {
            Event::TypesAddedUntilEndOfTurn(pb::VisibleEventTypesAddedUntilEndOfTurn { object })
        }
        VisibleEvent::ReanimatedCreatureBecame { object } => {
            Event::ReanimatedCreatureBecame(pb::VisibleEventReanimatedCreatureBecame { object })
        }
        VisibleEvent::AddedSubtypes { object } => {
            Event::AddedSubtypes(pb::VisibleEventAddedSubtypes { object })
        }
        VisibleEvent::BecameCopy { object } => {
            Event::BecameCopy(pb::VisibleEventBecameCopy { object })
        }
        VisibleEvent::KeywordsStripped { object } => {
            Event::KeywordsStripped(pb::VisibleEventKeywordsStripped { object })
        }
        VisibleEvent::ControlGainedUntilEndOfTurn { object, controller } => {
            Event::ControlGainedUntilEndOfTurn(pb::VisibleEventControlGainedUntilEndOfTurn {
                object,
                controller: u32::from(controller),
            })
        }
        VisibleEvent::ControlEndedUntilEndOfTurn { object } => {
            Event::ControlEndedUntilEndOfTurn(pb::VisibleEventControlEndedUntilEndOfTurn { object })
        }
        VisibleEvent::AbilitiesGranted { target, source } => {
            Event::AbilitiesGranted(pb::VisibleEventAbilitiesGranted { target, source })
        }
        VisibleEvent::GrantedAbilitiesEnded => {
            Event::GrantedAbilitiesEnded(pb::VisibleEventGrantedAbilitiesEnded {})
        }
        VisibleEvent::ControlGained { object, controller } => {
            Event::ControlGained(pb::VisibleEventControlGained {
                object,
                controller: u32::from(controller),
            })
        }
        VisibleEvent::AttackerDeclared { object, defender } => {
            Event::AttackerDeclared(pb::VisibleEventAttackerDeclared {
                object,
                defender: u32::from(defender),
            })
        }
        VisibleEvent::TokenEnteredAttacking { token, defender } => {
            Event::TokenEnteredAttacking(pb::VisibleEventTokenEnteredAttacking {
                token,
                defender: u32::from(defender),
            })
        }
        VisibleEvent::Goaded { object, by } => Event::Goaded(pb::VisibleEventGoaded {
            object,
            by: u32::from(by),
        }),
        VisibleEvent::GoadCleared { by } => {
            Event::GoadCleared(pb::VisibleEventGoadCleared { by: u32::from(by) })
        }
        VisibleEvent::NextUntapSkipMarked { object } => {
            Event::NextUntapSkipMarked(pb::VisibleEventNextUntapSkipMarked { object })
        }
        VisibleEvent::NextUntapSkipConsumed { object } => {
            Event::NextUntapSkipConsumed(pb::VisibleEventNextUntapSkipConsumed { object })
        }
        VisibleEvent::VowCountersPlaced { object, protected } => {
            Event::VowCountersPlaced(pb::VisibleEventVowCountersPlaced {
                object,
                protected: u32::from(protected),
            })
        }
        VisibleEvent::TimeCountersPlaced { card, count } => {
            Event::TimeCountersPlaced(pb::VisibleEventTimeCountersPlaced { card, count })
        }
        VisibleEvent::TimeCountersRemoved { card } => {
            Event::TimeCountersRemoved(pb::VisibleEventTimeCountersRemoved { card })
        }
        VisibleEvent::MustAttackDeclared { object, defender } => {
            Event::MustAttackDeclared(pb::VisibleEventMustAttackDeclared {
                object,
                defender: u32::from(defender),
            })
        }
        VisibleEvent::DelayedTriggerScheduled { controller, source } => {
            Event::DelayedTriggerScheduled(pb::VisibleEventDelayedTriggerScheduled {
                controller: u32::from(controller),
                source,
            })
        }
        VisibleEvent::DelayedTriggersFired => {
            Event::DelayedTriggersFired(pb::VisibleEventDelayedTriggersFired {})
        }
        VisibleEvent::NextCastTriggerArmed { controller, source } => {
            Event::NextCastTriggerArmed(pb::VisibleEventNextCastTriggerArmed {
                controller: u32::from(controller),
                source,
            })
        }
        VisibleEvent::NextCastTriggerConsumed { controller, source } => {
            Event::NextCastTriggerConsumed(pb::VisibleEventNextCastTriggerConsumed {
                controller: u32::from(controller),
                source,
            })
        }
        VisibleEvent::CombatDamageWatchArmed {
            controller,
            source,
            watched,
        } => Event::CombatDamageWatchArmed(pb::VisibleEventCombatDamageWatchArmed {
            controller: u32::from(controller),
            source,
            watched,
        }),
        VisibleEvent::CombatDamageWatchConsumed { controller, source } => {
            Event::CombatDamageWatchConsumed(pb::VisibleEventCombatDamageWatchConsumed {
                controller: u32::from(controller),
                source,
            })
        }
        VisibleEvent::CombatDamageCopyArmed {
            controller,
            source,
            card,
        } => Event::CombatDamageCopyArmed(pb::VisibleEventCombatDamageCopyArmed {
            controller: u32::from(controller),
            source,
            card,
        }),
        VisibleEvent::ExiledFromLibraryMayPlay {
            player,
            card,
            from,
            until_next_turn,
        } => Event::ExiledFromLibraryMayPlay(pb::VisibleEventExiledFromLibraryMayPlay {
            player: u32::from(player),
            card,
            from,
            until_next_turn,
        }),
        VisibleEvent::ExiledFromLibraryToChooseCastFree { player, card, from } => {
            Event::ExiledFromLibraryToChooseCastFree(
                pb::VisibleEventExiledFromLibraryToChooseCastFree {
                    player: u32::from(player),
                    card,
                    from,
                },
            )
        }
        VisibleEvent::PlayFromExilePermissionArmed { card } => {
            Event::PlayFromExilePermissionArmed(pb::VisibleEventPlayFromExilePermissionArmed {
                card,
            })
        }
        VisibleEvent::PlayFromExileEnded => {
            Event::PlayFromExileEnded(pb::VisibleEventPlayFromExileEnded {})
        }
        VisibleEvent::BlockerDeclared { blocker, attacker } => {
            Event::BlockerDeclared(pb::VisibleEventBlockerDeclared { blocker, attacker })
        }
        VisibleEvent::CombatDamageDivided {
            attacker,
            assignment,
        } => Event::CombatDamageDivided(pb::VisibleEventCombatDamageDivided {
            attacker,
            assignment: assignment.into_iter().map(object_amount_to_pb).collect(),
        }),
        VisibleEvent::SpellDamageDivided {
            spell,
            assignment,
            players,
        } => Event::SpellDamageDivided(pb::VisibleEventSpellDamageDivided {
            spell,
            assignment: assignment.into_iter().map(object_amount_to_pb).collect(),
            players: players.into_iter().map(player_amount_to_pb).collect(),
        }),
        VisibleEvent::SpellCountersDivided { spell, assignment } => {
            Event::SpellCountersDivided(pb::VisibleEventSpellCountersDivided {
                spell,
                assignment: assignment.into_iter().map(object_amount_to_pb).collect(),
            })
        }
        VisibleEvent::DeathtouchMarked { object } => {
            Event::DeathtouchMarked(pb::VisibleEventDeathtouchMarked { object })
        }
        VisibleEvent::CombatCleared => Event::CombatCleared(pb::VisibleEventCombatCleared {}),
        VisibleEvent::CommanderCastFromCommandZone { player } => {
            Event::CommanderCastFromCommandZone(pb::VisibleEventCommanderCastFromCommandZone {
                player: u32::from(player),
            })
        }
        VisibleEvent::FlashPermissionGranted { player } => {
            Event::FlashPermissionGranted(pb::VisibleEventFlashPermissionGranted {
                player: u32::from(player),
            })
        }
        VisibleEvent::ChannelColorlessManaGranted { player } => {
            Event::ChannelColorlessManaGranted(pb::VisibleEventChannelColorlessManaGranted {
                player: u32::from(player),
            })
        }
        VisibleEvent::CommanderDamageDealt {
            source,
            player,
            amount,
        } => Event::CommanderDamageDealt(pb::VisibleEventCommanderDamageDealt {
            source,
            player: u32::from(player),
            amount,
        }),
        VisibleEvent::CombatDamageDealtToPlayer {
            source,
            player,
            amount,
        } => Event::CombatDamageDealtToPlayer(pb::VisibleEventCombatDamageDealtToPlayer {
            source,
            player: u32::from(player),
            amount,
        }),
        VisibleEvent::CombatDamageDealtToCreature {
            source,
            target,
            amount,
        } => Event::CombatDamageDealtToCreature(pb::VisibleEventCombatDamageDealtToCreature {
            source,
            target,
            amount,
        }),
        VisibleEvent::CombatDamagePrevented { player, amount } => {
            Event::CombatDamagePrevented(pb::VisibleEventCombatDamagePrevented {
                player: u32::from(player),
                amount,
            })
        }
        VisibleEvent::MovedToCommandZone { card, from } => {
            Event::MovedToCommandZone(pb::VisibleEventMovedToCommandZone { card, from })
        }
        VisibleEvent::ManaEmptied { player } => Event::ManaEmptied(pb::VisibleEventManaEmptied {
            player: u32::from(player),
        }),
        VisibleEvent::DamageCleared { object } => {
            Event::DamageCleared(pb::VisibleEventDamageCleared { object })
        }
        VisibleEvent::ManaAdded {
            player,
            mana,
            amount,
        } => Event::ManaAdded(pb::VisibleEventManaAdded {
            player: u32::from(player),
            mana: u32::from(mana),
            amount: u32::from(amount),
        }),
        VisibleEvent::ManaSpent { player, mana } => Event::ManaSpent(pb::VisibleEventManaSpent {
            player: u32::from(player),
            mana: mana.into_iter().map(u32::from).collect(),
        }),
        VisibleEvent::PriorityPassed { player } => {
            Event::PriorityPassed(pb::VisibleEventPriorityPassed {
                player: u32::from(player),
            })
        }
        VisibleEvent::PermanentEntered { permanent, from } => {
            Event::PermanentEntered(pb::VisibleEventPermanentEntered { permanent, from })
        }
        VisibleEvent::ReanimatedToBattlefield {
            permanent,
            from,
            controller,
            finality,
            tapped,
        } => Event::ReanimatedToBattlefield(pb::VisibleEventReanimatedToBattlefield {
            permanent,
            from,
            controller: u32::from(controller),
            finality,
            tapped,
        }),
        VisibleEvent::TokenCreated {
            token,
            controller,
            creator,
        } => Event::TokenCreated(pb::VisibleEventTokenCreated {
            token,
            controller: u32::from(controller),
            creator,
        }),
        VisibleEvent::TokenCeasedToExist { token } => {
            Event::TokenCeasedToExist(pb::VisibleEventTokenCeasedToExist { token })
        }
        VisibleEvent::SpellCopied {
            copy,
            original,
            controller,
        } => Event::SpellCopied(pb::VisibleEventSpellCopied {
            copy,
            original,
            controller: u32::from(controller),
        }),
        VisibleEvent::SpellCeasedToExist { spell } => {
            Event::SpellCeasedToExist(pb::VisibleEventSpellCeasedToExist { spell })
        }
        VisibleEvent::DamageMarked {
            object,
            amount,
            source,
        } => Event::DamageMarked(pb::VisibleEventDamageMarked {
            object,
            amount,
            source,
        }),
        VisibleEvent::MovedToGraveyard { card, from } => {
            Event::MovedToGraveyard(pb::VisibleEventMovedToGraveyard { card, from })
        }
        VisibleEvent::MovedToExile { card, from } => {
            Event::MovedToExile(pb::VisibleEventMovedToExile { card, from })
        }
        VisibleEvent::ExiledOnAdventure { card, from, owner } => {
            Event::ExiledOnAdventure(pb::VisibleEventExiledOnAdventure {
                card,
                from,
                owner: u32::from(owner),
            })
        }
        VisibleEvent::ExiledUntilSourceLeaves { source, object } => {
            Event::ExiledUntilSourceLeaves(pb::VisibleEventExiledUntilSourceLeaves {
                source,
                object,
            })
        }
        VisibleEvent::ExiledUntilSourceLeavesMintingIllusion { source, object } => {
            Event::ExiledUntilSourceLeavesMintingIllusion(
                pb::VisibleEventExiledUntilSourceLeavesMintingIllusion { source, object },
            )
        }
        VisibleEvent::LeavesIllusionMinted { source, object } => {
            Event::LeavesIllusionMinted(pb::VisibleEventLeavesIllusionMinted { source, object })
        }
        VisibleEvent::TokenGrantedReturnExiledOnLeave { token, exiled } => {
            Event::TokenGrantedReturnExiledOnLeave(
                pb::VisibleEventTokenGrantedReturnExiledOnLeave { token, exiled },
            )
        }
        VisibleEvent::ReturnedExiledCardToGraveyard { card, from } => {
            Event::ReturnedExiledCardToGraveyard(pb::VisibleEventReturnedExiledCardToGraveyard {
                card,
                from,
            })
        }
        VisibleEvent::ExiledWithSource { source, object } => {
            Event::ExiledWithSource(pb::VisibleEventExiledWithSource { source, object })
        }
        VisibleEvent::CardExiledWithSourceLeftExile { source, object } => {
            Event::CardExiledWithSourceLeftExile(pb::VisibleEventCardExiledWithSourceLeftExile {
                source,
                object,
            })
        }
        VisibleEvent::CastFromExileFreePermissionGranted { card, player } => {
            Event::CastFromExileFreePermissionGranted(
                pb::VisibleEventCastFromExileFreePermissionGranted {
                    card,
                    player: u32::from(player),
                },
            )
        }
        VisibleEvent::CastFromExileFreeBottomsLibraryOnLeave { card } => {
            Event::CastFromExileFreeBottomsLibraryOnLeave(
                pb::VisibleEventCastFromExileFreeBottomsLibraryOnLeave { card },
            )
        }
        VisibleEvent::CastFromExileFreeEnded => {
            Event::CastFromExileFreeEnded(pb::VisibleEventCastFromExileFreeEnded {})
        }
        VisibleEvent::ReturnedFromLinkedExile {
            permanent,
            from,
            controller,
            source,
        } => Event::ReturnedFromLinkedExile(pb::VisibleEventReturnedFromLinkedExile {
            permanent,
            from,
            controller: u32::from(controller),
            source,
        }),
        VisibleEvent::ReturnedToHand { card, from } => {
            Event::ReturnedToHand(pb::VisibleEventReturnedToHand { card, from })
        }
        // `second_from_top` (Whirlpool Whelm) isn't carried on the wire: the library is a hidden
        // zone, so its exact insertion index is unobservable to the client — the animation only
        // shows the card entering the library. Add a proto field if a client ever needs it.
        VisibleEvent::TuckedToLibrary {
            card,
            from,
            to_top,
            second_from_top: _,
        } => Event::TuckedToLibrary(pb::VisibleEventTuckedToLibrary { card, from, to_top }),
        VisibleEvent::LibraryShuffled { player } => {
            Event::LibraryShuffled(pb::VisibleEventLibraryShuffled {
                player: u32::from(player),
            })
        }
        VisibleEvent::RevealedTopOfLibrary { player, card, def } => {
            Event::RevealedTopOfLibrary(pb::VisibleEventRevealedTopOfLibrary {
                player: u32::from(player),
                card,
                def,
            })
        }
        VisibleEvent::PutOnBottomOfLibrary { player, card } => {
            Event::PutOnBottomOfLibrary(pb::VisibleEventPutOnBottomOfLibrary {
                player: u32::from(player),
                card,
            })
        }
        VisibleEvent::SearchedToHand {
            player,
            object,
            from,
            card,
        } => Event::SearchedToHand(pb::VisibleEventSearchedToHand {
            player: u32::from(player),
            object,
            from,
            card,
        }),
        VisibleEvent::SearchedToBattlefield {
            permanent,
            from,
            controller,
            tapped,
        } => Event::SearchedToBattlefield(pb::VisibleEventSearchedToBattlefield {
            permanent,
            from,
            controller: u32::from(controller),
            tapped,
        }),
        VisibleEvent::Manifested {
            permanent,
            controller,
        } => Event::Manifested(pb::VisibleEventManifested {
            permanent,
            controller: u32::from(controller),
        }),
        VisibleEvent::TurnedFaceUp { permanent } => {
            Event::TurnedFaceUp(pb::VisibleEventTurnedFaceUp { permanent })
        }
        VisibleEvent::PutOntoBattlefieldFromHand {
            permanent,
            from,
            controller,
            tapped,
        } => Event::PutOntoBattlefieldFromHand(pb::VisibleEventPutOntoBattlefieldFromHand {
            permanent,
            from,
            controller: u32::from(controller),
            tapped,
        }),
        VisibleEvent::Milled { player, card, from } => Event::Milled(pb::VisibleEventMilled {
            player: u32::from(player),
            card,
            from,
        }),
        VisibleEvent::LifeChanged {
            player,
            amount,
            source,
        } => Event::LifeChanged(pb::VisibleEventLifeChanged {
            player: u32::from(player),
            amount,
            source,
        }),
        VisibleEvent::DrewFromEmptyLibrary { player } => {
            Event::DrewFromEmptyLibrary(pb::VisibleEventDrewFromEmptyLibrary {
                player: u32::from(player),
            })
        }
        VisibleEvent::PlayerLost { player } => Event::PlayerLost(pb::VisibleEventPlayerLost {
            player: u32::from(player),
        }),
        VisibleEvent::CitysBlessingGained { player } => {
            Event::CitysBlessingGained(pb::VisibleEventCitysBlessingGained {
                player: u32::from(player),
            })
        }
        VisibleEvent::MulliganTaken { .. }
        | VisibleEvent::HandKept { .. }
        | VisibleEvent::MulligansFinished => {
            // Mulligan state is snapshot-sourced; no event payload is sent until the wire owns
            // explicit mulligan variants.
            return None;
        }
        VisibleEvent::CardDrawn {
            player,
            object,
            from,
            card,
        } => Event::CardDrawn(pb::VisibleEventCardDrawn {
            player: u32::from(player),
            object,
            from,
            card,
        }),
        VisibleEvent::Sacrificed { object, by } => Event::Sacrificed(pb::VisibleEventSacrificed {
            object,
            by: u32::from(by),
        }),
        VisibleEvent::Discarded { card, from, player } => {
            Event::Discarded(pb::VisibleEventDiscarded {
                card,
                from,
                player: u32::from(player),
            })
        }
        VisibleEvent::PutFromHandOnTop {
            player,
            card,
            from,
            def,
        } => Event::PutFromHandOnTop(pb::VisibleEventPutFromHandOnTop {
            player: u32::from(player),
            card,
            from,
            def,
        }),
        VisibleEvent::ExiledFromGraveyardMayPlay { player, card, from } => {
            Event::ExiledFromGraveyardMayPlay(pb::VisibleEventExiledFromGraveyardMayPlay {
                player: u32::from(player),
                card,
                from,
            })
        }
        VisibleEvent::AbilityCountered { source } => {
            Event::AbilityCountered(pb::VisibleEventAbilityCountered { source })
        }
        VisibleEvent::ConditionedControlGained { object, controller } => {
            Event::ConditionedControlGained(pb::VisibleEventConditionedControlGained {
                object,
                controller: u32::from(controller),
            })
        }
        VisibleEvent::ConditionedControlEnded { object } => {
            Event::ConditionedControlEnded(pb::VisibleEventConditionedControlEnded { object })
        }
        VisibleEvent::DamageDealtToPlayer {
            source,
            player,
            amount,
        } => Event::DamageDealtToPlayer(pb::VisibleEventDamageDealtToPlayer {
            source,
            player: u32::from(player),
            amount,
        }),
        VisibleEvent::FlickeredToBattlefield {
            permanent,
            from,
            controller,
        } => Event::FlickeredToBattlefield(pb::VisibleEventFlickeredToBattlefield {
            permanent,
            from,
            controller: u32::from(controller),
        }),
    };
    Some(pb::VisibleEvent { event: Some(event) })
}

pub fn visible_state_to_pb(state: VisibleState) -> pb::VisibleState {
    pb::VisibleState {
        viewer: u32::from(state.viewer),
        active_player: u32::from(state.active_player),
        step: u32::from(state.step),
        priority: u32::from(state.priority),
        players: state.players.into_iter().map(player_view_to_pb).collect(),
        objects: state.objects.into_iter().map(object_view_to_pb).collect(),
        stack: state
            .stack
            .into_iter()
            .map(stack_object_view_to_pb)
            .collect(),
        combat: Some(combat_view_to_pb(state.combat)),
        can_act: state.can_act,
        yielded: state.yielded,
        turn_yielded: state.turn_yielded,
        stack_hold_remaining_ms: state.stack_hold_remaining_ms,
        pending_choice: state.pending_choice.map(pending_choice_view_to_pb),
        actions: state.actions.into_iter().map(action_view_to_pb).collect(),
        mulliganing: state.mulliganing,
    }
}

pub fn stream_frame_to_pb(frame: StreamFrame) -> pb::StreamFrame {
    use pb::stream_frame::Frame;
    let frame = match frame {
        StreamFrame::Snapshot { seq, state } => Frame::Snapshot(pb::SnapshotFrame {
            seq,
            state: Some(visible_state_to_pb(state)),
        }),
        StreamFrame::Delta(envelope) => Frame::Delta(pb::DeltaEnvelope {
            seq: envelope.seq,
            events: envelope
                .events
                .into_iter()
                .filter_map(visible_event_to_pb)
                .collect(),
            state: Some(visible_state_to_pb(envelope.state)),
            auto_actions: envelope.auto_actions,
        }),
        StreamFrame::Heartbeat => Frame::Heartbeat(pb::Heartbeat {}),
    };
    pb::StreamFrame { frame: Some(frame) }
}

#[cfg(test)]
mod tests {
    use schema::{
        ActionView, ChoiceItem, CombatView, DeltaEnvelope, ObjectView, PendingChoiceView,
        PlayerView, StackObjectView, StreamFrame, VisibleEvent, VisibleState, WireCost, WireKind,
        WireManaPool,
    };

    use super::*;
    use crate::grpc::map::common::object_id_list_to_pb;

    fn empty_player(player: u8) -> PlayerView {
        PlayerView {
            player,
            username: format!("p{player}"),
            life: 40,
            commander_tax: 0,
            lost: false,
            hand_count: 0,
            library_count: 99,
            mulligans_taken: 0,
            hand_kept: false,
            can_mulligan: false,
            mana_pool: WireManaPool::default(),
            commander_damage: vec![],
        }
    }

    fn rich_snapshot_state() -> VisibleState {
        VisibleState {
            viewer: 0,
            active_player: 0,
            step: 3,
            priority: 0,
            players: vec![empty_player(0), empty_player(1)],
            objects: vec![ObjectView {
                id: 10,
                zone: 1,
                owner: 0,
                controller: 0,
                card_id: "card-1".into(),
                name: "Bear".into(),
                print: "print-1".into(),
                kind: WireKind::Creature {
                    power: 2,
                    toughness: 2,
                },
                mana_cost: WireCost {
                    generic: 1,
                    colored: [0, 0, 0, 0, 1],
                    has_x: false,
                    x_symbols: 0,
                },
                needs_target: false,
                tapped: false,
                summoning_sick: false,
                has_haste: false,
                keywords: vec!["trample".into()],
                power: 2,
                toughness: 2,
                loyalty: 0,
                plus_counters: 0,
                marked_damage: 0,
                is_commander: false,
                goaded: false,
                taps_for_mana: false,
                prepared: false,
                phased_out: false,
                face_down: false,
                attached_to: None,
                modifiers: vec![],
            }],
            stack: vec![StackObjectView {
                kind: "spell".into(),
                source: 10,
                controller: 0,
                label: "Shock".into(),
                target: Some(schema::WireTarget::Player { player: 1 }),
            }],
            combat: CombatView::default(),
            can_act: true,
            mulliganing: false,
            yielded: false,
            turn_yielded: false,
            stack_hold_remaining_ms: 0,
            pending_choice: Some(PendingChoiceView::ChooseTarget {
                player: 0,
                source: 10,
                label: "Deal 2".into(),
                items: vec![ChoiceItem {
                    id: 11,
                    label: "Goblin".into(),
                    print: String::new(),
                    player: None,
                }],
                optional: false,
                max: 1,
            }),
            actions: vec![ActionView {
                id: 42,
                kind: "cast".into(),
                object: Some(12),
                ability_index: None,
                section: "hand".into(),
                label: "Lightning Bolt".into(),
                needs_target: true,
                targets: vec![schema::WireTarget::Player { player: 1 }],
                modal: None,
                sacrifice_choices: Some(vec![13, 14]),
                discard_choices: None,
                discard_count: 0,
                graveyard_exile_choices: None,
                graveyard_exile_min: 0,
                graveyard_exile_max: 0,
                has_x: false,
                min_x: 0,
                max_x: 0,
                x_cost: None,
                auto_tap: vec![],
                required_attacks: vec![],
            }],
        }
    }

    #[test]
    fn stream_frame_heartbeat_maps_to_the_heartbeat_variant() {
        let pb = stream_frame_to_pb(StreamFrame::Heartbeat);
        assert!(matches!(
            pb.frame,
            Some(pb::stream_frame::Frame::Heartbeat(_))
        ));
    }

    #[test]
    fn rich_snapshot_preserves_choice_actions_and_oneof_kinds() {
        let state = rich_snapshot_state();
        let pb = stream_frame_to_pb(StreamFrame::Snapshot {
            seq: 9,
            state: state.clone(),
        });
        let Some(pb::stream_frame::Frame::Snapshot(snap)) = pb.frame else {
            panic!("expected Snapshot frame");
        };
        assert_eq!(snap.seq, 9);
        let st = snap.state.expect("snapshot state");
        assert_eq!(st.viewer, 0);
        assert_eq!(st.objects.len(), 1);
        assert_eq!(
            st.objects[0].kind.as_ref().and_then(|k| k.kind.as_ref()),
            Some(&pb::wire_kind::Kind::Creature(pb::wire_kind::Creature {
                power: 2,
                toughness: 2,
            }))
        );
        assert!(matches!(
            st.pending_choice.as_ref().and_then(|c| c.choice.as_ref()),
            Some(pb::pending_choice_view::Choice::ChooseTarget(_))
        ));
        assert_eq!(st.actions.len(), 1);
        assert_eq!(
            st.actions[0].sacrifice_choices,
            object_id_list_to_pb(Some(vec![13, 14]))
        );
        assert_eq!(
            st.stack[0].target.as_ref().and_then(|t| t.kind.as_ref()),
            Some(&pb::wire_target::Kind::Player(
                pb::wire_target::PlayerTarget { player: 1 }
            ))
        );
    }

    #[test]
    fn delta_collapses_divided_damage_into_object_amount_rows() {
        let state = rich_snapshot_state();
        let pb = stream_frame_to_pb(StreamFrame::Delta(DeltaEnvelope {
            seq: 10,
            events: vec![
                VisibleEvent::CombatDamageDivided {
                    attacker: 10,
                    assignment: vec![(11, 2), (12, 1)],
                },
                VisibleEvent::SpellDamageDivided {
                    spell: 20,
                    assignment: vec![(11, 1)],
                    players: vec![(1, 3)],
                },
            ],
            state,
            auto_actions: vec!["auto-pass".into()],
        }));
        let Some(pb::stream_frame::Frame::Delta(delta)) = pb.frame else {
            panic!("expected Delta frame");
        };
        assert_eq!(delta.seq, 10);
        assert_eq!(delta.auto_actions, vec!["auto-pass"]);
        assert_eq!(delta.events.len(), 2);
        match delta.events[0].event.as_ref() {
            Some(pb::visible_event::Event::CombatDamageDivided(e)) => {
                assert_eq!(e.assignment.len(), 2);
                assert_eq!(e.assignment[0].id, 11);
                assert_eq!(e.assignment[0].amount, 2);
            }
            other => panic!("expected CombatDamageDivided, got {other:?}"),
        }
        match delta.events[1].event.as_ref() {
            Some(pb::visible_event::Event::SpellDamageDivided(e)) => {
                assert_eq!(e.players[0].player, 1);
                assert_eq!(e.players[0].amount, 3);
            }
            other => panic!("expected SpellDamageDivided, got {other:?}"),
        }
    }

    #[test]
    fn delta_omits_mulligan_lifecycle_events_from_wire() {
        let state = rich_snapshot_state();
        let pb = stream_frame_to_pb(StreamFrame::Delta(DeltaEnvelope {
            seq: 11,
            events: vec![
                VisibleEvent::MulliganTaken {
                    player: 0,
                    mulligans_taken: 1,
                    hand_size: 6,
                },
                VisibleEvent::HandKept { player: 0 },
                VisibleEvent::MulligansFinished,
            ],
            state,
            auto_actions: vec![],
        }));
        let Some(pb::stream_frame::Frame::Delta(delta)) = pb.frame else {
            panic!("expected Delta frame");
        };
        assert!(delta.events.is_empty());
    }
}
