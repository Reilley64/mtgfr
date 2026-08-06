//! Deferred resolution resume — riders parked while a pause blocks the current
//! effect body, drained in CR-faithful order by [`Game::resume_deferred_sequence`].
//!
//! Not board facts: these are submit-path orchestration fields (same class as
//! [`crate::Game::pending_choice`]), collapsed here so PlayerLost cleanup and
//! drain order share one table.

use super::SequenceCont;
use crate::{ObjectId, PlayerId};

/// What runs once an interrupted draw batch finishes every seat it owes.
///
/// Every draw in the game is drawn one event at a time through
/// [`Game::run_draw_batch`](crate::Game::run_draw_batch), because either replacement it may meet
/// — dredge (CR 702.52) or Chains of Mephistopheles' discard (CR 614) — pauses mid-batch for an
/// answer. Whatever the batch's caller meant to do *after* the draws therefore can't just follow
/// the call; it rides here and runs when the last seat is paid.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DrawAfter {
    /// A mid-resolution draw: any sequence tail is resumed by
    /// [`Game::resume_deferred_sequence`](crate::Game::resume_deferred_sequence) as usual.
    #[default]
    Nothing,
    /// The draw step's turn-based draw — resume the interrupted step transition.
    DrawStep,
    /// Trade Secrets: the opponent's two draws are followed by the caster's "up to `max`".
    TradeSecretsCaster {
        caster: PlayerId,
        opponent: PlayerId,
        source: ObjectId,
        max: u8,
    },
    /// Trade Secrets: the caster's "up to `max`" is followed by the opponent's repeat offer.
    TradeSecretsRepeat {
        caster: PlayerId,
        opponent: PlayerId,
        source: ObjectId,
        max: u8,
    },
}

/// A draw batch parked mid-way through by a draw replacement's pause.
///
/// `seats` is the still-unpaid work in the order it must happen — `seats[0]` is the seat whose
/// draw is being answered right now, and its count has already had that draw taken off it.
#[derive(Clone)]
pub(crate) struct DrawBatch {
    pub(crate) seats: Vec<(PlayerId, u32)>,
    pub(crate) after: DrawAfter,
    /// Whether any draw in this batch actually paused. The draw step's caller advances the step
    /// itself when nothing paused, so `after` must not advance it a second time.
    pub(crate) paused: bool,
}

/// Parked mid-resolution work waiting for the current pause to clear.
///
/// Drain order in [`Game::resume_deferred_sequence`] (bit-identical to the prior
/// flat fields): clash scry → sequence tail → demonstrate opponent copy → spell finish.
#[derive(Clone, Default)]
pub(crate) struct ResumeState {
    /// Clash (CR 701.22): opponent still owed a keep-on-top-or-bottom scry after the
    /// controller's. `None` unless a clash is mid-way between the two reveals' decisions.
    pub(crate) clash_scry: Option<PlayerId>,
    /// Remaining [`Effect::Sequence`](crate::Effect::Sequence) steps after a pausing step.
    pub(crate) sequence: Option<SequenceCont>,
    /// Demonstrate (CR 702.147a) second copy: `(opponent, spell)`, after the controller's
    /// own copy is minted.
    pub(crate) demonstrate_opponent_copy: Option<(PlayerId, ObjectId)>,
    /// Instant/sorcery that paused mid-body and still needs to leave the stack.
    pub(crate) spell_finish: Option<ObjectId>,
    /// A draw batch a draw replacement paused mid-way through (dredge, or Chains of
    /// Mephistopheles' discard). Unlike the riders above this one is *not* drained by
    /// [`Game::resume_deferred_sequence`](crate::Game::resume_deferred_sequence): the replacement's
    /// own answer handler resumes it, because the rest of the batch has to be drawn before the
    /// effect that asked for it moves on.
    pub(crate) draw_batch: Option<DrawBatch>,
}

