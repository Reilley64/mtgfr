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
                count,
                down_to_fewest,
                lose_game_if_short,
                then,
            }) => {
                let count = self.resolve_count(count, controller, source, target, ctx.x);
                self.sacrifice_edict(
                    scope,
                    keep_one,
                    filter,
                    life_loss,
                    count,
                    down_to_fewest,
                    lose_game_if_short,
                    then,
                    controller,
                    source,
                    events,
                )
            }
            // Syphon Mind's "Each other player discards a card." A fan-out over the scoped seats
            // in APNAP order (empty hands skipped), tallying `cards_discarded_this_way` so the
            // enclosing `Sequence`'s draw step reads it. Balance's `down_to_fewest` measures the
            // smallest hand among those seats first, and each of them pitches its own excess.
            Effect::Choice(ChoiceEffect::EachPlayerDiscards {
                scope,
                down_to_fewest,
            }) => {
                self.resolution_frame.cards_discarded_this_way = 0;
                let affected: Vec<PlayerId> = self
                    .apnap_order()
                    .into_iter()
                    .filter(|&p| scope != EdictScope::EachOpponent || p != controller)
                    .collect();
                let floor = down_to_fewest.then(|| {
                    affected
                        .iter()
                        .map(|&p| self.hand_of(p).len() as u32)
                        .min()
                        .unwrap_or(0)
                });
                pending::raise(
                    self,
                    pending::ChoiceRequest::NextDiscardEdict {
                        remaining: affected,
                        source,
                        floor,
                    },
                )
            }
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
                        prevent_up_to: None,
                    },
                )
            }
            // Kudzu: "That land's controller may attach this Aura to a land of their choice."
            // The same choose-host pause a deployed Aura raises, with two differences the card
            // spells out — the chooser is the triggering permanent's controller, not this Aura's,
            // and the "may" makes declining legal (an unattached Aura is then swept by CR 704.5m).
            // No legal land means no pause at all, and the same sweep takes it.
            Effect::Choice(ChoiceEffect::TriggeringPlayerMayAttachThisAuraToChosen {
                filter,
                player,
            }) => {
                let chooser = player.expect("the triggering player is filled in at placement");
                let candidates: Vec<ObjectId> = self
                    .battlefield()
                    .into_iter()
                    .filter(|&id| self.permanent_matches(&filter, id, chooser, Some(source)))
                    .collect();
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseAttachHost {
                        player: chooser,
                        attachment: source,
                        candidates,
                        optional: true,
                    },
                )
            }
            // Power Leak: "that player may pay any amount of mana … Prevent X of that damage."
            // The same payment pause as join forces with a one-seat guest list, and a cap that
            // turns the payment into a prevention shield on the payer instead of a shared X.
            Effect::Choice(ChoiceEffect::TriggeringPlayerMayPayAnyAmountToPrevent {
                prevent_up_to,
                player,
            }) => {
                let payer = player.expect("the triggering player is filled in at placement");
                let cap = self
                    .resolve_amount(prevent_up_to, payer, source, None, 0)
                    .clamp(0, i32::from(u8::MAX)) as u8;
                pending::raise(
                    self,
                    pending::ChoiceRequest::NextJoinForcesPayment {
                        remaining: vec![payer],
                        source,
                        prevent_up_to: Some(cap),
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
            // Archangel of Strife: "As this creature enters, each player chooses war or peace."
            // No "starting with you" wording, so CR 101.4 default APNAP order, unlike Fateful
            // Tempest's caster-relative `turn_order_from`. Reuses the council's-dilemma `CastVote`
            // fan-out — `answer_vote` also recognizes "war"/"peace" ballots, writing each answer
            // to that player's own `Player::war_choices`, keyed by this source, instead of a tally.
            Effect::Choice(ChoiceEffect::EachPlayerChoosesWarOrPeace) => pending::raise(
                self,
                pending::ChoiceRequest::NextVote {
                    remaining: self.apnap_order(),
                    source,
                    options: &["war", "peace"],
                },
            ),
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
                let count = self.counters_after_replacements(controller, source, 1);
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
            Effect::Choice(ChoiceEffect::SacrificeOwn {
                filter,
                count,
                opponent_chooses,
            }) => {
                // Demonic Hordes hands the pick to the next living seat in turn order; every other
                // card leaves it with the player losing the permanents (CR 701.16a).
                let chooser = if opponent_chooses {
                    self.next_player(controller)
                } else {
                    controller
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseOwnSacrifices {
                        player: chooser,
                        owner: controller,
                        source,
                        filter,
                        count,
                    },
                );
                if !self.resolution_is_paused() {
                    let options = self.edict_options(controller, filter, Some(source));
                    self.sacrifice_ids(&options, events);
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
                        owner: defender,
                        source,
                        filter,
                        count: count as u32,
                    },
                );
                if !self.resolution_is_paused() {
                    let options = self.edict_options(defender, filter, Some(source));
                    self.sacrifice_ids(&options, events);
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
                        Effect::Sacrifice(SacrificeEffect::Object {
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
