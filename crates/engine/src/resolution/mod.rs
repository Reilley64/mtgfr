//! Effect resolution behind [`Game::run`]: sequence continuations, resolution-local
//! [`ResolutionFrame`] scratch, and (growing) family mint / resolve locality. Pause
//! bookkeeping lives in [`crate::pending`]; board mutation stays in [`crate::apply`].
//!
//! Primary: CR 608. External seam: [`Game::run`] (in `effects`) is the sole Effect→board verb —
//! callers never choose mint vs pause. Internals here: [`SequenceCont`] / resume, [`ResolutionFrame`],
//! pure mint dispatcher ([`mint`]) + families ([`draw`], [`damage`], [`life`], …), and pause peels
//! ([`pause_arrange`], [`pause_look`], [`pause_hand`], [`pause_may`], [`pause_choose`],
//! [`pause_exile_cast`], [`pause_edict`], [`pause_fight`], [`pause_counter_spell`]). Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

mod control;
mod counters;
mod damage;
mod destroy;
mod draw;
mod frame;
mod life;
mod mana;
mod mill;
mod mint;
mod misc;
mod pause_arrange;
mod pause_choose;
mod pause_counter_spell;
mod pause_edict;
mod pause_exile_cast;
mod pause_fight;
mod pause_hand;
mod pause_look;
mod pause_may;
mod pump;
mod resolve_misc;
mod resume;
mod reveal;
mod sequence_steps;
mod tokens;
mod zones;

pub(crate) use frame::ResolutionFrame;
pub(crate) use resume::ResumeState;

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
    /// The multiset of mana actually spent activating the resolving ability
    /// ([`StackItem::Ability::spent_mana`]), read by [`Effect::CastCreatureFaceDown`]'s CR 107.3
    /// payability test. All zeroes except when [`Game::resolve_top`] resolves a real activation —
    /// the pending-answer paths that reconstruct a ctx pass zeroes (none can reach
    /// `CastCreatureFaceDown`, which pauses only on its own choice).
    pub(crate) spent_mana: [u8; 6],
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

    /// The owner (or controller, if `to_controller`) of a [`Sequence`](Effect::Sequence)'s shared
    /// target — an ordinary live [`Game::owner_of`]/[`Game::controller_of`] read, except when a
    /// *preceding* step in the same Sequence already vanished that target (a token tucked by
    /// `ShuffleTargetPermanentIntoLibrary`/`ShuffleTargetPermanentIntoLibraryThenReveal`, CR
    /// 111.7): `object` is by then `Object::Removed`, so this falls back to
    /// [`ResolutionFrame::vanished_permanent_owner`] recorded at that vanish. A vanished token has
    /// no live controller distinct from its owner, so both reads collapse to the same recorded
    /// value in that case.
    pub(crate) fn owner_of_shared_target(&self, object: ObjectId, to_controller: bool) -> PlayerId {
        if matches!(self.objects[object as usize], Object::Removed) {
            let (recorded_object, owner) = self
                .resolution_frame
                .vanished_permanent_owner
                .expect("a vanished shared target must have left a last-known owner behind");
            assert_eq!(
                recorded_object, object,
                "the vanished-owner record is for a different object"
            );
            return owner;
        }
        if to_controller {
            self.controller_of(object)
        } else {
            self.owner_of(object)
        }
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
                    self.resume.sequence = Some(SequenceCont { steps: rest, ctx });
                }
                return;
            }
        }
    }

    /// After a choice answer fully clears the pause, resume any deferred sequence tail into
    /// `events`. The tail may itself pause and re-stash a continuation. Once resolution is truly
    /// unpaused, also finishes any [`ResumeState::spell_finish`] spell whose own resolution was
    /// waiting on this same pause (Sevinne's Reclamation's "may copy this spell" rider).
    pub(crate) fn resume_deferred_sequence(&mut self, events: &mut Vec<Event>) {
        if self.resolution_is_paused() {
            return;
        }
        // Clash (CR 701.22): the controller's keep/bottom scry just cleared — raise the opponent's
        // before running the ability's own tail (the deal-damage/won-clash riders), so both reveals
        // are decided first. An opponent with an empty library raises no scry and falls straight
        // through to the tail below.
        if let Some(opponent) = self.resume.clash_scry.take() {
            crate::pending::raise(
                self,
                crate::pending::ChoiceRequest::ArrangeTop {
                    player: opponent,
                    count: 1,
                    to_graveyard: false,
                },
            );
            if self.resolution_is_paused() {
                return;
            }
        }
        if let Some(cont) = self.resume.sequence.take() {
            self.run_sequence(cont.steps, cont.ctx, events);
        }
        if self.resolution_is_paused() {
            return;
        }
        if let Some((opponent, spell)) = self.resume.demonstrate_opponent_copy.take() {
            self.mint_spell_copies(Amount::Fixed(1), opponent, spell, None, 0, events);
        }
        if self.resolution_is_paused() {
            return;
        }
        if let Some(object) = self.resume.spell_finish.take() {
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
        devoid: false,
        enters_tapped: false,
        enters_tapped_unless: None,
        free_cast_if: None,
        cast_only_during_combat: false,
        approximates: None,
        oracle: None,
        set: "",
        subtypes: &[],
        otags: &[],
        keywords: &[],
        conditional_keywords: &[],
        abilities: &[],
        cycling: None,
        cycling_sacrifice: SacrificeCost::None,
        flashback: None,
        echo: None,
        cumulative_upkeep: None,
        recover: None,
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
        forecast: None,
        may_choose_not_to_untap: false,
        dredge: None,
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
            spent_mana: [0; 6],
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
            game.resume.sequence.is_some(),
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
        assert!(game.resume.sequence.is_some());

        game.finish_answer();
        game.resume_deferred_sequence(&mut events);

        assert!(!game.resolution_is_paused());
        assert!(game.resume.sequence.is_none());
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
        assert!(game.resume.sequence.is_some());

        events.extend(
            game.arrange_top(PlayerId(0), vec![lib[0], lib[1]], vec![])
                .expect("keeping both on top is legal"),
        );
        game.resume_deferred_sequence(&mut events);

        assert!(!game.resolution_is_paused());
        assert!(game.resume.sequence.is_none());
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
