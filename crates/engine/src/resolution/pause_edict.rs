//! Edict / multiplayer fan-out pause family — sacrifices, graveyard exile, votes, keep-one.
//!
//! Pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`]; dig/edict *handlers* stay in [`crate::pending::handlers`].

use crate::*;

impl Game {
    /// Resolve the matching edict / fan-out pause effect (may auto-complete when no pause).
    pub(crate) fn run_edict_pause(
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
            // A multi-player sacrifice edict (Deadly Brew, Promise of Loyalty) pauses per
            // affected player.
            Effect::Choice(ChoiceEffect::EachPlayerSacrifices {
                scope,
                keep_one,
                filter,
                life_loss,
                then,
            }) => self.sacrifice_edict(
                scope, keep_one, filter, life_loss, then, controller, source, events,
            ),
            // A multi-player graveyard-exile fan-out (Augusta) pauses per affected player; its
            // reflexive counter payoff rides in the enclosing `Sequence`, resumed once all answer.
            // ponytail: this "when you do" is CR 603.3b's separate reflexive trigger, modeled here
            // as a same-resolution sequenced payoff (no response window). `Effect::Zone(ZoneEffect::ReflexiveTrigger)`
            // is the real-stack-object primitive; migrate to it when Augusta's "you do" condition
            // (its own exile fan-out, not a token creation) is threadable through it.
            Effect::Choice(ChoiceEffect::EachPlayerExilesFromGraveyard) => {
                self.resolution_frame.nonland_cards_exiled_this_way = 0;
                pending::raise(
                    self,
                    pending::ChoiceRequest::NextGraveyardExile {
                        remaining: self.apnap_order(),
                        source,
                    },
                )
            }
            // Relic of Progenitus: "Target player exiles a card from their graveyard." The one-
            // player special case of the fan-out above — no `follow_up`, no payoff.
            Effect::Choice(ChoiceEffect::TargetPlayerExilesFromGraveyard { .. }) => {
                let Some(Target::Player(player)) = target else {
                    panic!(
                        "target player exiles from graveyard resolves with a chosen player target"
                    );
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::NextGraveyardExile {
                        remaining: vec![player],
                        source,
                    },
                )
            }
            // The caster-directed keep-one-of-each-type sweep (Tragic Arrogance): for each player,
            // the caster picks up to one nonland permanent of each type to keep; the rest are
            // sacrificed. Pauses per player on a CasterKeepPermanents choice answered by the caster.
            Effect::Choice(ChoiceEffect::CasterKeepsOneOfEachTypePerPlayer) => pending::raise(
                self,
                pending::ChoiceRequest::NextCasterKeep {
                    remaining: self.apnap_order(),
                    caster: controller,
                    source,
                },
            ),
            // Nils' end step: for each player, its controller puts a +1/+1 counter on up to one
            // creature that player controls. Pauses per player on a ChooseCounterTargetForPlayer.
            Effect::Choice(ChoiceEffect::EachPlayerControllerChoosesCounterTarget) => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::NextCounterTarget {
                        remaining: self.apnap_order(),
                        chooser: controller,
                        source,
                    },
                )
            }
            // Join forces (Collective Voyage): "Starting with you, each player may pay any amount
            // of mana." A per-player payment round; the X-scaled payoff rides in the enclosing
            // `Sequence`, resumed once every player has answered — the vote round's twin.
            Effect::Choice(ChoiceEffect::JoinForcesPayMana) => {
                self.resolution_frame.join_forces_mana = 0;
                pending::raise(
                    self,
                    pending::ChoiceRequest::NextJoinForcesPayment {
                        remaining: self.turn_order_from(controller),
                        source,
                    },
                )
            }
            // Council's dilemma (Fateful Tempest): a per-player vote round pauses each seat on a
            // CastVote choice; the tally-scaled payoff rides in the enclosing `Sequence`, resumed
            // once every player has voted (the same deferred-tail path as the graveyard fan-out).
            Effect::Choice(ChoiceEffect::CouncilsDilemmaVote { options }) => {
                self.resolution_frame.council_past_votes = 0;
                self.resolution_frame.council_present_votes = 0;
                pending::raise(
                    self,
                    pending::ChoiceRequest::NextVote {
                        remaining: self.turn_order_from(controller),
                        source,
                        options,
                    },
                )
            }
            // Conundrum Sphinx's attack trigger: "each player chooses a card name" (CR 101.4
            // default APNAP order — the trigger carries no "starting with you," but its
            // controller is always the active player, since only the active player's creatures
            // attack). Each seat pauses on a ChooseCardName; the reveal-and-match resolves inside
            // that seat's own answer (see `PendingChoice::ChooseCardName`'s doc), not here.
            Effect::Choice(ChoiceEffect::EachPlayerNamesCardThenRevealsTop) => pending::raise(
                self,
                pending::ChoiceRequest::NextCardName {
                    remaining: self.apnap_order(),
                    source,
                },
            ),
            // Brudiclad: "you may choose a token you control; if you do, each other token you
            // control becomes a copy of that token." Pauses on a ChooseTokenToCopy choice; with no
            // token to choose there's nothing to convert (guarded like MaySacrifice).
            Effect::Choice(ChoiceEffect::EachOtherTokenBecomesCopyOfChosen) => pending::raise(
                self,
                pending::ChoiceRequest::ChooseTokenToCopy {
                    player: controller,
                    source,
                },
            ),
            // Spirit of Resilience: "put a +1/+1 counter on this creature, then you may have this
            // creature become a copy of an artifact or creature card from among those cards until
            // end of turn." Places the counter, then pauses on a ChooseCopyCardFromList choice
            // over the artifact/creature cards that left; no copyable card means no pause.
            Effect::Choice(ChoiceEffect::PutCounterThenMayBecomeCopyOfCardFromList { cards }) => {
                let count = self.counters_after_replacements(source, 1);
                if count > 0 {
                    self.push_apply(
                        events,
                        Event::CountersPlaced {
                            object: source,
                            count,
                            source_name: self.source_name_of(source),
                        },
                    );
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseCopyCardFromList {
                        player: controller,
                        source,
                        cards,
                    },
                )
            }
            // A forced sacrifice the affected player directs (Lotus Field's ETB "sacrifice two
            // lands", Smothering Abomination's upkeep "sacrifice a creature") pauses on a
            // ChooseOwnSacrifices choice; with count-or-fewer legal permanents it resolves
            // immediately instead (CR 700.2's "as many as possible").
            Effect::Choice(ChoiceEffect::SacrificeOwn { filter, count }) => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseOwnSacrifices {
                        player: controller,
                        source,
                        filter,
                        count,
                    },
                );
                if !self.resolution_is_paused() {
                    let options = self.edict_options(controller, filter);
                    self.sacrifice_ids(&options, controller, events);
                }
            }
            // Annihilator N (Eldrazi Conscription): the defending player, not the controller,
            // directs the forced sacrifice — same ChooseOwnSacrifices machinery, any permanent.
            Effect::Choice(ChoiceEffect::DefendingPlayerSacrifices { count, defender }) => {
                let defender = defender.expect("filled from attack context when placed");
                let filter = PermanentFilter::default();
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseOwnSacrifices {
                        player: defender,
                        source,
                        filter,
                        count: count as u32,
                    },
                );
                if !self.resolution_is_paused() {
                    let options = self.edict_options(defender, filter);
                    self.sacrifice_ids(&options, defender, events);
                }
            }
            // Treva's Ruins' own ETB trigger: "sacrifice it unless you return a non-Lair land you
            // control." Pauses on a candidate-land pick (or sacrifices outright with none).
            Effect::Choice(ChoiceEffect::SacrificeSelfUnlessReturnLand { filter }) => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::SacrificeUnlessReturnLand {
                        player: controller,
                        source,
                        filter,
                    },
                );
                if !self.resolution_is_paused() {
                    self.run(
                        Effect::Destroy(DestroyEffect::SacrificeObject {
                            object: Some(source),
                        }),
                        ResolveCtx {
                            controller,
                            source,
                            target: None,
                            targets_second: TargetList::default(),
                            x: 0,
                            spent_mana: [0; 6],
                        },
                        events,
                    );
                }
            }
            _ => unreachable!("edict pause family received a non-family effect"),
        }
    }
}
