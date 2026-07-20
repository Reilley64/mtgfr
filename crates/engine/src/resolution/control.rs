//! Control-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_control_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            // Equip resolves by attaching the Equipment (the ability's source) to the chosen
            // creature, replacing any prior attachment.
            Effect::Equip => {
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
            // even though the "you may" was accepted (FIDELITY_BACKLOG #156).
            Effect::AttachSelfToEntering { entering } => {
                let host = entering.expect("filled in from the entering trigger at placement");
                if !self.attachment_host_legal(source, host) {
                    return Vec::new();
                }
                vec![Event::AttachedTo {
                    object: source,
                    host: Some(host),
                }]
            }
            Effect::GoadTarget { .. } => {
                let object = expect_object_target(target, "goad");
                vec![Event::Goaded {
                    object,
                    by: controller,
                    source_name,
                }]
            }
            Effect::TapTarget { .. } => {
                let object = expect_object_target(target, "tap");
                vec![Event::Tapped { object }]
            }
            Effect::RegenerateShield { .. } => {
                let object = expect_object_target(target, "a regeneration shield");
                vec![Event::RegenerationShieldCreated { object }]
            }
            Effect::UntapTarget { .. } => {
                let object = expect_object_target(target, "untap");
                vec![Event::Untapped { object }]
            }
            Effect::RemoveFromCombat { .. } => {
                let object = expect_object_target(target, "remove from combat");
                vec![Event::RemovedFromCombat { object }]
            }
            Effect::GainControlUntilEndOfTurn { .. } => {
                let object = expect_object_target(target, "a steal");
                vec![Event::ControlGainedUntilEndOfTurn {
                    object,
                    controller,
                    source_name,
                }]
            }
            Effect::GainControl { .. } => {
                let object = expect_object_target(target, "a permanent control change");
                vec![Event::ControlGained { object, controller }]
            }
            // Reins of Power (CR 720): the mass, two-player until-EOT control exchange. `target` is
            // the opponent player. Snapshot both creature sets BEFORE writing any swap (so the first
            // steal can't feed the second — CR 800.4a), untap them all, swap each to the OTHER
            // player (each `ControlGainedUntilEndOfTurn` is freshly timestamped at apply, so it
            // outranks any earlier steal/donation), and grant haste. Ownership is untouched.
            Effect::ExchangeAllCreaturesUntilEndOfTurn { .. } => {
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
                    events.push(Event::Untapped { object });
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
            Effect::GainControlAllUntilEndOfTurn { filter } => {
                let creatures: Vec<ObjectId> = self
                    .battlefield()
                    .into_iter()
                    .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                    .collect();
                let mut events = Vec::new();
                // "Untap all creatures ..."
                for &object in &creatures {
                    events.push(Event::Untapped { object });
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
            Effect::GainControlWhile {
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
            Effect::GrantSourceAbilitiesUntilEndOfTurn => {
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
            Effect::UntapAll { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.controller_of(id) == controller
                        && self.permanent_matches(&filter, id, controller, Some(source))
                })
                .map(|object| Event::Untapped { object })
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
    pub(crate) fn resolve_exchange_control(
        &mut self,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
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
