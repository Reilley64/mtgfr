//! Snapshot: the redacted full view of the game a client renders from.
//!
//! Game *setup* mutates state without emitting events, so a client can't rebuild the
//! starting board from deltas alone. The stream's first frame is a `VisibleState`
//! snapshot for the viewer; live `VisibleEvent` deltas fold on top of it.
//!
//! Pending-choice projection lives in [`crate::projection::choice`]; this module
//! assembles the rest of the snapshot around it.

use serde::{Deserialize, Serialize};

use crate::catalog::{wire_cost, wire_kind};
use crate::dto::{
    ActionView, CombatView, CommanderDamageView, ModalView, ModeView, ModifierSourceView,
    ObjectView, PlayerView, StackObjectView, VisibleState, WireKind, WireManaPool,
};
use crate::event::DeltaEnvelope;
use crate::intent::{WireAttack, WireBlock, WireTarget};
use crate::projection::project_pending_choice;

fn format_modifier_contribution(contribution: engine::ModifierContribution) -> String {
    use engine::ModifierContribution;
    match contribution {
        ModifierContribution::PowerToughness { power, toughness } => {
            format!("{power:+}/{toughness:+}")
        }
        ModifierContribution::SetBasePowerToughness { power, toughness } => {
            format!("base {power}/{toughness}")
        }
        ModifierContribution::Keyword(keyword) => crate::catalog::keyword_label(keyword),
        ModifierContribution::PlusCounters(n) => {
            if n == 1 {
                "+1/+1 counter".into()
            } else {
                format!("+1/+1 ×{n}")
            }
        }
        ModifierContribution::Goaded => "goaded".into(),
        ModifierContribution::Controls => "controls".into(),
        ModifierContribution::ManaAbility => "mana ability".into(),
    }
}

/// The `VisibleState::viewer` value for a spectator — a watcher with no seat, so no owned zone.
/// Distinct from every real seat (ids run 0..4); the client renders this view read-only.
pub const SPECTATOR_VIEWER: u8 = u8::MAX;

/// Table-owned facts that finish a [`VisibleState`]. Pure data — no `Seat` / tokio coupling.
///
/// Yield, stack-hold remaining, and display names live on the server's `Table`, not the `Game`
/// (ADR 0026/0027). Callers map table state into this DTO and pass it to [`complete_visible`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewExtras {
    pub yields: [bool; 4],
    pub turn_yields: [bool; 4],
    pub stack_hold_remaining_ms: u32,
    pub usernames: [String; 4],
    /// Per-seat Card id → Printing UUID from the seat's deck (art preference). Empty maps mean
    /// every object uses its CardDef `default_print`.
    pub prints: [std::collections::HashMap<String, String>; 4],
}

/// One wire-complete [`VisibleState`] for `viewer` (`Some` = seated, `None` = spectator).
///
/// Redacts private zones, projects the board, then stamps Table policy from `extras` in one
/// pass — yield, hold remaining, and usernames. Incomplete board projection is not a public
/// wire path (ADR 0005/0006).
pub fn complete_visible(
    game: &engine::Game,
    viewer: Option<engine::PlayerId>,
    extras: &ViewExtras,
) -> VisibleState {
    use engine::PlayerId;

    let mut state = project_board(game, viewer);
    // A spectator never yields; a seated viewer reads their seat's flag from extras.
    if let Some(PlayerId(v)) = viewer {
        state.yielded = extras.yields[v as usize];
        state.turn_yielded = extras.turn_yields[v as usize];
    }
    state.stack_hold_remaining_ms = extras.stack_hold_remaining_ms;
    for (i, player) in state.players.iter_mut().enumerate() {
        player.username = extras.usernames[i].clone();
    }
    // Overlay deck-chosen Printings onto objects (by owner seat + Card id).
    for obj in &mut state.objects {
        if obj.card_id.is_empty() {
            continue;
        }
        let seat = obj.owner as usize;
        if seat >= extras.prints.len() {
            continue;
        }
        if let Some(print) = extras.prints[seat].get(&obj.card_id)
            && !print.is_empty()
        {
            obj.print = print.clone();
        }
    }
    // Same overlay for pending-choice items — library/scry picks never appear in `objects`
    // (libraries aren't itemized), so their art rides on ChoiceItem.print alone.
    if let Some(ref mut pc) = state.pending_choice {
        pc.for_each_item_mut(|item| {
            if item.player.is_some() || item.print.is_empty() {
                return;
            }
            let owner = game.owner_of(item.id);
            let card_id = game.def_of(item.id).id;
            let seat = owner.0 as usize;
            if seat >= extras.prints.len() {
                return;
            }
            if let Some(print) = extras.prints[seat].get(card_id)
                && !print.is_empty()
            {
                item.print = print.clone();
            }
        });
    }
    state
}

