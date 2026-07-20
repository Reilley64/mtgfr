//! Sequence-step choreography — arms that read prior events (or arm/read this same
//! resolution's own scratch) from within a `Sequence`. Peeled out of [`Game::run`]
//! (card-dsl-and-card-pool spec deepen); pure event mint for the underlying variants stays elsewhere. The
//! shared idiom is a reverse scan of `events` for a specific prior emission
//! (`TokenCreated`, `ReanimatedToBattlefield`, `SearchedToBattlefield`) that this step
//! then acts on (attach to it, schedule a delayed return, grant haste, untap).

use crate::*;

impl Game {
    /// Resolve one of the event-readback (or otherwise `Sequence`-scoped) attach/schedule
    /// arms behind [`Game::run`]. Each match arm is a 1:1 relocation of its (formerly
    /// inline) [`Game::run`] body — no behavior change.
    pub(crate) fn run_sequence_step(
        &mut self,
        effect: Effect,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller,
            source,
            target,
            ..
        } = ctx;
        match effect {
            // Feral Appetite: exile the targeted graveyard card, then — CR "if a creature card
            // is exiled this way" — run `then` (mints the Pest token) only if the just-exiled
            // card's own printed type is a creature. Reads the def before the move, the same
            // shape `ExileTargetFromGraveyardCreateTokenCopy` reads `def_of` before it exiles.
            Effect::ExileTargetGraveyardCardThenIfCreature { then } => {
                let object =
                    expect_object_target(target, "exile target graveyard card, then if creature");
                let is_creature = matches!(self.def_of(object).kind, CardKind::Creature { .. });
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                self.push_apply(events, move_event);
                if is_creature {
                    self.run_sequence(then, ctx, events);
                }
            }
            // Untap the permanent this same resolution's own search step already put onto the
            // battlefield (Fabled Passage's "then … untap that land") — reads it back from the
            // SearchedToBattlefield event already recorded in `events` (see the variant doc).
            // No such event yet (the search failed to find, or hasn't run): nothing to untap.
            Effect::UntapSearchedLand => {
                let found = events.iter().rev().find_map(|e| match e {
                    Event::SearchedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                });
                if let Some(permanent) = found {
                    self.push_apply(events, Event::Untapped { object: permanent });
                }
            }
            // Ajani's Chosen: attach the triggering Aura to the token this same resolution's
            // preceding `CreateToken` step already minted — read back from `events`. A non-Aura
            // entering (`entering` is `None`, or its kind isn't Aura) or a missing token is a
            // no-op (guard-return).
            Effect::AttachTriggeringAuraToMintedToken { entering } => {
                let Some(entering) = entering else {
                    return;
                };
                if !matches!(self.def_of(entering).kind, CardKind::Aura) {
                    return;
                }
                let Some(token) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: entering,
                        host: Some(token),
                    },
                );
            }
            // A reflexive "when you do" trigger (CR 603.3b — Forum Filibuster): the "you do" is
            // that this resolution's preceding `CreateToken` step minted a token (read back from
            // `events`, the same idiom as `AttachTriggeringAuraToMintedToken` above). No such
            // token: no reflexive trigger (guard-return). Otherwise enqueue each `then` effect as
            // its own reflexive triggered ability — a separate, respondable stack object placed
            // the next time a player would get priority — threading the minted token in.
            Effect::ReflexiveTrigger { then } => {
                let Some(token) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                self.queue_reflexive_trigger(controller, source, then, token);
            }
            // The reflexive ability's own resolution: return the chosen graveyard card (CR 601.2c
            // target, may be `None` — "up to one") to the battlefield attached to the minted
            // `token`. Guard-return (CR 608.2b) if the token has left the battlefield since — with
            // the host gone the returned card can't be attached, so nothing happens.
            Effect::ReturnFromGraveyardAttachedToToken { token, .. } => {
                let Some(token) = token.filter(|&t| self.as_permanent(t).is_some()) else {
                    return;
                };
                let Some(card) = target.and_then(Target::object_id) else {
                    return;
                };
                let event = self.reanimate_event(card, controller, false);
                let Event::ReanimatedToBattlefield { permanent, .. } = event else {
                    unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
                };
                self.push_apply(events, event);
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: permanent,
                        host: Some(token),
                    },
                );
            }
            // Animate Dead: attach this Aura to the creature this same resolution's preceding
            // `ReanimateToBattlefield` step already put onto the battlefield — read back from
            // `events`. No such event yet: nothing to attach to (guard-return).
            Effect::AttachSelfToReanimated => {
                let Some(permanent) = events.iter().rev().find_map(|e| match e {
                    Event::ReanimatedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: source,
                        host: Some(permanent),
                    },
                );
            }
            // Fractal Harness: attach this Equipment to the token this same resolution's
            // preceding `CreateToken` step already minted — read back from `events`, the same
            // idiom as `AttachSelfToReanimated` above. No such token yet: nothing to attach to
            // (guard-return).
            Effect::AttachSelfToMintedToken => {
                let Some(token) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: source,
                        host: Some(token),
                    },
                );
            }
            // Scriv, the Obligator: attach the Aura token this same resolution's preceding
            // `CreateToken` step just minted to the ability's chosen target (a creature an opponent
            // controls) — the mirror of `AttachSelfToMintedToken` above, attaching the *minted
            // token* rather than the source. No token minted yet, a non-Aura token, or a
            // non-object target: nothing to attach (guard-return).
            // ponytail: only an Aura can be attached (CR 303); a non-Aura minted token is a no-op
            // rather than a phantom attachment. The pool mints only the Contract Aura here.
            Effect::AttachMintedAuraToTarget { .. } => {
                let Some(host) = target.and_then(Target::object_id) else {
                    return;
                };
                let Some(aura) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                if !matches!(self.def_of(aura).kind, CardKind::Aura) {
                    return;
                }
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: aura,
                        host: Some(host),
                    },
                );
            }
            // Gift of Immortality: schedule the delayed return of this Aura (CR 603.7), attached
            // to the creature this same resolution's preceding `ReanimateDyingEnchantedCreature`
            // step just reanimated — read back from `events`, mirroring `AttachSelfToReanimated`'s
            // idiom above. No such event yet (the enchanted creature wasn't reanimated): nothing
            // to schedule (guard-return).
            Effect::ScheduleReturnThisAuraAttachedToReanimated => {
                let Some(permanent) = events.iter().rev().find_map(|e| match e {
                    Event::ReanimatedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::ReturnThisAuraAttachedTo {
                            creature: Some(permanent),
                        },
                    },
                );
            }
            // Cauldron Dance: grant haste to the creature this same resolution's preceding
            // `ReanimateToBattlefield` step just put onto the battlefield — read back from
            // `events`, mirroring `ScheduleReturnThisAuraAttachedToReanimated`'s idiom above —
            // then schedule its return to hand at the next end step (CR 603.7). No such event
            // yet (the reanimation target was illegal): nothing to grant or schedule
            // (guard-return).
            Effect::ScheduleReturnReanimatedToHand => {
                let Some(permanent) = events.iter().rev().find_map(|e| match e {
                    Event::ReanimatedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                }) else {
                    return;
                };
                const HASTE: &[Keyword] = &[Keyword::Haste];
                let source_name = self.source_name_of(source);
                self.push_apply(
                    events,
                    Event::TempBoost {
                        object: permanent,
                        power: 0,
                        toughness: 0,
                        keywords: HASTE,
                        source_name,
                    },
                );
                self.push_apply(
                    events,
                    Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::ReturnObjectToHand {
                            object: Some(permanent),
                        },
                    },
                );
            }
            // Screams from Within: the immediate dies-return, choosing a new host (unlike Gift
            // of Immortality's same-creature return above). Pauses via the shared helper — see
            // its doc comment.
            Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost => {
                self.return_aura_from_graveyard_attached_to_chosen_host(source, events)
            }
            // Ghoulish Impetus: schedule the same choose-host return above at the next end step
            // (CR 603.7), mirroring `ScheduleReturnThisAuraAttachedToReanimated`'s emit shape. No
            // read-back needed — this Aura's own `source` is all the delayed payload needs.
            Effect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost => {
                self.push_apply(
                    events,
                    Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost,
                    },
                );
            }
            _ => unreachable!("sequence-step choreo received a non-family effect"),
        }
    }
}
