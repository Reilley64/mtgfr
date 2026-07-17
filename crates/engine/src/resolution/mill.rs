//! Mill-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// Mint events for the Mill Effect family, or [`None`] if `effect` is not in this family.
    pub(crate) fn try_mint_mill(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Option<Vec<Event>> {
        if !matches!(
            effect,
            Effect::ExileDiscardedWithThis { .. }
                | Effect::ExileFromGraveyardMayPlay { .. }
                | Effect::ExileTargetFromGraveyardCreateTokenCopy { .. }
                | Effect::ExileTargetFromGraveyardWithThis
                | Effect::ExileTopMayPlay { .. }
                | Effect::Mill { .. }
                | Effect::MillSelf { .. }
        ) {
            return None;
        }
        Some(self.mint_mill_family(effect, controller, source, target, x))
    }

    fn mint_mill_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            Effect::Mill { count, .. } => {
                let Some(Target::Player(player)) = target else {
                    panic!("mill resolves with a chosen player target");
                };
                self.mill_events(
                    player,
                    self.resolve_count(count, controller, source, target, x),
                )
            }
            Effect::ExileTopMayPlay {
                count,
                until_next_turn,
            } => {
                let n = self.resolve_count(count, controller, source, target, x);
                self.exile_top_may_play_events(controller, n, until_next_turn)
            }
            // Containment Construct's payoff: exile the just-discarded card from the graveyard
            // and grant permission to play it until end of turn.
            Effect::ExileFromGraveyardMayPlay { card } => {
                let from = card.expect("the discarded card is filled in at placement");
                vec![Event::ExiledFromGraveyardMayPlay {
                    player: controller,
                    card: self.next_object_id(),
                    from,
                }]
            }
            // Currency Converter's payoff: exile the just-discarded card into this ability's own
            // source-linked pile (no impulse-play permission — unlike `ExileFromGraveyardMayPlay`).
            // ponytail: guard-returns rather than panics if `card` is missing or has already moved
            // out of the graveyard (e.g. a second effect exiled it first) — the "may" just does
            // nothing, same shape as a fizzled optional trigger.
            Effect::ExileDiscardedWithThis { card } => {
                let Some(from) = card else {
                    return Vec::new();
                };
                if self.zone_of(from) != Zone::Graveyard {
                    return Vec::new();
                }
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(from, exiled),
                    Event::ExiledWithSource {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Quintorius's end step: exile the chosen graveyard card into this source's own
            // exiled-with pile — same shape as `ExileDiscardedWithThis` above, but the card is a
            // chosen target rather than a just-discarded one, and there's no impulse-play
            // permission (the free-cast permission comes later, from the activated ability). (CR 602, CR 601, CR 113)
            Effect::ExileTargetFromGraveyardWithThis => {
                let object = expect_object_target(target, "exile target from graveyard with this");
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(object, exiled),
                    Event::ExiledWithSource {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Restore Relic: exile the targeted graveyard card, then mint a token copy of its
            // copiable characteristics (CR 707.2) — `CreateTokenCopy`'s target-a-battlefield-
            // permanent shape, but reading `def` off the graveyard card before it moves.
            Effect::ExileTargetFromGraveyardCreateTokenCopy { .. } => {
                let object =
                    expect_object_target(target, "exile target from graveyard, create a copy");
                let def = self.def_of(object);
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                let token = exiled + 1;
                vec![
                    move_event,
                    Event::TokenCreated {
                        token,
                        controller,
                        def,
                    },
                ]
            }
            // Perpetual Timepiece: untargeted self-mill (unlike Mill's target-player shape).
            Effect::MillSelf { count } => {
                let count = self.resolve_count(count, controller, source, target, x);
                self.mill_events(controller, count)
            }

            _ => unreachable!("mill family mint received a non-family effect"),
        }
    }
}