/// Wire form of one of `game`'s stored [`engine::LegalAction`]s. `MeaningfulAction::PlayLand`/
/// `Cast` bucket by their carried zone; `Activate` is always "battlefield"; the combat
/// declarations are "combat" (the board UI drives them, not the action bar).
fn action_view(game: &engine::Game, action: &engine::LegalAction) -> ActionView {
    use engine::{MeaningfulAction, TargetSpec, Zone};

    // The legal target set, straight from the engine — the one enumeration the cast gate, auto-pass
    // and the client's highlight all read (`Game::legal_targets`).
    let targets = |object, ability_index| {
        game.legal_targets(object, ability_index)
            .into_iter()
            .map(WireTarget::of)
            .collect()
    };

    // A modal spell's modes, each with the targets legal for it (CR 700.2). `None` for a card that
    // isn't modal, so `modal.is_some()` is the client's "does this need a mode picker?".
    let modal = |card| {
        let def = game.def_of(card);
        if !def.modal {
            return None;
        }
        Some(ModalView {
            choose: def.modal_choose,
            // `Game::modal_choose_max` gates the range on `action.player` controlling a commander
            // when the card's modal_choose_max_if_commander asks for it (Nexus Mentality) — the
            // same check `validate_modes` enforces, so the prompt never offers a range the cast
            // would reject.
            choose_max: game.modal_choose_max(def, action.player),
            modes: game
                .modes_of(card)
                .into_iter()
                .map(|m| ModeView {
                    label: m.label,
                    needs_target: m.needs_target,
                    targets: m.targets.into_iter().map(WireTarget::of).collect(),
                })
                .collect(),
        })
    };

    // ponytail: PlayLand/Cast only ever carry Hand/Command/Exile (`Game::playable_zone`) — no
    // pool card is played from the graveyard yet, so that arm is unreached in practice; kept for
    // when one exists rather than panicking on a zone this function doesn't expect.
    let section_of = |zone: Zone| match zone {
        Zone::Command => "command",
        Zone::Exile => "exile",
        Zone::Graveyard => "graveyard",
        _ => "hand",
    };

    let mut view = match action.kind {
        MeaningfulAction::PlayLand { card, zone } => ActionView {
            id: action.id,
            kind: "play_land".to_string(),
            object: Some(card),
            ability_index: None,
            section: section_of(zone).to_string(),
            label: game.def_of(card).name.to_string(),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        MeaningfulAction::Cast { card, zone } => {
            let def = game.def_of(card);
            let (discard_choices, discard_count) = match game.discard_cost_candidates(card) {
                Some((choices, n)) => (Some(choices), n),
                None => (None, 0),
            };
            let (graveyard_exile_choices, graveyard_exile_min, graveyard_exile_max) =
                match game.graveyard_exile_cost(card, zone) {
                    Some((choices, min, max)) => (Some(choices), min, max),
                    None => (None, 0, 0),
                };
            // The mana cost being paid — flashback/escape replace the printed cost from the GY.
            let has_x = if zone == Zone::Graveyard {
                def.flashback
                    .or(def.escape.map(|e| e.cost))
                    .map(|c| c.x > 0)
                    .unwrap_or(def.cost.x > 0)
            } else {
                def.cost.x > 0
            };
            ActionView {
                id: action.id,
                kind: "cast".to_string(),
                object: Some(card),
                ability_index: None,
                section: section_of(zone).to_string(),
                label: def.name.to_string(),
                needs_target: game.target_spec_of(card) != TargetSpec::None,
                targets: targets(card, None),
                modal: modal(card),
                sacrifice_choices: None,
                discard_choices,
                discard_count,
                graveyard_exile_choices,
                graveyard_exile_min,
                graveyard_exile_max,
                has_x,
                auto_tap: Vec::new(),
                required_attacks: Vec::new(),
            }
        }
        MeaningfulAction::Activate { source, ability } => ActionView {
            id: action.id,
            kind: "activate".to_string(),
            object: Some(source),
            ability_index: Some(ability as u32),
            section: "battlefield".to_string(),
            label: game
                .ability_at(source, ability)
                .map(|a| a.effect.label())
                .unwrap_or_default(),
            needs_target: game.ability_target_spec(source, ability) != TargetSpec::None,
            targets: targets(source, Some(ability)),
            modal: None,
            sacrifice_choices: game.sacrifice_candidates(source, ability),
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        MeaningfulAction::Cycle { card } => ActionView {
            id: action.id,
            kind: "cycle".to_string(),
            object: Some(card),
            ability_index: None,
            section: "hand".to_string(),
            label: format!("Cycle: {}", game.def_of(card).name),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        MeaningfulAction::ActivateHandAbility { card } => ActionView {
            id: action.id,
            kind: "activate_hand_ability".to_string(),
            object: Some(card),
            ability_index: None,
            section: "hand".to_string(),
            label: format!("Discard: {}", game.def_of(card).name),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        // The label must not leak the hidden card's identity (CR 708.2) — a plain "Cast face down".
        MeaningfulAction::CastFaceDown { card } => ActionView {
            id: action.id,
            kind: "cast_face_down".to_string(),
            object: Some(card),
            ability_index: None,
            section: "hand".to_string(),
            label: "Cast face down".to_string(),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        MeaningfulAction::Suspend { card } => ActionView {
            id: action.id,
            kind: "suspend".to_string(),
            object: Some(card),
            ability_index: None,
            section: "hand".to_string(),
            label: format!("Suspend: {}", game.def_of(card).name),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        MeaningfulAction::Encore { card } => ActionView {
            id: action.id,
            kind: "encore".to_string(),
            object: Some(card),
            ability_index: None,
            section: "graveyard".to_string(),
            label: format!("Encore: {}", game.def_of(card).name),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        // The label must not leak the hidden card's identity (CR 708.2) — a plain "Turn face up".
        MeaningfulAction::TurnFaceUp { permanent } => ActionView {
            id: action.id,
            kind: "turn_face_up".to_string(),
            object: Some(permanent),
            ability_index: None,
            section: "battlefield".to_string(),
            label: "Turn face up".to_string(),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
        MeaningfulAction::CastPrepared { source } => {
            let back = game
                .def_of(source)
                .back
                .expect("CastPrepared implies a back face");
            let back_def = *back;
            let (spec, legal) = game.prepared_cast_targets(source);
            ActionView {
                id: action.id,
                kind: "cast_prepared".to_string(),
                object: Some(source),
                ability_index: None,
                section: "battlefield".to_string(),
                label: back_def.name.to_string(),
                needs_target: spec != TargetSpec::None,
                targets: legal.into_iter().map(WireTarget::of).collect(),
                modal: None,
                sacrifice_choices: None,
                discard_choices: None,
                discard_count: 0,
                graveyard_exile_choices: None,
                graveyard_exile_min: 0,
                graveyard_exile_max: 0,
                // The back face is what you cast — never the front permanent's mana cost.
                has_x: back_def.cost.x > 0,
                auto_tap: Vec::new(),
                required_attacks: Vec::new(),
            }
        }
        MeaningfulAction::DeclareAttackers => ActionView {
            id: action.id,
            kind: "declare_attackers".to_string(),
            object: None,
            ability_index: None,
            section: "combat".to_string(),
            label: "Declare attackers".to_string(),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: game
                .required_attacks(action.player)
                .into_iter()
                .map(|(attacker, defender)| WireAttack {
                    attacker,
                    defender: defender.0,
                })
                .collect(),
        },
        MeaningfulAction::DeclareBlockers => ActionView {
            id: action.id,
            kind: "declare_blockers".to_string(),
            object: None,
            ability_index: None,
            section: "combat".to_string(),
            label: "Declare blockers".to_string(),
            needs_target: false,
            targets: Vec::new(),
            modal: None,
            sacrifice_choices: None,
            discard_choices: None,
            discard_count: 0,
            graveyard_exile_choices: None,
            graveyard_exile_min: 0,
            graveyard_exile_max: 0,
            has_x: false,
            auto_tap: Vec::new(),
            required_attacks: Vec::new(),
        },
    };
    view.auto_tap = game.auto_tap_objects(action);
    view
}

/// Redacted board projection only — yield / hold / usernames stay at their incomplete defaults
/// until [`complete_visible`] stamps them. Not a public wire entry point.
fn project_board(game: &engine::Game, viewer: Option<engine::PlayerId>) -> VisibleState {
    use engine::{PlayerId, TargetSpec, Zone};

    let live = game.live_object_ids();

    let players = (0..game.player_count() as u8)
        .map(|p| {
            let pid = PlayerId(p);
            let hand_count = live
                .iter()
                .filter(|&&id| game.zone_of(id) == Zone::Hand && game.owner_of(id) == pid)
                .count() as u32;
            PlayerView {
                player: p,
                username: String::new(),
                life: game.life(pid),
                commander_tax: game.commander_tax(pid),
                lost: game.has_lost(pid),
                hand_count,
                library_count: game.library_size(pid) as u32,
                mana_pool: WireManaPool::from_engine(game.mana_pool(pid)),
                commander_damage: game
                    .commander_damage(pid)
                    .iter()
                    .map(|&(from, amount)| CommanderDamageView {
                        from: from.0,
                        amount,
                    })
                    .collect(),
            }
        })
        .collect();

    let objects = live
        .iter()
        .copied()
        .filter(|&id| {
            // A hand card is private to its owner; libraries are never itemized.
            match game.zone_of(id) {
                Zone::Library => false,
                Zone::Hand => viewer == Some(game.owner_of(id)),
                _ => true,
            }
        })
        .map(|id| {
            let def = game.def_of(id);
            // CR 708.2: a face-down permanent (a manifest) is anonymized — its real name, card
            // kind, and mana cost are hidden from every viewer (the engine already reports its
            // 2/2 P/T, creature type, and empty keywords). The client renders it as a card back.
            let manifest_face_down = game.is_face_down(id);
            // CR 701.9: a face-down exile-pile card (Abstract Performance's first pile) is
            // anonymized for every viewer but its owner while it awaits the opponent's pick —
            // same anonymization shape as a manifest, but per-viewer rather than hidden from
            // everyone (the owner *does* know their own exiled cards).
            // ponytail: reuses the manifest's 2/2-creature placeholder kind/cost rather than a
            // dedicated "hidden pile card" shape — harmless, since the client only ever branches
            // on `face_down` (a card back) and never reads `kind`/`mana_cost` while it's set.
            let hidden_pile_card = game.is_card_face_down(id) && viewer != Some(game.owner_of(id));
            let face_down = manifest_face_down || hidden_pile_card;
            ObjectView {
                id,
                zone: game.zone_of(id) as u8,
                owner: game.owner_of(id).0,
                controller: game.controller_of(id).0,
                card_id: if face_down {
                    String::new()
                } else {
                    def.id.to_string()
                },
                name: if face_down {
                    String::new()
                } else {
                    def.name.to_string()
                },
                print: if face_down {
                    String::new()
                } else {
                    def.default_print.to_string()
                },
                kind: if face_down {
                    WireKind::Creature {
                        power: 2,
                        toughness: 2,
                    }
                } else {
                    wire_kind(def)
                },
                mana_cost: if face_down {
                    wire_cost(engine::Cost::FREE)
                } else {
                    wire_cost(def.cost)
                },
                needs_target: game.target_spec_of(id) != TargetSpec::None,
                tapped: game.is_tapped(id),
                summoning_sick: game.is_summoning_sick(id),
                has_haste: game.has_haste(id),
                keywords: {
                    let mut out = Vec::new();
                    for keyword in game.effective_keywords(id) {
                        let label = crate::catalog::wire_keyword(keyword);
                        if !out.contains(&label) {
                            out.push(label);
                        }
                    }
                    out
                },
                power: game.power(id),
                toughness: game.toughness(id),
                loyalty: game.loyalty(id),
                plus_counters: game.plus_counters(id),
                marked_damage: game.marked_damage(id),
                is_commander: game.is_commander(id),
                goaded: game.is_goaded(id),
                taps_for_mana: game.taps_for_mana(id),
                prepared: game.prepared(id),
                phased_out: game.is_phased_out(id),
                face_down,
                attached_to: game.attached_to(id),
                modifiers: game
                    .modifier_sources(id)
                    .into_iter()
                    .map(|group| ModifierSourceView {
                        source_name: group.source_name.to_string(),
                        source_card_id: group.source_card_id.to_string(),
                        contributions: group
                            .contributions
                            .into_iter()
                            .map(format_modifier_contribution)
                            .collect(),
                    })
                    .collect(),
            }
        })
        .collect();

    let stack = game
        .stack()
        .into_iter()
        .map(|entry| match entry {
            engine::StackEntry::Spell(id) => StackObjectView {
                kind: "spell".to_string(),
                source: id,
                controller: game.controller_of(id).0,
                label: game.def_of(id).name.to_string(),
                target: game.spell_target(id).map(WireTarget::of),
            },
            engine::StackEntry::Ability {
                controller,
                source,
                effect,
                target,
            } => StackObjectView {
                kind: "ability".to_string(),
                source,
                controller: controller.0,
                label: effect.label(),
                target: target.map(WireTarget::of),
            },
        })
        .collect();

    let pending_choice = game
        .pending_choice()
        .map(|pc| project_pending_choice(game, viewer, pc));

    let combat = CombatView {
        attackers: game
            .attack_targets()
            .into_iter()
            .map(|(attacker, defender)| WireAttack {
                attacker,
                defender: defender.0,
            })
            .collect(),
        blocks: game
            .blocks()
            .into_iter()
            .map(|(blocker, attacker)| WireBlock { blocker, attacker })
            .collect(),
        attackers_declared: game.attackers_declared(),
        blockers_declared: game.blockers_declared().into_iter().map(|p| p.0).collect(),
    };

    // Only the viewer's own actions — an opponent's are never listed, and a spectator (no seat)
    // matches no stored action's `player` and so always gets an empty list.
    let actions = game
        .legal_actions()
        .iter()
        .filter(|a| Some(a.player) == viewer)
        .map(|a| action_view(game, a))
        .collect();

    VisibleState {
        viewer: viewer.map_or(SPECTATOR_VIEWER, |v| v.0),
        active_player: game.active_player().0,
        step: game.current_step() as u8,
        priority: game.priority_holder().0,
        players,
        objects,
        stack,
        combat,
        can_act: game.has_meaningful_action(game.priority_holder()),
        // Table policy — stamped by `complete_visible` from `ViewExtras`.
        yielded: false,
        turn_yielded: false,
        stack_hold_remaining_ms: 0,
        pending_choice,
        actions,
    }
}

/// One event of the SSE stream: the opening snapshot, then a delta per change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum StreamFrame {
    Snapshot {
        seq: u64,
        state: VisibleState,
    },
    Delta(DeltaEnvelope),
    /// A periodic liveness ping (server emits one every few seconds) so the client can tell an
    /// idle-but-alive game from a silently-dropped connection and time out on the *absence* of any
    /// frame. Carries no game state — the client filters it out before it reaches the store.
    Heartbeat,
}

#[cfg(test)]
mod tests {
    use crate::dto::{CommanderDamageView, PendingChoiceView, WireKind};
    use crate::intent::{WireAttack, WireTarget};
    use crate::test_support::{def, pass_until_choice, refresh_via_mana_tap, resolve_top_of_stack};
    use engine::{Game, ObjectId, PlayerId};

    use super::{SPECTATOR_VIEWER, ViewExtras, complete_visible};

    /// Board-only fixture: empty Table extras. Production wire paths must pass real extras.
    fn snapshot(game: &Game, viewer: PlayerId) -> crate::dto::VisibleState {
        complete_visible(game, Some(viewer), &ViewExtras::default())
    }

    fn spectator_snapshot(game: &Game) -> crate::dto::VisibleState {
        complete_visible(game, None, &ViewExtras::default())
    }

    #[test]
    fn complete_visible_stamps_yield_hold_and_usernames_in_one_pass() {
        let game = Game::new();
        let extras = ViewExtras {
            yields: [false, true, false, false],
            turn_yields: [true, false, false, false],
            stack_hold_remaining_ms: 1500,
            usernames: ["alice".into(), "bob".into(), String::new(), String::new()],
            prints: Default::default(),
        };

        let seated = complete_visible(&game, Some(PlayerId(1)), &extras);
        assert!(seated.yielded, "P1's yield flag comes from extras");
        assert!(!seated.turn_yielded, "P1's turn yield is false in extras");
        let p0 = complete_visible(&game, Some(PlayerId(0)), &extras);
        assert!(p0.turn_yielded, "P0's turn yield flag comes from extras");
        assert_eq!(seated.stack_hold_remaining_ms, 1500);
        assert_eq!(seated.players[0].username, "alice");
        assert_eq!(seated.players[1].username, "bob");

        let spectating = complete_visible(&game, None, &extras);
        assert!(
            !spectating.yielded,
            "a spectator never yields even when a seat has"
        );
        assert!(
            !spectating.turn_yielded,
            "a spectator never turn-yields even when a seat has"
        );
        assert_eq!(spectating.stack_hold_remaining_ms, 1500);
        assert_eq!(spectating.players[0].username, "alice");
    }

    #[test]
    fn complete_visible_overlays_seat_prints_and_skips_empty_values() {
        let mut game = Game::new();
        let p0 = PlayerId(0);
        let shock = game.spawn_in_hand(p0, def("Shock"));
        let shock_id = game.def_of(shock).id.to_string();
        let default_print = game.def_of(shock).default_print.to_string();

        let mut prints: [std::collections::HashMap<String, String>; 4] = Default::default();
        prints[0].insert(
            shock_id.clone(),
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
        );
        prints[0].insert("unused".into(), String::new());

        let extras = ViewExtras {
            prints,
            ..ViewExtras::default()
        };
        let snap = complete_visible(&game, Some(p0), &extras);
        let obj = snap.objects.iter().find(|o| o.id == shock).expect("shock");
        assert_eq!(obj.print, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
        assert_ne!(obj.print, default_print);

        // An empty map value must not clobber the CardDef default print.
        let mut empty_override = ViewExtras::default();
        empty_override.prints[0].insert(shock_id, String::new());
        let snap2 = complete_visible(&game, Some(p0), &empty_override);
        let obj2 = snap2.objects.iter().find(|o| o.id == shock).expect("shock");
        assert_eq!(obj2.print, default_print);
    }

    #[test]
    fn complete_visible_overlays_seat_prints_onto_library_search_items() {
        // Library cards never appear in `objects`, so ChoiceItem.print is the only art path —
        // deck-chosen Printings must overlay there too (ADR 0031).
        let mut game = Game::new();
        let p0 = PlayerId(0);
        game.fund_mana(p0);
        game.stack_library(p0, &[def("Forest"), def("Grizzly Bear"), def("Island")]);
        let tutor = game.spawn_in_hand(p0, def("Diabolic Tutor"));
        game.submit(engine::Intent::Cast {
            player: p0,
            object: tutor,
            target: None,
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .unwrap();
        resolve_top_of_stack(&mut game);

        let forest_id = def("Forest").id.to_string();
        let preferred = "ffffffff-1111-2222-3333-444444444444";
        let mut prints: [std::collections::HashMap<String, String>; 4] = Default::default();
        prints[0].insert(forest_id, preferred.into());
        let extras = ViewExtras {
            prints,
            ..ViewExtras::default()
        };

        let snap = complete_visible(&game, Some(p0), &extras);
        match snap.pending_choice {
            Some(PendingChoiceView::SearchLibrary { items, .. }) => {
                let forest = items
                    .iter()
                    .find(|it| it.label == "Forest")
                    .expect("Forest among matches");
                assert_eq!(forest.print, preferred);
                assert!(
                    items.iter().all(|it| !it.print.is_empty()),
                    "every library-search item carries a print"
                );
            }
            other => panic!("expected SearchLibrary, got {other:?}"),
        }
    }

    #[test]
    fn a_snapshot_hides_opponent_hands_and_libraries_but_shows_own_hand() {
        let mut game = Game::new();
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let mine = game.spawn_in_hand(p0, def("Shock"));
        let theirs = game.spawn_in_hand(p1, def("Grizzly Bear"));
        let bear = game.spawn_on_battlefield(p0, def("Grizzly Bear"));
        game.stack_library(p1, &[def("Forest"), def("Forest"), def("Forest")]);

        let snap = snapshot(&game, p0);

        assert!(
            snap.objects
                .iter()
                .any(|o| o.id == mine && o.name == "Shock"),
            "the viewer sees their own hand card by name",
        );
        assert!(
            !snap.objects.iter().any(|o| o.id == theirs),
            "an opponent's hand card is not itemized",
        );
        assert!(
            snap.objects.iter().any(|o| o.id == bear),
            "a battlefield permanent is public",
        );
        assert_eq!(snap.players[0].hand_count, 1, "own hand counted");
        assert_eq!(
            snap.players[1].hand_count, 1,
            "opponent hand counted, not shown"
        );
        assert_eq!(
            snap.players[1].library_count, 3,
            "libraries are counts only"
        );
    }

    #[test]
    fn a_spectator_snapshot_hides_every_hand_and_library() {
        // 6.3 hard rule: a spectator (no seat) sees the public board but *no* player's hand or
        // library — not even one card id. The projection must be provably hand-private.
        let mut game = Game::new();
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        let mine = game.spawn_in_hand(p0, def("Shock"));
        let theirs = game.spawn_in_hand(p1, def("Grizzly Bear"));
        let bear = game.spawn_on_battlefield(p0, def("Grizzly Bear"));
        game.stack_library(p1, &[def("Forest"), def("Forest")]);

        let spec = spectator_snapshot(&game);

        assert_eq!(spec.viewer, SPECTATOR_VIEWER, "no seat");
        assert!(
            !spec.objects.iter().any(|o| o.id == mine || o.id == theirs),
            "no hand card is itemized to a spectator",
        );
        assert!(
            spec.objects.iter().any(|o| o.id == bear),
            "battlefield permanents are public",
        );
        assert!(
            spec.objects
                .iter()
                .all(|o| o.zone != engine::Zone::Library as u8),
            "libraries are never itemized",
        );
        // Counts are public; identities are not.
        assert_eq!(spec.players[0].hand_count, 1);
        assert_eq!(spec.players[1].hand_count, 1);
        assert_eq!(spec.players[1].library_count, 2);
    }

    #[test]
    fn a_snapshot_cost_carries_the_x_marker() {
        let mut game = Game::new();
        let insight = game.spawn_in_hand(PlayerId(0), def("Commander's Insight"));
        let shock = game.spawn_in_hand(PlayerId(0), def("Shock"));

        let snap = snapshot(&game, PlayerId(0));
        let cost_of = |id| snap.objects.iter().find(|o| o.id == id).unwrap().mana_cost;
        assert!(
            cost_of(insight).has_x,
            "Commander's Insight is {{X}}{{U}}{{U}}{{U}} — the client must prompt for X"
        );
        assert!(!cost_of(shock).has_x);
    }

    #[test]
    fn a_snapshot_carries_card_kind_and_target_need() {
        let mut game = Game::new();
        let shock = game.spawn_in_hand(PlayerId(0), def("Shock"));
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));

        let snap = snapshot(&game, PlayerId(0));
        let shock_view = snap.objects.iter().find(|o| o.id == shock).unwrap();
        assert_eq!(shock_view.kind, WireKind::Instant);
        assert!(shock_view.needs_target, "Shock targets a creature");

        let bear_view = snap.objects.iter().find(|o| o.id == bear).unwrap();
        assert_eq!(
            bear_view.kind,
            WireKind::Creature {
                power: 2,
                toughness: 2,
            },
        );
        assert!(!bear_view.needs_target);
    }

    /// A targeted action carries the targets that are legal *right now*, straight from
    /// `Game::legal_targets` — the client picks from this list rather than reimplementing
    /// `TargetSpec`. Shock is "any target", so its list spans both `WireTarget` variants: every
    /// battlefield creature and every living player.

    #[test]
    fn a_targeted_action_carries_its_legal_targets() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let shock = game.spawn_in_hand(PlayerId(0), def("Shock"));
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let cast = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(shock))
            .expect("Shock is castable");

        assert!(cast.needs_target);
        assert!(
            cast.targets.contains(&WireTarget::Object { id: bear }),
            "a battlefield creature is a legal target for Shock; got {:?}",
            cast.targets,
        );
        assert!(
            cast.targets.contains(&WireTarget::Player { player: 0 }),
            "Shock is 'any target', so a player is legal too — the client cannot know this \
                 without being told; got {:?}",
            cast.targets,
        );
    }

    /// Commander damage is a Commander win condition (21 from one commander, CR 903.10a) that the
    /// client had no way to see: it lives only in the engine's player state and the `lost` flag it
    /// eventually flips. Surface the running tally, keyed by the commander's owner.

    #[test]
    fn a_snapshot_carries_commander_damage_taken() {
        let mut game = Game::new();
        let bear = game.spawn_on_battlefield(PlayerId(1), def("Grizzly Bear"));
        game.designate_commander(PlayerId(1), def("Grizzly Bear"));

        let snap = snapshot(&game, PlayerId(0));
        let me = snap.players.iter().find(|p| p.player == 0).unwrap();
        assert!(
            me.commander_damage.is_empty(),
            "nothing has connected yet; got {:?}",
            me.commander_damage,
        );

        game.deal_commander_damage(bear, PlayerId(0), 7);

        let snap = snapshot(&game, PlayerId(0));
        let me = snap.players.iter().find(|p| p.player == 0).unwrap();
        assert_eq!(
            me.commander_damage,
            vec![CommanderDamageView { from: 1, amount: 7 }],
            "P0 has taken 7 from P1's commander",
        );
    }

    /// A modal spell's targets travel per mode (CR 700.2), so the cast action itself reports no
    /// target — the client has to be handed the printed modes and each one's legal targets, or it
    /// can only fire `modes: []` and be rejected.

    #[test]
    fn a_modal_cast_action_carries_its_modes() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let command = game.spawn_in_hand(PlayerId(0), def("Prismari Command"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let cast = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(command))
            .expect("Prismari Command is castable");

        assert!(!cast.needs_target, "a modal spell's targets ride its modes");
        assert!(cast.targets.is_empty());

        let modal = cast.modal.as_ref().expect("Prismari Command is modal");
        assert_eq!((modal.choose, modal.choose_max), (2, 2), "choose two");
        assert_eq!(modal.modes.len(), 4);

        // Mode 0 — "deals 2 damage to any target": a creature or a player.
        assert!(modal.modes[0].needs_target);
        assert!(
            modal.modes[0]
                .targets
                .contains(&WireTarget::Object { id: bear })
        );
        assert!(
            modal.modes[0]
                .targets
                .contains(&WireTarget::Player { player: 0 })
        );

        // Mode 2 — "target player creates a Treasure": needs a Player target.
        assert!(modal.modes[2].needs_target);
        assert!(
            modal.modes[2]
                .targets
                .contains(&WireTarget::Player { player: 0 })
        );

        // Mode 3 — wants a noncreature permanent; none is on the battlefield, so it wants a target
        // and has none. A picker must not offer it.
        assert!(modal.modes[3].needs_target);
        assert!(modal.modes[3].targets.is_empty());
    }

    /// An "Sacrifice a creature: …" ability can't be activated at all until the client names which
    /// creature pays — so the action has to carry the candidates.

    #[test]
    fn an_activate_action_carries_its_sacrifice_candidates() {
        let mut game = Game::new();
        let seer = game.spawn_on_battlefield(PlayerId(0), def("Viscera Seer"));
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Forest"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let activate = snap
            .actions
            .iter()
            .find(|a| a.kind == "activate" && a.object == Some(seer))
            .expect("the Seer's sacrifice ability is offered");

        let mut choices = activate
            .sacrifice_choices
            .clone()
            .expect("its cost sacrifices a creature");
        choices.sort_unstable();
        let mut expected = vec![seer, bear];
        expected.sort_unstable();
        assert_eq!(choices, expected);
    }

    #[test]
    fn a_non_modal_cast_action_has_no_modal_block() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let shock = game.spawn_in_hand(PlayerId(0), def("Shock"));
        game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let cast = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(shock))
            .expect("Shock is castable");
        assert!(cast.modal.is_none());
    }

    /// An untargeted action carries an empty target list, so `targets` never reads as "we just
    /// didn't compute them."

    #[test]
    fn an_untargeted_action_carries_no_targets() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let hand_land = game.spawn_in_hand(PlayerId(0), def("Forest"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Forest"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let play = snap
            .actions
            .iter()
            .find(|a| a.kind == "play_land" && a.object == Some(hand_land))
            .expect("the hand land is playable");
        assert!(!play.needs_target);
        assert!(play.targets.is_empty());
    }

    #[test]
    fn a_seated_viewers_snapshot_lists_only_their_own_actions() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let commander = game.designate_commander(PlayerId(0), def("Grizzly Bear"));
        let hand_land = game.spawn_in_hand(PlayerId(0), def("Forest"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Forest"));
        refresh_via_mana_tap(&mut game, tapland);

        let mine = snapshot(&game, PlayerId(0));
        assert!(
            mine.actions.iter().any(|a| a.kind == "play_land"
                && a.object == Some(hand_land)
                && a.section == "hand"
                && a.label == "Forest"
                && !a.needs_target),
            "the hand land is a play_land action from the hand section; got {:?}",
            mine.actions,
        );
        assert!(
            mine.actions.iter().any(|a| a.kind == "cast"
                && a.object == Some(commander)
                && a.section == "command"
                && a.label == "Grizzly Bear"
                && !a.needs_target),
            "the castable commander is a cast action from the command section; got {:?}",
            mine.actions,
        );
        let ids: std::collections::HashSet<u64> = mine.actions.iter().map(|a| a.id).collect();
        assert_eq!(
            ids.len(),
            mine.actions.len(),
            "every action has a distinct id"
        );

        let theirs = snapshot(&game, PlayerId(1));
        assert!(
            theirs.actions.is_empty(),
            "an opponent's snapshot carries none of player 0's actions; got {:?}",
            theirs.actions,
        );

        let spectating = spectator_snapshot(&game);
        assert!(
            spectating.actions.is_empty(),
            "a spectator has no seat, so no actions",
        );
    }

    #[test]
    fn a_discard_cost_cast_action_lists_hand_choices() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let spell = game.spawn_in_hand(PlayerId(0), def("Big Score"));
        let pitch = game.spawn_in_hand(PlayerId(0), def("Mountain"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let action = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(spell))
            .expect("Big Score is a listed cast");
        assert_eq!(action.discard_count, 1);
        assert_eq!(action.discard_choices.as_deref(), Some([pitch].as_slice()));
    }

    #[test]
    fn a_cast_action_lists_auto_tap_permanents() {
        let mut game = Game::new();
        let spell = game.spawn_in_hand(PlayerId(0), def("Grizzly Bear")); // {1}{G}
        let f1 = game.spawn_on_battlefield(PlayerId(0), def("Forest"));
        let f2 = game.spawn_on_battlefield(PlayerId(0), def("Forest"));
        let f3 = game.spawn_on_battlefield(PlayerId(0), def("Forest"));
        let ring = game.spawn_on_battlefield(PlayerId(0), def("Sol Ring"));
        // Tap f3 to refresh the action list; floating {G} pays the green pip.
        refresh_via_mana_tap(&mut game, f3);

        let snap = snapshot(&game, PlayerId(0));
        let action = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(spell))
            .expect("Grizzly Bear is a listed cast");
        assert_eq!(
            action.auto_tap.len(),
            1,
            "one more forest covers the remaining {{1}}; got {:?}",
            action.auto_tap
        );
        assert!(
            action.auto_tap.contains(&f1) || action.auto_tap.contains(&f2),
            "auto_tap names a forest; got {:?}",
            action.auto_tap
        );
        assert!(
            !action.auto_tap.contains(&ring),
            "lands are preferred over Sol Ring"
        );
    }

    #[test]
    fn a_delve_cast_action_previews_auto_tap_with_graveyard_exile() {
        // Treasure Cruise is {7}{U}. With 6 GY cards, max delve leaves {1}{U}. Floating {U}
        // from the refresh tap pays the pip; auto_tap must still name the remaining island for
        // {1}. A delve=0 preview would fail the plan and emit [] while the cast stays listed.
        let mut game = Game::new();
        let cruise = game.spawn_in_hand(PlayerId(0), def("Treasure Cruise"));
        for _ in 0..6 {
            game.spawn_in_graveyard(PlayerId(0), def("Island"));
        }
        let island = game.spawn_on_battlefield(PlayerId(0), def("Island"));
        let filler = game.spawn_on_battlefield(PlayerId(0), def("Island"));
        refresh_via_mana_tap(&mut game, filler);

        let snap = snapshot(&game, PlayerId(0));
        let action = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(cruise))
            .expect("delve makes Cruise listable");
        assert_eq!(
            action.auto_tap,
            vec![island],
            "max delve leaves {{1}}; the untapped island covers it"
        );
    }

    #[test]
    fn an_escape_cast_action_lists_graveyard_exile_picks() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let strider = game.spawn_in_graveyard(PlayerId(0), def("Woe Strider"));
        let fodder: Vec<ObjectId> = (0..4)
            .map(|_| game.spawn_in_graveyard(PlayerId(0), def("Forest")))
            .collect();
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Swamp"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let action = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(strider))
            .expect("escape cast is listed");
        assert_eq!(action.section, "graveyard");
        assert_eq!(action.graveyard_exile_min, 4);
        assert_eq!(action.graveyard_exile_max, 4);
        let mut choices = action.graveyard_exile_choices.clone().unwrap();
        choices.sort();
        let mut expected = fodder;
        expected.sort();
        assert_eq!(choices, expected);
    }

    #[test]
    fn a_cycle_action_is_listed_from_the_hand() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let massif = game.spawn_in_hand(PlayerId(0), def("Glittering Massif"));
        let other = game.spawn_in_hand(PlayerId(0), def("Mountain"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        refresh_via_mana_tap(&mut game, tapland);
        // Spend the land drop on the Mountain so Cycle is what the Massif offers.
        let land_id = game
            .legal_actions()
            .iter()
            .find(|a| {
                matches!(
                    a.kind,
                    engine::MeaningfulAction::PlayLand { card, .. } if card == other
                )
            })
            .unwrap()
            .id;
        game.submit(engine::Intent::TakeAction {
            player: PlayerId(0),
            id: land_id,
            target: None,
            x: 0,
            modes: vec![],
            sacrifice: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            attackers: vec![],
            blocks: vec![],
        })
        .unwrap();

        let snap = snapshot(&game, PlayerId(0));
        let cycle = snap
            .actions
            .iter()
            .find(|a| a.kind == "cycle" && a.object == Some(massif))
            .expect("cycling land lists a cycle action");
        assert_eq!(cycle.section, "hand");
        assert!(cycle.label.starts_with("Cycle:"));
        assert!(!cycle.needs_target);
    }

    #[test]
    fn a_delve_cast_action_caps_exile_at_graveyard_size_not_generic() {
        // Treasure Cruise is {7}{U}; CR 702.66 lets you exile any number from the GY, even past
        // the generic you can reduce. The picker max must be the GY size so a client never
        // auto-skips when listability needed more delve than the printed generic.
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let cruise = game.spawn_in_hand(PlayerId(0), def("Treasure Cruise"));
        for _ in 0..10 {
            game.spawn_in_graveyard(PlayerId(0), def("Forest"));
        }
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Island"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let action = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast" && a.object == Some(cruise))
            .expect("Treasure Cruise is listed");
        assert_eq!(action.graveyard_exile_min, 0);
        assert_eq!(
            action.graveyard_exile_max, 10,
            "delve max is the full GY, not the printed generic of 7"
        );
        assert_eq!(
            action.graveyard_exile_choices.as_ref().map(|c| c.len()),
            Some(10)
        );
    }

    #[test]
    fn a_cast_prepared_action_lists_the_back_face() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        game.stack_library(PlayerId(0), &[def("Forest"), def("Forest")]);
        let kirol = game.spawn_on_battlefield(PlayerId(0), def("Kirol, History Buff"));
        // Reanimate a bear out of the GY to fire Kirol's prepare trigger.
        let corpse = game.spawn_in_graveyard(PlayerId(0), def("Grizzly Bear"));
        let reanimate = game.spawn_in_hand(PlayerId(0), def("Reanimate"));
        game.submit(engine::Intent::Cast {
            player: PlayerId(0),
            object: reanimate,
            target: Some(engine::Target::Object(corpse)),
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .unwrap();
        resolve_top_of_stack(&mut game);
        resolve_top_of_stack(&mut game);
        assert!(game.prepared(kirol));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Mountain"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let action = snap
            .actions
            .iter()
            .find(|a| a.kind == "cast_prepared" && a.object == Some(kirol))
            .expect("prepared Kirol lists cast_prepared");
        assert_eq!(action.section, "battlefield");
        assert_eq!(action.label, "Pack a Punch");
        assert!(action.needs_target);
        assert!(
            !action.has_x,
            "Pack a Punch has no {{X}}; has_x comes from the back face, not the front"
        );
        assert!(
            action
                .targets
                .iter()
                .any(|t| matches!(t, WireTarget::Object { id } if *id == game.current_id(corpse))),
            "legal targets include the reanimated bear"
        );
    }

    #[test]
    fn an_activate_actions_label_and_target_need_come_from_its_ability() {
        // Bonesplitter's ability 0 is its static "grant +2/+0 while attached"; ability 1 is the
        // targeted Equip {1} — the one that should show up as an activate action.
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let equipment = game.spawn_on_battlefield(PlayerId(0), def("Bonesplitter"));
        game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let tapland = game.spawn_on_battlefield(PlayerId(0), def("Forest"));
        refresh_via_mana_tap(&mut game, tapland);

        let snap = snapshot(&game, PlayerId(0));
        let equip = snap
            .actions
            .iter()
            .find(|a| a.kind == "activate" && a.object == Some(equipment))
            .expect("Equip is a listed activate action");
        assert_eq!(equip.ability_index, Some(1));
        assert_eq!(equip.section, "battlefield");
        assert_eq!(equip.label, "Equip");
        assert!(
            equip.needs_target,
            "Equip targets the creature to attach to"
        );
    }

    /// A test-only enchantment with two independent abilities sharing one trigger tag ("a
    /// creature you control dies") — exercises the wire `OrderTriggers` choice, which needs a
    /// genuine two-simultaneous-triggers source. Moldervine Reclamation used to be this fixture,
    /// but #97 made it faithful (one `Effect::Sequence` trigger, not two), so it no longer raises
    /// an ordering choice.
    fn two_simultaneous_death_triggers() -> engine::CardDef {
        use engine::*;
        const ABILITIES: [Ability; 2] = [
            Ability {
                timing: Timing::Triggered(Trigger::CreatureYouControlDies),
                effect: Effect::GainLife {
                    amount: Amount::Fixed(1),
                },
                optional: false,
                min_level: 0,
                once_each_turn: false,
                condition: None,
                cost: Cost::FREE,
            },
            Ability {
                timing: Timing::Triggered(Trigger::CreatureYouControlDies),
                effect: Effect::DrawCards {
                    count: Amount::Fixed(1),
                },
                optional: false,
                min_level: 0,
                once_each_turn: false,
                condition: None,
                cost: Cost::FREE,
            },
        ];
        CardDef {
            name: "Two Death Triggers (test)",
            id: "",
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Enchantment,
            legendary: false,
            uncounterable: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            identity_pips: &[],
            colors: &[],
            enters_tapped: false,
            enters_tapped_unless: None,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            keywords: &[],
            conditional_keywords: &[],
            abilities: &ABILITIES,
            cycling: None,
            flashback: None,
            echo: None,
            bestow: None,
            morph: None,
            evoke: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            enchant: None,
            enchant_graveyard: false,
            back: None,
            adventure: None,
            suspend: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: None,
            may_choose_not_to_untap: false,
        }
    }

    #[test]
    fn order_triggers_view_carries_a_label_per_effect() {
        // A source with two independent abilities sharing one death trigger fires both off one
        // death: "gain 1 life" and "draw a card". Its controller must order them, so the wire
        // choice needs one label each.
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        game.spawn_on_battlefield(PlayerId(0), two_simultaneous_death_triggers());
        let victim = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let shock = game.spawn_in_hand(PlayerId(0), def("Shock"));

        game.submit(engine::Intent::Cast {
            player: PlayerId(0),
            object: shock,
            target: Some(engine::Target::Object(victim)),
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .unwrap();
        // Shock resolves, lethal damage kills the Bear via SBA, and the two abilities
        // queue up as one ordering choice — all before priority is handed back out.
        game.submit(engine::Intent::PassPriority {
            player: game.priority_holder(),
        })
        .unwrap();
        game.submit(engine::Intent::PassPriority {
            player: game.priority_holder(),
        })
        .unwrap();

        let snap = snapshot(&game, PlayerId(0));
        match snap.pending_choice {
            Some(PendingChoiceView::OrderTriggers { count, labels, .. }) => {
                assert_eq!(count, 2);
                assert_eq!(labels, vec!["Gain 1 life", "Draw 1"]);
            }
            other => panic!("expected an OrderTriggers choice, got {other:?}"),
        }
    }

    #[test]
    fn a_snapshot_carries_haste_so_the_client_can_let_it_attack_while_sick() {
        // Goblin Guide has haste; Grizzly Bear does not. The client combines has_haste with
        // summoning_sick to decide what can attack (it can't infer haste otherwise).
        let mut game = Game::new();
        let hasty = game.spawn_on_battlefield(PlayerId(0), def("Goblin Guide"));
        let vanilla = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));

        let snap = snapshot(&game, PlayerId(0));
        assert!(
            snap.objects
                .iter()
                .find(|o| o.id == hasty)
                .unwrap()
                .has_haste,
            "Goblin Guide reports haste on the wire",
        );
        assert!(
            !snap
                .objects
                .iter()
                .find(|o| o.id == vanilla)
                .unwrap()
                .has_haste,
            "Grizzly Bear has no haste",
        );
    }

    #[test]
    fn a_snapshot_carries_effective_keywords_for_arena_style_badges() {
        // Base keywords (Serra Angel) and granted keywords (Lightning Greaves → host) both land
        // on ObjectView so the canvas can paint Arena-style ability badges without a catalog lookup.
        let mut game = Game::new();
        let angel = game.spawn_on_battlefield(PlayerId(0), def("Serra Angel"));
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let greaves = game.spawn_on_battlefield(PlayerId(0), def("Lightning Greaves"));
        game.submit(engine::Intent::ActivateAbility {
            player: PlayerId(0),
            object: greaves,
            ability_index: 1,
            target: Some(engine::Target::Object(bear)),
            sacrifice: vec![],
            x: 0,
        })
        .expect("Equip {0}");
        resolve_top_of_stack(&mut game);

        let snap = snapshot(&game, PlayerId(0));
        let angel_kw = &snap
            .objects
            .iter()
            .find(|o| o.id == angel)
            .unwrap()
            .keywords;
        assert!(
            angel_kw.iter().any(|k| k == "flying"),
            "Serra Angel reports flying: {angel_kw:?}"
        );
        assert!(
            angel_kw.iter().any(|k| k == "vigilance"),
            "Serra Angel reports vigilance: {angel_kw:?}"
        );

        let bear_kw = &snap.objects.iter().find(|o| o.id == bear).unwrap().keywords;
        assert!(
            bear_kw.iter().any(|k| k == "haste"),
            "equipped Bear gains haste: {bear_kw:?}"
        );
        assert!(
            bear_kw.iter().any(|k| k == "shroud"),
            "equipped Bear gains shroud: {bear_kw:?}"
        );
    }

    #[test]
    fn a_snapshot_carries_parametrized_keywords_as_stable_wire_ids() {
        // Ward {N} and protection from a color must round-trip as `ward:N` / `protection:color`
        // — the canvas matches those prefixes for badges.
        let mut game = Game::new();
        let guard = game.spawn_on_battlefield(PlayerId(0), def("Tomakul Honor Guard"));
        let knight = game.spawn_on_battlefield(PlayerId(0), def("White Knight"));

        let snap = snapshot(&game, PlayerId(0));
        let guard_kw = &snap
            .objects
            .iter()
            .find(|o| o.id == guard)
            .unwrap()
            .keywords;
        assert!(
            guard_kw.iter().any(|k| k == "ward:2"),
            "Ward {{2}} reports as ward:2: {guard_kw:?}"
        );

        let knight_kw = &snap
            .objects
            .iter()
            .find(|o| o.id == knight)
            .unwrap()
            .keywords;
        assert!(
            knight_kw.iter().any(|k| k == "protection:black"),
            "protection from black reports as protection:black: {knight_kw:?}"
        );
        assert!(
            knight_kw.iter().any(|k| k == "first_strike"),
            "White Knight also reports first_strike: {knight_kw:?}"
        );
    }

    #[test]
    fn a_snapshot_carries_goaded_for_arena_style_badges() {
        // Martial Impetus's continuous goad lands on the host so the canvas can paint a goad chip.
        let mut game = Game::new();
        let bear = game.spawn_on_battlefield(PlayerId(1), def("Grizzly Bear"));
        game.fund_mana(PlayerId(0));
        let aura = game.spawn_in_hand(PlayerId(0), def("Martial Impetus"));
        game.submit(engine::Intent::Cast {
            player: PlayerId(0),
            object: aura,
            target: Some(engine::Target::Object(bear)),
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .expect("cast Martial Impetus");
        resolve_top_of_stack(&mut game);

        let snap = snapshot(&game, PlayerId(0));
        let bear_view = snap.objects.iter().find(|o| o.id == bear).unwrap();
        assert!(
            bear_view.goaded,
            "enchanted Bear reports goaded on the wire"
        );
        let vanilla = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let snap2 = snapshot(&game, PlayerId(0));
        assert!(
            !snap2
                .objects
                .iter()
                .find(|o| o.id == vanilla)
                .unwrap()
                .goaded,
            "an ungoaded Bear stays clear"
        );
    }

    #[test]
    fn a_snapshot_carries_current_loyalty_like_power_toughness() {
        // WireKind carries starting loyalty; ObjectView.loyalty is the live total the badge paints.
        let mut game = Game::new();
        let walker = game.spawn_on_battlefield(PlayerId(0), def("Quintorius, History Chaser"));
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));

        let snap = snapshot(&game, PlayerId(0));
        let walker_view = snap.objects.iter().find(|o| o.id == walker).unwrap();
        assert_eq!(walker_view.loyalty, 5, "Quintorius enters at loyalty 5");
        assert_eq!(
            snap.objects.iter().find(|o| o.id == bear).unwrap().loyalty,
            0,
            "creatures report 0 loyalty"
        );
    }

    #[test]
    fn a_snapshot_carries_loyalty_after_a_plus_ability() {
        // The badge must track the live total, not WireKind's starting loyalty (5 → 6 on +1).
        let mut game = Game::new();
        game.stack_library(PlayerId(0), &[def("Forest"), def("Forest"), def("Forest")]);
        let walker = game.spawn_on_battlefield(PlayerId(0), def("Quintorius, History Chaser"));

        game.submit(engine::Intent::ActivateAbility {
            player: PlayerId(0),
            object: walker,
            ability_index: 0, // +1: draw two
            target: None,
            sacrifice: vec![],
            x: 0,
        })
        .expect("+1 loyalty ability");
        // Loyalty is paid as a cost on activation — the wire must already show 6 before resolve.
        assert_eq!(game.loyalty(walker), 6);

        let snap = snapshot(&game, PlayerId(0));
        assert_eq!(
            snap.objects
                .iter()
                .find(|o| o.id == walker)
                .unwrap()
                .loyalty,
            6,
            "ObjectView.loyalty tracks the post-+1 total"
        );
    }

    #[test]
    fn a_snapshot_carries_attached_to_so_the_client_can_stack_on_the_host() {
        // Lightning Greaves: Equip {0} — free attach so the snapshot field is easy to assert.
        let mut game = Game::new();
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let greaves = game.spawn_on_battlefield(PlayerId(0), def("Lightning Greaves"));
        game.submit(engine::Intent::ActivateAbility {
            player: PlayerId(0),
            object: greaves,
            ability_index: 1, // static grant is 0; Equip is 1
            target: Some(engine::Target::Object(bear)),
            sacrifice: vec![],
            x: 0,
        })
        .expect("Equip {0}");
        resolve_top_of_stack(&mut game);

        let snap = snapshot(&game, PlayerId(0));
        let equip_view = snap.objects.iter().find(|o| o.id == greaves).unwrap();
        assert_eq!(
            equip_view.attached_to,
            Some(bear),
            "equipped Greaves reports its host on the wire",
        );
        let bear_view = snap.objects.iter().find(|o| o.id == bear).unwrap();
        assert_eq!(
            bear_view.attached_to, None,
            "the host itself is not attached to anything",
        );
    }

    #[test]
    fn a_snapshot_carries_modifiers_grouped_by_source_card_def() {
        let mut game = Game::new();
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        let greaves = game.spawn_on_battlefield(PlayerId(0), def("Lightning Greaves"));
        game.submit(engine::Intent::ActivateAbility {
            player: PlayerId(0),
            object: greaves,
            ability_index: 1,
            target: Some(engine::Target::Object(bear)),
            sacrifice: vec![],
            x: 0,
        })
        .expect("Equip {0}");
        resolve_top_of_stack(&mut game);

        let snap = snapshot(&game, PlayerId(0));
        let bear_view = snap.objects.iter().find(|o| o.id == bear).unwrap();
        let greaves_mod = bear_view
            .modifiers
            .iter()
            .find(|m| m.source_name == "Lightning Greaves")
            .expect("Greaves grants appear under its card name");
        assert_eq!(
            greaves_mod.source_card_id,
            game.def_of(greaves).id,
            "modifier sources carry Card id for inspect"
        );
        assert!(
            greaves_mod.contributions.iter().any(|c| c == "Haste"),
            "expected Haste from Greaves, got {:?}",
            greaves_mod.contributions
        );
        assert!(
            greaves_mod.contributions.iter().any(|c| c == "Shroud"),
            "expected Shroud from Greaves, got {:?}",
            greaves_mod.contributions
        );
    }

    #[test]
    fn a_fresh_snapshot_has_empty_combat_and_reports_can_act() {
        let mut game = Game::new();
        game.spawn_in_hand(PlayerId(0), def("Forest"));
        let snap = snapshot(&game, PlayerId(0));
        assert!(snap.combat.attackers.is_empty());
        assert!(snap.combat.blocks.is_empty());
        assert!(!snap.combat.attackers_declared);
        assert!(snap.combat.blockers_declared.is_empty());
        assert!(snap.can_act, "P0 can play a land, so can_act is true");
    }

    #[test]
    fn an_empty_attack_declaration_sets_attackers_declared_on_the_wire() {
        // Zero attackers leave combat.attackers empty, but the declaration is still final —
        // the client must read attackers_declared or it sticks on "No attackers".
        use engine::{Intent, Step};
        let mut game = Game::new();
        while game.current_step() != Step::DeclareAttackers {
            game.submit(Intent::PassPriority {
                player: game.priority_holder(),
            })
            .unwrap();
        }
        game.submit(Intent::DeclareAttackers {
            player: PlayerId(0),
            attackers: vec![],
        })
        .unwrap();
        let snap = snapshot(&game, PlayerId(0));
        assert!(snap.combat.attackers.is_empty());
        assert!(
            snap.combat.attackers_declared,
            "empty declare must still flag attackers_declared"
        );
    }

    #[test]
    fn a_declare_attackers_action_lists_goaded_required_attacks() {
        // Client must not offer "No attackers" when goad makes empty illegal — the action
        // carries the required (attacker, defender) pairs so staging can seed them.
        use engine::{Intent, Step};
        let mut game = Game::with_players(3, 0);
        let c = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        game.goad(c, PlayerId(1));
        while game.current_step() != Step::DeclareAttackers {
            game.submit(Intent::PassPriority {
                player: game.priority_holder(),
            })
            .unwrap();
        }
        let snap = snapshot(&game, PlayerId(0));
        let action = snap
            .actions
            .iter()
            .find(|a| a.kind == "declare_attackers")
            .expect("declare attackers is listed");
        assert_eq!(
            action.required_attacks,
            vec![WireAttack {
                attacker: c,
                defender: 2
            }],
            "goaded creature must attack a non-goader when one is available"
        );
    }

    #[test]
    fn an_empty_block_declaration_lists_the_defender_in_blockers_declared() {
        use engine::{Intent, Step};
        let mut game = Game::new();
        let bear = game.spawn_on_battlefield(PlayerId(0), def("Grizzly Bear"));
        while game.current_step() != Step::DeclareAttackers {
            game.submit(Intent::PassPriority {
                player: game.priority_holder(),
            })
            .unwrap();
        }
        game.submit(Intent::DeclareAttackers {
            player: PlayerId(0),
            attackers: vec![(bear, PlayerId(1))],
        })
        .unwrap();
        while game.current_step() != Step::DeclareBlockers {
            game.submit(Intent::PassPriority {
                player: game.priority_holder(),
            })
            .unwrap();
        }
        game.submit(Intent::DeclareBlockers {
            player: PlayerId(1),
            blocks: vec![],
        })
        .unwrap();
        let snap = snapshot(&game, PlayerId(1));
        assert!(snap.combat.blocks.is_empty());
        assert_eq!(
            snap.combat.blockers_declared,
            vec![1],
            "empty block declare must list the defending seat"
        );
    }

    #[test]
    fn a_snapshot_lists_the_stack_with_labels_and_targets() {
        let mut game = Game::new();
        game.fund_mana(PlayerId(0));
        let bear = game.spawn_on_battlefield(PlayerId(1), def("Grizzly Bear"));
        let shock = game.spawn_in_hand(PlayerId(0), def("Shock"));
        game.submit(engine::Intent::Cast {
            player: PlayerId(0),
            object: shock,
            target: Some(engine::Target::Object(bear)),
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .unwrap();

        let snap = snapshot(&game, PlayerId(0));
        assert_eq!(snap.stack.len(), 1, "Shock is on the stack");
        assert_eq!(snap.stack[0].kind, "spell");
        assert_eq!(snap.stack[0].label, "Shock");
        assert_eq!(snap.stack[0].target, Some(WireTarget::Object { id: bear }));
    }

    #[test]
    fn a_scry_choices_looked_at_cards_are_visible_only_to_the_scrying_player() {
        // Augury Owl ETB's "scry 3" pauses on an ArrangeTop choice; project_board's mapping to
        // PendingChoiceView::Scry must hide the looked-at cards from everyone but the scryer —
        // the same private-items gate Surveil shares (same match arm, `to_graveyard = true`).
        let mut game = Game::new();
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        game.fund_mana(p0);
        game.stack_library(p0, &[def("Forest"), def("Island"), def("Mountain")]);
        let owl = game.spawn_in_hand(p0, def("Augury Owl"));

        game.submit(engine::Intent::Cast {
            player: p0,
            object: owl,
            target: None,
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .unwrap();
        resolve_top_of_stack(&mut game); // The Owl enters; its ETB scry-3 trigger goes on the stack.
        resolve_top_of_stack(&mut game); // The trigger resolves: scry 3 → pause on ArrangeTop.

        match snapshot(&game, p0).pending_choice {
            Some(PendingChoiceView::Scry { items, .. }) => {
                assert_eq!(items.len(), 3, "the scryer sees the three looked-at cards")
            }
            other => panic!("expected a Scry choice for the scryer, got {other:?}"),
        }
        match snapshot(&game, p1).pending_choice {
            Some(PendingChoiceView::Scry { items, .. }) => {
                assert!(items.is_empty(), "an opponent sees no looked-at cards")
            }
            other => panic!("expected a Scry choice for the opponent, got {other:?}"),
        }
        match spectator_snapshot(&game).pending_choice {
            Some(PendingChoiceView::Scry { items, .. }) => {
                assert!(items.is_empty(), "a spectator sees no looked-at cards")
            }
            other => panic!("expected a Scry choice for a spectator, got {other:?}"),
        }
    }

    #[test]
    fn a_tutors_matching_cards_are_visible_only_to_the_searching_player() {
        // Diabolic Tutor pauses on a SearchLibrary choice; project_board's mapping must hide the
        // matching library cards from everyone but the searcher.
        let mut game = Game::new();
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        game.fund_mana(p0);
        game.stack_library(p0, &[def("Forest"), def("Grizzly Bear"), def("Island")]);
        let tutor = game.spawn_in_hand(p0, def("Diabolic Tutor"));

        game.submit(engine::Intent::Cast {
            player: p0,
            object: tutor,
            target: None,
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .unwrap();
        resolve_top_of_stack(&mut game); // Diabolic Tutor resolves → pause on the search choice.

        match snapshot(&game, p0).pending_choice {
            Some(PendingChoiceView::SearchLibrary { items, .. }) => {
                assert_eq!(
                    items.len(),
                    3,
                    "the searcher sees every matching library card"
                );
                for item in &items {
                    assert!(
                        !item.print.is_empty(),
                        "library-search items must carry a Printing UUID for art (got empty for {})",
                        item.label
                    );
                }
            }
            other => panic!("expected a SearchLibrary choice for the searcher, got {other:?}"),
        }
        match snapshot(&game, p1).pending_choice {
            Some(PendingChoiceView::SearchLibrary { items, .. }) => {
                assert!(items.is_empty(), "an opponent sees no matching cards")
            }
            other => panic!("expected a SearchLibrary choice for the opponent, got {other:?}"),
        }
        match spectator_snapshot(&game).pending_choice {
            Some(PendingChoiceView::SearchLibrary { items, .. }) => {
                assert!(items.is_empty(), "a spectator sees no matching cards")
            }
            other => panic!("expected a SearchLibrary choice for a spectator, got {other:?}"),
        }
    }

    #[test]
    fn a_discarders_hand_is_visible_only_to_the_discarding_player() {
        // Nine cards in hand (two over the hand-size limit) pauses cleanup on a
        // DiscardToHandSize choice; project_board's mapping must hide the discardable hand from
        // everyone but the discarder — the count is public, the identities are not.
        let mut game = Game::new();
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        for _ in 0..9 {
            game.spawn_in_hand(p0, def("Grizzly Bear"));
        }

        pass_until_choice(&mut game);

        match snapshot(&game, p0).pending_choice {
            Some(PendingChoiceView::Discard { count, items, .. }) => {
                assert_eq!(count, 2, "two cards over the seven-card limit");
                assert_eq!(items.len(), 9, "the discarder sees their whole hand");
            }
            other => panic!("expected a Discard choice for the discarder, got {other:?}"),
        }
        match snapshot(&game, p1).pending_choice {
            Some(PendingChoiceView::Discard { count, items, .. }) => {
                assert_eq!(count, 2, "the count is public");
                assert!(items.is_empty(), "an opponent sees none of the hand");
            }
            other => panic!("expected a Discard choice for the opponent, got {other:?}"),
        }
        match spectator_snapshot(&game).pending_choice {
            Some(PendingChoiceView::Discard { count, items, .. }) => {
                assert_eq!(count, 2, "the count is public");
                assert!(items.is_empty(), "a spectator sees none of the hand");
            }
            other => panic!("expected a Discard choice for a spectator, got {other:?}"),
        }
    }

    #[test]
    fn abstract_performance_first_pile_hidden_from_chooser() {
        // Abstract Performance exiles its first four cards face down (CR 701.9): the choosing
        // opponent must see both piles' sizes but not the face-down pile's card identities,
        // while the pile's owner sees both piles in full.
        let mut game = Game::new();
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        game.fund_mana(p0);
        game.stack_library(
            p0,
            &[
                def("Forest"),
                def("Forest"),
                def("Forest"),
                def("Forest"),
                def("Grizzly Bear"),
                def("Forest"),
                def("Forest"),
                def("Forest"),
            ],
        );
        let ap = game.spawn_in_hand(p0, def("Abstract Performance"));
        game.submit(engine::Intent::Cast {
            player: p0,
            object: ap,
            target: None,
            x: 0,
            modes: vec![],
            discard_cost: vec![],
            graveyard_exile: vec![],
            sacrifice_cost: vec![],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        })
        .unwrap();
        resolve_top_of_stack(&mut game); // resolves, pausing on the opponent's pile choice

        let (pile_a, pile_b) = match game.pending_choice() {
            Some(engine::PendingChoice::OpponentChoosesPile { pile_a, pile_b, .. }) => {
                (pile_a, pile_b)
            }
            other => panic!("expected the opponent-chooses-pile pause, got {other:?}"),
        };

        // The chooser (P1) sees pile_a's slots but not its identities; pile_b is fully visible.
        match snapshot(&game, p1).pending_choice {
            Some(PendingChoiceView::OpponentChoosesPile {
                pile_a: a,
                pile_b: b,
                ..
            }) => {
                assert_eq!(a.len(), 4, "the face-down pile's size is still visible");
                assert!(
                    a.iter().all(|item| item.label.is_empty()),
                    "the chooser can't see the face-down pile's identities"
                );
                assert!(
                    b.iter().all(|item| !item.label.is_empty()),
                    "the face-up pile is fully visible to the chooser"
                );
            }
            other => panic!("expected the opponent's own pile choice, got {other:?}"),
        }

        // The owner (P0) sees both piles in full.
        match snapshot(&game, p0).pending_choice {
            Some(PendingChoiceView::OpponentChoosesPile {
                pile_a: a,
                pile_b: b,
                ..
            }) => {
                assert!(
                    a.iter().all(|item| !item.label.is_empty()),
                    "the owner can see their own face-down pile"
                );
                assert!(b.iter().all(|item| !item.label.is_empty()));
            }
            other => panic!("expected the owner's own pile choice, got {other:?}"),
        }

        // The board-object projection also anonymizes the face-down pile's cards for the chooser.
        let opp_view = snapshot(&game, p1);
        for &id in &pile_a {
            let obj = opp_view
                .objects
                .iter()
                .find(|o| o.id == id)
                .expect("still on the board");
            assert!(
                obj.face_down,
                "a face-down pile card renders as a card back to a non-owner"
            );
        }
        for &id in &pile_b {
            let obj = opp_view
                .objects
                .iter()
                .find(|o| o.id == id)
                .expect("still on the board");
            assert!(!obj.face_down, "the face-up pile is not anonymized");
        }
        let owner_view = snapshot(&game, p0);
        for &id in &pile_a {
            let obj = owner_view
                .objects
                .iter()
                .find(|o| o.id == id)
                .expect("still on the board");
            assert!(
                !obj.face_down,
                "the owner sees their own face-down pile plainly"
            );
        }
    }
}
