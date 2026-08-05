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
                        ends_at_end_of_combat: false,
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
                        ends_at_end_of_combat: false,
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
            // The Wretched (CR 611.2b): `GainControlAllUntilEndOfTurn`'s filtered sweep handed over
            // under `GainControlWhile`'s condition instead of a turn — no untap, no haste, the card
            // prints neither. `blocking_source = true` needs the source passed to the filter, which
            // is how "blocking **this** creature" resolves to the right attacker. Each match gets
            // its own `ControlCondition`, so `check_conditioned_control_reversions` hands them all
            // back the moment the source leaves the battlefield or changes controller.
            ControlEffect::GainControlAllWhile { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                .map(|object| Event::ConditionedControlGained {
                    object,
                    controller,
                    condition: crate::ControlCondition {
                        source,
                        needs_tapped: false,
                    },
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

            // Dread Cacodemon's "tap all other creatures you control" and Arena of the Ancients'
            // table-wide "tap all legendary creatures" — the tap-side mirror of `UntapAll` just
            // above. The seat restriction is the filter's own `controller` axis, so a card that
            // says "all" reaches across the table. `filter.other` (Dread's) relies on
            // `permanent_matches`'s `Some(source)` to exclude the source's own permanent id.
            ControlEffect::TapAll { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                .filter_map(|object| self.tap_change(object, true))
                .collect(),

            // Feint's "tap all creatures blocking target attacking creature": `TapAll` narrowed
            // to one attacker's declared blockers instead of a filter sweep. Tapping them does not
            // un-block anything (CR 509.1h) — the fog rider beside it is what saves the attacker.
            ControlEffect::TapBlockersOfTarget { .. } => {
                let Some(Target::Object(attacker)) = target else {
                    return Vec::new();
                };
                self.blockers_of(attacker)
                    .into_iter()
                    .filter_map(|object| self.tap_change(object, true))
                    .collect()
            }

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

    /// Rohgahh of Kher Keep's "then an opponent gains control of them": the swept set is read at
    /// resolution (plus the source itself when `with_source`, which no filter can name), the
    /// controller picks one opponent to hand *all* of it to, and a table with a single opponent
    /// left skips the pause. Handing the set over is [`Self::resolve_target_opponent_gains_control`]'s
    /// `ControlGained` per permanent.
    pub(crate) fn resolve_opponent_gains_control_all(
        &mut self,
        ctx: ResolveCtx,
        filter: PermanentFilter,
        with_source: bool,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller, source, ..
        } = ctx;
        let mut objects: Vec<ObjectId> = self
            .battlefield()
            .into_iter()
            .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
            .collect();
        if with_source && self.as_permanent(source).is_some() && !objects.contains(&source) {
            objects.push(source);
        }
        if objects.is_empty() {
            return;
        }
        let legal: Vec<PlayerId> = self.living_players().filter(|&p| p != controller).collect();
        match legal.as_slice() {
            // ponytail: no opponent left to hand them to — unreachable in a real game.
            [] => {}
            [only] => self.gain_control_of_all(*only, &objects, events),
            _ => pending::raise(
                self,
                pending::ChoiceRequest::ChooseSplittingOpponent {
                    player: controller,
                    source,
                    legal,
                    then: SplittingContinuation::GainControlOf { objects },
                },
            ),
        }
    }

    /// Hand `objects` to `recipient` — the tail of [`Self::resolve_opponent_gains_control_all`],
    /// reached directly when one opponent is left and through the chooser's answer otherwise.
    pub(crate) fn gain_control_of_all(
        &mut self,
        recipient: PlayerId,
        objects: &[ObjectId],
        events: &mut Vec<Event>,
    ) {
        for &object in objects {
            if self.as_permanent(object).is_none() {
                continue;
            }
            self.push_apply(
                events,
                Event::ControlGained {
                    object,
                    controller: recipient,
                },
            );
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
    ///
    /// `destroy_attached_auras` is Gauntlets of Chaos' rider: "If those permanents are exchanged
    /// this way, destroy all Auras attached to them." The early returns above are exactly that
    /// "if" — a cancelled swap reaches no destruction.
    pub(crate) fn resolve_exchange_control(
        &mut self,
        ctx: ResolveCtx,
        destroy_attached_auras: bool,
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
        self.swap_control(first, second, events);
        if !destroy_attached_auras {
            return;
        }
        for aura in [first, second]
            .into_iter()
            .flat_map(|host| self.attachments(host))
            .filter(|&id| matches!(self.def_of(id).kind, CardKind::Aura))
            .collect::<Vec<_>>()
        {
            self.destroy_permanent(aura, events);
        }
    }

    /// Enchantment Alteration: "Attach target Aura attached to a creature or land to another
    /// permanent of that type." Clause 0 is the Aura (`ctx.target`), clause 1 the new host
    /// (`ctx.targets_second`, chosen at announcement — CR 601.2c). Either target having left the
    /// battlefield since (CR 608.2b) drops the whole move, and the attach is still gated on the
    /// Aura's own enchant restriction and the host's protection (CR 303.4f/702.16e) — clause 1's
    /// legality was narrowed to the *old* host's types, which a type-changing effect in response
    /// can outdate.
    pub(crate) fn resolve_move_aura(&mut self, ctx: ResolveCtx, events: &mut Vec<Event>) {
        let ResolveCtx {
            target,
            targets_second,
            ..
        } = ctx;
        let Some(aura) = target.and_then(Target::object_id) else {
            return;
        };
        let Some(Target::Object(host)) = targets_second.iter().next() else {
            return;
        };
        if self.as_permanent(aura).is_none() || self.as_permanent(host).is_none() {
            return;
        }
        if !self.noncast_attach_legal(aura, host) {
            return;
        }
        self.push_apply(
            events,
            Event::AttachedTo {
                object: aura,
                host: Some(host),
            },
        );
    }

    /// Juxtapose (CR 701.10): "You and target player exchange control of the creature you each
    /// control with the greatest mana value" — an exchange of two *chosen* permanents rather than
    /// two targeted ones. One permanent per seat: the greatest printed mana value among the
    /// permanents that seat controls whose types intersect `types`. Nothing happens unless both
    /// seats have one (CR 701.10c — an exchange of one object isn't an exchange), and nothing
    /// happens when the chosen player is the resolving controller (both reads land on the same
    /// permanent).
    ///
    /// Juxtapose runs this twice — creatures, then artifacts — as two `Sequence` steps. The second
    /// step reads the board `run_sequence` has already applied the first step's swap to, which is
    /// what "then exchange control of artifacts the same way" wants: an artifact creature that just
    /// crossed the table is one of the *recipient's* artifacts by then.
    pub(crate) fn resolve_exchange_greatest_mana_value(
        &mut self,
        ctx: ResolveCtx,
        types: TypeSet,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller, target, ..
        } = ctx;
        let Some(Target::Player(other)) = target else {
            return;
        };
        if other == controller {
            return;
        }
        let Some(mine) = self.greatest_mana_value_permanent(controller, types) else {
            return;
        };
        let Some(theirs) = self.greatest_mana_value_permanent(other, types) else {
            return;
        };
        self.swap_control(mine, theirs, events);
    }

    /// The permanent `player` controls with the greatest printed mana value among those whose card
    /// types (CR 613.4, post-layer) intersect `types`, or `None` when they control none.
    ///
    /// ponytail: "If two or more permanents a player controls are tied for greatest, their
    /// controller chooses one of them" is resolved deterministically here — `max_by_key` keeps the
    /// last maximum, so the most recently created of a tied group wins. Exactly one of the tied
    /// group is exchanged either way; only *which* one is unfaithful. Upgrade path: a
    /// `PendingChoice` per tied seat, which is what Juxtapose's `approximates` note and
    /// leg-increments #124 track.
    fn greatest_mana_value_permanent(&self, player: PlayerId, types: TypeSet) -> Option<ObjectId> {
        self.battlefield()
            .into_iter()
            .filter(|&id| self.controller_of(id) == player)
            .filter(|&id| self.effective_types(id).intersects(types))
            .max_by_key(|&id| self.def_of(id).mana_value())
    }

    /// Hand `a` to `b`'s controller and `b` to `a`'s, as two freshly-timestamped `ControlGained`
    /// events (CR 800.4a) that leave ownership alone (CR 108.3).
    fn swap_control(&mut self, a: ObjectId, b: ObjectId, events: &mut Vec<Event>) {
        let a_controller = self.controller_of(a);
        let b_controller = self.controller_of(b);
        self.push_apply(
            events,
            Event::ControlGained {
                object: a,
                controller: b_controller,
            },
        );
        self.push_apply(
            events,
            Event::ControlGained {
                object: b,
                controller: a_controller,
            },
        );
    }

    /// Destroy one permanent, honoring indestructible, an available regeneration shield
    /// (CR 701.15b) and tokens' ceasing to exist — the single-permanent form of
    /// `DestroyEffect::All`'s per-permanent choreography (CR 704).
    fn destroy_permanent(&mut self, id: ObjectId, events: &mut Vec<Event>) {
        if self.has_keyword(id, Keyword::Indestructible) {
            return;
        }
        if self.regeneration_shield_available(id) {
            self.push_apply(events, Event::Regenerated { object: id });
            return;
        }
        let event = match self.objects[id as usize] {
            Object::Permanent(ref p) if p.token => Event::TokenCeasedToExist {
                token: id,
                controller: p.owner,
                def: p.def,
            },
            Object::Permanent(_) => self.graveyard_or_command(id, self.next_object_id()),
            _ => return,
        };
        self.push_apply(events, event);
    }
}