impl ResumeState {
    /// Drop resume riders that reference a departing player or already-removed objects
    /// (CR 800.4a — nobody left to answer / no live object to finish).
    pub(crate) fn clear_for_removed(
        &mut self,
        player: PlayerId,
        removed: impl Fn(ObjectId) -> bool,
    ) {
        if self.clash_scry == Some(player) {
            self.clash_scry = None;
        }
        if self
            .sequence
            .as_ref()
            .is_some_and(|cont| cont.ctx.controller == player)
        {
            self.sequence = None;
        }
        if self.spell_finish.is_some_and(&removed) {
            self.spell_finish = None;
        }
        if self
            .demonstrate_opponent_copy
            .is_some_and(|(opponent, spell)| opponent == player || removed(spell))
        {
            self.demonstrate_opponent_copy = None;
        }
        // A departed seat draws nothing more (CR 800.4a), and a follow-up owed to or by them has
        // nobody left to answer it. The batch's other seats still get theirs.
        if let Some(batch) = &mut self.draw_batch {
            batch.seats.retain(|&(seat, _)| seat != player);
            if batch.after.names(player) {
                batch.after = DrawAfter::Nothing;
            }
            if batch.seats.is_empty() && batch.after == DrawAfter::Nothing {
                self.draw_batch = None;
            }
        }
    }
}

impl DrawAfter {
    /// Whether this follow-up needs `player` — a Trade Secrets leg owed to or by a seat that has
    /// left the game can't run.
    fn names(self, player: PlayerId) -> bool {
        match self {
            DrawAfter::Nothing | DrawAfter::DrawStep => false,
            DrawAfter::TradeSecretsCaster {
                caster, opponent, ..
            }
            | DrawAfter::TradeSecretsRepeat {
                caster, opponent, ..
            } => caster == player || opponent == player,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TargetList;
    use crate::resolution::ResolveCtx;

    fn ctx(controller: PlayerId) -> ResolveCtx {
        ResolveCtx {
            controller,
            source: 0,
            target: None,
            targets_second: TargetList::default(),
            x: 0,
            spent_mana: [0; 6],
        }
    }

    #[test]
    fn clear_for_removed_drops_clash_scry_for_the_departing_opponent() {
        let mut resume = ResumeState {
            clash_scry: Some(PlayerId(1)),
            ..ResumeState::default()
        };
        resume.clear_for_removed(PlayerId(1), |_| false);
        assert!(resume.clash_scry.is_none());
    }

    #[test]
    fn clear_for_removed_keeps_clash_scry_for_a_different_seat() {
        let mut resume = ResumeState {
            clash_scry: Some(PlayerId(1)),
            ..ResumeState::default()
        };
        resume.clear_for_removed(PlayerId(0), |_| false);
        assert_eq!(resume.clash_scry, Some(PlayerId(1)));
    }

    #[test]
    fn clear_for_removed_drops_sequence_when_controller_leaves() {
        let mut resume = ResumeState {
            sequence: Some(SequenceCont {
                steps: std::sync::Arc::from([]),
                ctx: ctx(PlayerId(0)),
            }),
            ..ResumeState::default()
        };
        resume.clear_for_removed(PlayerId(0), |_| false);
        assert!(resume.sequence.is_none());
    }

    #[test]
    fn clear_for_removed_drops_spell_finish_when_object_is_removed() {
        let mut resume = ResumeState {
            spell_finish: Some(7),
            ..ResumeState::default()
        };
        resume.clear_for_removed(PlayerId(0), |id| id == 7);
        assert!(resume.spell_finish.is_none());
    }

    #[test]
    fn clear_for_removed_drops_demonstrate_when_opponent_leaves_or_spell_is_gone() {
        let mut by_opponent = ResumeState {
            demonstrate_opponent_copy: Some((PlayerId(2), 9)),
            ..ResumeState::default()
        };
        by_opponent.clear_for_removed(PlayerId(2), |_| false);
        assert!(by_opponent.demonstrate_opponent_copy.is_none());

        let mut by_spell = ResumeState {
            demonstrate_opponent_copy: Some((PlayerId(2), 9)),
            ..ResumeState::default()
        };
        by_spell.clear_for_removed(PlayerId(0), |id| id == 9);
        assert!(by_spell.demonstrate_opponent_copy.is_none());
    }
}
