//! Explicit post-intent pipeline for the sequential state machine.
//!
//! After an intent handler applies its events, [`PostIntentPipeline::run`] performs
//! the named phases in rules order before control returns to the caller. The same
//! pipeline is shared by [`Game::submit`] and [`Game::begin_first_turn`] so the
//! two paths cannot drift.
//!
//! Primary: CR 704 (SBA fixpoint), CR 603 (trigger enqueue / APNAP placement), CR 608
//! (priority rounds emptying the stack). Deferred / gaps: per-deck increments under `docs/fidelity/` (fidelity-grind skill).

use crate::*;

/// A named phase of the post-intent pipeline, in execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PostIntentPhase {
    /// Sweep state-based actions to a fixpoint (CR 704).
    StateBasedActions,
    /// If the priority holder was eliminated by an SBA, hand priority to the next living player.
    PriorityHandoffOnElimination,
    /// Queue triggered abilities fired by the batch's events.
    TriggerEnqueue,
    /// Fire scheduled delayed triggers (CR 603.7) whose step has arrived.
    DelayedTriggers,
    /// Fire event-armed delayed one-shots (CR 603.7) whose watched cast just happened.
    NextCastTriggers,
    /// Fire event-armed delayed watches (CR 603.7) whose watched creature just dealt combat
    /// damage to a player.
    CombatDamageWatchTriggers,
    /// Fire this-turn, controller-scoped, repeatable delayed watches (CR 603.7) whose
    /// controller's creature just dealt combat damage to a player.
    CombatDamageCopyTriggers,
    /// Place pending triggers onto the stack (APNAP ordering).
    TriggerPlacement,
    /// Recompute each living player's legal-action list.
    RefreshActions,
}

impl PostIntentPhase {
    pub(crate) const ALL: &'static [PostIntentPhase] = &[
        Self::StateBasedActions,
        Self::PriorityHandoffOnElimination,
        Self::TriggerEnqueue,
        Self::DelayedTriggers,
        Self::NextCastTriggers,
        Self::CombatDamageWatchTriggers,
        Self::CombatDamageCopyTriggers,
        Self::TriggerPlacement,
        Self::RefreshActions,
    ];
}

/// What happens when every living player passes in succession (CR 608).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PriorityRoundOutcome {
    /// The stack is empty — advance to the next step.
    AdvanceStep,
    /// The top stack object resolves — priority returns to the active player.
    ResolveStackTop,
    /// Declare attackers/blockers still needs a legal declaration; priority stayed put.
    AwaitCombatDeclaration,
}

/// Coordinator for post-change phases of the sequential state machine.
pub(crate) struct PostIntentPipeline;

impl PostIntentPipeline {
    /// Run every post-intent phase in order.
    pub(crate) fn run(game: &mut Game, events: &mut Vec<Event>) {
        for &phase in PostIntentPhase::ALL {
            Self::run_phase(game, phase, events);
        }
    }

    fn run_phase(game: &mut Game, phase: PostIntentPhase, events: &mut Vec<Event>) {
        match phase {
            PostIntentPhase::StateBasedActions => game.sweep_state_based_actions(events),
            PostIntentPhase::PriorityHandoffOnElimination => {
                Self::handoff_priority_on_elimination(game);
            }
            PostIntentPhase::TriggerEnqueue => game.enqueue_triggers(events),
            PostIntentPhase::DelayedTriggers => game.fire_delayed_triggers(events),
            PostIntentPhase::NextCastTriggers => game.fire_next_cast_triggers(events),
            PostIntentPhase::CombatDamageWatchTriggers => {
                game.fire_combat_damage_watch_triggers(events);
            }
            PostIntentPhase::CombatDamageCopyTriggers => {
                game.fire_combat_damage_copy_triggers(events);
            }
            PostIntentPhase::TriggerPlacement => game.place_pending_triggers(events),
            PostIntentPhase::RefreshActions => game.refresh_actions(),
        }
    }

