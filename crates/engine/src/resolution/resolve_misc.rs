//! Misc resolution choreography that needs `&mut self` — the pause-free "one-off" arms
//! peeled out of [`Game::run`] (card-dsl-and-card-pool spec deepen). Pure event mint for these effect variants
//! lives in [`crate::resolution::misc`]; this module is the choreography twin, calling into
//! game state directly (RNG, snapshotted resolution-frame reads, arm-armed runtime flags,
//! per-player fan-outs) rather than through the pure `mint_*` families.

use crate::*;

impl Game {
    /// Resolve one of the misc, no-pause choreography arms behind [`Game::run`]. Each match
    /// arm mirrors the (formerly inline) [`Game::run`] arm 1:1 — no behavior change, just
    /// the body relocated so [`Game::run`] can stay a thin dispatcher.
    pub(crate) fn run_misc_choreo(
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
            // Creative Technique's "Shuffle your library, then reveal…" lead-in step.
            Effect::Dig(DigEffect::ShuffleLibrary) => {
                self.push_apply(events, Event::LibraryShuffled { player: controller })
            }
            // "Each player creates a 0/0 green and blue Fractal creature token and puts a number
            // of +1/+1 counters on it equal to the total power of creatures they controlled that
            // were exiled this way." (Oversimplify): mint one `token` per living player in APNAP
            // order, applying each mint before computing its counters — `counters_after_replacements`
            // reads the token's controller off game state, mirroring `CreateToken`'s `enters_with`
            // below. No player choice, so this resolves in one pass, never pausing.
            Effect::Choice(ChoiceEffect::EachPlayerCreatesFractalFromExiledPower { token }) => {
                for player in self.apnap_order() {
                    let minted = self.next_object_id();
                    self.push_apply(
                        events,
                        Event::TokenCreated {
                            token: minted,
                            controller: player,
                            def: token,
                            creator: source,
                        },
                    );
                    let power: i32 = self
                        .resolution_frame
                        .power_exiled_this_way
                        .iter()
                        .filter(|snap| snap.controller == player)
                        .map(|snap| snap.power)
                        .sum();
                    let n = self.counters_after_replacements(minted, power);
                    if n > 0 {
                        self.push_apply(
                            events,
                            Event::CountersPlaced {
                                object: minted,
                                count: n,
                                source_name: self.def_of(source).name,
                            },
                        );
                    }
                }
            }
            // "Each player discards their hand, then draws seven cards." (Wheel of Fortune):
            // loop APNAP order, each living player discarding their whole hand (`discard_ids` —
            // no choice, so no `PendingChoice`, unlike a partial-hand `Effect::Choice(ChoiceEffect::Discard)`) then
            // drawing `count`.
            Effect::Choice(ChoiceEffect::EachPlayerDiscardsHandThenDraws { count }) => {
                let n = self.resolve_count(count, controller, source, target, x);
                for player in self.apnap_order() {
                    let hand = self.hand_of(player);
                    self.discard_ids(&hand, player, events);
                    for event in self.draw_events(player, n) {
                        self.push_apply(events, event);
                    }
                }
            }
            // Advanced Reconstruction's base ability: "exile a card from your graveyard at
            // random. You may play the exiled card this turn." The card is picked by the
            // injected RNG here (needs `&mut self`, unlike `ExileFromGraveyardMayPlay`'s
            // trigger-supplied card), then reuses that same event/permission plumbing.
            Effect::Dig(DigEffect::ExileRandomFromGraveyardMayPlay) => {
                let graveyard = self.graveyard_cards(controller);
                // CR 701.19a: if there's nothing to exile, this is a no-op.
                if graveyard.is_empty() {
                    return;
                }
                let idx = self.with_op_rng(controller, |rng| rng.gen_index(graveyard.len()));
                let from = graveyard[idx];
                self.push_apply(
                    events,
                    Event::ExiledFromGraveyardMayPlay {
                        player: controller,
                        card: self.next_object_id(),
                        from,
                    },
                );
            }
            // Ruhan of the Fomori's base ability: "Choose an opponent at random. ~ attacks that
            // player this combat if able." The opponent is picked by the injected RNG here (needs
            // `&mut self`), then reuses the same `must_attack` requirement plumbing a token's
            // `must_attack_defender` uses.
            Effect::Misc(MiscEffect::MustAttackRandomOpponent) => {
                let opponents: Vec<PlayerId> =
                    self.living_players().filter(|&p| p != controller).collect();
                // CR 800.4a: no living opponents (a solitaire test rig) — nothing to choose.
                if opponents.is_empty() {
                    return;
                }
                let idx = self.with_op_rng(controller, |rng| rng.gen_index(opponents.len()));
                self.push_apply(
                    events,
                    Event::MustAttackDeclared {
                        object: source,
                        defender: opponents[idx],
                    },
                );
            }
            // Inkshield (CR 615): arm a this-turn combat-damage prevention shield protecting the
            // ability's controller ("dealt to *you*"), carrying the Inkling profile minted per
            // point prevented. The tokens are created at the prevention itself (in `damage_player`),
            // not here — at resolution no combat damage has been prevented yet. Runtime
            // orchestration state (like the delayed combat-damage watches), not an event.
            Effect::Misc(MiscEffect::PreventCombatDamageToYouCreatingTokens { token }) => self
                .combat_extras
                .combat_damage_prevention_shields
                .push((controller, token)),
            // Moment's Peace (#150): arm the this-turn table-wide combat-damage shield — every
            // player's combat damage, not just this ability's controller's, and no token mint.
            // Runtime orchestration state (like Inkshield's shield above), not an event.
            Effect::Misc(MiscEffect::PreventAllCombatDamageThisTurn) => {
                self.combat_extras.prevent_all_combat_damage_this_turn = true;
            }
            // "Exile [this card] with N time counters on it" (Rousing Refrain): mark the resolving
            // spell so `finish_instant_sorcery_resolution` sends it to exile with time counters
            // instead of the graveyard (the resolving spell, `source`, is the card exiled).
            Effect::Zone(ZoneEffect::ExileSelfWithTimeCounters { counters, .. }) => {
                self.self_exile_time_counters = Some(counters);
            }
            // "Then put [this card] on the bottom of its owner's library" (Spell Crumple): mark
            // the resolving spell so `finish_instant_sorcery_resolution` sends it to the bottom
            // of its owner's library instead of the graveyard (`source`, the resolving spell
            // itself, is the card tucked).
            Effect::Zone(ZoneEffect::TuckSelfToLibraryBottom) => {
                self.self_tuck_to_library_bottom = true;
            }
            // "Exile [this card]" (Vengeful Rebirth): mark the resolving spell so
            // `finish_instant_sorcery_resolution` sends it to exile instead of the graveyard
            // (`source`, the resolving spell itself, is the card exiled).
            Effect::Zone(ZoneEffect::ExileSelfOnResolve) => {
                self.self_exile_on_resolve = true;
            }
            // Opal Palace's spend-to-cast rider: the commander spell (baked in as
            // `triggering_spell` when the `SpendManaToCast` trigger fired) is still on the stack, so
            // record the additional-counter count keyed by its id for `resolve_spell` to place as it
            // enters. Guard-return if that spell already left the stack (countered in response, CR
            // 603.4) — nothing to enter, so nothing to record.
            Effect::Counters(CountersEffect::CommanderEntersWithBonusCounters {
                triggering_spell,
                count,
            }) => {
                let Some(spell) = triggering_spell else {
                    return;
                };
                if !matches!(self.objects[spell as usize], Object::Spell(_)) {
                    return;
                }
                let n = self.resolve_count(count, controller, source, target, x);
                if n == 0 {
                    return;
                }
                self.pending_enter_bonus_counters.push((spell, n));
            }
            // Renegade Bull's attack trigger: "exile up to one target instant or sorcery card
            // from your graveyard and copy it. You may cast the copy without paying its mana
            // cost." "Up to one": no chosen target (declined, or none legal — CR 603.3c already
            // drops the ability before this runs) is a no-op. Exile the chosen card, then grant
            // the free-cast permission (CR 118.5) for it — the same `CastFromExileFreePermissionGranted`
            // plumbing `CastExiledWithThisFree` (Quintorius) grants — so the controller can
            // genuinely *cast* it (CR 601) at their next opportunity, firing real "whenever you
            // cast" watchers off it (including this card's own first ability above).
            Effect::Dig(DigEffect::ExileTargetGraveyardSpellCastFree { .. }) => {
                let Some(object) = target.and_then(Target::object_id) else {
                    return;
                };
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                self.push_apply(events, move_event);
                self.push_apply(
                    events,
                    Event::CastFromExileFreePermissionGranted {
                        card: exiled,
                        player: controller,
                    },
                );
            }
            // Surge to Victory: "Exile target instant or sorcery card from your graveyard."
            // Mandatory single target (unlike Renegade Bull's "up to one" above), so a legal
            // target is guaranteed by the time this runs (CR 608.2b already fizzled the whole
            // ability otherwise). Snapshot the exiled card's id + mana value for the following
            // team-pump (`Amount::ExiledCardManaValueThisWay`) and combat-damage-copy arm
            // (`ScheduleThisTurnCombatDamageCopy`) steps sharing this resolution's `Sequence`.
            Effect::Dig(DigEffect::ExileTargetGraveyardCardRecordManaValue { .. }) => {
                let object =
                    expect_object_target(target, "exile target graveyard card, record mana value");
                let mana_value = self.def_of(object).mana_value();
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                self.push_apply(events, move_event);
                self.resolution_frame.surge_exiled_card = Some((exiled, mana_value));
            }
            _ => unreachable!("misc resolution choreo received a non-family effect"),
        }
    }
}
