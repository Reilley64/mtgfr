//! Sequence continuations during effect resolution (pause bookkeeping lives in `pending`).
//!
//! Primary: CR 608 resolution interrupted by choices; [`Effect::Sequence`] tails replayed
//! after a pending answer. Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

/// Shared resolution context for one [`Effect`] (or a sequence of them): who controls it,
/// which object is the source, the chosen target, and the spell's `{X}` (abilities use 0).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ResolveCtx {
    pub(crate) controller: PlayerId,
    pub(crate) source: ObjectId,
    pub(crate) target: Option<Target>,
    /// A triggered ability's second independent target clause's chosen targets (CR 603.3d), read by
    /// [`Effect::DoubleCountersOnTargetCreatures`]. Empty for every other resolution.
    pub(crate) targets_second: TargetList,
    pub(crate) x: u32,
}

/// The deferred tail of an [`Effect::Sequence`]: the steps left to run, plus the resolution
/// context they share. Stashed when a step pauses and replayed when its choice is answered.
/// `Copy` — every field is (the steps are `&'static`).
#[derive(Clone, Copy)]
pub(crate) struct SequenceCont {
    pub(crate) steps: &'static [Effect],
    pub(crate) ctx: ResolveCtx,
}

impl Game {
    /// Whether effect resolution is blocked on a player answer.
    pub(crate) fn resolution_is_paused(&self) -> bool {
        self.pending_choice.is_some()
    }

    /// Run an [`Effect::Sequence`]'s `steps` in order, each sharing `ctx`. A step that pauses
    /// (surveil, discard) sets `pending_choice`; the remaining steps are stashed as a
    /// [`SequenceCont`] and replayed by [`Self::resume_deferred_sequence`] once that choice is
    /// answered. Fully non-pausing steps run to completion here.
    pub(crate) fn run_sequence(
        &mut self,
        steps: &'static [Effect],
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        for (i, &step) in steps.iter().enumerate() {
            self.run(step, ctx, events);
            if self.resolution_is_paused() {
                let rest = &steps[i + 1..];
                if !rest.is_empty() {
                    self.pending_sequence = Some(SequenceCont { steps: rest, ctx });
                }
                return;
            }
        }
    }

    /// After a choice answer fully clears the pause, resume any deferred sequence tail into
    /// `events`. The tail may itself pause and re-stash a continuation. Once resolution is truly
    /// unpaused, also finishes any [`Game::pending_spell_finish`] spell whose own resolution was
    /// waiting on this same pause (Sevinne's Reclamation's "may copy this spell" rider).
    pub(crate) fn resume_deferred_sequence(&mut self, events: &mut Vec<Event>) {
        if self.resolution_is_paused() {
            return;
        }
        if let Some(cont) = self.pending_sequence.take() {
            self.run_sequence(cont.steps, cont.ctx, events);
        }
        if self.resolution_is_paused() {
            return;
        }
        if let Some((opponent, spell)) = self.pending_demonstrate_opponent_copy.take() {
            self.mint_spell_copies(Amount::Fixed(1), opponent, spell, None, 0, events);
        }
        if self.resolution_is_paused() {
            return;
        }
        if let Some(object) = self.pending_spell_finish.take() {
            self.finish_instant_sorcery_resolution(object, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LAND: CardDef = CardDef {
        name: "Test Land",
        id: "",
        default_print: "",
        cost: Cost::FREE,
        kind: CardKind::Land {
            produces: None,
            subtypes: &[],
            basic: false,
        },
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
        abilities: &[],
        cycling: None,
        flashback: None,
        echo: None,
        bestow: None,
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
    };

    const SURVEIL_THEN_DRAW: &[Effect] = &[
        Effect::Surveil { count: 2 },
        Effect::DrawCards {
            count: Amount::Fixed(1),
        },
    ];

    fn hand_count(game: &Game, player: PlayerId) -> usize {
        game.live_object_ids()
            .into_iter()
            .filter(|&id| game.zone_of(id) == Zone::Hand && game.owner_of(id) == player)
            .count()
    }

    fn ctx(controller: PlayerId) -> ResolveCtx {
        ResolveCtx {
            controller,
            source: 0,
            target: None,
            targets_second: TargetList::default(),
            x: 0,
        }
    }

    #[test]
    fn run_sequence_stashes_tail_when_a_step_pauses() {
        let mut game = Game::with_players(2, 0);
        game.stack_library(PlayerId(0), &[TEST_LAND, TEST_LAND, TEST_LAND]);
        let mut events = Vec::new();

        game.run_sequence(SURVEIL_THEN_DRAW, ctx(PlayerId(0)), &mut events);

        assert!(
            game.resolution_is_paused(),
            "surveil should pause before the draw"
        );
        assert!(
            game.pending_sequence.is_some(),
            "the deferred draw should be stashed"
        );
        assert_eq!(hand_count(&game, PlayerId(0)), 0, "draw is deferred");
    }

    #[test]
    fn resume_deferred_sequence_runs_the_tail_after_the_pause_clears() {
        let mut game = Game::with_players(2, 0);
        let lib = game.stack_library(PlayerId(0), &[TEST_LAND, TEST_LAND, TEST_LAND]);
        let mut events = Vec::new();

        game.run_sequence(SURVEIL_THEN_DRAW, ctx(PlayerId(0)), &mut events);
        assert!(game.pending_sequence.is_some());

        game.finish_answer();
        game.resume_deferred_sequence(&mut events);

        assert!(!game.resolution_is_paused());
        assert!(game.pending_sequence.is_none());
        assert_eq!(
            hand_count(&game, PlayerId(0)),
            1,
            "the deferred draw should run once the pause clears"
        );
        assert_eq!(
            game.current_id(lib[0]),
            game.live_object_ids()
                .into_iter()
                .find(|&id| game.zone_of(id) == Zone::Hand)
                .expect("one card in hand"),
        );
    }

    #[test]
    fn submit_resumes_a_deferred_sequence_after_a_surveil_answer() {
        let mut game = Game::with_players(2, 0);
        let lib = game.stack_library(PlayerId(0), &[TEST_LAND, TEST_LAND, TEST_LAND]);
        let mut events = Vec::new();

        game.run_sequence(SURVEIL_THEN_DRAW, ctx(PlayerId(0)), &mut events);
        assert!(game.pending_sequence.is_some());

        events.extend(
            game.arrange_top(PlayerId(0), vec![lib[0], lib[1]], vec![])
                .expect("keeping both on top is legal"),
        );
        game.resume_deferred_sequence(&mut events);

        assert!(!game.resolution_is_paused());
        assert!(game.pending_sequence.is_none());
        assert_eq!(hand_count(&game, PlayerId(0)), 1);
    }

    #[test]
    fn run_applies_a_pure_effect() {
        let mut game = Game::with_players(2, 0);
        game.stack_library(PlayerId(0), &[TEST_LAND]);
        let mut events = Vec::new();

        game.run(
            Effect::DrawCards {
                count: Amount::Fixed(1),
            },
            ctx(PlayerId(0)),
            &mut events,
        );

        assert!(!game.resolution_is_paused());
        assert_eq!(hand_count(&game, PlayerId(0)), 1);
        assert!(events.iter().any(|e| matches!(e, Event::CardDrawn { .. })));
    }
}
