//! Pending-choice lifecycle (choices-actions-and-resolution spec): raise → answer → resume elsewhere.
//!
//! External seam for callers (`Game::submit`, effect/cast/trigger/combat/priority pause sites):
//! - [`raise`] / [`ChoiceRequest`] — typed raise for common effect/cast pause sites
//! - [`raise_choice`] — pause on an already-built [`PendingChoice`] (triggers/combat/TBAs)
//! - [`answer`] — apply an answer [`Intent`] for the pending discriminant (does **not** resume sequences)
//! - [`forced`] — conservative singleton auto-answer (`None` for most discriminants)
//!
//! [`resume_deferred_sequence`](crate::Game::resume_deferred_sequence) stays on submit /
//! resolution — Choice owns pause ↔ answer ↔ events only.
//!
//! Handlers and dig-loop kickoff helpers live in [`handlers`]. Answer / forced routing is the
//! choice-discriminant table in [`dispatch`]. `pause_for` is private to this module so other
//! engine modules must not poke `PendingChoice` raw — use [`raise`] / [`raise_choice`] instead.
//!
//! [`ChoiceRequest`] construction and skip guards live in [`raise`] (split by family). Dig-loop
//! / multi-step effect kickoffs (cascade, reveal-until, dance, edict prep, …) remain non-pure
//! constructors: call sites emit prep/dig events then [`raise`] — they are not pure
//! `ChoiceRequest` factories because prep mutates via events before the pause.

mod dispatch;
mod handlers;
mod raise;

pub(crate) use dispatch::{answer, forced};
pub(crate) use raise::ChoiceRequest;

use crate::{Game, Intent, PendingChoice};

/// Raise a Choice from resolution (or cast). Constructs [`PendingChoice`] and pauses.
/// Some variants skip when there is nothing to choose (empty board / hand).
pub(crate) fn raise(game: &mut Game, request: ChoiceRequest) {
    let Some(choice) = raise::choice_from_request(game, request) else {
        return;
    };
    game.pause_for(choice);
}

/// Pause on an already-built [`PendingChoice`]. Production sites outside this module
/// (triggers, combat, turn-based discard, cast targeting) must use this instead of writing
/// `pending_choice` directly.
pub(crate) fn raise_choice(game: &mut Game, choice: PendingChoice) {
    game.pause_for(choice);
}

/// Whether `intent` is an answer to a pending Choice (not cast / pass / concede / …).
pub(crate) fn is_answer(intent: &Intent) -> bool {
    intent.is_choice_answer()
}

impl Game {
    /// Begin waiting on `choice` before resolution can continue.
    /// Private to [`pending`]: effects/cast use [`raise`] / [`raise_choice`].
    fn pause_for(&mut self, choice: PendingChoice) {
        self.pending_choice = Some(choice);
    }

    /// Take the pending choice for validation; invalid answers must call [`Self::restore_pause`].
    fn take_pending_choice(&mut self) -> Option<PendingChoice> {
        self.pending_choice.take()
    }

    /// Put back a pending choice after rejecting an invalid answer.
    fn restore_pause(&mut self, choice: PendingChoice) {
        self.pending_choice = Some(choice);
    }

    /// Clear the pause after a valid answer.
    pub(crate) fn finish_answer(&mut self) {
        self.pending_choice = None;
    }
}
