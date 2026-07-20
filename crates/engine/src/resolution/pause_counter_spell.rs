//! Counter-spell destination / unless-pays pause (and forced-bottom) family.
//!
//! Behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in [`crate::pending`].

use crate::*;

impl Game {
    /// Resolve CounterTargetSpell arms that need `&mut self` (unless-pays pause, destination
    /// choice, or forced library-bottom apply). Unconditional counters stay on the catch-all mint path.
    pub(crate) fn run_counter_spell(
        &mut self,
        effect: Effect,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller,
            source,
            target,
            x,
            ..
        } = ctx;
        match effect {
            // "Counter target spell unless its controller pays {N}" (CR 701.5c-style): pause on
            // a PayOrCounter choice for the *target spell's* controller instead of countering
            // outright. `unless_pays: None` falls through to the catch-all's unconditional counter.
            Effect::CounterTargetSpell {
                unless_pays: Some(amount),
                ..
            } => {
                let original = expect_object_target(target, "a spell to counter");
                // If the target already left the stack (countered/resolved in response), there's
                // nothing to hold hostage — same no-op as the unconditional counter (CR 608.2b).
                if !matches!(self.objects[original as usize], Object::Spell(_)) {
                    return;
                }
                let generic = self.resolve_count(amount, controller, source, target, x);
                pending::raise(
                    self,
                    pending::ChoiceRequest::PayOrCounter {
                        player: self.controller_of(original),
                        cost: Cost {
                            generic: generic as u8,
                            ..Cost::FREE
                        },
                        spell: original,
                    },
                );
            }
            // Hinder's destination rider (CR 701.5b — `countered_dest`): pause this ability's
            // controller on a top/bottom pick before the countered card moves, unless it's not
            // going to a graveyard anyway — already left the stack / uncounterable (CR 608.2b /
            // 701.5g), or exiles instead (flashback/escape, CR 702.34e/702.19d; Quintorius's CR
            // 614.6 bottom-library redirect) — those cases fall through to the ordinary
            // `counter_spell`, which has nothing left for this rider to redirect.
            Effect::CounterTargetSpell {
                unless_pays: None,
                countered_dest: Some(CounteredDest::LibraryTopOrBottom),
                ..
            } => {
                let original = expect_object_target(target, "a spell to counter");
                let is_spell = matches!(self.objects[original as usize], Object::Spell(_));
                let goes_to_graveyard = is_spell
                    && !self.def_of(original).uncounterable
                    && !self.spell(original).flashback
                    && !self.spell(original).escape
                    && !self
                        .play_permissions
                        .stack_object_bottoms_library_on_leave
                        .iter()
                        .any(|&flagged| self.current_id(flagged) == original);
                if !goes_to_graveyard {
                    let evs = self.counter_spell(original);
                    self.apply_all(&evs);
                    events.extend(evs);
                    return;
                }
                pending::raise_choice(
                    self,
                    PendingChoice::ChooseCounteredSpellDestination {
                        player: controller,
                        spell: original,
                    },
                );
            }
            // Spell Crumple's destination rider (CR 701.5b — `countered_dest`): the same "would
            // it actually reach a graveyard" gate as the `LibraryTopOrBottom` arm above, but
            // forced straight to the bottom — no player choice, so no pause. Unlike that arm
            // (whose pause answer never checks this), a copy (CR 707.10a) ceases to exist here
            // rather than tucking — reusing `Game::is_copy_object`, the #213 copy guard.
            Effect::CounterTargetSpell {
                unless_pays: None,
                countered_dest: Some(CounteredDest::LibraryBottom),
                ..
            } => {
                let original = expect_object_target(target, "a spell to counter");
                let is_spell = matches!(self.objects[original as usize], Object::Spell(_));
                let goes_to_graveyard = is_spell
                    && !self.def_of(original).uncounterable
                    && !self.spell(original).flashback
                    && !self.spell(original).escape
                    && !self
                        .play_permissions
                        .stack_object_bottoms_library_on_leave
                        .iter()
                        .any(|&flagged| self.current_id(flagged) == original);
                if !goes_to_graveyard {
                    let evs = self.counter_spell(original);
                    self.apply_all(&evs);
                    events.extend(evs);
                    return;
                }
                let evs = if self.is_copy_object(original) {
                    vec![Event::SpellCeasedToExist { spell: original }]
                } else {
                    vec![Event::TuckedToLibrary {
                        card: self.next_object_id(),
                        from: original,
                        to_top: false,
                        second_from_top: false,
                    }]
                };
                self.apply_all(&evs);
                events.extend(evs);
            }
            _ => unreachable!("counter-spell family received a non-family effect"),
        }
    }
}
