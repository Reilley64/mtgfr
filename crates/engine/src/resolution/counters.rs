//! Counters-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_counters(
        &self,
        effect: CountersEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            // `kind = Some(k)` (Staff of the Storyteller's story counter) runs through the
            // any-kind half of the replacement pipeline (Winding Constrictor, Vorinclex), not the
            // +1/+1-only one.
            CountersEffect::PutCounters {
                count,
                kind: Some(kind),
                max_total,
                ..
            } => {
                let object = expect_object_target(target, "a kind-counter effect");
                let count = self.resolve_count(count, controller, source, target, x) as i32;
                // "This ability can't cause the total number of +1/+0 counters on this creature
                // to be greater than seven" (Clockwork Beast, CR 121.1): the ceiling is on the
                // recipient's total, so what lands is the room left, not the announced amount.
                let count = match max_total {
                    Some(cap) => {
                        count.min(i32::from(cap) - i32::from(self.counters_of_kind(object, kind)))
                    }
                    None => count,
                };
                let count = self.kind_counters_after_replacements(controller, object, kind, count);
                if count <= 0 {
                    return Vec::new();
                }
                vec![Event::KindCountersPlaced {
                    object,
                    kind,
                    count,
                }]
            }
            CountersEffect::PutCounters {
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
                // path, mirroring `Effect::Damage(DamageEffect::Target)`'s own divided read).
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
                let n = self.counters_after_replacements(controller, object, count);
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
            CountersEffect::DoubleCounters { .. } => {
                let object = expect_object_target(target, "a counter-doubling effect");
                self.doubled_counters_event(controller, object, source_name)
                    .into_iter()
                    .collect()
            }
            // Put `count` counters on each battlefield permanent matching `filter`
            // (Mazirek: "each creature you control"; Shadrix Silverquill's begin-combat "Target
            // player puts a +1/+1 counter on each creature they control" reads `filter`'s
            // `you`/`opponent` axis from the chosen Player target's perspective instead).
            // Ids are snapshotted via `battlefield()` up front, same as `DestroyAll`.
            CountersEffect::PutCountersEach {
                filter,
                count,
                target_player,
                kind,
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
                let matching = self
                    .battlefield()
                    .into_iter()
                    .filter(|&id| self.permanent_matches(&filter, id, you, Some(source)));
                // `kind = Some(k)` (Contagion Engine's "-1/-1 counter on each creature target
                // player controls") takes the any-kind half of the replacement pipeline, same as
                // `PutCounters`'s own kind split above — the per-recipient count is replaced
                // separately, since each recipient's own controller's replacements apply.
                if let Some(kind) = kind {
                    if count <= 0 {
                        return Vec::new();
                    }
                    return matching
                        .filter_map(|object| {
                            let n = self
                                .kind_counters_after_replacements(controller, object, kind, count);
                            (n > 0).then_some(Event::KindCountersPlaced {
                                object,
                                kind,
                                count: n,
                            })
                        })
                        .collect();
                }
                matching
                    .filter_map(|object| {
                        let n = self.counters_after_replacements(controller, object, count);
                        (n > 0).then_some(Event::CountersPlaced {
                            object,
                            count: n,
                            source_name,
                        })
                    })
                    .collect()
            }
            // "Put a loyalty counter on each Garruk you control" (the Wolf token minted by Garruk,
            // Cursed Huntsman's `0`) — same battlefield-filter walk as `PutCountersEach`, but
            // loyalty is the scalar `Permanent::loyalty`, mutated by `Event::LoyaltyChanged`
            // directly, not through the +1/+1-counter replacement pipeline.
            CountersEffect::PutLoyaltyCounterEach { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                .map(|object| Event::LoyaltyChanged { object, amount: 1 })
                .collect(),
            // "Each opponent gets a poison counter" (Infectious Inquiry, Vraska's Fall) / "each
            // player gets a poison counter" (Ichor Rats) / "target opponent gets a poison counter"
            // (Venerated Rotpriest): counters on the *players* `who` names, not
            // on any permanent (CR 122.1). A player who has already lost is no longer in the game
            // and gets nothing (`living_players`).
            CountersEffect::PutCountersOnPlayer { kind, count, who } => {
                let count = self.resolve_count(count, controller, source, target, x) as i32;
                if count <= 0 {
                    return Vec::new();
                }
                self.players_in(who, controller, target)
                    .into_iter()
                    .filter_map(|player| {
                        let n = self.player_counters_after_replacements(controller, player, count);
                        (n > 0).then_some(Event::PlayerCountersPlaced {
                            player,
                            kind,
                            count: n,
                        })
                    })
                    .collect()
            }
            // "Each opponent loses all counters" (Final Act) — CR 122.1/121.2: every counter of
            // every kind on each player in `who` is removed, not just poison. A player who has
            // already lost is out of the game and loses nothing (`living_players`).
            CountersEffect::RemoveAllPlayerCounters { who } => self
                .players_in(who, controller, target)
                .into_iter()
                .flat_map(|player| {
                    PlayerCounterKind::ALL.iter().filter_map(move |&kind| {
                        let count = self.player_counters(player, kind) as i32;
                        (count > 0).then_some(Event::PlayerCountersPlaced {
                            player,
                            kind,
                            count: -count,
                        })
                    })
                })
                .collect(),
            // "If target player has fewer than nine poison counters, they get a number of poison
            // counters equal to the difference" (Vraska, Betrayal's Sting's −9): a top-up, so a
            // target already at or above `to` gets no counters and mints no event at all.
            CountersEffect::TopUpCountersOnPlayer { kind, to } => {
                let Some(Target::Player(player)) = target else {
                    return Vec::new();
                };
                if !self.living_players().any(|p| p == player) {
                    return Vec::new();
                }
                let count = to.saturating_sub(self.player_counters(player, kind));
                let count =
                    self.player_counters_after_replacements(controller, player, count as i32);
                if count <= 0 {
                    return Vec::new();
                }
                vec![Event::PlayerCountersPlaced {
                    player,
                    kind,
                    count,
                }]
            }
            // Promise of Loyalty's rider: place a vow counter on each surviving creature, marking
            // the controller (the caster — "can't attack *you*") as the protected player. Scans
            // every player's creatures matching `filter` (the survivors an all-players keep-one
            // edict left — see the `PlaceVowCounters` doc), not just the controller's own.
            CountersEffect::PlaceVowCounters { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                .map(|object| Event::VowCountersPlaced {
                    object,
                    protected: controller,
                })
                .collect(),
            // Breena's counter half: the controller's chosen creature gets `counters` +1/+1
            // counters. The attacking player's draw is *not* here — a draw can pause on a
            // replacement, which a pure mint can't, so `Game::run` runs it through the draw funnel.
            CountersEffect::AttackerDrawsControllerCounters { counters, .. } => {
                let object = expect_object_target(target, "Breena's counter half");
                let n = self.counters_after_replacements(controller, object, counters as i32);
                if n == 0 {
                    return Vec::new();
                }
                vec![Event::CountersPlaced {
                    object,
                    count: n,
                    source_name,
                }]
            }
            // A Class's "Level N" ability (CR 717.2): the activation gate only offered this while
            // the source sat at level N-1, so resolution just records the new level.
            CountersEffect::LevelUp { level } => vec![Event::LeveledUp { source, level }],
            // "Monstrosity N" (CR 701.28a): already monstrous is a total no-op (CR 701.28c) — not
            // even a `BecameMonstrous` event. Otherwise the +1/+1 counters route through the same
            // replacement pipeline `PutCounters` uses, and the source becomes monstrous even if a
            // replacement effect drove the count to zero.
            CountersEffect::Monstrosity { count } => {
                if self.permanent(source).monstrous {
                    return Vec::new();
                }
                let n = self.counters_after_replacements(controller, source, count as i32);
                let mut events = Vec::new();
                if n > 0 {
                    events.push(Event::CountersPlaced {
                        object: source,
                        count: n,
                        source_name,
                    });
                }
                events.push(Event::BecameMonstrous { object: source });
                events
            }
            // Ingenious Prodigy: "you may remove a +1/+1 counter from it." A negative
            // `CountersPlaced`, mirroring `Game::remove_counters_events`; guarded so
            // a source with none doesn't go negative (unreachable in practice — the enclosing
            // ability's `SourceHasCounters` intervening-if already requires at least one).
            CountersEffect::RemoveCounterFromSelf { kind: None } => {
                if self.plus_counters(source) <= 0 {
                    return vec![];
                }
                vec![Event::CountersPlaced {
                    object: source,
                    count: -1,
                    source_name,
                }]
            }
            // Living Artifact: "you may remove a vitality counter from this Aura." The named-kind
            // twin of the arm above, down the `KindCountersPlaced` path a named kind lives on.
            CountersEffect::RemoveCounterFromSelf { kind: Some(kind) } => {
                if self.counters_of_kind(source, kind) == 0 {
                    return vec![];
                }
                vec![Event::KindCountersPlaced {
                    object: source,
                    kind,
                    count: -1,
                }]
            }
            // Venarian Gold: "remove a sleep counter from that creature" — the same tick as the
            // arm above, one zone over onto whatever the Aura is attached to.
            CountersEffect::RemoveCounterFromAttached { kind } => {
                let Some(host) = self.as_permanent(source).and_then(|p| p.attached_to) else {
                    return vec![];
                };
                let Some(kind) = kind else {
                    if self.plus_counters(host) <= 0 {
                        return vec![];
                    }
                    return vec![Event::CountersPlaced {
                        object: host,
                        count: -1,
                        source_name,
                    }];
                };
                if self.counters_of_kind(host, kind) == 0 {
                    return vec![];
                }
                vec![Event::KindCountersPlaced {
                    object: host,
                    kind,
                    count: -1,
                }]
            }
            _ => unreachable!("counters family mint received a non-family effect"),
        }
    }

    /// The shared core of "double `object`'s +1/+1 counters" (CR 614): as many more as it
    /// already has, through the same replaceable-step pipeline [`CountersEffect::PutCounters`] uses.
    /// `None` when doubling is a no-op — zero counters, or a replacement effect zeroes the
    /// result out — the same "no event for a no-op doubling" rule
    /// [`CountersEffect::DoubleCounters`] and [`CountersEffect::DoubleCountersOnAttachedCreature`] both follow.
    pub(crate) fn doubled_counters_event(
        &self,
        placer: PlayerId,
        object: ObjectId,
        source_name: &'static str,
    ) -> Option<Event> {
        let current = self.permanent(object).plus_counters;
        let n = self.counters_after_replacements(placer, object, current);
        (n > 0).then_some(Event::CountersPlaced {
            object,
            count: n,
            source_name,
        })
    }

    /// [`CountersEffect::RemoveCounters`] — Nexus Mentality's "Remove all counters from target
    /// nonland permanent you control", Lily Bowen's "remove all but one +1/+1 counter from it".
    ///
    /// Resolves here rather than on the mint path because it writes
    /// [`ResolutionFrame::counters_removed_this_way`] for a following step to read back through
    /// [`Amount::CountersRemovedThisWay`], and minting is `&self`. Same reason
    /// [`Game::resolve_mill_self`] lives off the mint path. The tally is overwritten, not
    /// accumulated, so a removal that takes nothing off reads as 0 rather than as a stale count.
    pub(crate) fn resolve_remove_counters(
        &mut self,
        target: Option<Target>,
        all_kinds: bool,
        keep: u32,
        events: &mut Vec<Event>,
    ) {
        let object = expect_object_target(target, "a remove-counters effect");
        let (evs, removed) = self.remove_counters_events(object, all_kinds, keep as i32);
        self.resolution_frame.counters_removed_this_way = removed.max(0) as u32;
        self.apply_all(&evs);
        events.extend(evs);
    }

    /// Kinetic Ooze's X≥10 rider (CR 601.2c/603.3d): double the +1/+1 counters on each of the
    /// "other target creatures" chosen at placement (read from this ability's second target
    /// clause). A target that has left the battlefield since is skipped (CR 608.2b).
    pub(crate) fn resolve_double_counters_on_target_creatures(
        &mut self,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller,
            source,
            targets_second,
            ..
        } = ctx;
        let source_name = self.source_name_of(source);
        for chosen in targets_second.iter() {
            let Some(object) = chosen.object_id() else {
                continue;
            };
            if self.as_permanent(object).is_none() {
                continue;
            }
            if let Some(event) = self.doubled_counters_event(controller, object, source_name) {
                self.push_apply(events, event);
            }
        }
    }

    /// Fractal Harness's attack trigger: double the +1/+1 counters on the creature this
    /// Equipment is attached to (CR 614) — a no-target sibling of [`CountersEffect::DoubleCounters`]
    /// pinned to `source`'s own `attached_to` instead of a chosen target. An unattached
    /// Equipment (unequipped, or between equip targets) has nothing to double (guard-return).
    pub(crate) fn resolve_double_counters_on_attached_creature(
        &mut self,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller, source, ..
        } = ctx;
        let Some(object) = self.permanent(source).attached_to else {
            return;
        };
        if let Some(event) =
            self.doubled_counters_event(controller, object, self.def_of(source).name)
        {
            self.push_apply(events, event);
        }
    }
}
