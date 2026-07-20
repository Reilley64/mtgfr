//! Misc resolution choreography that needs `&mut self` — the pause-free "one-off" arms
//! peeled out of [`Game::run`] (ADR 0002 deepen). Pure event mint for these effect variants
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
            Effect::ShuffleLibrary => {
                self.push_apply(events, Event::LibraryShuffled { player: controller })
            }
            // "Each player creates a 0/0 green and blue Fractal creature token and puts a number
            // of +1/+1 counters on it equal to the total power of creatures they controlled that
            // were exiled this way." (Oversimplify): mint one `token` per living player in APNAP
            // order, applying each mint before computing its counters — `counters_after_replacements`
            // reads the token's controller off game state, mirroring `CreateToken`'s `enters_with`
            // below. No player choice, so this resolves in one pass, never pausing.
            Effect::EachPlayerCreatesFractalFromExiledPower { token } => {
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
            // no choice, so no `PendingChoice`, unlike a partial-hand `Effect::Discard`) then
            // drawing `count`.
            Effect::EachPlayerDiscardsHandThenDraws { count } => {
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
            Effect::ExileRandomFromGraveyardMayPlay => {
                let graveyard = self.graveyard_cards(controller);
                // CR 701.19a: if there's nothing to exile, this is a no-op.
                if graveyard.is_empty() {
                    return;
                }
                let idx = (self.next_u64() % graveyard.len() as u64) as usize;
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
            Effect::MustAttackRandomOpponent => {
                let opponents: Vec<PlayerId> =
                    self.living_players().filter(|&p| p != controller).collect();
                // CR 800.4a: no living opponents (a solitaire test rig) — nothing to choose.
                if opponents.is_empty() {
                    return;
                }
                let idx = (self.next_u64() % opponents.len() as u64) as usize;
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
            Effect::PreventCombatDamageToYouCreatingTokens { token } => self
                .combat_extras
                .combat_damage_prevention_shields
                .push((controller, token)),
            // Moment's Peace (#150): arm the this-turn table-wide combat-damage shield — every
            // player's combat damage, not just this ability's controller's, and no token mint.
            // Runtime orchestration state (like Inkshield's shield above), not an event.
            Effect::PreventAllCombatDamageThisTurn => {
                self.combat_extras.prevent_all_combat_damage_this_turn = true;
            }
            // "Exile [this card] with N time counters on it" (Rousing Refrain): mark the resolving
            // spell so `finish_instant_sorcery_resolution` sends it to exile with time counters
            // instead of the graveyard (the resolving spell, `source`, is the card exiled).
            Effect::ExileSelfWithTimeCounters { counters, .. } => {
                self.self_exile_time_counters = Some(counters);
            }
            // "Then put [this card] on the bottom of its owner's library" (Spell Crumple): mark
            // the resolving spell so `finish_instant_sorcery_resolution` sends it to the bottom
            // of its owner's library instead of the graveyard (`source`, the resolving spell
            // itself, is the card tucked).
            Effect::TuckSelfToLibraryBottom => {
                self.self_tuck_to_library_bottom = true;
            }
            _ => unreachable!("misc resolution choreo received a non-family effect"),
        }
    }
}
