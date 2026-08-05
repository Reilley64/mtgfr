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
        match effect {
            TokenEffect::Create {
                token,
                count,
                who,
                per_opponent,
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
                link_as_twin,
            } => {
                // Mint sequential ids matching the order `apply` will push them (CR 111.1).
                let count = self.resolve_count(count, controller, source, target, x);
                // "…tokens … that attack that opponent this turn if able" (Furygale Flocking):
                // the flattened single-opponent defender every non-`per_opponent` batch
                // binds its tokens to (the one legal defending player in a
                // 1v1 game; with more opponents, still just the first one found — CR 508.1a).
                let flattened_defender = must_attack_defender
                    .then(|| self.living_players().find(|&p| p != controller))
                    .flatten();
                // Who receives the token(s), paired with the must-attack defender (if any) that
                // batch is bound to. `per_opponent` repeats the batch once per opponent without
                // moving the recipient (Eccentric Pestfinder's "for each opponent, *you*
                // create..." — Furygale Flocking's "for each opponent, create two ... tokens ...
                // that attack that opponent" additionally binds each repeat to *that* opponent
                // rather than to the one flattened defender). Combat Calligrapher's
                // tapped-and-attacking rider overrides `who` entirely (CR 111.4): the token is
                // minted under the *attacking* player from `attacking_context`.
                let batches: Vec<(PlayerId, Option<PlayerId>)> =
                    match (attacking_context, per_opponent) {
                        (Some((attacker, _defender)), _) => vec![(attacker, None)],
                        (None, true) => self
                            .living_players()
                            .filter(|&p| p != controller)
                            .map(|opponent| (controller, must_attack_defender.then_some(opponent)))
                            .collect(),
                        (None, false) => self
                            .players_in(who, controller, target)
                            .into_iter()
                            .map(|player| (player, flattened_defender))
                            .collect(),
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
                let def = intern_card_def(def);
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
                        // "Create Stangg Twin…" — tie the minted token to its creator so either
                        // half can name the other once one of them has left (CR 400.7 doesn't
                        // apply; the link is board state on both permanents).
                        if link_as_twin {
                            events.push(Event::TwinLinked { a: source, b: next });
                        }
                        if exile_at_next_end_step {
                            events.push(Event::DelayedTriggerScheduled {
                                controller,
                                source,
                                fire_at: Step::End,
                                effect: Effect::Exile(ExileEffect::Object { object: Some(next) }),
                            });
                        }
                        next += 1;
                    }
                }
                events
            }
            // Treasures reuse the token machinery with the shared `treasure_token` def, entering
            // under the ability's controller or a chosen target player (Prismari Command).
            TokenEffect::CreateTreasure { count, who, tapped } => {
                let Some(recipient) = self.sole_player_in(who, controller, target) else {
                    return Vec::new();
                };
                let count = self.resolve_count(count, controller, source, target, x);
                // Doubling Season doubles Treasures too — they are tokens (CR 614).
                let count = self.token_count_after_replacements(recipient, count);
                let mut events = Vec::new();
                let def = intern_card_def(treasure_token());
                for next in (self.next_object_id()..).take(count as usize) {
                    events.push(Event::TokenCreated {
                        token: next,
                        controller: recipient,
                        def,
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
                let def = self.def_id_of(object);
                // CR 707.2: a copy uses the copied object's *current copiable* values, which
                // include any copy-effect exception rider it already carries — so copying a
                // first-generation copy (a Twinflame haste token, Muddle's myriad form) preserves
                // that rider on the new token, not just its `def`.
                let copied_rider = self.copiable_keywords(object);
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
                    // "…a copy of that creature, except it has haste" (Twinflame, Determined
                    // Iteration, Rionya): the haste is part of the copy's copiable values (CR
                    // 707.2), so a copy of this token keeps it — a `CopyRiderKeywordsGranted`
                    // rider, not a transient `TempBoost`.
                    if haste {
                        events.push(Event::CopyRiderKeywordsGranted {
                            object: token,
                            keywords: HASTE,
                        });
                    }
                    // Carry the copied object's own copiable rider (CR 707.2) onto the new copy —
                    // unioned with any `haste` this effect adds of its own.
                    if !copied_rider.is_empty() {
                        events.push(Event::CopyRiderKeywordsGranted {
                            object: token,
                            keywords: copied_rider,
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
                let def = self.def_id_of(chosen);
                // CR 707.2: if the chosen creature is itself already a copy carrying a rider (a
                // Twinflame haste token you control), Muddle's copy keeps that rider too, unioned
                // with the myriad this ability adds of its own.
                let copied_rider = self.copiable_keywords(chosen);
                const MYRIAD: &[Keyword] = &[Keyword::Myriad];
                let mut events = vec![
                    Event::BecameCopy {
                        object: source,
                        def,
                        until_eot: true,
                        also_types: TypeSet::NONE,
                    },
                    // "…except it has myriad" is a copiable value (CR 707.2): a copy of Muddle's
                    // copied form keeps myriad — a `CopyRiderKeywordsGranted` rider, not a
                    // transient `TempBoost`. Cleared when the until-end-of-turn copy reverts.
                    Event::CopyRiderKeywordsGranted {
                        object: source,
                        keywords: MYRIAD,
                    },
                ];
                if !copied_rider.is_empty() {
                    events.push(Event::CopyRiderKeywordsGranted {
                        object: source,
                        keywords: copied_rider,
                    });
                }
                events
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
                let def = self.def_id_of(source);
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
                    let def = self.def_id_of(id);
                    // CR 707.2: "a copy of that token" carries the copied token's own copy-effect
                    // exception rider (a Twinflame haste token that entered this turn keeps haste
                    // on its copy).
                    let copied_rider = self.copiable_keywords(id);
                    events.push(Event::TokenCreated {
                        token: next,
                        controller: attacker,
                        def,
                        creator: source,
                    });
                    if !copied_rider.is_empty() {
                        events.push(Event::CopyRiderKeywordsGranted {
                            object: next,
                            keywords: copied_rider,
                        });
                    }
                    events.push(Event::Tapped { object: next });
                    events.push(Event::TokenEnteredAttacking {
                        token: next,
                        defender,
                    });
                    events.push(Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::Sacrifice(SacrificeEffect::Object { object: Some(next) }),
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
                let n = self.counters_after_replacements(controller, id, n_raw);
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
