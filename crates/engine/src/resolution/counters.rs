//! Counters-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// Mint events for the Counters Effect family, or [`None`] if `effect` is not in this family.
    pub(crate) fn try_mint_counters(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Option<Vec<Event>> {
        if !matches!(
            effect,
            Effect::AttackerDrawsControllerCounters { .. }
                | Effect::DoubleCounters { .. }
                | Effect::LevelUp { .. }
                | Effect::PlaceVowCounters { .. }
                | Effect::PutCounters { .. }
                | Effect::PutCountersEach { .. }
                | Effect::RemoveAllCountersThenDraw { .. }
                | Effect::RemoveCounterFromSelf
        ) {
            return None;
        }
        Some(self.mint_counters_family(effect, controller, source, target, x))
    }

    fn mint_counters_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            // `kind = Some(k)` (Staff of the Storyteller's story counter) bypasses the +1/+1
            // replacement pipeline entirely, same as `EntersWithCounters`'s own kind split above.
            Effect::PutCounters {
                count,
                kind: Some(kind),
                ..
            } => {
                let object = expect_object_target(target, "a kind-counter effect");
                let count = self.resolve_count(count, controller, source, target, x) as i32;
                if count <= 0 {
                    return Vec::new();
                }
                vec![Event::KindCountersPlaced {
                    object,
                    kind,
                    count,
                }]
            }
            Effect::PutCounters {
                count,
                kind: None,
                divided,
                ..
            } => {
                let object = expect_object_target(target, "a counter effect");
                // A divided spell's per-target count was already settled (CR 601.2d) right after
                // targets were chosen — see `Game::maybe_begin_counter_division` — and recorded
                // on the resolving spell (`source` is that spell's own object id; `divided` only
                // appears on `Timing::Spell` effects, so this always resolves through the spell
                // path, mirroring `Effect::DealDamage`'s own divided read).
                let count = if divided {
                    self.spell(source)
                        .counter_division
                        .pairs()
                        .into_iter()
                        .find_map(|(t, amt)| (t == object).then_some(amt))
                        .unwrap_or(0)
                } else {
                    self.resolve_count(count, controller, source, target, x) as i32
                };
                let n = self.counters_after_replacements(object, count);
                if n <= 0 {
                    return Vec::new();
                }
                vec![Event::CountersPlaced {
                    object,
                    count: n,
                    source_name,
                }]
            }
            // Double the target's +1/+1 counters: place as many more as it already has (CR 614).
            Effect::DoubleCounters { .. } => {
                let object = expect_object_target(target, "a counter-doubling effect");
                self.doubled_counters_event(object, source_name)
                    .into_iter()
                    .collect()
            }
            // Put `count` +1/+1 counters on each battlefield permanent matching `filter`
            // (Mazirek: "each creature you control"; Shadrix Silverquill's begin-combat "Target
            // player puts a +1/+1 counter on each creature they control" reads `filter`'s
            // `you`/`opponent` axis from the chosen Player target's perspective instead).
            // Ids are snapshotted via `battlefield()` up front, same as `DestroyAll`.
            Effect::PutCountersEach {
                filter,
                count,
                target_player,
            } => {
                let you = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!(
                            "a target-player counters-each effect resolves with a chosen player target"
                        );
                    };
                    player
                } else {
                    controller
                };
                let count = self.resolve_count(count, controller, source, target, x) as i32;
                self.battlefield()
                    .into_iter()
                    .filter(|&id| self.permanent_matches(&filter, id, you, Some(source)))
                    .filter_map(|object| {
                        let n = self.counters_after_replacements(object, count);
                        (n > 0).then_some(Event::CountersPlaced {
                            object,
                            count: n,
                            source_name,
                        })
                    })
                    .collect()
            }
            // Promise of Loyalty's rider: place a vow counter on each surviving creature, marking
            // the controller (the caster — "can't attack *you*") as the protected player. Scans
            // every player's creatures matching `filter` (the survivors an all-players keep-one
            // edict left — see the `PlaceVowCounters` doc), not just the controller's own.
            Effect::PlaceVowCounters { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                .map(|object| Event::VowCountersPlaced {
                    object,
                    protected: controller,
                })
                .collect(),
            // Nexus Mentality's other mode: "Remove all counters from target nonland permanent
            // you control. Draw a card for each counter removed this way."
            Effect::RemoveAllCountersThenDraw { .. } => {
                let object = expect_object_target(target, "a remove-all-counters-then-draw effect");
                let (mut events, removed) = self.remove_all_counters_events(object);
                events.extend(self.draw_events(controller, removed as u32));
                events
            }
            // Breena: the attacking player (context) draws one; the controller's chosen creature
            // gets `counters` +1/+1 counters.
            Effect::AttackerDrawsControllerCounters { attacker, counters } => {
                let drawer = attacker.expect("the attacking player is filled in at placement");
                let object = expect_object_target(target, "Breena's counter half");
                let mut events = self.draw_events(drawer, 1);
                let n = self.counters_after_replacements(object, counters as i32);
                if n > 0 {
                    events.push(Event::CountersPlaced {
                        object,
                        count: n,
                        source_name,
                    });
                }
                events
            }
            // A Class's "Level N" ability (CR 717.2): the activation gate only offered this while
            // the source sat at level N-1, so resolution just records the new level.
            Effect::LevelUp { level } => vec![Event::LeveledUp { source, level }],
            // Ingenious Prodigy: "you may remove a +1/+1 counter from it." A negative
            // `CountersPlaced`, mirroring `RemoveAllCountersThenDraw`'s removal above; guarded so
            // a source with none doesn't go negative (unreachable in practice — the enclosing
            // ability's `SourceHasCounters` intervening-if already requires at least one).
            Effect::RemoveCounterFromSelf => {
                if self.plus_counters(source) <= 0 {
                    return vec![];
                }
                vec![Event::CountersPlaced {
                    object: source,
                    count: -1,
                    source_name,
                }]
            }

            _ => unreachable!("counters family mint received a non-family effect"),
        }
    }
}