    /// An elimination may have struck the priority holder (e.g. a draw-from-empty on their
    /// own step): hand priority to the next living player so the game keeps moving.
    fn handoff_priority_on_elimination(game: &mut Game) {
        if game.players[game.priority.0 as usize].lost && game.winner().is_none() {
            game.priority = game.next_player(game.priority);
            game.consecutive_passes = 0;
        }
    }

    /// Before leaving a combat declaration step on an empty stack, seal any missing declaration
    /// as empty when legal. If empty is illegal (goad must-attack), keep the step open and hand
    /// priority back to the player who must declare — all-pass must not skip the declaration.
    fn seal_combat_declarations(game: &mut Game, events: &mut Vec<Event>) -> bool {
        if game.step == Step::DeclareAttackers && !game.combat.attackers_declared {
            match game.declare_attackers(game.active_player, &[]) {
                Ok(ev) => {
                    events.extend(ev);
                    true
                }
                Err(_) => {
                    game.consecutive_passes = 0;
                    game.priority = game.active_player;
                    false
                }
            }
        } else if game.step == Step::DeclareBlockers {
            let defenders: Vec<PlayerId> = game
                .living_players()
                .filter(|&p| game.is_attacked_player(p) && !game.combat.blocked_by.contains(&p))
                .collect();
            for defender in defenders {
                match game.declare_blockers(defender, &[]) {
                    Ok(ev) => events.extend(ev),
                    Err(_) => {
                        game.consecutive_passes = 0;
                        game.priority = defender;
                        return false;
                    }
                }
            }
            true
        } else {
            true
        }
    }

    /// Complete a priority round when every living player has passed in succession.
    pub(crate) fn complete_priority_round(
        game: &mut Game,
        events: &mut Vec<Event>,
    ) -> PriorityRoundOutcome {
        if game.stack.is_empty() {
            if !Self::seal_combat_declarations(game, events) {
                return PriorityRoundOutcome::AwaitCombatDeclaration;
            }
            events.extend(game.advance_step());
            PriorityRoundOutcome::AdvanceStep
        } else {
            game.resolve_top(events);
            game.consecutive_passes = 0;
            game.priority = game.active_player;
            PriorityRoundOutcome::ResolveStackTop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    #[test]
    fn phases_run_in_rules_order() {
        assert_eq!(
            PostIntentPhase::ALL,
            &[
                PostIntentPhase::StateBasedActions,
                PostIntentPhase::PriorityHandoffOnElimination,
                PostIntentPhase::TriggerEnqueue,
                PostIntentPhase::DelayedTriggers,
                PostIntentPhase::NextCastTriggers,
                PostIntentPhase::CombatDamageWatchTriggers,
                PostIntentPhase::CombatDamageCopyTriggers,
                PostIntentPhase::TriggerPlacement,
                PostIntentPhase::RefreshActions,
            ],
        );
    }

    #[test]
    fn priority_handoff_when_priority_holder_is_eliminated() {
        let mut game = Game::with_players(4, 0);
        game.priority = P1;
        game.consecutive_passes = 2;
        game.players[1].lost = true;

        let mut events = Vec::new();
        PostIntentPipeline::run(&mut game, &mut events);

        assert_eq!(game.priority_holder(), P2);
        assert_eq!(game.consecutive_passes, 0);
    }

    #[test]
    fn priority_handoff_skipped_when_game_has_a_winner() {
        let mut game = Game::with_players(2, 0);
        game.priority = P0;
        game.players[0].lost = true;
        game.players[1].lost = false;

        let mut events = Vec::new();
        PostIntentPipeline::run(&mut game, &mut events);

        assert_eq!(game.priority_holder(), P0);
    }

    #[test]
    fn empty_stack_priority_round_advances_the_step() {
        let mut game = Game::with_players(2, 0);
        assert_eq!(game.current_step(), Step::Main1);

        let mut events = Vec::new();
        let outcome = PostIntentPipeline::complete_priority_round(&mut game, &mut events);

        assert_eq!(outcome, PriorityRoundOutcome::AdvanceStep);
        assert_eq!(game.current_step(), Step::BeginCombat);
        assert!(events.iter().any(|e| matches!(
            e,
            Event::StepBegan {
                step: Step::BeginCombat,
                ..
            }
        )),);
    }
}
