//! Tokens-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_tokens(
        &self,
        effect: TokenEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            TokenEffect::Create {
                token,
                count,
                controller: token_controller,
                // `enters_with` needs the just-minted token already in game state to route
                // through `counters_after_replacements` (it reads the token's controller), so it
                // can't be placed here — `execute_effect` is pure (`&self`). `Game::run`
                // special-cases `CreateToken` to place counters right after applying this batch;
                // this arm only reaches direct `execute_effect` callers (a mana ability, a (CR 605, CR 113)
                // sacrifice edict's `then`), none of which mint a token with counters today.
                enters_with: _,
                set_base_pt,
                exile_at_next_end_step,
                enters_tapped_and_attacking: _,
                attacking_context,
                must_attack_defender,
            } => {
                // Mint sequential ids matching the order `apply` will push them (CR 111.1).
                let count = self.resolve_count(count, controller, source, target, x);
                // "…tokens … that attack that opponent this turn if able" (Furygale Flocking):
                // the flattened single-opponent defender every `controller` value but
                // `one_per_opponent` binds its tokens to (the one legal defending player in a
                // 1v1 game; with more opponents, still just the first one found — CR 508.1a).
                let flattened_defender = must_attack_defender
                    .then(|| self.living_players().find(|&p| p != controller))
                    .flatten();
                // Who receives the token(s), paired with the must-attack defender (if any) that
                // recipient's batch is bound to: the ability's own controller by default, the
                // shared target's controller (Beast Within's "its controller creates..."), one
                // copy per opponent under that opponent (a hostile edict), or one copy per
                // opponent under the ability's own controller (Eccentric Pestfinder's "for each
                // opponent, you create..." — Furygale Flocking's "for each opponent, create
                // two ... tokens ... that attack that opponent" additionally binds each
                // opponent's own batch to *that* opponent, not the flattened one). Combat
                // Calligrapher's tapped-and-attacking rider overrides all of that (CR 111.4): the
                // token is minted under the *attacking* player from `attacking_context`, not the
                // ability's controller.
                let batches: Vec<(PlayerId, Option<PlayerId>)> = match attacking_context {
                    Some((attacker, _defender)) => vec![(attacker, None)],
                    None => match token_controller {
                        TokenController::You => vec![(controller, flattened_defender)],
                        TokenController::TargetController => {
                            let object =
                                expect_object_target(target, "a token's target-controller");
                            vec![(self.controller_of(object), flattened_defender)]
                        }
                        TokenController::EachOpponent => self
                            .living_players()
                            .filter(|&p| p != controller)
                            .map(|p| (p, flattened_defender))
                            .collect(),
                        TokenController::OnePerOpponent => self
                            .living_players()
                            .filter(|&p| p != controller)
                            .map(|opponent| (controller, must_attack_defender.then_some(opponent)))
                            .collect(),
                        // Questing Phelddagrif's green rider: "Target opponent creates a 1/1 ...
                        // Hippo ... token" — same `Target::Player` resolution as `TargetPlayer`
                        // above, just narrowed to an opponent by `Effect::target`'s `TargetSpec`.
                        TokenController::TargetPlayer | TokenController::TargetOpponent => {
                            let Some(Target::Player(player)) = target else {
                                panic!(
                                    "a token's target-player recipient resolves with a chosen player target"
                                );
                            };
                            vec![(player, flattened_defender)]
                        }
                        // Death by Dragons: "Each player other than target player creates a..." —
                        // the chosen Player target is the one player excluded, not the recipient.
                        TokenController::EachOtherPlayer => {
                            let Some(Target::Player(excluded)) = target else {
                                panic!(
                                    "a token's each-other-player recipient set resolves with a chosen player target"
                                );
                            };
                            self.living_players()
                                .filter(|&p| p != excluded)
                                .map(|p| (p, flattened_defender))
                                .collect()
                        }
                    },
                };
                // "…create an X/X … token …, where X is …" (Manaform Hellkite): bake the
                // resolved base power/toughness straight into the minted def before any copies
                // are minted — a genuine base-P/T set, not `enters_with`'s counters. Resolving
                // needs no just-minted game state (unlike `enters_with`), so it's safe here.
                let mut def = token;
                if let Some(amount) = set_base_pt {
                    let n = self.resolve_amount(amount, controller, source, target, x);
                    if let CardKind::Creature {
                        power, toughness, ..
                    } = &mut def.kind
                    {
                        *power = n;
                        *toughness = n;
                    }
                }
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for (recipient, batch_defender) in batches {
                    // Doubling Season (CR 614): each batch may enter under a different player
                    // (Combat Calligrapher), so apply the recipient's token-creation replacements
                    // per batch.
                    let count = self.token_count_after_replacements(recipient, count);
                    for _ in 0..count {
                        events.push(Event::TokenCreated {
                            token: next,
                            controller: recipient,
                            def,
                            creator: source,
                        });
                        // Attach the "attacks this turn if able" requirement to each minted token
                        // — bound to this batch's own defender (see `batches` above).
                        if let Some(defender) = batch_defender {
                            events.push(Event::MustAttackDeclared {
                                object: next,
                                defender,
                            });
                        }
                        // "…creates a tapped … token … that's attacking that opponent" (Combat
                        // Calligrapher): the token enters already tapped and joins combat as an
                        // attacker against the baked defender — CR 508.4, not a declared attack,
                        // so `TokenEnteredAttacking` (not `AttackerDeclared`) carries it.
                        if let Some((_attacker, defender)) = attacking_context {
                            events.push(Event::Tapped { object: next });
                            events.push(Event::TokenEnteredAttacking {
                                token: next,
                                defender,
                            });
                        }
                        // "Exile that token at the beginning of the next end step." (Manaform
                        // Hellkite, CR 603.7b): schedule a delayed exile against this specific
                        // minted token, not a re-scan (mirrors `CreateTokenCopy`'s
                        // `sacrifice_at_next_end_step`).
                        if exile_at_next_end_step {
                            events.push(Event::DelayedTriggerScheduled {
                                controller,
                                source,
                                fire_at: Step::End,
                                effect: Effect::Exile(ExileEffect::Object {
                                    object: Some(next),
                                }),
                            });
                        }
                        next += 1;
                    }
                }
                events
            }
            // Treasures reuse the token machinery with the shared `treasure_token` def, entering
            // under the ability's controller or a chosen target player (Prismari Command).
            TokenEffect::CreateTreasure {
                count,
                target_player,
                tapped,
            } => {
                let recipient = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!(
                            "target-player create-treasure resolves with a chosen player target"
                        );
                    };
                    player
                } else {
                    controller
                };
                let count = self.resolve_count(count, controller, source, target, x);
                // Doubling Season doubles Treasures too — they are tokens (CR 614).
                let count = self.token_count_after_replacements(recipient, count);
                let mut events = Vec::new();
                for next in (self.next_object_id()..).take(count as usize) {
                    events.push(Event::TokenCreated {
                        token: next,
                        controller: recipient,
                        def: treasure_token(),
                        creator: source,
                    });
                    // "create a number of tapped Treasure tokens" (Goldvein Hydra): each minted
                    // Treasure enters already tapped.
                    if tapped {
                        events.push(Event::Tapped { object: next });
                    }
                }
                events
            }
            // A token copy of the target creature: reuse the token machinery with the target's
            // current copiable characteristics (its `CardDef`). If the target is itself a token,
            // `def_of` returns its token def — which is exactly what we want to copy.
            TokenEffect::CreateCopy {
                count,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                haste,
                entering,
                ..
            } => {
                const HASTE: &[Keyword] = &[Keyword::Haste];
                // Riku of Two Reflections: "create a token that's a copy of that creature" reads
                // the entering-permanent context instead of a chosen target (see `entering`'s doc).
                let object =
                    entering.unwrap_or_else(|| expect_object_target(target, "a token copy"));
                let def = self.def_of(object);
                let count = self.resolve_count(count, controller, source, target, x);
                // Doubling Season (CR 614): the copies enter under `controller`.
                let count = self.token_count_after_replacements(controller, count);
                let mut events = Vec::new();
                for token in (self.next_object_id()..).take(count as usize) {
                    events.push(Event::TokenCreated {
                        token,
                        controller,
                        def,
                        creator: source,
                    });
                    // Determined Iteration: "The token created this way gains haste."
                    if haste {
                        events.push(Event::TempBoost {
                            object: token,
                            power: 0,
                            toughness: 0,
                            keywords: HASTE,
                            source_name,
                        });
                    }
                    // Determined Iteration: "Sacrifice it at the beginning of the next end step"
                    // — schedule the delayed sacrifice against this specific minted token, not a
                    // re-scan (see `Effect::Destroy(DestroyEffect::SacrificeObject)`).
                    if sacrifice_at_next_end_step {
                        events.push(Event::DelayedTriggerScheduled {
                            controller,
                            source,
                            fire_at: Step::End,
                            effect: Effect::Sacrifice(SacrificeEffect::Object {
                                object: Some(token),
                            }),
                        });
                    }
                    // Twinflame: "Exile those tokens at the beginning of the next end step" —
                    // schedule the delayed exile against this specific minted token, not a
                    // re-scan (mirrors `CreateToken`'s own `exile_at_next_end_step`).
                    if exile_at_next_end_step {
                        events.push(Event::DelayedTriggerScheduled {
                            controller,
                            source,
                            fire_at: Step::End,
                            effect: Effect::Exile(ExileEffect::Object {
                                object: Some(token),
                            }),
                        });
                    }
                }
                events
            }
            // Muddle, the Ever-Changing's magecraft ability: become a copy of the chosen target
            // until end of turn, except it has myriad — the copy overwrite mirrors
            // `Game::answer_enter_as_copy`'s `BecameCopy`, and the myriad grant reuses the same
            // "gains a keyword" `TempBoost` shape that answer's `gains_haste` rider uses.
            TokenEffect::BecomeCopyOfTargetCreatureGainingMyriad { .. } => {
                let chosen =
                    expect_object_target(target, "become-copy-of-target-creature-gaining-myriad");
                let def = self.def_of(chosen);
                const MYRIAD: &[Keyword] = &[Keyword::Myriad];
                vec![
                    Event::BecameCopy {
                        object: source,
                        def,
                        until_eot: true,
                    },
                    Event::TempBoost {
                        object: source,
                        power: 0,
                        toughness: 0,
                        keywords: MYRIAD,
                        source_name,
                    },
                ]
            }
            // Myriad's payload (CR 702.114a): for each opponent other than the defending player,
            // mint a token copy of the attacker's current (possibly copied) characteristics that
            // enters tapped and attacking that opponent (`Event::Tapped`/`Event::TokenEnteredAttacking`,
            // never `AttackerDeclared` — CR 508.4, so a minted copy can't re-trigger myriad), then
            // schedule it to be exiled at the true end of combat.
            TokenEffect::MyriadTokenCopies { attacking_context } => {
                let (attacker, defender) = attacking_context.expect(
                    "filled in by Game::queue_myriad_triggers when the ability is synthesized",
                );
                let def = self.def_of(source);
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for opponent in self.living_players() {
                    if opponent == attacker || opponent == defender {
                        continue;
                    }
                    // Doubling Season (CR 614): each copy is its own token creation.
                    let count = self.token_count_after_replacements(attacker, 1);
                    for _ in 0..count {
                        let token = next;
                        events.push(Event::TokenCreated {
                            token,
                            controller: attacker,
                            def,
                            creator: source,
                        });
                        events.push(Event::Tapped { object: token });
                        events.push(Event::TokenEnteredAttacking {
                            token,
                            defender: opponent,
                        });
                        events.push(Event::DelayedTriggerScheduled {
                            controller: attacker,
                            source,
                            fire_at: Step::EndCombat,
                            effect: Effect::Exile(ExileEffect::Object {
                                object: Some(token),
                            }),
                        });
                        next += 1;
                    }
                }
                events
            }
            // Redoubled Stormsinger: "for each creature token you control that entered this
            // turn, create a tapped and attacking token that's a copy of that token. At the
            // beginning of the next end step, sacrifice those tokens." No chosen target — scan
            // the attacker's own battlefield for the matching tokens (CR 508.4: each mint enters
            // tapped and attacking, never declared, so it can't re-trigger this ability).
            TokenEffect::CopyEachEnteredThisTurnTokenTappedAttacking { attacking_context } => {
                let (attacker, defender) = attacking_context
                    .expect("filled in by contextualize_effect from the Attacks trigger context");
                let filter = PermanentFilter {
                    types: TypeSet::CREATURE,
                    token: TokenFilter::Token,
                    controller: FilterController::You,
                    entered_this_turn: true,
                    ..Default::default()
                };
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    if !self.permanent_matches(&filter, id, attacker, Some(source)) {
                        continue;
                    }
                    let def = self.def_of(id);
                    events.push(Event::TokenCreated {
                        token: next,
                        controller: attacker,
                        def,
                        creator: source,
                    });
                    events.push(Event::Tapped { object: next });
                    events.push(Event::TokenEnteredAttacking {
                        token: next,
                        defender,
                    });
                    events.push(Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::Sacrifice(SacrificeEffect::Object {
                            object: Some(next),
                        }),
                    });
                    next += 1;
                }
                events
            }
        }
    }

    /// [`TokenEffect::Create`]'s `enters_with` choreography: mint the token(s) (unchanged
    /// batch via `execute_effect`), then — "Put X +1/+1 counters on it" (Deekah's Magecraft
    /// Fractal) — place `enters_with` counters on each minted token, routed through the same
    /// doubler/Hardened-Scales replacement pipeline as a spell's own `EntersWithCounters`
    /// (`Game::resolve_spell`'s enters-with path). `counters_after_replacements` reads the
    /// token's controller off game state, so the mint must already be applied — mirrors
    /// `resolve_spell` applying `PermanentEntered` before reading its counters.
    pub(crate) fn resolve_create_token(
        &mut self,
        effect: TokenEffect,
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
        let TokenEffect::Create { enters_with, .. } = effect else {
            unreachable!("resolve_create_token received a non-family effect")
        };
        let evs = self.execute_effect(Effect::Token(effect), controller, source, target, x);
        self.apply_all(&evs);
        let minted: Vec<ObjectId> = evs
            .iter()
            .filter_map(|e| match e {
                Event::TokenCreated { token, .. } => Some(*token),
                _ => None,
            })
            .collect();
        events.extend(evs);
        let n_raw = self.resolve_amount(enters_with, controller, source, target, x);
        if n_raw > 0 {
            for id in minted {
                let n = self.counters_after_replacements(id, n_raw);
                if n > 0 {
                    self.push_apply(
                        events,
                        Event::CountersPlaced {
                            object: id,
                            count: n,
                            source_name: self.def_of(source).name,
                        },
                    );
                }
            }
        }
    }
}
