//! Deferred resolution resume — riders parked while a pause blocks the current
//! effect body, drained in CR-faithful order by [`Game::resume_deferred_sequence`].
//!
//! Not board facts: these are submit-path orchestration fields (same class as
//! [`crate::Game::pending_choice`]), collapsed here so PlayerLost cleanup and
//! drain order share one table.

use super::SequenceCont;
use crate::{ObjectId, PlayerId};

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
                steps: &[],
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
