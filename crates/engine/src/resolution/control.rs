//! Control-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// The tap or untap event `object` would actually undergo, or `None` when it is already in that
    /// state.
    ///
    /// CR 701.21a/701.21c: "becomes tapped" and "becomes untapped" name the *change*, so tapping an
    /// already-tapped permanent does nothing — a second Icy Manipulator aimed at a land Psychic
    /// Venom watches must not bite again. Every tap and untap this family mints goes through here so
    /// the arms can't drift apart on that.
    fn tap_change(&self, object: ObjectId, tapped: bool) -> Option<Event> {
        match (self.is_tapped(object) == tapped, tapped) {
            (true, _) => None,
            (false, true) => Some(Event::Tapped { object }),
            (false, false) => Some(Event::Untapped { object }),
        }
    }

    pub(crate) fn mint_control(
        &self,
        effect: ControlEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            // Equip resolves by attaching the Equipment (the ability's source) to the chosen
            // creature, replacing any prior attachment.
            ControlEffect::Equip => {
                let host = expect_object_target(target, "equip");
                vec![Event::AttachedTo {
                    object: source,
                    host: Some(host),
                }]
            }
            // Shielded by Faith / Prison Term: attach this Aura (the ability's source) to the
            // entering creature — moving it off any host it's already attached to (CR 704.5n
            // simply drops the old attachment once `apply` overwrites `attached_to`). `entering`
            // is filled at trigger placement; `None` only in an unplaced card template, which
            // never reaches resolution. Re-checks the Aura's own `enchant` filter against the
            // entering permanent (CR 303.4f-style legality) — a no-op if it isn't a legal host,
            // even though the "you may" was accepted (deck fidelity increments #156).
            ControlEffect::AttachSelfToEntering { entering } => {
                let host = entering.expect("filled in from the entering trigger at placement");
                if !self.attachment_host_legal(source, host) {
                    return Vec::new();
                }
                vec![Event::AttachedTo {
                    object: source,
                    host: Some(host),
                }]
            }
            ControlEffect::GoadTarget { .. } => {
                let object = expect_object_target(target, "goad");
                vec![Event::Goaded {
                    object,
                    by: controller,
                    source_name,
                }]
            }
            ControlEffect::TapTarget { .. } => {
                let object = expect_object_target(target, "tap");
                self.tap_change(object, true).into_iter().collect()
            }
            ControlEffect::RegenerateShield { .. } => {
                let object = expect_object_target(target, "a regeneration shield");
                vec![Event::RegenerationShieldCreated { object }]
            }
            ControlEffect::UntapTarget { .. } => {
                let object = expect_object_target(target, "untap");
                self.tap_change(object, false).into_iter().collect()
            }
            ControlEffect::RemoveFromCombat {
                release_solely_blocked,
                ..
            } => {
                let object = expect_object_target(target, "remove from combat");
                vec![Event::RemovedFromCombat {
                    object,
                    release_solely_blocked,
                }]
            }
            ControlEffect::GainControlUntilEndOfTurn { .. } => {
                let object = expect_object_target(target, "a steal");
                vec![Event::ControlGainedUntilEndOfTurn {
                    object,
                    controller,
                    source_name,
                }]
            }
            ControlEffect::GainControl { .. } => {
                let object = expect_object_target(target, "a permanent control change");
                vec![Event::ControlGained { object, controller }]
            }
            // Reins of Power (CR 720): the mass, two-player until-EOT control exchange. `target` is
            // the opponent player. Snapshot both creature sets BEFORE writing any swap (so the first
            // steal can't feed the second — CR 800.4a), untap them all, swap each to the OTHER
            // player (each `ControlGainedUntilEndOfTurn` is freshly timestamped at apply, so it
            // outranks any earlier steal/donation), and grant haste. Ownership is untouched.
            ControlEffect::ExchangeAllCreaturesUntilEndOfTurn { .. } => {
                let Some(Target::Player(opponent)) = target else {
                    return Vec::new();
                };
                let creatures = |who: PlayerId| -> Vec<ObjectId> {
                    self.battlefield()
                        .into_iter()
                        .filter(|&id| {
                            self.is_creature_on_battlefield(id) && self.controller_of(id) == who
                        })
                        .collect()
                };
                let yours = creatures(controller);
                let theirs = creatures(opponent);
                let mut events = Vec::new();
                // "Untap all creatures you control and all creatures target opponent controls."
                for &object in yours.iter().chain(theirs.iter()) {
                    events.extend(self.tap_change(object, false));
                }
                // "You and that opponent each gain control of all creatures the other controls until
                // end of turn."
                for &object in &yours {
                    events.push(Event::ControlGainedUntilEndOfTurn {
                        object,
                        controller: opponent,
                        source_name,
                    });
                }
                for &object in &theirs {
                    events.push(Event::ControlGainedUntilEndOfTurn {
                        object,
                        controller,
                        source_name,
                    });
                }
                // "Those creatures gain haste until end of turn."
                for &object in yours.iter().chain(theirs.iter()) {
                    events.push(Event::TempBoost {
                        object,
                        power: 0,
                        toughness: 0,
                        keywords: &[Keyword::Haste],
                        source_name,
                    });
                }
                events
            }
            // Insurrection (CR 720): the mass, one-sided, all-creatures-of-any-controller twin of
            // `GainControlUntilEndOfTurn`. `filter` is evaluated against every creature on the
            // battlefield regardless of controller, including the caster's own (no `you`/`opponent`
            // scoping, unlike `UntapAll` below). Snapshot the matching set BEFORE minting any event,
            // untap them all, hand each to the caster (freshly timestamped so it outranks any
            // earlier steal/donation — CR 800.4a), and grant haste. Ownership is untouched.
            ControlEffect::GainControlAllUntilEndOfTurn { filter } => {
                let creatures: Vec<ObjectId> = self
                    .battlefield()
                    .into_iter()
                    .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                    .collect();
                let mut events = Vec::new();
                // "Untap all creatures ..."
                for &object in &creatures {
                    events.extend(self.tap_change(object, false));
                }
                // "... and gain control of them until end of turn."
                for &object in &creatures {
                    events.push(Event::ControlGainedUntilEndOfTurn {
                        object,
                        controller,
                        source_name,
                    });
                }
                // "They gain haste until end of turn."
                for &object in &creatures {
                    events.push(Event::TempBoost {
                        object,
                        power: 0,
                        toughness: 0,
                        keywords: &[Keyword::Haste],
                        source_name,
                    });
                }
                events
            }
            // Homeward Path (CR 720): "Each player gains control of all creatures they own."
            // Snapshot every mismatched creature BEFORE minting any event (so an earlier
            // reversion in this same resolution can't feed a later one, mirroring
            // `ExchangeAllCreaturesUntilEndOfTurn` above), then mint one `ControlGained` per
            // creature naming its owner — every player's stolen creatures, not just the
            // activator's.
            ControlEffect::RevertAllCreaturesToOwners => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.is_creature_on_battlefield(id)
                        && self.controller_of(id) != self.owner_of(id)
                })
                .map(|object| Event::ControlGained {
                    object,
                    controller: self.owner_of(object),
                })
                .collect(),
            ControlEffect::GainControlWhile {
                while_source_tapped,
                ..
            } => {
                let object = expect_object_target(target, "a conditioned steal");
                vec![Event::ConditionedControlGained {
                    object,
                    controller,
                    condition: crate::ControlCondition {
                        source,
                        needs_tapped: while_source_tapped,
                    },
                }]
            }
            // Backup's rider (CR 702.166): the shared target creature gains the source's other
            // abilities until end of turn — but only "if that's another creature", so the source
            // targeting itself grants nothing (the counter still landed in the preceding step).
            ControlEffect::GrantSourceAbilitiesUntilEndOfTurn => {
                let object = expect_object_target(target, "Backup's ability grant");
                if object == source {
                    return Vec::new();
                }
                vec![Event::AbilitiesGranted {
                    target: object,
                    source,
                }]
            }
            // Beledros: untap every matching permanent the controller controls — the mass
            // mirror of UntapTarget, same "you control" scoping as PumpCreaturesYouControlUntilEndOfTurn.
            ControlEffect::UntapAll { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.controller_of(id) == controller
                        && self.permanent_matches(&filter, id, controller, Some(source))
                })
                .filter_map(|object| self.tap_change(object, false))
                .collect(),

            // Dread Cacodemon: "tap all other creatures you control" — the tap-side mirror of
            // `UntapAll` just above. `filter.other` (already set by the card's TOML) relies on
            // `permanent_matches`'s `Some(source)` to exclude Dread's own permanent id.
            ControlEffect::TapAll { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.controller_of(id) == controller
                        && self.permanent_matches(&filter, id, controller, Some(source))
                })
                .filter_map(|object| self.tap_change(object, true))
                .collect(),

            // Mana Short's "tap all lands target player controls": `TapAll` aimed at the chosen
            // seat instead of your own. `you` for the filter is still that player — Power Sink's
            // "lands with mana abilities **they** control" reads from their side of the table.
            ControlEffect::TapAllTargetPlayerControls { filter } => {
                let Some(Target::Player(player)) = target else {
                    return Vec::new();
                };
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        self.controller_of(id) == player
                            && self.permanent_matches(&filter, id, player, Some(source))
                    })
                    .filter_map(|object| self.tap_change(object, true))
                    .collect()
            }

            // Demonic Hordes' "tap this creature": no scan and no filter, just the one permanent
            // the ability came from — gone from the battlefield, it taps nothing (CR 608.2b).
            ControlEffect::TapSource => self
                .battlefield()
                .contains(&source)
                .then(|| self.tap_change(source, true))
                .flatten()
                .into_iter()
                .collect(),

            _ => unreachable!("control family mint received a non-family effect"),
        }
    }

    /// Donation (Zedruu, CR 720): `target` is the donated permanent (first clause);
    /// `targets_second` holds the recipient opponent (second clause, chosen at placement).
    /// Mint the permanent-control change with that player as the new controller — the same
    /// freshly-timestamped `permanent_control_overrides` write `GainControl` uses
    /// (apply.rs), leaving ownership with the donor (CR 108.3). A target that has left the
    /// battlefield since is skipped (CR 608.2b); with no chosen recipient the donation does
    /// nothing.
    pub(crate) fn resolve_target_opponent_gains_control(
        &mut self,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            target,
            targets_second,
            ..
        } = ctx;
        let Some(object) = target.and_then(Target::object_id) else {
            return;
        };
        if self.as_permanent(object).is_none() {
            return;
        }
        let Some(Target::Player(recipient)) = targets_second.iter().next() else {
            return;
        };
        self.push_apply(
            events,
            Event::ControlGained {
                object,
                controller: recipient,
            },
        );
    }

    /// Exchange control (Vedalken Plotter / Chromeshell Crab, CR 720): `target` is the first
    /// permanent (its "you control" clause); `targets_second` holds the second (its "an
    /// opponent controls" clause, chosen at placement). Swap their controllers — each new
    /// controller is the OTHER's prior `controller_of`, minted as two freshly-timestamped
    /// `ControlGained` events (CR 800.4a: the swap outranks any earlier steal), leaving
    /// ownership untouched (CR 108.3). Both must still be on the battlefield — an exchange
    /// needs both, so a target that has left since (CR 608.2b) cancels the whole swap.
    pub(crate) fn resolve_exchange_control(&mut self, ctx: ResolveCtx, events: &mut Vec<Event>) {
        let ResolveCtx {
            target,
            targets_second,
            ..
        } = ctx;
        let Some(first) = target.and_then(Target::object_id) else {
            return;
        };
        let Some(Target::Object(second)) = targets_second.iter().next() else {
            return;
        };
        if self.as_permanent(first).is_none() || self.as_permanent(second).is_none() {
            return;
        }
        let first_controller = self.controller_of(first);
        let second_controller = self.controller_of(second);
        self.push_apply(
            events,
            Event::ControlGained {
                object: first,
                controller: second_controller,
            },
        );
        self.push_apply(
            events,
            Event::ControlGained {
                object: second,
                controller: first_controller,
            },
        );
    }
}
