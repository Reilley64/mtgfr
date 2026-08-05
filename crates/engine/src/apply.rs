//! Event application and state-based actions.
//!
//! Primary: CR 704 (state-based actions). Also the apply path that mutates board facts
//! from [`Event`]s after each intent.
//! Deferred / gaps: per-deck increments under `docs/fidelity/` (fidelity-grind skill).

use crate::*;

impl Game {
    /// Record `amount` damage dealt by `source` to `recipient` in the turn-scoped ledger
    /// ([`Game::damage_dealt_this_turn`]) and in this resolution's "dealt this way" tally
    /// ([`ResolutionFrame::damage_dealt_this_way`](crate::resolution::ResolutionFrame)). Called
    /// from the damage arms of [`Game::apply`], so what lands here is damage *dealt* — prevention
    /// and redirection already had their say (CR 615), and CR 120.8's "0 damage is never dealt"
    /// is the early return.
    pub(crate) fn record_damage_dealt(&mut self, source: ObjectId, recipient: Target, amount: i32) {
        if amount <= 0 {
            return;
        }
        self.damage_dealt_this_turn
            .push((source, recipient, amount));
        self.resolution_frame.damage_dealt_this_way += amount as u32;
    }

    /// Whether `host` is a legal object for `attachment` (an Aura or Equipment) to be attached to
    /// right now: `attachment`'s own `def.enchant` filter re-checked against its live host, or the
    /// default "enchant creature" filter when it has none (a plain Aura, or Equipment — the DSL
    /// has no attach-filter surface for Equipment beyond "must be a creature"). Used both at an
    /// Aura's cast-time legality re-check (CR 303.4f) and by the CR 704.5m/n state-based action
    /// below, so an Aura like Confiscate ("Enchant permanent") isn't held to the default
    /// enchant-creature restriction its `enchant` filter doesn't actually impose.
    pub(crate) fn attachment_host_legal(&self, attachment: ObjectId, host: ObjectId) -> bool {
        // A host another Aura has closed off (Consecrate Land's "can't be enchanted by other
        // Auras") is illegal for every Aura but that one — so an Aura already sitting there when
        // the closing Aura arrives goes to the graveyard on the CR 704.5n sweep below. Equipment
        // is untouched: it attaches, it doesn't enchant.
        if matches!(self.def_of(attachment).kind, CardKind::Aura)
            && self.host_cant_be_enchanted_by(host, attachment)
        {
            return false;
        }
        // An enchant-graveyard Aura's printed enchant names a graveyard card, which no
        // battlefield host satisfies; only its own ETB rewrite ("enchant creature put onto the
        // battlefield with this Aura") makes a host legal — exactly the one it reanimated.
        if self.def_of(attachment).enchant_graveyard {
            // Like the filter path below, a host that is no longer a live permanent is never
            // legal — the rewrite names an object, and that object has left the battlefield.
            return self.permanent(attachment).enchant_rewrite_host == Some(host)
                && matches!(&self.objects[host as usize], Object::Permanent(_));
        }
        let filter = self
            .def_of(attachment)
            .enchant
            .unwrap_or(PermanentFilter::of(TypeSet::CREATURE));
        self.permanent_matches(
            &filter,
            host,
            self.controller_of(attachment),
            Some(attachment),
        )
    }

    /// Whether an Aura may be attached to `host` by an effect that isn't a normal cast (Ajani's
    /// Chosen's "you may attach it to the token", Gift of Immortality's delayed "return this card
    /// attached to that creature"). A cast Aura gets its legality from a target choice (CR 601.2c),
    /// but these force-attach with no target step, so this is the only gate: `host` must satisfy
    /// the Aura's `enchant` restriction (CR 303.4f) *and* not have protection that stops the Aura
    /// (CR 702.16e). Reuses [`Game::attachment_host_legal`] so the check stays one seam.
    pub(crate) fn noncast_attach_legal(&self, attachment: ObjectId, host: ObjectId) -> bool {
        self.attachment_host_legal(attachment, host)
            && !self.protection_blocks_source(host, attachment)
    }

    /// Re-check state-based actions and return the events they produce.
    /// A player at 0-or-less life loses; a creature with lethal marked damage dies.
    pub(crate) fn check_state_based_actions(&self) -> Vec<Event> {
        let mut events = Vec::new();

        // CR 704.5r: if a permanent has both +1/+1 and −1/−1 counters on it, N of each are
        // removed as a state-based action, where N is the smaller of the two counts. Emitted
        // before death checks so `apply_all` still sees a live permanent when stripping pairs
        // (a simultaneous 0-toughness death in the same snapshot still uses pre-annihilation P/T).
        for id in self.battlefield() {
            let Object::Permanent(ref p) = self.objects[id as usize] else {
                continue;
            };
            let plus = p.plus_counters;
            let minus = p.kind_counters[CounterKind::MinusOneMinusOne as usize] as i32;
            let pairs = plus.min(minus);
            if pairs <= 0 {
                continue;
            }
            events.push(Event::CountersPlaced {
                object: id,
                count: -pairs,
                source_name: "",
            });
            events.push(Event::KindCountersPlaced {
                object: id,
                kind: CounterKind::MinusOneMinusOne,
                count: -pairs,
            });
        }

        // Deaths (and Aura state) are emitted before player eliminations: a player can lose in the
        // same sweep that kills one of their creatures, and `PlayerLost` tombstones every object
        // they own — so it must run last, after those death events have already been minted. The
        // loser's fresh graveyard/command-zone objects are then simply removed by `PlayerLost`.
        // Each dying creature becomes a graveyard (or command-zone) card; ids are minted
        // sequentially, matching the order `apply` will push into the arena.
        let mut next = self.next_object_id();
        // Creatures leaving the battlefield this sweep — so their Auras die and their Equipment
        // detaches simultaneously (CR 704.5), rather than one SBA sweep behind.
        let mut leaving = Vec::new();
        for id in self.battlefield() {
            let Object::Permanent(ref p) = self.objects[id as usize] else {
                continue;
            };
            let printed = card_def(p.def);
            // A creature with lethal marked damage dies (CR 704.5g); a planeswalker with 0 loyalty
            // is put into its owner's graveyard (CR 704.5i).
            // "Creature" here is the *effective* card type, not the printed one: a land animated by
            // a type-changing continuous effect (Living Plane's "All lands are 1/1 creatures that
            // are still lands") is a creature for the death SBAs too (CR 613.4, CR 704.5f/g).
            let is_creature = self.effective_types(id).intersects(TypeSet::CREATURE);
            let dies = match &printed.kind {
                // CR 702.103e: a bestowed permanent that's attached is an Aura, not a creature —
                // the toughness-≤0 / lethal-damage creature death SBAs don't apply to it.
                _ if is_creature && !self.is_bestowed_and_attached(id) => {
                    let toughness = self.toughness(id);
                    // 0-or-less toughness is a death SBA even for an indestructible creature (CR 702.12, CR 704)
                    // (CR 704.5f); lethal damage / deathtouch is not, if it's indestructible
                    // (CR 702.12b).
                    toughness <= 0
                        || (!self.has_keyword(id, Keyword::Indestructible)
                            && (p.marked_damage >= toughness || p.deathtouched))
                }
                CardKind::Planeswalker { .. } | CardKind::Battle { .. } => p.loyalty <= 0,
                _ => false,
            };
            if !dies {
                continue;
            }
            // A regeneration shield replaces this "destroyed" state-based action with a
            // regeneration instead (CR 701.15b) — the same substitution `DestroyTarget` already
            // honors, since CR 704.5g's lethal-damage/deathtouch destroy is a "destroy" too. CR
            // 704.5f's 0-toughness death is not a "destroy" and isn't replaceable this way, so the
            // shield only applies when toughness is still positive (i.e. lethal damage or
            // deathtouch is the reason, not 0-or-less toughness).
            if self.regeneration_shield_available(id) && is_creature && self.toughness(id) > 0 {
                events.push(Event::Regenerated { object: id });
                continue;
            }
            leaving.push(id);
            // A dying token ceases to exist rather than becoming a graveyard card (CR 111.7).
            // ponytail: it skips the graveyard entirely — revisit if a "when a token dies"
            // trigger that reads the graveyard is scripted (Lorehold/Witherbloom).
            if p.token {
                events.push(Event::TokenCeasedToExist {
                    token: id,
                    controller: p.owner,
                    def: p.def,
                });
                continue;
            }
            events.push(self.graveyard_or_command(id, next));
            next += 1;
        }

        // CR 704.5k (the world rule): if two or more permanents have the World supertype, all but
        // the one that has had it for the shortest amount of time are put into their owners'
        // graveyards. Unlike the legend rule (CR 704.5j) it is global — it groups by neither
        // controller nor name — and no one chooses: the newest simply wins. `battlefield()` yields
        // ascending object ids and every permanent entering the battlefield mints a fresh, higher
        // id, so the last World permanent in that order is the newest one.
        // ponytail: CR 704.5k's tie clause ("in the event of a tie for the shortest amount of
        // time, all are put into their owners' graveyards") is unreachable — ids are minted one at
        // a time even when a single effect puts several permanents onto the battlefield, so there
        // is always exactly one newest. Stamp a batch-scoped entry epoch on each permanent if a
        // card ever puts two World permanents onto the battlefield simultaneously.
        let worlds: Vec<ObjectId> = self
            .battlefield()
            .into_iter()
            .filter(|&id| {
                matches!(&self.objects[id as usize], Object::Permanent(p) if card_def(p.def).world)
            })
            .collect();
        // Everything but the last (newest) — empty for zero or one World permanent.
        let doomed = worlds
            .split_last()
            .map(|(_, older)| older)
            .unwrap_or_default();
        for &id in doomed {
            let Object::Permanent(ref p) = self.objects[id as usize] else {
                continue;
            };
            leaving.push(id);
            // A token ceases to exist rather than becoming a graveyard card (CR 111.7), same as
            // the death and Aura sweeps above.
            if p.token {
                events.push(Event::TokenCeasedToExist {
                    token: id,
                    controller: p.owner,
                    def: p.def,
                });
                continue;
            }
            events.push(self.graveyard_or_command(id, next));
            next += 1;
        }

        // CR 704.5m/n: an Aura attached to nothing/an illegal object is put into the graveyard;
        // an Equipment attached to an illegal object simply becomes unattached (no death). A
        // deployed Aura mid-[`PendingChoice::ChooseAttachHost`] is exempted — it's unattached
        // only until that choice is answered, not actually illegal.
        let awaiting_host = match &self.pending_choice {
            Some(PendingChoice::ChooseAttachHost { attachment, .. }) => Some(*attachment),
            _ => None,
        };
        for id in self.battlefield() {
            let Object::Permanent(ref p) = self.objects[id as usize] else {
                continue;
            };
            let printed = card_def(p.def);
            let host_illegal = match p.attached_to {
                // unattached Aura is illegal, unless it's this Aura awaiting its host choice, or
                // Animate Dead's own reanimator Aura freshly entered and still waiting on its own
                // ETB ability (CR 303.4a/608.2b) to reanimate and attach it. This SBA sweep runs
                // before that ETB ability is even placed on the stack (`TriggerEnqueue` is the
                // next pipeline phase, see `pipeline.rs`), so without this exemption the Aura
                // would destroy itself here before ever getting the chance to reanimate anything.
                // Its cast-time graveyard target still sitting untouched in the graveyard is what
                // marks that window; once the ETB ability reanimates (or drops for want of a
                // legal target), the target's card object has left the graveyard and this
                // exemption naturally lapses — the ordinary CR 704.5m sweep then applies to it,
                // both while it's still unattached and later, once its reanimated host dies.
                None => {
                    matches!(&printed.kind, CardKind::Aura)
                        && awaiting_host != Some(id)
                        && !(printed.enchant_graveyard
                            && p.cast_time_enchant_target
                                .is_some_and(|card| self.zone_of(card) == Zone::Graveyard))
                }
                Some(host) => !self.attachment_host_legal(id, host) || leaving.contains(&host),
            };
            if !host_illegal {
                continue;
            }
            if matches!(&printed.kind, CardKind::Aura) {
                // CR 111.7: a token Aura (a Replicate copy, CR 707.10a) that falls off ceases to
                // exist rather than becoming a graveyard card, the same token-cease rule any other
                // token's death/leaves-the-battlefield path already honors.
                if p.token {
                    events.push(Event::TokenCeasedToExist {
                        token: id,
                        controller: p.owner,
                        def: p.def,
                    });
                } else {
                    events.push(self.graveyard_or_command(id, next));
                    next += 1;
                }
            } else if p.attached_to.is_some() {
                events.push(Event::AttachedTo {
                    object: id,
                    host: None,
                });
            }
        }

        // Player eliminations last (see the note above): a player at 0-or-less life, who tried to
        // draw from an empty library, who has ten or more poison counters, or who took lethal
        // commander damage loses (CR 704.5a/b/c, CR 903.10a).
        for (id, player) in self.players.iter().enumerate() {
            if player.lost {
                continue;
            }
            let lethal_commander_damage = player
                .commander_damage
                .iter()
                .any(|&(_, amount)| amount >= LETHAL_COMMANDER_DAMAGE);
            // CR 704.5c: ten or more poison counters loses the game.
            let lethal_poison =
                player.kind_counters[PlayerCounterKind::Poison as usize] >= LETHAL_POISON;
            // CR 704.5a, with Lich's exemption: only the life-total clause is waived, so the
            // other three still eliminate a player sitting comfortably at -12.
            let zero_life = player.life <= 0 && !self.ignores_zero_life(PlayerId(id as u8));
            if zero_life || player.attempted_empty_draw || lethal_poison || lethal_commander_damage
            {
                events.push(Event::PlayerLost {
                    player: PlayerId(id as u8),
                });
            }
        }

        // Ascend / the city's blessing (CR 702.131b): a living player who doesn't yet have it
        // and controls ten or more permanents gets it. The `!has_citys_blessing` guard makes
        // this sticky for free — once granted it's never re-emitted, and nothing ever clears
        // the flag (CR 702.130's "for the rest of the game").
        // ponytail: CR 702.131c says ascend is checked continuously; this checks only at each
        // state-based-action sweep. Behaviorally identical for the pool — nothing observes the
        // sub-SBA window between a tenth permanent entering and the next sweep. (CR 704)
        for (id, player) in self.players.iter().enumerate() {
            if player.lost || player.has_citys_blessing {
                continue;
            }
            let controller = PlayerId(id as u8);
            if self.permanents_controlled(controller) >= 10 {
                events.push(Event::CitysBlessingGained { player: controller });
            }
        }
        events
    }

    /// Sweep state-based actions to a fixpoint (CR 704.3): one creature's death can drop a static
    /// anthem that puts another creature's toughness at 0 or below (or an elimination can cascade
    /// similarly), so a sweep that changed state must be re-checked. Applies each sweep's events
    /// and accumulates them into `events`. Bounded by objects+players — each non-empty iteration
    /// applies at least one event and the pool of live objects only shrinks, so it always
    /// terminates well inside the bound.
    pub(crate) fn sweep_state_based_actions(&mut self, events: &mut Vec<Event>) {
        // CR 704.3: state-based actions are checked only when a player would receive priority.
        // While a choice is pending, the game is paused mid-resolution and no one has priority —
        // an enter-as-a-copy replacement (CR 614/616 — Altered Ego) pauses with the object briefly
        // a 0/0 before the copy is chosen, and it must not die to the 0-toughness SBA in that gap.
        // The sweep runs on the answer that clears the choice (`finish_answer`), once resolution
        // completes.
        if self.pending_choice.is_some() {
            return;
        }
        let bound = self.objects.len() + self.players.len() + 1;
        for _ in 0..bound {
            let mut sba = self.check_state_based_actions();
            sba.extend(self.check_conditioned_control_reversions());
            sba.extend(self.check_linked_exile_returns());
            sba.extend(self.check_leaves_battlefield_illusions());
            if sba.is_empty() {
                // CR 704.5j: after event-producing SBAs settle, pause for the legend rule if a
                // controller still has two+ legendary permanents with the same name. One conflict
                // group per sweep (lowest seat, then name); the answer resumes the pipeline.
                if let Some(choice) = self.legend_rule_choice() {
                    pending::raise_choice(self, choice);
                }
                return;
            }
            self.apply_all(&sba);
            events.extend(sba);
        }
        // Reaching here means SBAs never converged — a real engine bug producing wrong state, not
        // something to limp past silently in release. Fail loudly; the server's catch_unwind
        // quarantines the one bad table rather than taking the process down (C3).
        panic!("state-based actions did not reach a fixpoint within {bound} sweeps");
    }

    /// First legend-rule conflict (CR 704.5j), if any: a living controller with two or more
    /// legendary permanents that share a printed name. Groups are ordered by controller seat,
    /// then name, so the raise is deterministic when several conflicts exist.
    pub(crate) fn legend_rule_choice(&self) -> Option<PendingChoice> {
        use std::collections::BTreeMap;

        let mut groups: BTreeMap<(u8, &str), Vec<ObjectId>> = BTreeMap::new();
        for id in self.battlefield() {
            let Object::Permanent(ref p) = self.objects[id as usize] else {
                continue;
            };
            let printed = card_def(p.def);
            if !printed.legendary {
                continue;
            }
            let controller = self.controller_of(id);
            if self.players[controller.0 as usize].lost {
                continue;
            }
            groups
                .entry((controller.0, printed.name))
                .or_default()
                .push(id);
        }
        for ((seat, name), options) in groups {
            if options.len() < 2 {
                continue;
            }
            return Some(PendingChoice::ChooseLegendaryKeep {
                player: PlayerId(seat),
                name,
                options,
            });
        }
        None
    }

    /// CR 611.2b: for each condition-scoped control override whose [`ControlCondition`] no longer
    /// holds — the source left the battlefield, its controller (the thief) lost control of it, or
    /// (Rubinia Soulsinger's clause) it untapped — end the steal. Detected the same state-based way
    /// as [`Game::check_linked_exile_returns`] (swept to a fixpoint), so control reverts on its own
    /// the instant the condition breaks rather than through a triggered ability.
    pub(crate) fn check_conditioned_control_reversions(&self) -> Vec<Event> {
        self.play_permissions
            .conditioned_control_overrides
            .iter()
            .filter(|&&(_, thief, condition, _)| !self.control_condition_holds(thief, condition))
            .map(|&(object, ..)| Event::ConditionedControlEnded { object })
            .collect()
    }

    /// Whether a condition-scoped steal's [`ControlCondition`] still holds for `thief` (the
    /// override's controller): its source is still a battlefield permanent controlled by `thief`
    /// and — when `needs_tapped` (Rubinia's "remains tapped") — still tapped.
    fn control_condition_holds(&self, thief: PlayerId, condition: ControlCondition) -> bool {
        let Some(source) = self.as_permanent(condition.source) else {
            return false;
        };
        if condition.needs_tapped && !source.tapped {
            return false;
        }
        self.controller_of(condition.source) == thief
    }

    /// The O-Ring pattern (CR 603.6e): for each `(source, exiled)` link still on
    /// [`Game::exiled_until_source_leaves`] whose `source` is no longer a battlefield permanent,
    /// return the linked card. Per the Oblivion Ring ruling this return "isn't a triggered
    /// ability — it won't use the stack, and it can't be responded to", so it's detected the same
    /// way as a state-based action (swept to a fixpoint alongside
    /// [`Game::check_state_based_actions`]) rather than queued onto the stack.
    /// ponytail: if the linked card already left exile some other way by the time its source
    /// leaves, it's simply skipped (CR 603.6e only returns a card that's "still exiled") and its
    /// now-stale link is left in place — harmless, since object ids are never reused, so it can
    /// only ever match this same dead end again. No pool card triggers this; add an explicit
    /// cleanup event if one ever does.
    pub(crate) fn check_linked_exile_returns(&self) -> Vec<Event> {
        let mut next = self.next_object_id();
        let mut events = Vec::new();
        for &(source, exiled) in &self.exile_links.until_source_leaves {
            if matches!(&self.objects[source as usize], Object::Permanent(_)) {
                continue; // the source is still on the battlefield — the link is still live.
            }
            let Object::Card(ref card) = self.objects[exiled as usize] else {
                continue;
            };
            if card.zone != Zone::Exile {
                continue;
            }
            events.push(Event::ReturnedFromLinkedExile {
                permanent: next,
                from: exiled,
                controller: card.owner,
                source,
            });
            next += 1;
        }
        events
    }

    /// Skyclave Apparition's leaves-battlefield drawback (a sibling of
    /// [`Game::check_linked_exile_returns`], not sharing its list): for each `(source, exiled)`
    /// link still on [`Game::exile_links`]'s `illusion_on_source_leave` whose `source` is no
    /// longer a battlefield permanent, mint the exiled card's owner an X/X blue Illusion token
    /// (X = the exiled card's mana value, CR 111.1) and drop the link so it fires exactly once —
    /// unlike the O-Ring return, the exiled card never leaves `Zone::Exile`, so there's no
    /// zone-change guard to stop a re-fire on the next sweep.
    /// ponytail: modeled as an SBA-style departure sweep, like `check_linked_exile_returns` — the
    /// real ability is a triggered ability that uses the stack (CR 603) and can be responded to;
    /// this can't be. Same divergence, same precedent.
    pub(crate) fn check_leaves_battlefield_illusions(&self) -> Vec<Event> {
        let mut next = self.next_object_id();
        let mut events = Vec::new();
        for &(source, exiled) in &self.exile_links.illusion_on_source_leave {
            if matches!(&self.objects[source as usize], Object::Permanent(_)) {
                continue; // the source is still on the battlefield — the link is still live.
            }
            let Object::Card(ref card) = self.objects[exiled as usize] else {
                continue;
            };
            if card.zone != Zone::Exile {
                continue; // already left exile some other way — nothing to size the Illusion off.
            }
            let mana_value = self.def_of(exiled).mana_value() as i32;
            let mut def = illusion_token();
            if let CardKind::Creature {
                power, toughness, ..
            } = &mut def.kind
            {
                *power = mana_value;
                *toughness = mana_value;
            }
            events.push(Event::TokenCreated {
                token: next,
                controller: card.owner,
                def: intern_card_def(def),
                creator: source,
            });
            next += 1;
            events.push(Event::LeavesIllusionMinted {
                source,
                object: exiled,
            });
        }
        events
    }

    /// Apply a batch of events in order. Events are the *only* mutator of state.
    pub(crate) fn apply_all(&mut self, events: &[Event]) {
        for event in events {
            self.apply(event);
        }
    }

    /// Remove a spell object from the stack (it resolved or left the stack).
    pub(crate) fn remove_spell_from_stack(&mut self, object: ObjectId) {
        self.stack
            .retain(|item| !matches!(item, StackItem::Spell(o) if *o == object));
    }

    /// Drop inspect-ledger batches for `object` (it left the battlefield). Aggregates on a still-live
    /// permanent are zeroed; usually called while `object` is still a permanent, just before the
    /// zone-change tombstone.
    pub(crate) fn clear_modifier_provenance(&mut self, object: ObjectId) {
        self.modifier_provenance
            .counter_batches
            .retain(|&(o, ..)| o != object);
        self.modifier_provenance
            .modifiers
            .retain(|m| m.host != object);
        if let Object::Permanent(p) = &mut self.objects[object as usize] {
            p.plus_counters = 0;
        }
    }

    /// Register one continuous modification of `host`, stamping it with the CR 613.7 timestamp it
    /// takes effect at. Entries are appended in stamp order, which is what lets every reader walk
    /// the registry in place rather than sorting a copy of it.
    pub(crate) fn register_modifier(
        &mut self,
        host: ObjectId,
        source_name: &'static str,
        duration: ModifierDuration,
        kind: ModifierKind,
    ) {
        let timestamp = self.stamp_continuous_timestamp();
        self.modifier_provenance.modifiers.push(Modifier {
            host,
            source_name,
            timestamp,
            duration,
            kind,
        });
    }

    /// Recompute `plus_counters` on the permanent from its provenance batches — batches are the
    /// write path; the aggregate stays a derived cache for hot characteristic / cleanup scans.
    /// Until-EOT boosts need no such cache: [`Game::runtime_continuous_effects`] reads their
    /// batches straight out of the registry, one CR 613 layer entry each.
    pub(crate) fn resync_counter_aggregate(&mut self, object: ObjectId) {
        let counters: i32 = self
            .modifier_provenance
            .counter_batches
            .iter()
            .filter(|&&(o, _, _)| o == object)
            .map(|&(_, c, _)| c)
            .sum();
        let Object::Permanent(p) = &mut self.objects[object as usize] else {
            return;
        };
        p.plus_counters = counters;
    }

    /// Apply one event's effect on game *facts* (objects, the stack, mana). A zone change
    /// mints a fresh object via [`Game::create_object`] and tombstones the old one; the
    /// event carries the (precomputed) new id, which must match the arena's next slot.
    /// Priority/pass bookkeeping is orchestration state and lives in the submit path — as are
    /// `CombatState::attackers_declared` and `blocked_by` (set directly by the declaration
    /// intents, cleared by [`Event::CombatCleared`]): they're bookkeeping over the
    /// already-event-sourced attacks/blocks, not facts of their own.
    pub(crate) fn apply(&mut self, event: &Event) {
        self.invalidate_characteristics_cache(event);
        match event.clone() {
            Event::SpellCast {
                spell,
                from,
                controller,
                target,
                x,
                modes,
                flashback,
                escape,
                sacrifice_count,
                sacrificed_mana_value,
                revealed_creature_mana_value,
                kicked,
                bought_back,
                strive_count,
                replicate_count,
                multikicker_count,
                bestowed,
                face_down,
                masked,
                evoked,
                spent_colors,
                phyrexian_life_paid,
            } => {
                let (def, commander) = match &self.objects[from as usize] {
                    Object::Card(c) => (c.def, c.commander),
                    _ => panic!("cast source {from} is not a card"),
                };
                let printed = card_def(def);
                // Cast zone is read off `from` before `create_object` below moves it onto the
                // stack (CR 601's default cast zone — Dirgur Focusmage's "from your hand").
                let from_zone = self.zone_of(from);
                let cast_from_hand = from_zone == Zone::Hand;
                // CR 505.1a/505.1b: read off ambient timing state (like `cast_from_hand` above),
                // not a player-declared cost — Sulfurous Blast's/Return to Dust's cast-timing
                // rider needs no wire field, unlike kicked/multikicker.
                let cast_during_main_phase = self.active_player == controller
                    && matches!(self.step, Step::Main1 | Step::Main2);
                // Serra Paragon (CR 118.9): a permanent spell cast from the graveyard by neither
                // flashback nor escape can only be its once-per-turn permission — flashback/escape (CR 702.34, CR 702.19, CR 500)
                // set their own flags, and no permanent card has retrace. The tag rides to the
                // resulting permanent so it gains the exile-and-gain-2-life rider.
                let serra_recursion = from_zone == Zone::Graveyard
                    && !flashback
                    && !escape
                    && !matches!(&printed.kind, CardKind::Spell { .. });
                // CR 107.3: a static cast-X modification (Unbound Flourishing) doubles the value of
                // X on the caster's permanent X-spells *after* payment. This is the single point
                // where the spell's X is frozen for its whole life, so the doubled value flows to
                // enters-with-X counters and every `Amount::X` reader downstream.
                let x = self.cast_x_after_replacements(controller, &printed, x);
                let id = self.create_object(
                    Some(from),
                    Object::Spell(Spell {
                        def,
                        controller,
                        // A single-target spell's lone target rides on the cast event; a
                        // multi-target spell casts with none and records them via
                        // `SpellTargetsChosen` (auto-fill or the caster's answer).
                        targets: TargetList::single(target),
                        targets_second: TargetList::default(),
                        commander,
                        x,
                        chosen_color: None,
                        set_color: None,
                        text_swap: None,
                        modes,
                        copy: false,
                        flashback,
                        escape,
                        cast_from_hand,
                        cast_during_main_phase,
                        damage_division: DamageAssignment::default(),
                        damage_division_players: [None; MAX_TARGETS],
                        counter_division: DamageAssignment::default(),
                        sacrifice_count,
                        sacrificed_mana_value,
                        revealed_creature_mana_value,
                        kicked,
                        bought_back,
                        strive_count,
                        replicate_count,
                        multikicker_count,
                        serra_recursion,
                        bestowed,
                        face_down,
                        masked,
                        evoked,
                        spent_colors,
                        phyrexian_life_paid,
                    }),
                );
                if serra_recursion {
                    self.players[controller.0 as usize].graveyard_play_used_this_turn = true;
                }
                assert_eq!(id, spell);
                self.stack.push(StackItem::Spell(spell));
                // A card cast from exile "on an adventure" (CR 715.3d) consumes its permission —
                // it's no longer in exile at this id.
                self.play_permissions
                    .on_adventure
                    .retain(|&(card, _)| card != from);
                self.players[controller.0 as usize].spells_cast_this_turn += 1;
                // North Star's "for one spell this turn" — the permission is spent here, after
                // this spell's mana was settled and before any other spell can plan a payment.
                // ponytail: spent by the *next* spell cast rather than by one the player picks,
                // which is the only spell anyone activates North Star for; give the flag an
                // explicit "use it on this cast" rider on `Intent::Cast` if a real line ever wants
                // to skip a spell.
                self.players[controller.0 as usize].spend_mana_as_any_type_this_turn = false;
                // Feeds the `has_x` `nth_each_turn` gate (Nev, Zimone Infinite Analyst) —
                // SpellFilter::HasXInCost's own predicate (characteristics.rs).
                if printed.cost.x > 0 {
                    self.players[controller.0 as usize].x_spells_cast_this_turn += 1;
                }
                // Feeds Condition::CastInstantOrSorceryThisTurn (Hall of Oracles's activation gate),
                // Amount::GreatestInstantOrSorceryManaValueCastThisTurn (Rootha, Mastering the
                // Moment's "X is the greatest mana value among instant and sorcery spells you've
                // cast this turn"), and Amount::InstantsAndSorceriesCastThisTurn (Rionya,
                // Fire Dancer's "X is one plus the number of instant and sorcery spells you've
                // cast this turn").
                if let CardKind::Spell { speed, .. } = &printed.kind {
                    let sorcery = matches!(speed, SpellSpeed::Sorcery);
                    let player = &mut self.players[controller.0 as usize];
                    player.instant_or_sorcery_cast_this_turn = true;
                    // Backdraft's "a player who cast one or more sorcery spells this turn".
                    player.sorcery_cast_this_turn |= sorcery;
                    player.greatest_instant_or_sorcery_mana_value_cast_this_turn = player
                        .greatest_instant_or_sorcery_mana_value_cast_this_turn
                        .max(printed.mana_value());
                    player.instants_and_sorceries_cast_this_turn += 1;
                }
            }
            Event::AdventureSpellCast {
                spell,
                source,
                controller,
                target,
                x,
            } => {
                // The card's *main* face is the creature (front); its `adventure` is the spell
                // being cast now. The card moves from hand onto the stack as a spell whose def is
                // the adventure face, stashing the front face to restore on resolution.
                let front = self.def_id_of(source);
                let adventure = card_def(front)
                    .adventure
                    .expect("an adventure cast's source card has an adventure half");
                let def = adventure;
                let adventure = card_def(adventure);
                let commander = self.is_commander(source);
                // CR 505.1a/505.1b: same ambient-timing read `Event::SpellCast` uses above.
                let cast_during_main_phase = self.active_player == controller
                    && matches!(self.step, Step::Main1 | Step::Main2);
                let id = self.create_object(
                    Some(source),
                    Object::Spell(Spell {
                        def,
                        controller,
                        targets: TargetList::single(target),
                        targets_second: TargetList::default(),
                        commander,
                        x,
                        chosen_color: None,
                        set_color: None,
                        text_swap: None,
                        modes: Modes::default(),
                        copy: false,
                        flashback: false,
                        escape: false,
                        // Cast from the card's owner's hand (CR 601's default cast zone).
                        cast_from_hand: true,
                        cast_during_main_phase,
                        damage_division: DamageAssignment::default(),
                        damage_division_players: [None; MAX_TARGETS],
                        counter_division: DamageAssignment::default(),
                        sacrifice_count: 0,
                        sacrificed_mana_value: 0,
                        revealed_creature_mana_value: 0,
                        kicked: false,
                        bought_back: false,
                        strive_count: 0,
                        replicate_count: 0,
                        multikicker_count: 0,
                        serra_recursion: false,
                        bestowed: false,
                        face_down: false,
                        masked: false,
                        evoked: false,
                        // ponytail: an adventure cast still pays real mana (`settle_payment` runs
                        // above), but no adventure card checks color-spent yet — wire this from
                        // the same `Event::ManaSpent` snapshot `Event::SpellCast` uses
                        // (`Game::cast_adventure`) if one ever does.
                        spent_colors: [false; Color::COUNT],
                        // No pool adventure card has a Phyrexian pip; wire this the same way as
                        // `spent_colors` above if one ever does.
                        phyrexian_life_paid: 0,
                    }),
                );
                assert_eq!(id, spell);
                self.stack.push(StackItem::Spell(spell));
                // Remember the creature front face to restore to exile when this spell resolves.
                self.play_permissions.adventure_fronts.push((spell, front));
                // Casting the adventure is casting a spell — the same bookkeeping `SpellCast` does.
                self.players[controller.0 as usize].spells_cast_this_turn += 1;
                if adventure.cost.x > 0 {
                    self.players[controller.0 as usize].x_spells_cast_this_turn += 1;
                }
                // The adventure half is always an instant/sorcery (CR 715.2a).
                if matches!(adventure.kind, CardKind::Spell { .. }) {
                    let player = &mut self.players[controller.0 as usize];
                    player.instant_or_sorcery_cast_this_turn = true;
                    player.greatest_instant_or_sorcery_mana_value_cast_this_turn = player
                        .greatest_instant_or_sorcery_mana_value_cast_this_turn
                        .max(adventure.mana_value());
                    player.instants_and_sorceries_cast_this_turn += 1;
                }
            }
            Event::SplitHalfSpellCast {
                spell,
                source,
                half,
                controller,
                target,
                x,
            } => {
                // Only the cast half is on the stack (CR 709.4); the card moves from hand onto the
                // stack as that face. `create_object` restores the fused card on the way out, off
                // the `split_halves_on_stack` entry recorded below.
                let fused = self.def_id_of(source);
                let fused_def = card_def(fused);
                let face = card_def(
                    *fused_def
                        .halves
                        .get(half as usize)
                        .expect("a split-half cast names one of the card's halves"),
                );
                let def = face.as_ref().clone();
                let commander = self.is_commander(source);
                // CR 505.1a/505.1b: same ambient-timing read `Event::SpellCast` uses above.
                let cast_during_main_phase = self.active_player == controller
                    && matches!(self.step, Step::Main1 | Step::Main2);
                let id = self.create_object(
                    Some(source),
                    Object::Spell(Spell {
                        def: intern_card_def(def.clone()),
                        controller,
                        targets: TargetList::single(target),
                        targets_second: TargetList::default(),
                        commander,
                        x,
                        chosen_color: None,
                        set_color: None,
                        text_swap: None,
                        modes: Modes::default(),
                        copy: false,
                        flashback: false,
                        escape: false,
                        // Cast from the card's owner's hand (CR 601's default cast zone).
                        cast_from_hand: true,
                        cast_during_main_phase,
                        damage_division: DamageAssignment::default(),
                        damage_division_players: [None; MAX_TARGETS],
                        counter_division: DamageAssignment::default(),
                        sacrifice_count: 0,
                        sacrificed_mana_value: 0,
                        revealed_creature_mana_value: 0,
                        kicked: false,
                        bought_back: false,
                        strive_count: 0,
                        replicate_count: 0,
                        multikicker_count: 0,
                        serra_recursion: false,
                        bestowed: false,
                        face_down: false,
                        masked: false,
                        evoked: false,
                        // ponytail: a split half pays real mana (`settle_payment` runs in
                        // `Game::cast_split_half`), but no pool split card reads color-spent —
                        // wire this from the same `Event::ManaSpent` snapshot `Event::SpellCast`
                        // uses if one ever does.
                        spent_colors: [false; Color::COUNT],
                        // No pool split card has a Phyrexian pip; wire this the same way as
                        // `spent_colors` above if one ever does.
                        phyrexian_life_paid: 0,
                    }),
                );
                assert_eq!(id, spell);
                self.stack.push(StackItem::Spell(spell));
                self.play_permissions
                    .split_halves_on_stack
                    .push((spell, fused));
                // Casting a half is casting a spell — the same bookkeeping `SpellCast` does.
                self.players[controller.0 as usize].spells_cast_this_turn += 1;
                if def.cost.x > 0 {
                    self.players[controller.0 as usize].x_spells_cast_this_turn += 1;
                }
                // Both halves of a split card are instants or sorceries (CR 709.1).
                if matches!(def.kind, CardKind::Spell { .. }) {
                    let player = &mut self.players[controller.0 as usize];
                    player.instant_or_sorcery_cast_this_turn = true;
                    player.greatest_instant_or_sorcery_mana_value_cast_this_turn = player
                        .greatest_instant_or_sorcery_mana_value_cast_this_turn
                        .max(def.mana_value());
                    player.instants_and_sorceries_cast_this_turn += 1;
                }
            }
            Event::SpellTargetsChosen {
                spell,
                targets,
                clause,
            } => {
                if clause == 0 {
                    self.spell_mut(spell).targets = targets;
                } else {
                    self.spell_mut(spell).targets_second = targets;
                }
            }
            Event::SpellDamageDivided {
                spell,
                assignment,
                players,
            } => {
                self.spell_mut(spell).damage_division = assignment;
                self.spell_mut(spell).damage_division_players = players;
            }
            Event::SpellCountersDivided { spell, assignment } => {
                self.spell_mut(spell).counter_division = assignment;
            }
            Event::SpellCopied {
                copy,
                original,
                controller,
                set_color,
            } => {
                // The copy takes the original's copiable characteristics/x/mode/target, but is
                // controlled by the copier and is a copy (not a commander, never graveyard-bound).
                // `original` is usually a live spell on the stack (Twincast); Surge to Victory's
                // "copy the exiled card" instead points at the already-exiled `Object::Card` — a
                // card-not-a-spell source carries no targets/x/mode of its own, so those default
                // and the copy's own target is chosen fresh via the trailing `RetargetSpellCopy`
                // step `mint_spell_copies` always queues.
                let id = self.create_object(
                    None,
                    Object::Spell(match &self.objects[original as usize] {
                        Object::Spell(src) => Spell {
                            controller,
                            commander: false,
                            copy: true,
                            // Fork's "except that the copy is red" — the recolor belongs to the
                            // copy alone, so it overrides whatever the original carried.
                            set_color,
                            ..src.clone()
                        },
                        _ => Spell {
                            def: self.def_id_of(original),
                            controller,
                            targets: TargetList::default(),
                            targets_second: TargetList::default(),
                            commander: false,
                            x: 0,
                            chosen_color: None,
                            set_color,
                            text_swap: None,
                            modes: Modes::default(),
                            copy: true,
                            flashback: false,
                            escape: false,
                            cast_from_hand: false,
                            // A copy isn't "cast" (CR 707.10) — no ambient timing to read.
                            cast_during_main_phase: false,
                            damage_division: DamageAssignment::default(),
                            damage_division_players: [None; MAX_TARGETS],
                            counter_division: DamageAssignment::default(),
                            sacrifice_count: 0,
                            sacrificed_mana_value: 0,
                            revealed_creature_mana_value: 0,
                            kicked: false,
                            bought_back: false,
                            strive_count: 0,
                            replicate_count: 0,
                            multikicker_count: 0,
                            serra_recursion: false,
                            bestowed: false,
                            face_down: false,
                            masked: false,
                            evoked: false,
                            // A copy pays no cost (CR 707.10) — nothing was spent to "cast" it.
                            spent_colors: [false; Color::COUNT],
                            phyrexian_life_paid: 0,
                        },
                    }),
                );
                assert_eq!(id, copy);
                self.stack.push(StackItem::Spell(copy));
            }
            Event::SpellCeasedToExist { spell } => {
                self.remove_spell_from_stack(spell);
                self.mark_removed(spell);
            }
            Event::PreparedChanged { object, prepared } => {
                self.permanent_mut(object).prepared = prepared;
            }
            Event::LeveledUp { source, level } => {
                self.permanent_mut(source).level = level;
            }
            // CR 701.28b: a one-way flag, never cleared by the Untap `StepBegan` turn-boundary reset.
            Event::BecameMonstrous { object } => {
                self.permanent_mut(object).monstrous = true;
            }
            // CR 712: the permanent flips to its back face (one-way, permanent). Its live
            // characteristics now come from `def.back` (via `def_of`); the object is otherwise
            // unchanged (CR 712.5 — counters, attachments, tapped state persist).
            Event::Flipped { object } => {
                self.permanent_mut(object).flipped = true;
            }
            // Phase out `object` and everything attached to it (CR 702.26g — indirect phasing);
            // phasing in clears the same set. `attachments` is unfiltered, so it still finds an
            // already-phased attachment to phase back in.
            Event::PhasedOut { object } | Event::PhasedIn { object } => {
                let phased = matches!(event, Event::PhasedOut { .. });
                self.permanent_mut(object).phased_out = phased;
                for attached in self.attachments(object) {
                    self.permanent_mut(attached).phased_out = phased;
                }
            }
            Event::CreatureTypeChosen { object, subtype } => {
                self.permanent_mut(object).chosen_subtype = Some(subtype);
            }
            // Most choose_color sources are permanents (Mother of Runes, Flickering Ward's
            // as-enters self); Bathe in Light's is the spell itself mid-resolution, which isn't
            // a permanent, so it gets its own `Spell::chosen_color` slot instead.
            Event::ColorChosen { object, color } => match &mut self.objects[object as usize] {
                Object::Permanent(p) => p.chosen_color = Some(color),
                Object::Spell(s) => s.chosen_color = Some(color),
                other => panic!("object {object} can't record a chosen color: {other:?}"),
            },
            // A CR 612.1 text change (Magical Hack, Sleight of Mind), on a permanent or on a spell
            // still on the stack — "target spell or permanent", so both slots are real targets.
            // Indefinite: nothing clears it, and a new object starts without one.
            Event::TextChanged { object, swap } => match &mut self.objects[object as usize] {
                Object::Permanent(p) => p.text_swap = Some(swap),
                Object::Spell(s) => s.text_swap = Some(swap),
                other => panic!("object {object} can't record a text change: {other:?}"),
            },
            // A layer-5 color SET, on a permanent (Wild Mongrel) or on a spell still on the stack
            // (Deathlace — a lace can recolor the spell itself, which is the whole reason the
            // cycle was printed). The spell slot is a plain field, not a registered modifier: a
            // spell has no duration to sweep and ceases to exist as it resolves.
            Event::ColorSet {
                object,
                color,
                until_end_of_turn,
            } => {
                let on_spell = match &mut self.objects[object as usize] {
                    Object::Spell(s) => {
                        s.set_color = Some(color);
                        true
                    }
                    Object::Permanent(_) => false,
                    other => panic!("object {object} can't record a color: {other:?}"),
                };
                if !on_spell {
                    let duration = match until_end_of_turn {
                        true => ModifierDuration::EndOfTurn,
                        // The lace cycle prints no duration at all (CR 400.7).
                        false => ModifierDuration::Indefinite,
                    };
                    self.register_modifier(object, "", duration, ModifierKind::SetColor(color));
                }
            }
            Event::PreparedSpellCast {
                spell,
                source,
                controller,
                target,
                x,
            } => {
                // The spell's characteristics come from the source permanent's back face — the
                // front permanent stays on the battlefield, so there's no card leaving a zone.
                let back = card_def(self.permanent(source).def)
                    .back
                    .expect("a prepared cast's source has a back face");
                // CR 505.1a/505.1b: same ambient-timing read `Event::SpellCast` uses above.
                let cast_during_main_phase = self.active_player == controller
                    && matches!(self.step, Step::Main1 | Step::Main2);
                let def = back;
                let back = card_def(back);
                let id = self.create_object(
                    None,
                    Object::Spell(Spell {
                        def,
                        controller,
                        targets: TargetList::single(target),
                        targets_second: TargetList::default(),
                        commander: false,
                        x,
                        chosen_color: None,
                        set_color: None,
                        text_swap: None,
                        modes: Modes::default(),
                        // "Cast a **copy**" (CR): it ceases to exist on resolve rather than
                        // becoming a graveyard card (there is no card behind it).
                        copy: true,
                        flashback: false,
                        escape: false,
                        // Cast from the source permanent's prepared state, not the hand.
                        cast_from_hand: false,
                        cast_during_main_phase,
                        damage_division: DamageAssignment::default(),
                        damage_division_players: [None; MAX_TARGETS],
                        counter_division: DamageAssignment::default(),
                        sacrifice_count: 0,
                        sacrificed_mana_value: 0,
                        revealed_creature_mana_value: 0,
                        kicked: false,
                        bought_back: false,
                        strive_count: 0,
                        replicate_count: 0,
                        multikicker_count: 0,
                        serra_recursion: false,
                        bestowed: false,
                        face_down: false,
                        masked: false,
                        evoked: false,
                        // ponytail: a prepared cast still pays real mana (`settle_payment` runs
                        // in `Game::cast_prepared`), but no prepare card checks color-spent yet —
                        // wire this from the same `Event::ManaSpent` snapshot `Event::SpellCast`
                        // uses if one ever does.
                        spent_colors: [false; Color::COUNT],
                        // No pool prepare back face has a Phyrexian pip; wire this the same way as
                        // `spent_colors` above if one ever does.
                        phyrexian_life_paid: 0,
                    }),
                );
                assert_eq!(id, spell);
                self.stack.push(StackItem::Spell(spell));
                // Casting a copy is still casting a spell (feeds `spells_cast_this_turn`).
                // ponytail: the broader cast-spell *triggers* (magecraft, CR 700's "whenever you
                // cast") aren't fired for the prepared copy — those hang off `Event::SpellCast`,
                // and no pool prepare card's controller runs one. Route this through the SpellCast
                // trigger scan if a magecraft interaction ever needs it.
                self.players[controller.0 as usize].spells_cast_this_turn += 1;
                if back.cost.x > 0 {
                    self.players[controller.0 as usize].x_spells_cast_this_turn += 1;
                }
            }
            Event::TriggeredAbilityOnStack {
                controller,
                source,
                effect,
                target,
                targets_second,
                x,
                spent_mana,
                activated,
            } => {
                self.stack.push(StackItem::Ability {
                    controller,
                    source,
                    effect,
                    activated,
                    target,
                    targets_second,
                    x,
                    spent_mana,
                });
            }
            Event::AbilityResolved { .. } => {
                // The resolving ability is always the top of the stack.
                debug_assert!(matches!(self.stack.last(), Some(StackItem::Ability { .. })));
                self.stack.pop();
            }
            // CR 701.5c/112.7a: a countered activated ability ceases to exist — remove the
            // topmost stack ability with this source (the target this counter resolved against;
            // see `TargetSpec::ActivatedAbilityOnStack`'s identity ponytail). No card moves.
            Event::AbilityCountered { source } => {
                if let Some(i) = self.stack.iter().rposition(
                    |item| matches!(item, StackItem::Ability { source: s, .. } if *s == source),
                ) {
                    self.stack.remove(i);
                }
            }
            Event::StepBegan {
                step,
                active_player,
            } => {
                self.step = step;
                self.active_player = active_player;
                // A new turn refreshes the active player's land drop and clears every player's
                // "this turn" tallies (life gained / spells cast) — the turn boundary.
                if step == Step::Untap {
                    self.players[active_player.0 as usize].lands_played = 0;
                    self.permanents_died_this_turn = 0;
                    self.damaged_this_turn.clear();
                    self.damage_dealt_this_turn.clear();
                    self.drawn_this_turn.clear();
                    for player in &mut self.players {
                        player.life_gained_this_turn = 0;
                        player.spells_cast_this_turn = 0;
                        player.damage_taken_this_turn = 0;
                        player.x_spells_cast_this_turn = 0;
                        player.draws_this_turn = 0;
                        player.life_losses_this_turn = 0;
                        player.creatures_died_this_turn = 0;
                        player.modified_creature_died_this_turn = false;
                        player.nontoken_creatures_entered_this_turn = 0;
                        player.land_entered_under_your_control_this_turn = false;
                        player.card_left_graveyard_this_turn = false;
                        player.instant_or_sorcery_cast_this_turn = false;
                        player.sorcery_cast_this_turn = false;
                        player.greatest_instant_or_sorcery_mana_value_cast_this_turn = 0;
                        player.instants_and_sorceries_cast_this_turn = 0;
                        player.flash_permission_this_turn = false;
                        player.channel_colorless_mana_this_turn = false;
                        player.spend_mana_as_any_type_this_turn = false;
                        player.graveyard_play_used_this_turn = false;
                        player.attacked_this_turn = false;
                    }
                    // "Activate only once each turn" (CR 602.2b) resets at the start of every
                    // turn, not just the capped ability's controller's own — same boundary as
                    // the tallies above.
                    self.once_per_turn.activated.clear();
                    // The triggered-ability twin (CR "this ability triggers only once each
                    // turn") resets at the same turn boundary.
                    self.once_per_turn.triggered.clear();
                    // ponytail: a `ScheduleNextCastTrigger` watch's CR 603.7 "this turn" duration
                    // expires at the *arming* turn's cleanup; nothing reads `pending_next_cast`
                    // between that cleanup and the next turn's Untap (this same step), so clearing
                    // everything left here is CR-equivalent to per-entry cleanup-step expiry —
                    // same boundary/reasoning as the `player.*_this_turn` tallies just above.
                    self.delayed_triggers.pending_next_cast.clear();
                    // Reincarnation's "when that creature dies **this turn**" watch expires at the
                    // same boundary, for the same reason `pending_next_cast` does.
                    self.delayed_triggers.pending_dies_this_turn.clear();
                    // ponytail: `ScheduleThisTurnCombatDamageCopy`'s CR 603.7 "this turn" watch
                    // is repeatable (unlike `pending_next_cast`'s one-shot), but the same
                    // turn-boundary reasoning applies — nothing reads it after this Untap step
                    // without a fresh arm, so clearing it here is CR-equivalent to per-entry
                    // cleanup-step expiry.
                    self.delayed_triggers.pending_combat_damage_copy.clear();
                    // Glyph of Life's "this turn" watch is repeatable too, and expires at the
                    // same boundary for the same reason `pending_combat_damage_copy` does.
                    self.delayed_triggers.pending_attacker_damage_life.clear();
                    // "Attacks this turn if able" (Furygale Flocking) expires at the turn
                    // boundary, the same "this turn" scope as the tallies above.
                    self.combat_extras.must_attack.clear();
                    // Blaze of Glory's two halves ("can block any number of creatures this turn",
                    // "blocks each attacking creature this turn if able") expire at the same turn
                    // boundary, for the same reason `must_attack` does.
                    self.combat_extras.may_block_any_number.clear();
                    self.combat_extras.must_block_all.clear();
                    // The Glyph cycle's "…blocked by that creature this turn" ledger expires at the
                    // same turn boundary, for the same reason `must_attack` does.
                    self.combat_extras.blocked_this_turn.clear();
                    // Floral Spuzzem's "assigns no combat damage this turn" expires at the same
                    // turn boundary, for the same reason `must_attack` does.
                    self.combat_extras
                        .assigns_no_combat_damage_this_turn
                        .clear();
                    // ponytail: "Prevent all combat damage … this turn" (Inkshield) shields expire
                    // at the next Untap — combat is always within the turn, so a combat-only shield
                    // cleared here is behavior-exact for "this turn", the same idiom `must_attack`
                    // and `pending_next_cast` use.
                    self.combat_extras.combat_damage_prevention_shields.clear();
                    // ponytail: "Prevent all combat damage … this turn" (Moment's Peace, #150)
                    // expires at the next Untap — same behavior-exact turn-boundary idiom as the
                    // per-player Inkshield shield just above.
                    self.combat_extras.prevent_all_combat_damage_this_turn = false;
                    // "Prevent the next N damage … this turn" (Healing Salve, Samite Healer,
                    // Conservator) expires here too. Unlike the two combat-only shields above this
                    // one also covers noncombat damage, so "this turn" has to mean through the
                    // cleanup step — it does: Untap is the first step of the *next* turn, so
                    // nothing between the shield's turn ending and this clear can be damaged.
                    self.damage_prevention_shields.clear();
                    // Guardian Angel's standing "you may pay {1}" offer is scoped to the same
                    // "this turn", and expires unpaid at the same boundary.
                    self.standing_preventions.clear();
                    // "Entered the battlefield this turn" (Oran-Rief, the Vastwood) and "attacked
                    // this turn" (Agent Frank Horrigan's indestructible grant, CR 508.1) both
                    // expire at the same turn boundary — every battlefield permanent's, not just
                    // the active player's (a new turn, anyone's, ends "this turn").
                    // "You choose which creatures attack/block this turn" (Master Warcraft)
                    // expires at the same turn boundary as the shields above — combat is always
                    // within the turn, so clearing at Untap is behavior-exact for "this turn".
                    // "Until your next turn" (Island Sanctuary): the shield lasts across the
                    // other seats' turns and lapses when its own player's turn comes back around,
                    // so only the active player's entry goes — unlike every "this turn" clear
                    // above, which is unconditional.
                    self.combat_extras
                        .repelled_until_next_turn
                        .retain(|&(shielded, _)| shielded != active_player);
                    // Firestorm Phoenix's returned card is revealed and unplayable "until that
                    // player's next turn" — the same active-player-only lapse as the line above.
                    self.revealed_unplayable_until_next_turn
                        .retain(|&(_, owner)| owner != active_player);
                    self.combat_extras.attack_declarer = None;
                    self.combat_extras.block_declarer = None;
                    // "Entered the battlefield this turn" (Oran-Rief, the Vastwood) expires at
                    // the same turn boundary — every battlefield permanent's, not just the
                    // active player's (a new turn, anyone's, ends "this turn").
                    for id in self.battlefield() {
                        let p = self.permanent_mut(id);
                        // Rasputin Dreamweaver's "if Rasputin started the turn untapped": this
                        // block runs before the untap step's own turn-based action, so the tapped
                        // state read here is the one the turn began in.
                        p.started_turn_untapped = !p.tapped;
                        p.entered_this_turn = false;
                        p.attacked_this_turn = false;
                        // Disintegrate's "this turn" riders expire at the same boundary.
                        p.cant_be_regenerated_this_turn = false;
                        p.exile_instead_of_dying_this_turn = false;
                    }
                } else if step == Step::Upkeep {
                    // Power Surge reads "the number of untapped lands they controlled at the
                    // beginning of this turn" when its upkeep trigger resolves, by which time the
                    // taxed player may have tapped out in response. Snapshot it here, the last
                    // moment before anyone can act (the untap step grants no priority, CR 502.3),
                    // rather than in the Untap arm above — that one applies *before* the untap
                    // turn-based action, so it would count the board as it stood last turn.
                    let counts: Vec<u32> = (0..self.players.len())
                        .map(|i| {
                            self.controlled_battlefield(PlayerId(i as u8))
                                .into_iter()
                                .filter(|&id| {
                                    self.def_of(id).kind.types().intersects(TypeSet::LAND)
                                        && !self.permanent(id).tapped
                                })
                                .count() as u32
                        })
                        .collect();
                    for (player, count) in self.players.iter_mut().zip(counts) {
                        player.untapped_lands_at_turn_start = count;
                    }
                } else if step == Step::EndCombat {
                    // CR "this combat": an `ArmCombatDamageWatch` watch that never fired this
                    // combat expires here, same silent-clear shape as `pending_next_cast`'s own
                    // turn-boundary expiry above.
                    self.delayed_triggers.pending_combat_damage_watch.clear();
                } else if step == Step::Cleanup {
                    self.roll_own_turn_history(active_player);
                }
            }
            Event::LandPlayed {
                permanent,
                from,
                player,
                tapped,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                // Serra Paragon (CR 118.9): a land can only be played from the graveyard under its
                // once-per-turn permission (no other effect plays lands from there), so a
                // graveyard land-play consumes that permission and the land gains the rider.
                let serra_recursion = self.zone_of(from) == Zone::Graveyard;
                let mut perm = fresh_permanent(def, player, false, commander);
                perm.serra_recursion = serra_recursion;
                // `tapped` is decided at the LandPlayed construction site ([`Game::play_land`] /
                // may-reveal answer) via [`Game::enters_tapped`] or the reveal choice — CR 614.13.
                perm.tapped = tapped;
                let id = self.create_object(Some(from), Object::Permanent(perm));
                assert_eq!(id, permanent);
                self.permanent_mut(permanent).continuous_timestamp =
                    self.stamp_continuous_timestamp();
                self.players[player.0 as usize].lands_played += 1;
                if serra_recursion {
                    self.players[player.0 as usize].graveyard_play_used_this_turn = true;
                }
            }
            Event::Tapped { object } => self.permanent_mut(object).tapped = true,
            Event::Untapped { object } => self.permanent_mut(object).tapped = false,
            Event::RegenerationShieldCreated { object } => {
                let p = self.permanent_mut(object);
                p.regeneration_shields = p.regeneration_shields.saturating_add(1);
            }
            Event::Regenerated { object } => {
                let p = self.permanent_mut(object);
                p.regeneration_shields = p.regeneration_shields.saturating_sub(1);
                p.tapped = true;
                p.marked_damage = 0;
                p.deathtouched = false;
                // Remove the regenerated creature from combat (CR 701.15b).
                self.remove_from_combat(object, false);
            }
            Event::RemovedFromCombat {
                object,
                release_solely_blocked,
            } => self.remove_from_combat(object, release_solely_blocked),
            Event::RegenerationShieldsExpired { object } => {
                self.permanent_mut(object).regeneration_shields = 0;
            }
            Event::LostSummoningSickness { object } => {
                self.permanent_mut(object).summoning_sick = false
            }
            Event::CountersPlaced {
                object,
                count,
                source_name,
            } => {
                if count > 0 {
                    self.modifier_provenance
                        .counter_batches
                        .push((object, count, source_name));
                } else if count < 0 {
                    let mut remaining = -count;
                    let batches = &mut self.modifier_provenance.counter_batches;
                    while remaining > 0 {
                        let Some(idx) = batches.iter().rposition(|&(o, _, _)| o == object) else {
                            break;
                        };
                        let take = batches[idx].1.min(remaining);
                        batches[idx].1 -= take;
                        remaining -= take;
                        if batches[idx].1 == 0 {
                            batches.remove(idx);
                        }
                    }
                }
                self.resync_counter_aggregate(object);
            }
            Event::KindCountersPlaced {
                object,
                kind,
                count,
            } => {
                let current = self.permanent(object).kind_counters[kind as usize] as i32;
                self.permanent_mut(object).kind_counters[kind as usize] =
                    (current + count).max(0) as u8;
            }
            Event::PlayerCountersPlaced {
                player,
                kind,
                count,
            } => {
                let slot = &mut self.players[player.0 as usize].kind_counters[kind as usize];
                *slot = (*slot as i32 + count).max(0) as u8;
            }
            Event::LoyaltyChanged { object, amount } => {
                self.permanent_mut(object).loyalty += amount
            }
            Event::LoyaltyActivated { object, active } => {
                self.permanent_mut(object).loyalty_activated = active
            }
            Event::AbilityActivatedThisTurn {
                object,
                ability_index,
            } => self.once_per_turn.activated.push((object, ability_index)),
            Event::TriggeredAbilityThisTurn { source } => self.once_per_turn.triggered.push(source),
            Event::AttachedTo { object, host } => {
                self.permanent_mut(object).attached_to = host;
                // An enchant-graveyard Aura attaching is its own ETB's rewrite taking hold: it
                // now reads "enchant creature put onto the battlefield with this Aura" — record
                // the object that ability names (see `Permanent::enchant_rewrite_host`).
                if host.is_some() && self.def_of(object).enchant_graveyard {
                    self.permanent_mut(object).enchant_rewrite_host = host;
                }
                if host.is_some() {
                    self.permanent_mut(object).continuous_timestamp =
                        self.stamp_continuous_timestamp();
                }
                // CR 302.6/720.3: gaining control of a permanent (here via a control-changing
                // Aura becoming attached) makes it summoning-sick for its new controller until
                // that controller's next untap — it hasn't been under their control since their
                // turn began. Cleared like any sickness in the untap turn-based action.
                // ponytail: only the control *gain* (attach) is marked; when the Aura later
                // leaves and control reverts to the owner, the creature's sickness isn't
                // re-set (untested edge of CR 302.6) — add it if a card cares.
                if let Some(host) = host {
                    let grants_control = self.def_of(object).abilities.iter().any(|a| {
                        matches!(
                            (a.timing, a.effect.clone()),
                            (
                                Timing::Static,
                                Effect::Static(StaticEffect::ControlAttached)
                            )
                        )
                    });
                    if grants_control {
                        self.permanent_mut(host).summoning_sick = true;
                        // CR 800.4a: record when this control Aura took hold so
                        // `controller_of` can rank it against a registry steal on the same host.
                        let ts = self.stamp_control_timestamp();
                        self.play_permissions
                            .aura_control_timestamps
                            .retain(|&(a, _)| a != object);
                        self.play_permissions
                            .aura_control_timestamps
                            .push((object, ts));
                    }
                }
            }
            Event::TempBoost {
                object,
                power,
                toughness,
                keywords,
                source_name,
                ends_at_end_of_combat,
            } => {
                let duration = match ends_at_end_of_combat {
                    true => ModifierDuration::EndOfCombat,
                    false => ModifierDuration::EndOfTurn,
                };
                self.register_modifier(
                    object,
                    source_name,
                    duration,
                    ModifierKind::Boost {
                        power,
                        toughness,
                        keywords,
                    },
                );
            }
            Event::BasePtSetUntilEndOfTurn {
                object,
                power,
                toughness,
                ends_at_end_of_combat,
            } => {
                let duration = match ends_at_end_of_combat {
                    true => ModifierDuration::EndOfCombat,
                    false => ModifierDuration::EndOfTurn,
                };
                self.register_modifier(
                    object,
                    "",
                    duration,
                    ModifierKind::BasePtSet { power, toughness },
                );
            }
            // Halfdane (CR 613.3(7b)): the same layer-7b set, on the one duration in the pool that
            // outlives cleanup without being indefinite.
            Event::BasePtSetUntilEndOfNextUpkeep {
                object,
                power,
                toughness,
                player,
            } => {
                self.register_modifier(
                    object,
                    "",
                    ModifierDuration::EndOfNextUpkeep {
                        player,
                        armed: false,
                    },
                    ModifierKind::BasePtSet { power, toughness },
                );
            }
            // Sentinel, Wall of Tombstones (CR 613.3(7b)): a base-*toughness* set with no duration,
            // so it stacks in the registry and the latest timestamp wins (CR 613.7).
            Event::BaseToughnessSetIndefinite { object, toughness } => {
                self.register_modifier(
                    object,
                    "",
                    ModifierDuration::Indefinite,
                    ModifierKind::BaseToughnessSet { toughness },
                );
            }
            // Quarum Trench Gnomes: durationless, so it stacks in the registry and lapses only
            // when the land leaves the battlefield and becomes a new object (CR 400.7).
            Event::LandProducesColorlessInsteadOf { land, color } => {
                self.register_modifier(
                    land,
                    "",
                    ModifierDuration::Indefinite,
                    ModifierKind::ProducesColorlessInsteadOf(color),
                );
            }
            // Transmutation (CR 613.4e).
            Event::PtSwitchedUntilEndOfTurn { object } => {
                self.register_modifier(
                    object,
                    "",
                    ModifierDuration::EndOfTurn,
                    ModifierKind::PtSwitch,
                );
            }
            // Gabriel Angelfire's "until your next upkeep": no arming — the sweep runs at the
            // *start* of the upkeep step, which the grant's own upkeep had already passed.
            Event::UpkeepStartDurationsEnded { object } => {
                self.modifier_provenance.modifiers.retain(|m| {
                    !matches!(m.duration, ModifierDuration::UntilNextUpkeep { .. })
                        || m.host != object
                });
            }
            Event::UpkeepDurationsEnded { object } => {
                self.modifier_provenance.modifiers.retain_mut(|m| {
                    let ModifierDuration::EndOfNextUpkeep { armed, .. } = &mut m.duration else {
                        return true;
                    };
                    if m.host != object {
                        return true;
                    }
                    // First sweep after registration only arms it: the effect was made during an
                    // upkeep, and it runs until the end of the *next* one.
                    let keep = !*armed;
                    *armed = true;
                    keep
                });
            }
            Event::TypesAddedUntilEndOfTurn {
                object,
                types,
                subtypes,
                colors,
                ends_at_end_of_combat,
            } => {
                let duration = match ends_at_end_of_combat {
                    true => ModifierDuration::EndOfCombat,
                    false => ModifierDuration::EndOfTurn,
                };
                self.register_modifier(
                    object,
                    "",
                    duration,
                    ModifierKind::Became {
                        types,
                        subtypes,
                        colors,
                    },
                );
            }
            // Excava, the Risen Past (CR 611.2c): the reanimated permanent's indefinite set, written
            // as it enters and never cleared at cleanup (resets with the object per CR 400.7).
            Event::ReanimatedCreatureBecame {
                object,
                add_types,
                add_subtypes,
                base_power,
                base_toughness,
                keywords,
            } => {
                let ts = self.stamp_continuous_timestamp();
                let p = self.permanent_mut(object);
                p.added_types = add_types;
                p.added_types_timestamp = ts;
                p.added_subtypes = add_subtypes;
                p.set_base_pt = Some((base_power, base_toughness));
                p.set_base_pt_timestamp = ts;
                p.granted_keywords = keywords;
            }
            // Hofri Ghostforge's minted copy (CR 613.4): the indefinite subtype set, written as the
            // token enters and never cleared at cleanup (resets with the object per CR 400.7).
            // ponytail: overwrites `added_subtypes` rather than unioning — a freshly minted token
            //   carries none, so there is nothing to union with; a second indefinite subtype-add on
            //   one permanent would need a union, but no pool card stacks two.
            Event::AddedSubtypes { object, subtypes } => {
                self.permanent_mut(object).added_subtypes = subtypes;
            }
            // Gaea's Liege (CR 613.4/305.7): the target land's whole land-type line, replaced for
            // as long as `source` stays on the battlefield. Nothing ever clears this — the read
            // side stops finding a live source, which is the entire duration.
            Event::SubtypesSetWhileSourceRemains {
                object,
                subtypes,
                source,
            } => {
                let timestamp = self.stamp_continuous_timestamp();
                self.permanent_mut(object).subtypes_set_while_source_remains =
                    Some((subtypes, source, timestamp));
            }
            // Trench Gorger (CR 613.3(7b)): the indefinite base-P/T-only sibling of
            // `ReanimatedCreatureBecame` above.
            Event::BasePtSetIndefinite {
                object,
                power,
                toughness,
            } => {
                let ts = self.stamp_continuous_timestamp();
                let p = self.permanent_mut(object);
                p.set_base_pt = Some((power, toughness));
                p.set_base_pt_timestamp = ts;
            }
            // A permanent became a copy of another creature as it entered (CR 706/707.2). Overwrite
            // its `def` with the copied `def`; for an until-EOT copy, stash the original first so
            // cleanup can restore it (Cursed Mirror). These are `CardId` handle swaps, not full
            // `CardDef` clones.
            Event::BecameCopy {
                object,
                def,
                until_eot,
                also_types,
            } => {
                let added_types_timestamp = self.stamp_continuous_timestamp();
                // An *indefinite* rewrite (CR 400.7) disarms any revert already armed on this
                // permanent: otherwise `Event::TempBoostsEnded` would restore the pre-copy def at
                // cleanup and undo a later-timestamped, durationless effect (Vraska, Betrayal's
                // Sting's −2 on a Cursed Mirror that is currently copying a creature). An
                // until-EOT rewrite re-arms with whatever def is live *now*, so a copy stacked on
                // a copy reverts one step, exactly as the single slot this replaced did.
                // ponytail: that rewrite snapshots whatever def is live now, so it inherits the
                // copy's name/cost rather than the printed card's — a CR 613 layered
                // recomputation would keep the printed copiable values under the layer-4/6 set.
                let printed = self.permanent_mut(object).def;
                self.modifier_provenance.modifiers.retain(|m| {
                    m.host != object || !matches!(m.kind, ModifierKind::RevertsToDef(_))
                });
                if until_eot {
                    self.register_modifier(
                        object,
                        "",
                        ModifierDuration::EndOfTurn,
                        ModifierKind::RevertsToDef(printed),
                    );
                }
                let p = self.permanent_mut(object);
                p.def = def;
                // Copy Artifact's "except it's an enchantment in addition to its other types" (CR
                // 707.2). The indefinite slot, not the until-EOT one: the added type lasts exactly
                // as long as the copy and resets with the object (CR 400.7). Assigned rather than
                // unioned, for the same reason `copy_rider_keywords` is cleared just below — a new
                // copy effect replaces the whole copiable picture, exceptions included.
                p.added_types = also_types;
                p.added_types_timestamp = added_types_timestamp;
                // A new copy effect replaces the object's copiable characteristics wholesale (CR
                // 707.2), so any "except it has <keywords>" rider from a *prior* copied form is
                // dropped. This effect's own rider (if any) is re-established by the
                // `CopyRiderKeywordsGranted` event(s) that follow this `BecameCopy`.
                p.copy_rider_keywords = &[];
            }
            // Gabriel Angelfire's "gains that ability until your next upkeep": a keyword-only
            // boost with its own start-of-upkeep sweep (see `Step::Upkeep` in `priority.rs`).
            Event::KeywordsGrantedUntilNextUpkeep {
                object,
                keywords,
                player,
            } => {
                self.register_modifier(
                    object,
                    "",
                    ModifierDuration::UntilNextUpkeep { player },
                    ModifierKind::Boost {
                        power: 0,
                        toughness: 0,
                        keywords,
                    },
                );
            }
            // Cocoon's "that creature gains flying": a keyword-only boost the cleanup sweep skips.
            Event::KeywordsGrantedIndefinitely { object, keywords } => {
                self.register_modifier(
                    object,
                    "",
                    ModifierDuration::Indefinite,
                    ModifierKind::Boost {
                        power: 0,
                        toughness: 0,
                        keywords,
                    },
                );
            }
            Event::TempBoostsEnded {
                object,
                end_of_combat_only,
            } => {
                // At cleanup every modifier on `object` that has a duration ends together (CR
                // 514.2); the durationless ones (a lace's "becomes black") survive, since nothing
                // but the object itself changing zones clears those. The end of combat step's
                // sweep is the narrower one (CR 511.3) — only what was scoped to this combat.
                let mut reverts_to = None;
                self.modifier_provenance.modifiers.retain(|m| {
                    let ends_now = match end_of_combat_only {
                        true => m.duration == ModifierDuration::EndOfCombat,
                        false => m.duration.ends_at_cleanup(),
                    };
                    if m.host != object || !ends_now {
                        return true;
                    }
                    if let ModifierKind::RevertsToDef(printed) = m.kind {
                        reverts_to = Some(printed);
                    }
                    false
                });
                // Revert an until-EOT enter-as-copy to the printed permanent (CR 514.2 — Cursed
                // Mirror's "become a copy … until end of turn"). The copy's "except it has
                // haste/myriad" rider ends with the copy, so clear the copiable rider too (an
                // indefinite copy or a token leaves it in place — it resets with the object).
                if let Some(printed) = reverts_to {
                    let p = self.permanent_mut(object);
                    p.def = printed;
                    p.copy_rider_keywords = &[];
                }
            }
            // A copy made "except it has <keywords>" (CR 707.2): union the exception keywords into
            // the object's copiable characteristics (they persist and are copied again), rather
            // than the until-end-of-turn `TempBoost` an ordinary keyword grant uses.
            Event::CopyRiderKeywordsGranted { object, keywords } => {
                let p = self.permanent_mut(object);
                if p.copy_rider_keywords.is_empty() {
                    p.copy_rider_keywords = keywords;
                } else {
                    // Union-not-clobber for a second rider landing on the same object (a copy of a
                    // copy that itself carries a different rider). Leaks a small deduped slice to
                    // keep `Permanent: Copy`, bounded by one leak per such collision (mirrors
                    // `KeywordsStripped`'s own union above).
                    let mut union: Vec<Keyword> = p.copy_rider_keywords.to_vec();
                    for k in keywords {
                        if !union.contains(k) {
                            union.push(*k);
                        }
                    }
                    p.copy_rider_keywords = Box::leak(union.into_boxed_slice());
                }
            }
            // A second strip landing on the same permanent the same turn is just a second
            // registered modifier — both are subtracted from the unioned keyword set, so nothing
            // has to leak a merged `&'static` slice to fit a single field.
            Event::KeywordsStripped {
                object,
                keywords,
                families,
                until_end_of_turn,
                cant_have,
            } => {
                let duration = if until_end_of_turn {
                    ModifierDuration::EndOfTurn
                } else {
                    ModifierDuration::Indefinite
                };
                self.register_modifier(
                    object,
                    "",
                    duration,
                    ModifierKind::LoseKeywords {
                        keywords,
                        families,
                        cant_have,
                    },
                );
            }
            Event::AttachedKeywordsLost {
                source, keywords, ..
            } => {
                // ponytail: clobber, not the union `KeywordsStripped` above does — one Aura gains
                // one such ability, and the only card that has it can only gain it once (its own
                // enters trigger). Union here when a second source can stack onto the same Aura.
                self.permanent_mut(source).attachment_lost_keywords = keywords;
            }
            Event::ControlGainedUntilEndOfTurn {
                object,
                controller,
                source_name,
            } => {
                let ts = self.stamp_control_timestamp();
                self.play_permissions
                    .control_overrides
                    .push((object, controller, source_name, ts));
                // CR 506.4c: any time a permanent's controller changes, it's removed from combat.
                self.remove_from_combat(object, false);
            }
            Event::ControlEndedUntilEndOfTurn { object } => self
                .play_permissions
                .control_overrides
                .retain(|&(o, ..)| o != object),
            Event::AbilitiesGranted { target, source } => {
                self.abilities_granted_until_eot.push((target, source));
            }
            Event::GrantedAbilitiesEnded => self.abilities_granted_until_eot.clear(),
            Event::ControlGained { object, controller } => {
                let ts = self.stamp_control_timestamp();
                self.play_permissions
                    .permanent_control_overrides
                    .push((object, controller, ts));
                // CR 506.4c: any time a permanent's controller changes, it's removed from combat
                // (Goblin Cadets' reminder text — "(This removes this creature from combat.)").
                self.remove_from_combat(object, false);
            }
            Event::ConditionedControlGained {
                object,
                controller,
                condition,
            } => {
                let ts = self.stamp_control_timestamp();
                self.play_permissions
                    .conditioned_control_overrides
                    .push((object, controller, condition, ts));
                // CR 506.4c: any time a permanent's controller changes, it's removed from combat.
                self.remove_from_combat(object, false);
            }
            Event::ConditionedControlEnded { object } => self
                .play_permissions
                .conditioned_control_overrides
                .retain(|&(o, ..)| o != object),
            Event::AttackerDeclared {
                object,
                defender,
                defender_planeswalker,
            } => {
                self.combat.attackers.push(object);
                let target = defender_planeswalker
                    .map_or(Defender::Player(defender), Defender::Planeswalker);
                self.combat.attack_targets.push((object, target));
                // CR 508.1: turn-scoped "attacked this turn" flag (`Condition::SourceAttackedThisTurn`)
                // — set here, not in `declare_attackers` (event-sourced state: intents mint events,
                // events mutate board facts); cleared at the next Untap step below.
                self.permanent_mut(object).attacked_this_turn = true;
                self.combat.attacked_or_blocked.push(object);
                // Angelic Arbiter's "attacked with a creature this turn" tracking (turn-scoped;
                // reset at Untap alongside the other this-turn tallies above).
                let controller = self.controller_of(object);
                self.players[controller.0 as usize].attacked_this_turn = true;
            }
            Event::TokenEnteredAttacking { token, defender } => {
                self.combat.attackers.push(token);
                self.combat
                    .attack_targets
                    .push((token, Defender::Player(defender)));
            }
            Event::Goaded {
                object,
                by,
                source_name,
            } => self.combat_extras.goaded.push((object, by, source_name)),
            Event::GoadCleared { by } => self.combat_extras.goaded.retain(|&(_, g, _)| g != by),
            Event::VowCountersPlaced { object, protected } => {
                let slot = &mut self.permanent_mut(object).kind_counters[CounterKind::Vow as usize];
                *slot = slot.saturating_add(1);
                self.permanent_mut(object).vow_protected = Some(protected);
            }
            Event::TimeCountersPlaced { card, count } => {
                self.exile_time_counters.push((card, count))
            }
            Event::TimeCountersRemoved { card } => {
                if let Some(idx) = self
                    .exile_time_counters
                    .iter()
                    .position(|(id, _)| *id == card)
                {
                    self.exile_time_counters[idx].1 =
                        self.exile_time_counters[idx].1.saturating_sub(1);
                    // The last counter gone: drop the entry (the card is no longer suspended;
                    // its owner is granted the free cast by the same upkeep turn-based action).
                    if self.exile_time_counters[idx].1 == 0 {
                        self.exile_time_counters.remove(idx);
                    }
                }
            }
            Event::MustAttackDeclared { object, defender } => {
                self.combat_extras.must_attack.push((object, defender))
            }
            Event::DelayedTriggerScheduled {
                controller,
                source,
                fire_at,
                effect,
            } => self
                .delayed_triggers
                .scheduled
                .push((controller, source, fire_at, effect)),
            Event::DelayedTriggerScheduledForYourNextUpkeep {
                controller,
                source,
                effect,
            } => self
                .delayed_triggers
                .scheduled_your_upkeep
                .push((controller, source, effect)),
            Event::DelayedTriggersFired {
                fire_at,
                active_player,
            } => {
                self.delayed_triggers.scheduled.retain(|&(c, _, f, _)| {
                    // Mirror `fire_delayed_triggers`'s `due` filter: `Main1` is controller-scoped
                    // (only the active player's entries fired, so only those are drained); every
                    // other timing fired regardless of whose step it is.
                    !(f == fire_at && (fire_at != Step::Main1 || c == active_player))
                });
                // "Your next upkeep" is always controller-scoped, so only the active player's
                // entries fired and only those drain.
                if fire_at == Step::Upkeep {
                    self.delayed_triggers
                        .scheduled_your_upkeep
                        .retain(|&(c, _, _)| c != active_player);
                }
            }
            Event::ExtraTurnQueued { player } => self.extra_turns.push(player),
            Event::NextUntapSkipMarked { object } => self.skip_next_untap.push(object),
            // One mark per skipped untap step, so a permanent marked twice (Telekinesis'
            // "next two untap steps") spends one now and keeps the other for the step after.
            Event::NextUntapSkipConsumed { object } => {
                if let Some(at) = self.skip_next_untap.iter().position(|&id| id == object) {
                    self.skip_next_untap.remove(at);
                }
            }
            Event::CantBeRegeneratedThisTurnMarked { object } => {
                self.permanent_mut(object).cant_be_regenerated_this_turn = true;
            }
            Event::NextCastTriggerArmed {
                controller,
                source,
                filter,
                then,
            } => self
                .delayed_triggers
                .pending_next_cast
                .push((controller, source, filter, then)),
            Event::NextCastTriggerConsumed { controller, source } => {
                self.delayed_triggers
                    .pending_next_cast
                    .retain(|&(c, s, _, _)| !(c == controller && s == source));
            }
            Event::DiesThisTurnWatchArmed {
                controller,
                source,
                watched,
                then,
            } => self
                .delayed_triggers
                .pending_dies_this_turn
                .push((controller, source, watched, then)),
            Event::DiesThisTurnWatchConsumed { controller, source } => {
                self.delayed_triggers
                    .pending_dies_this_turn
                    .retain(|&(c, s, _, _)| !(c == controller && s == source));
            }
            Event::CombatDamageWatchArmed {
                controller,
                source,
                watched,
            } => self
                .delayed_triggers
                .pending_combat_damage_watch
                .push((controller, source, watched)),
            Event::CombatDamageWatchConsumed { controller, source } => {
                self.delayed_triggers
                    .pending_combat_damage_watch
                    .retain(|&(c, s, _)| !(c == controller && s == source));
            }
            Event::CombatDamageCopyArmed {
                controller,
                source,
                card,
            } => self
                .delayed_triggers
                .pending_combat_damage_copy
                .push((controller, source, card)),
            Event::ExiledFromLibraryMayPlay {
                player,
                card,
                from,
                until_next_turn,
                face_down,
                free_while_source,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner: self.owner_of(from),
                        zone: Zone::Exile,
                        commander,
                        face_down,
                    }),
                );
                assert_eq!(id, card);
                self.players[player.0 as usize]
                    .library
                    .retain(|&o| o != from);
                // Intet, the Dreamer's grant lives in its own registry: free, and scoped to the
                // granting permanent's presence rather than to a cleanup step.
                match free_while_source {
                    Some(source) => self
                        .play_permissions
                        .play_from_exile_free_while_source
                        .push((card, player, source)),
                    None => {
                        self.play_permissions
                            .play_from_exile
                            .push((card, player, until_next_turn))
                    }
                }
            }
            // Herald of Amity's dig: exile face-up, no permission attached — the follow-up
            // choice grants `CastFromExileFreePermissionGranted` for at most one of the batch.
            Event::ExiledFromLibraryToChooseCastFree {
                player,
                card,
                from,
                face_down,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner: self.owner_of(from),
                        zone: Zone::Exile,
                        commander,
                        face_down,
                    }),
                );
                assert_eq!(id, card);
                self.players[player.0 as usize]
                    .library
                    .retain(|&o| o != from);
            }
            Event::PlayFromExilePermissionArmed { card } => {
                if let Some(entry) = self
                    .play_permissions
                    .play_from_exile
                    .iter_mut()
                    .find(|(c, _, _)| *c == card)
                {
                    entry.2 = false;
                }
            }
            Event::PlayFromExileEnded => self
                .play_permissions
                .play_from_exile
                .retain(|&(_, _, extended)| extended),
            Event::ExiledFromGraveyardMayPlay { player, card, from } => {
                let def = self.def_id_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Exile,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.play_permissions
                    .play_from_exile
                    .push((card, player, false));
            }
            // A pure signal event for trigger-scanning (`Game::queue_discard_triggers`) — the
            // actual zone change is the `MovedToGraveyard` emitted alongside it at the same call
            // site.
            Event::Discarded { .. } => {}
            Event::BlockerDeclared { blocker, attacker } => {
                // The Glyph cycle's turn-scoped ledger, snapshotting the attacker's controller as
                // it becomes blocked (`CombatExtras::blocked_this_turn`) — the combat-scoped
                // `blocked_ever` beside it dies at end of combat, which is where those three
                // Glyphs start reading.
                let attacker_controller = self.controller_of(attacker);
                self.combat_extras
                    .blocked_this_turn
                    .push((blocker, attacker, attacker_controller));
                self.combat.blocks.push((blocker, attacker));
                self.combat.blocked_ever.push((blocker, attacker));
                self.combat.attacked_or_blocked.push(blocker);
            }
            Event::CombatDamageDivided { source, assignment } => {
                self.combat.damage.push((source, assignment.pairs()))
            }
            Event::DeathtouchMarked { object } => self.permanent_mut(object).deathtouched = true,
            Event::CombatCleared => self.combat = CombatState::default(),
            Event::CommanderCastFromCommandZone { player } => {
                self.players[player.0 as usize].command_casts += 1
            }
            Event::FlashPermissionGranted { player } => {
                self.players[player.0 as usize].flash_permission_this_turn = true
            }
            Event::ChannelColorlessManaGranted { player } => {
                self.players[player.0 as usize].channel_colorless_mana_this_turn = true
            }
            Event::SpendManaAsAnyTypeGranted { player } => {
                self.players[player.0 as usize].spend_mana_as_any_type_this_turn = true
            }
            Event::CommanderDamageDealt {
                source,
                player,
                amount,
            } => {
                // Keyed by the source commander's owner (each player has one commander).
                let key = self.owner_of(source);
                let taken = &mut self.players[player.0 as usize].commander_damage;
                match taken.iter_mut().find(|(o, _)| *o == key) {
                    Some(entry) => entry.1 += amount,
                    None => taken.push((key, amount)),
                }
            }
            // `Game::queue_combat_damage_triggers` reads this off the events batch in
            // `enqueue_triggers`; the life loss it accompanies already applied via `LifeChanged`,
            // so the only state here is the turn-scoped damage-taken tally behind
            // `Amount::DamageTakenThisTurn` (Simulacrum's "the damage dealt to you this turn").
            Event::CombatDamageDealtToPlayer {
                player,
                amount,
                source,
            } => {
                self.players[player.0 as usize].damage_taken_this_turn += amount.max(0) as u32;
                self.record_damage_dealt(source, Target::Player(player), amount);
            }
            // A marker only — `Game::enqueue_triggers`'s `Event::CombatDamageDealtToCreature` arm
            // reads it, but it mutates no state of its own (the marked damage it accompanies
            // already applied via `DamageMarked`).
            Event::CombatDamageDealtToCreature { .. } => {}
            // The noncombat twin of the arm above. A marker for trigger purposes, but it also
            // feeds the turn-scoped tally behind `Amount::DamageTakenThisTurn` (Simulacrum) —
            // these two arms are the only places damage reaches a player, and life loss that
            // isn't damage never passes through either.
            Event::DamageDealtToPlayer {
                player,
                amount,
                source,
            } => {
                self.players[player.0 as usize].damage_taken_this_turn += amount.max(0) as u32;
                self.record_damage_dealt(source, Target::Player(player), amount);
            }
            // A marker only — the prevented damage's absence (no `LifeChanged`) and the Inkling
            // mints (accompanying `TokenCreated` events) carry all the state; this event mutates
            // nothing itself.
            Event::CombatDamagePrevented { .. } => {}
            // Unlike the marker above, this one is the whole state change: spend `amount` points
            // off `target`'s shields, oldest first, dropping each as it empties.
            // `Game::spend_prevention_shields` minted the event by walking the same list in the
            // same order, so the two never disagree about what there was to spend.
            Event::DamagePrevented {
                target,
                amount,
                source,
            } => {
                let mut left = amount;
                let stands: Vec<bool> = self
                    .damage_prevention_shields
                    .iter()
                    .map(|shield| self.shield_stands_between(shield, target, source))
                    .collect();
                let mut index = 0;
                self.damage_prevention_shields.retain_mut(|shield| {
                    let stood = stands[index];
                    index += 1;
                    if left <= 0 || !stood {
                        return true;
                    }
                    // CR 615.6: a "prevent all damage … this turn" shield isn't used up by what
                    // it prevents — it stays until end of turn, however many hits it eats.
                    if shield.persistent {
                        left = 0;
                        return true;
                    }
                    // "Prevent that damage" and Forcefield's "prevent all but 1 of that damage"
                    // are both spent outright by the hit they stood in front of, however big; a
                    // point shield keeps whatever the hit didn't need.
                    if shield.keep.is_some() {
                        left = 0;
                        return false;
                    }
                    let Some(points) = shield.amount.as_mut() else {
                        left = 0;
                        return false;
                    };
                    let spent = left.min(*points);
                    left -= spent;
                    *points -= spent;
                    *points > 0
                });
            }
            Event::MovedToCommandZone { card, from } => {
                let def = self.def_id_of(from);
                let owner = self.owner_of(from);
                if matches!(&self.objects[from as usize], Object::Permanent(_)) {
                    self.clear_modifier_provenance(from);
                }
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Command,
                        commander: true,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.remove_spell_from_stack(from);
            }
            // CR 114.1/114.3: the emblem is created in its owner's command zone with only the
            // abilities `def` carries. `commander: false` is what distinguishes it from an actual
            // commander there — the engine's only other two ways into `Zone::Command`
            // (`Game::designate_commander` and `MovedToCommandZone` above) both hardcode `true`,
            // and both castability gates (`Game::cast`, `Game::playable`) require it, so an
            // emblem is never castable. Nothing ever removes it (CR 114.5).
            Event::EmblemCreated {
                emblem,
                controller,
                def,
            } => {
                let id = self.create_object(
                    None,
                    Object::Card(Card {
                        def: intern_card_def(def),
                        owner: controller,
                        zone: Zone::Command,
                        commander: false,
                        face_down: false,
                    }),
                );
                assert_eq!(id, emblem);
            }
            Event::ManaEmptied {
                player,
                end_of_turn,
                to,
            } => {
                // Drain Power: what the pool loses, the drainer gains — read before the clear
                // below wipes it. Credit kinds carry over whole, so a dual land's either-credit
                // arrives as an either-credit and stays as flexible as it was.
                if let Some(to) = to {
                    let taken = self.players[player.0 as usize].mana_pool;
                    self.players[to.0 as usize].mana_pool.merge(&taken);
                }
                let p = &mut self.players[player.0 as usize];
                // Provenance is never persistent (no pool card combines `track_provenance` with
                // `persist_until_end_of_turn`) — always cleared with the pool.
                p.mana_provenance.clear();
                if end_of_turn {
                    // The turn actually ending (CR 514.2 cleanup) — even "until end of turn"
                    // mana empties now.
                    p.mana_pool = ManaPool::default();
                    p.persistent_mana = ManaPool::default();
                } else {
                    // A mid-turn step/phase boundary — keep only the credits still floating in
                    // both pools (some persistent mana may already have been spent), CR 500.4's
                    // "until end of turn" exception.
                    let keep = p.mana_pool.componentwise_min(&p.persistent_mana);
                    p.mana_pool = keep;
                    p.persistent_mana = keep;
                }
            }
            Event::DamageCleared { object } => {
                let p = self.permanent_mut(object);
                p.marked_damage = 0;
                p.deathtouched = false;
            }
            Event::ManaAdded {
                player,
                mana,
                amount,
                persist,
            } => {
                let p = &mut self.players[player.0 as usize];
                p.mana_pool.add(mana, amount);
                if persist {
                    p.persistent_mana.add(mana, amount);
                }
            }
            Event::ManaSpent { player, mana } => {
                self.players[player.0 as usize].mana_pool.subtract(&mana)
            }
            Event::PriorityPassed { .. } => {}
            Event::PermanentEntered { permanent, from } => {
                let (
                    def,
                    owner,
                    commander,
                    x,
                    serra_recursion,
                    bestowed,
                    copy,
                    cast_target,
                    face_down,
                    masked,
                    evoked,
                    multikicker_count,
                    spent_colors,
                    phyrexian_life_paid,
                    cast_from_hand,
                ) = match &self.objects[from as usize] {
                    Object::Spell(s) => (
                        s.def,
                        s.controller,
                        s.commander,
                        s.x,
                        s.serra_recursion,
                        s.bestowed,
                        s.copy,
                        s.targets.primary(),
                        s.face_down,
                        s.masked,
                        s.evoked,
                        s.multikicker_count,
                        s.spent_colors,
                        s.phyrexian_life_paid,
                        s.cast_from_hand,
                    ),
                    _ => panic!("PermanentEntered source {from} is not a spell"),
                };
                let id = self.create_object(
                    Some(from),
                    Object::Permanent(fresh_permanent(def, owner, true, commander)),
                );
                assert_eq!(id, permanent);
                self.permanent_mut(permanent).continuous_timestamp =
                    self.stamp_continuous_timestamp();
                // See `Permanent::entered_with_x`'s doc — locked in here while `from` is still
                // the resolving Spell, before `remove_spell_from_stack` below takes it away.
                self.permanent_mut(permanent).entered_with_x = x;
                // See `Permanent::entered_multikicker_count`'s doc — same "read it before the
                // spell is gone" idiom as `entered_with_x` above (Lightkeeper of Emeria's ETB).
                self.permanent_mut(permanent).entered_multikicker_count = multikicker_count;
                // See `Permanent::cast_time_enchant_target`'s doc — same "read it before the
                // spell is gone" idiom as `entered_with_x` above. Harmless to set for every
                // permanent (not just `enchant_graveyard` ones): `ThisAurasGraveyardTarget` is
                // the only reader, and it's never a card's own effect target otherwise.
                self.permanent_mut(permanent).cast_time_enchant_target =
                    cast_target.and_then(Target::object_id);
                // Serra Paragon (CR 118.9): a permanent cast from the graveyard this way carries
                // the granted exile-and-gain-2-life rider.
                self.permanent_mut(permanent).serra_recursion = serra_recursion;
                // Bestow (CR 702.103d): a bestowed spell enters as a dual-nature Aura/creature — it
                // is an Aura while attached, a creature once it stops being attached.
                self.permanent_mut(permanent).bestowed = bestowed;
                // Morph (CR 702.37b/708): a face-down creature spell enters as a face-down 2/2 —
                // its real characteristics stay hidden (the characteristics choke reads this flag)
                // until it's turned face up.
                self.permanent_mut(permanent).face_down = face_down;
                // Masked (CR 615 — Illusionary Mask): a face-down creature it put onto the
                // battlefield turns face up when it would assign or deal damage, be dealt damage,
                // or become tapped. `false` for a plain morph/manifest face-down permanent.
                self.permanent_mut(permanent).masked = masked;
                // Evoke (CR 702.74a): an evoked spell's resulting permanent is sacrificed the
                // instant it enters — the self-sacrifice fires as its own trigger, queued
                // alongside the permanent's ETB triggers (`Game::enqueue_triggers`), so an ETB
                // payoff (Mulldrifter's draw two) still resolves first.
                self.permanent_mut(permanent).evoked = evoked;
                // Compleated (CR 107.4f — Vraska, Betrayal's Sting): a {a/P} pip paid with life
                // means the planeswalker enters with two fewer loyalty counters, two per pip so
                // paid. A one-shot as-enters adjustment, not durable state — no new `Permanent`
                // field needed, the same idiom as `entered_with_x` above.
                if phyrexian_life_paid > 0 {
                    self.permanent_mut(permanent).loyalty -= 2 * i32::from(phyrexian_life_paid);
                }
                // See `Permanent::spent_colors`'s doc — same "read it before the spell is gone"
                // idiom as `entered_with_x` above (Court Hussar's "unless {W} was spent to cast it").
                self.permanent_mut(permanent).spent_colors = spent_colors;
                // Dread Cacodemon/Reiver Demon: "if you cast it from your hand" — same
                // "read it before the spell is gone" idiom as `spent_colors` just above.
                self.permanent_mut(permanent).cast_from_hand = cast_from_hand;
                // CR 707.10a: a copy of a permanent spell becomes a token as it resolves — it
                // ceases to exist (rather than going to the graveyard) once it leaves the
                // battlefield, via the same `Permanent::token` machinery any other token uses.
                self.permanent_mut(permanent).token = copy;
                self.remove_spell_from_stack(from);
            }
            Event::ReanimatedToBattlefield {
                permanent,
                from,
                controller,
                finality,
                tapped,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                // ponytail: the engine conflates control with ownership for permanents (there is no
                // separate controller field — `controller_of` returns the owner), so "under your
                // control" is expressed as owner = the reanimator. A reanimated creature therefore
                // also counts as *owned* by the reanimator (its death would route to their
                // graveyard, not the true owner's) — acceptable for the pool; add a real control
                // field if a card ever cares about the owner/controller split.
                let id = self.create_object(
                    Some(from),
                    Object::Permanent(fresh_permanent(def, controller, true, commander)),
                );
                assert_eq!(id, permanent);
                self.permanent_mut(permanent).continuous_timestamp =
                    self.stamp_continuous_timestamp();
                // Excava, the Risen Past (CR 614.12): the finality counter is present the instant
                // the reanimated permanent enters — mirrors `EntersWithCounters`'s `plus_counters`
                // set right after `create_object`, above.
                self.permanent_mut(permanent).finality_counter = finality;
                // Teacher's Pest: "... to the battlefield tapped." `fresh_permanent` already
                // covers a def's own `enters_tapped`; this ORs in the effect-level `tapped` rider.
                if tapped {
                    self.permanent_mut(permanent).tapped = true;
                }
            }
            Event::TokenCreated {
                token,
                controller,
                def,
                creator: _,
            } => {
                let id = self.create_object(None, Object::Permanent(fresh_token(def, controller)));
                assert_eq!(id, token);
                self.permanent_mut(token).continuous_timestamp = self.stamp_continuous_timestamp();
            }
            // Wood Elemental: "the number of Forests sacrificed as it entered" is remembered the
            // same way a cast X is (see `Permanent::entered_with_x`).
            Event::EnteredWithXSet { object, x } => {
                if self.as_permanent(object).is_some() {
                    self.permanent_mut(object).entered_with_x = x;
                }
            }
            // Stangg and his Twin: point each half at the other, so whichever one survives can
            // still name its partner from the battlefield after the other has left.
            Event::TwinLinked { a, b } => {
                if self.as_permanent(a).is_some() && self.as_permanent(b).is_some() {
                    self.permanent_mut(a).linked_twin = Some(b);
                    self.permanent_mut(b).linked_twin = Some(a);
                }
            }
            Event::TokenCeasedToExist {
                token,
                controller,
                def,
            } => {
                // CR 506.4: a token that ceases to exist is removed from combat.
                self.remove_from_combat(token, false);
                let printed = card_def(def);
                // CR 603.6c/704.5m last-known information: capture the Aura(s) attached to this
                // token *before* it vanishes, so `Trigger::EnchantedCreatureDies` can still find
                // them once the token's arena slot (and the Aura's own `attached_to`) is gone —
                // see `Game::dying_creature_attachments`.
                if matches!(&printed.kind, CardKind::Creature { .. }) {
                    for aura in self.attachments(token) {
                        let aura_controller = self.controller_of(aura);
                        let aura_def = self.def_of(aura);
                        self.batch_trigger_scratch.dying_creature_attachments.push((
                            token,
                            aura,
                            aura_controller,
                            aura_def,
                        ));
                    }
                    // CR 603.10a last-known information — see `Game::dying_creature_stats`.
                    self.batch_trigger_scratch
                        .dying_creature_stats
                        .push(DyingCreatureStats {
                            id: token,
                            power: self.power(token),
                            toughness: self.toughness(token),
                            plus_counters: self.plus_counters(token),
                            controller: self.controller_of(token),
                        });
                    // CR 700.4/701.29 last-known information: a token ceasing to exist is a
                    // "died" too — read `is_modified` before `Object::Removed` below erases its
                    // attachments/counters. Feeds `Condition::ModifiedCreatureDiedThisTurn`.
                    if self.is_modified(token, controller) {
                        self.players[controller.0 as usize].modified_creature_died_this_turn = true;
                    }
                }
                // CR 603.10a last-known information: the host this token was attached to (if it
                // was itself an Aura/Equipment), captured before `Object::Removed` below erases
                // it — the accumulator behind `Trigger::ThisPermanentLeavesBattlefield` (Animate
                // Dead). Unconditional, like the `ThisAuraLeaves` scan below: any permanent kind.
                self.batch_trigger_scratch
                    .permanents_left_battlefield
                    .push((token, self.attached_to(token)));
                self.clear_modifier_provenance(token);
                // Resolution-scoped last-known owner (see `ResolutionFrame::vanished_permanent_owner`)
                // for a later same-`Sequence` step (Oblation's `target_owner_draws` rider) that
                // would otherwise panic reading `owner_of` a now-`Object::Removed` id.
                self.resolution_frame.vanished_permanent_owner = Some((token, controller));
                self.objects[token as usize] = Object::Removed {
                    def,
                    owner: controller,
                };
            }
            Event::DamageMarked {
                object,
                amount,
                cant_be_regenerated,
                exile_instead_of_dying,
                source,
            } => {
                if let Some(source) = source {
                    self.record_damage_dealt(source, Target::Object(object), amount);
                }
                let p = self.permanent_mut(object);
                p.marked_damage += amount;
                // Disintegrate's riders mark the creature, not the damage — they stay set for the
                // rest of the turn even when this hit isn't the one that kills it.
                p.cant_be_regenerated_this_turn |= cant_be_regenerated;
                p.exile_instead_of_dying_this_turn |= exile_instead_of_dying;
            }
            // A pure signal event for trigger-scanning (`Game::queue_sacrifice_triggers`) — the
            // actual zone change is a separate event (`MovedToGraveyard`/`MovedToCommandZone`/
            // `TokenCeasedToExist`) emitted alongside it at the same call site.
            Event::Sacrificed { .. } => {}
            Event::MovedToGraveyard { card, from } => {
                // CR 506.4: a permanent that leaves the battlefield is removed from combat.
                if matches!(&self.objects[from as usize], Object::Permanent(_)) {
                    self.remove_from_combat(from, false);
                }
                // Feeds `Amount::PermanentsDiedThisTurn` (Ominous Harvest's Gravestorm): `from`
                // being a live battlefield `Object::Permanent` (not a hand/exile/stack card
                // heading to the graveyard by discard, resolution, or counter) is exactly CR
                // 700.4's "died" — put into a graveyard from the battlefield. A token's death is
                // the separate `TokenCeasedToExist` event, not counted here (see that `Amount`
                // variant's doc).
                if matches!(&self.objects[from as usize], Object::Permanent(_)) {
                    self.permanents_died_this_turn += 1;
                    // CR "put into a graveyard from the battlefield" — `Trigger::ThisAuraLeaves`
                    // (Fallen Ideal) reads this in `enqueue_triggers`, once the pre-move object
                    // below has been overwritten into `Object::Moved` and can no longer answer
                    // "was this a permanent?" on its own.
                    self.batch_trigger_scratch
                        .permanents_put_into_graveyard_from_battlefield
                        .push(from);
                    // CR 603.10a last-known information: the host this permanent was attached to
                    // (if it was itself an Aura/Equipment) — the accumulator behind
                    // `Trigger::ThisPermanentLeavesBattlefield` (Animate Dead), read before the
                    // exit tears the attachment down.
                    self.batch_trigger_scratch
                        .permanents_left_battlefield
                        .push((from, self.attached_to(from)));
                    // Serra Paragon's granted rider (CR 118.9): last-known information for the
                    // real placed trigger `Game::enqueue_triggers` fabricates once this permanent
                    // is gone — see `serra_recursion_deaths`'s doc comment.
                    if self.as_permanent(from).is_some_and(|p| p.serra_recursion) {
                        self.batch_trigger_scratch.serra_recursion_deaths.push(from);
                    }
                }
                let def = self.def_id_of(from);
                let printed = card_def(def);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                // CR 603.6c/704.5m last-known information: capture the Aura(s) attached to this
                // creature *before* `create_object` tombstones it — whether this death was a
                // state-based action (lethal damage) or a direct effect (Destroy), the Aura's own (CR 704, CR 303.4, CR 120.3)
                // orphan-to-graveyard SBA hasn't run yet, so it's still attached right now. Read (CR 704, CR 303.4, CR 403.5)
                // back by `Trigger::EnchantedCreatureDies` in `enqueue_triggers`; see
                // `Game::dying_creature_attachments`.
                if matches!(&printed.kind, CardKind::Creature { .. }) {
                    for aura in self.attachments(from) {
                        let aura_controller = self.controller_of(aura);
                        let aura_def = self.def_of(aura);
                        self.batch_trigger_scratch.dying_creature_attachments.push((
                            from,
                            aura,
                            aura_controller,
                            aura_def,
                        ));
                    }
                    // CR 603.10a last-known information — see `Game::dying_creature_stats`.
                    self.batch_trigger_scratch
                        .dying_creature_stats
                        .push(DyingCreatureStats {
                            id: from,
                            power: self.power(from),
                            toughness: self.toughness(from),
                            plus_counters: self.plus_counters(from),
                            controller: self.controller_of(from),
                        });
                    // CR 800.4a last-known information: def/owner for a death-watch scan that
                    // must still run if `PlayerLost` (later in this same batch) tombstones `from`
                    // out from under it — see `Game::dying_creature_lki`.
                    self.batch_trigger_scratch.dying_creature_lki.push((
                        from,
                        printed.as_ref().clone(),
                        owner,
                    ));
                    // CR 700.4/701.29 last-known information: read `is_modified` before
                    // `clear_modifier_provenance`/`create_object` below tear down its
                    // attachments/counters. Feeds `Condition::ModifiedCreatureDiedThisTurn`
                    // (Intermediate Chirography's Level 3 morbid-of-modified end step). Keyed by
                    // controller ("died under *your* control", CR 700.4) — the sibling
                    // `creatures_died_this_turn` tally uses `dead_controller` too, not owner.
                    let controller = self.controller_of(from);
                    if self.is_modified(from, controller) {
                        self.players[controller.0 as usize].modified_creature_died_this_turn = true;
                    }
                }
                if matches!(&self.objects[from as usize], Object::Permanent(_)) {
                    self.clear_modifier_provenance(from);
                }
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Graveyard,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.remove_spell_from_stack(from);
            }
            Event::MovedToExile { card, from } => {
                let def = self.def_id_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                if matches!(&self.objects[from as usize], Object::Permanent(_)) {
                    // CR 603.10a last-known information — see `MovedToGraveyard`'s
                    // `permanents_left_battlefield` push above.
                    self.batch_trigger_scratch
                        .permanents_left_battlefield
                        .push((from, self.attached_to(from)));
                    self.clear_modifier_provenance(from);
                }
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Exile,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.remove_spell_from_stack(from);
            }
            Event::ExiledOnAdventure { card, from, owner } => {
                // Restore the *creature* front face (not the spent adventure face) to exile, then
                // grant the owner an open-ended permission to cast it from exile (CR 715.3d).
                let idx = self
                    .play_permissions
                    .adventure_fronts
                    .iter()
                    .position(|&(spell, _)| spell == from)
                    .expect("an adventure spell finish has a recorded front face");
                let (_, def) = self.play_permissions.adventure_fronts.remove(idx);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Exile,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.remove_spell_from_stack(from);
                self.play_permissions.on_adventure.push((card, owner));
            }
            // The O-Ring pattern (CR 603.6e): record the link — read back by
            // `Game::check_linked_exile_returns` once `source` leaves the battlefield.
            Event::ExiledUntilSourceLeaves { source, object } => {
                self.exile_links.until_source_leaves.push((source, object));
            }
            // Skyclave Apparition's linked exile — record the link, read back by
            // `Game::check_leaves_battlefield_illusions` once `source` leaves the battlefield.
            Event::ExiledUntilSourceLeavesMintingIllusion { source, object } => {
                self.exile_links
                    .illusion_on_source_leave
                    .push((source, object));
            }
            // The link finished minting its Illusion — drop it so it can't fire again.
            Event::LeavesIllusionMinted { source, object } => {
                self.exile_links
                    .illusion_on_source_leave
                    .retain(|&(s, o)| !(s == source && o == object));
            }
            // Hofri Ghostforge's minted Spirit token: record the granted leaves-battlefield
            // return link — read back by `Game::queue_token_return_exiled_trigger` once `token`
            // leaves the battlefield.
            Event::TokenGrantedReturnExiledOnLeave { token, exiled } => {
                self.exile_links
                    .token_leaves_returns_exiled
                    .push((token, exiled));
            }
            // The granted rider's payoff: move the exile card `from` into its owner's graveyard
            // as `card` — deliberately not routed through `MovedToGraveyard`'s "died" bookkeeping
            // (see the variant doc).
            Event::ReturnedExiledCardToGraveyard { card, from } => {
                let def = self.def_id_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Graveyard,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
            }
            // The "exiled with" pattern (CR 400.10a): record the link — read back by
            // `Game::begin_cash_out_exiled_with_this` when the source's cash-out ability activates.
            Event::ExiledWithSource { source, object } => {
                self.exile_links.with_source.push((source, object));
            }
            // The other half: `source`'s cash-out ability pulled `object` back out of the pile.
            // Drop the now-spent link; the actual zone move is a separate event alongside this one.
            Event::CardExiledWithSourceLeftExile { source, object } => {
                self.exile_links
                    .with_source
                    .retain(|&(s, o)| !(s == source && o == object));
            }
            // Quintorius's activated ability: grant the free-cast permission for the chosen (CR 602, CR 601, CR 113)
            // exiled-with card (it stays in `exile_links.with_source`, unlike a cash-out).
            Event::CastFromExileFreePermissionGranted { card, player } => {
                self.play_permissions
                    .cast_from_exile_free
                    .push((card, player));
            }
            // Quintorius, Loremaster's CR 614.6 rider (see `PlayPermissions::stack_object_bottoms_library_on_leave`).
            Event::CastFromExileFreeBottomsLibraryOnLeave { card } => {
                self.play_permissions
                    .stack_object_bottoms_library_on_leave
                    .push(card);
            }
            // Cleanup: every free-cast permission expires at once (CR 118.5's "this turn" — no
            // `until_next_turn` extension exists for this permission).
            Event::CastFromExileFreeEnded => {
                self.play_permissions.cast_from_exile_free.clear();
                self.play_permissions
                    .stack_object_bottoms_library_on_leave
                    .clear();
            }
            // The other half: `source`'s linked exile ended, so the card it exiled (`from`)
            // returns to the battlefield as a fresh permanent under its owner's control
            // (`controller`), same shape as `ReanimatedToBattlefield`. Drop the now-spent link.
            Event::ReturnedFromLinkedExile {
                permanent,
                from,
                controller,
                source,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Permanent(fresh_permanent(def, controller, true, commander)),
                );
                assert_eq!(id, permanent);
                self.exile_links
                    .until_source_leaves
                    .retain(|&(s, o)| !(s == source && o == from));
            }
            // A flicker's return (immediate `FlickerTarget` or the delayed `ReturnFlickeredCard`):
            // the exiled card `from` returns as the fresh permanent `permanent`, same shape as
            // `ReturnedFromLinkedExile` above.
            Event::FlickeredToBattlefield {
                permanent,
                from,
                controller,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Permanent(fresh_permanent(def, controller, true, commander)),
                );
                assert_eq!(id, permanent);
            }
            Event::ReturnedToHand { card, from } => {
                // Firestorm Phoenix's death replacement routes here, and only that one arrives on
                // a permanent still carrying the static — an ordinary bounce is a bounce. Read it
                // before the move, while `from` is still a permanent to read it off of.
                let phoenix = self.returns_to_hand_instead_of_dying(from);
                // A bounce sends the permanent to its *owner's* hand, not the caster's.
                let def = self.def_id_of(from);
                let printed = card_def(def);
                let owner = self.owner_of(from);
                // Vengeful Rebirth's "If you return a nonland card to your hand this way" — record
                // it for a later step of this same resolution (`Amount::ReturnedNonlandCardManaValue`).
                // Written unconditionally (a bounce or a land clears it), the same
                // apply-time-scratch shape `vanished_permanent_owner` uses.
                self.resolution_frame.returned_nonland_card_mana_value = (self.zone_of(from)
                    == Zone::Graveyard
                    && !matches!(&printed.kind, CardKind::Land { .. }))
                .then(|| printed.mana_value());
                let commander = self.is_commander(from);
                if matches!(&self.objects[from as usize], Object::Permanent(_)) {
                    // CR 603.10a last-known information — see `MovedToGraveyard`'s
                    // `permanents_left_battlefield` push above.
                    self.batch_trigger_scratch
                        .permanents_left_battlefield
                        .push((from, self.attached_to(from)));
                    self.clear_modifier_provenance(from);
                }
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Hand,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.remove_spell_from_stack(from);
                // "Until that player's next turn, that player plays with that card revealed in
                // their hand and can't play it" — armed on the card that came back, not on the
                // permanent that left, since the two are different objects (CR 400.7).
                if phoenix {
                    self.revealed_unplayable_until_next_turn.push((card, owner));
                }
            }
            Event::TuckedToLibrary {
                card,
                from,
                to_top,
                second_from_top,
            } => {
                let def = self.def_id_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                if matches!(&self.objects[from as usize], Object::Permanent(_)) {
                    // CR 603.10a last-known information — see `MovedToGraveyard`'s
                    // `permanents_left_battlefield` push above.
                    self.batch_trigger_scratch
                        .permanents_left_battlefield
                        .push((from, self.attached_to(from)));
                    self.clear_modifier_provenance(from);
                }
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner,
                        zone: Zone::Library,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                let library = &mut self.players[owner.0 as usize].library;
                if second_from_top {
                    // Second from the top (Whirlpool Whelm's win rider): under the current top
                    // card. A library with fewer than one card lands it on top (CR 120 "as close
                    // as possible").
                    library.insert(1.min(library.len()), id);
                } else if to_top {
                    // Top of the library: index 0 is the top, drawn first.
                    library.insert(0, id);
                } else {
                    // Bottom of the library: appended after the current contents, matching
                    // `spawn_in_library`'s "push = bottom" convention.
                    library.push(id);
                }
                // A no-op unless `from` was a stack object (Quintorius's CR 614.6 redirect) — the
                // permanent/graveyard tuck origins were never on the stack, same as
                // `MovedToGraveyard`'s unconditional call below.
                self.remove_spell_from_stack(from);
            }
            Event::SearchedToHand {
                player,
                object,
                from,
                card,
            } => {
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def: card,
                        owner: player,
                        zone: Zone::Hand,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, object);
                self.players[player.0 as usize]
                    .library
                    .retain(|&o| o != from);
            }
            Event::SearchedToBattlefield {
                permanent,
                from,
                controller,
                tapped,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let mut perm = fresh_permanent(def, controller, true, commander);
                perm.tapped = tapped;
                let id = self.create_object(Some(from), Object::Permanent(perm));
                assert_eq!(id, permanent);
                self.players[controller.0 as usize]
                    .library
                    .retain(|&o| o != from);
            }
            Event::PutOntoBattlefieldFromHand {
                permanent,
                from,
                controller,
                tapped,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let mut perm = fresh_permanent(def, controller, true, commander);
                perm.tapped = tapped;
                let id = self.create_object(Some(from), Object::Permanent(perm));
                assert_eq!(id, permanent);
            }
            // Manifest (CR 701.34): the library card `from` enters face down as a 2/2 — its real
            // `def` is carried on the permanent (hidden by the characteristics/redaction layers)
            // so a later turn-face-up can reveal it.
            Event::Manifested {
                permanent,
                from,
                controller,
            } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let mut perm = fresh_permanent(def, controller, true, commander);
                perm.face_down = true;
                let id = self.create_object(Some(from), Object::Permanent(perm));
                assert_eq!(id, permanent);
                self.players[controller.0 as usize]
                    .library
                    .retain(|&o| o != from);
            }
            // Turn face up (CR 701.34e): reveal the real card by clearing the face-down flag.
            Event::TurnedFaceUp { permanent } => {
                self.permanent_mut(permanent).face_down = false;
            }
            Event::Milled { player, card, from } => {
                let def = self.def_id_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner: player,
                        zone: Zone::Graveyard,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.players[player.0 as usize]
                    .library
                    .retain(|&o| o != from);
            }
            Event::LifeChanged { player, amount, .. } => {
                self.players[player.0 as usize].life += amount;
                if amount > 0 {
                    self.players[player.0 as usize].life_gained_this_turn += amount as u32;
                }
                // A life *loss* (CR 118.9/119.3 — a decrease only, not a gain) — feeds
                // `Trigger::YouLoseLifeFirstTimeEachTurn`, which fires on the turn's first.
                if amount < 0 {
                    self.players[player.0 as usize].life_losses_this_turn += 1;
                }
            }
            Event::DrewFromEmptyLibrary { player } => {
                self.players[player.0 as usize].attempted_empty_draw = true
            }
            Event::CitysBlessingGained { player } => {
                self.players[player.0 as usize].has_citys_blessing = true;
            }
            // CR 104.4: the game ends in a draw. No player's `lost` flag is set and none of the
            // CR 800.4a elimination bookkeeping below runs — nobody left the game, the game left.
            Event::GameDrawn => {
                self.drawn = true;
            }
            Event::PlayerLost { player } => {
                self.players[player.0 as usize].lost = true;
                // CR 800.4a: everything the departing player owns leaves the game — including a
                // permanent they own but someone else controls (a donation they made stays owned by
                // them, so it leaves too).
                for slot in self.objects.iter_mut() {
                    let identity = match slot {
                        Object::Card(c) if c.owner == player => Some((c.def, c.owner)),
                        Object::Spell(s) if s.controller == player => Some((s.def, s.controller)),
                        Object::Permanent(p) if p.owner == player => Some((p.def, p.owner)),
                        Object::Moved { .. }
                        | Object::Removed { .. }
                        | Object::Card(_)
                        | Object::Spell(_)
                        | Object::Permanent(_) => None,
                    };
                    if let Some((def, owner)) = identity {
                        *slot = Object::Removed { def, owner };
                    }
                }
                // CR 800.4a: any effect that gives the departing player control of an object also
                // ends — a permanent they stole returns to its owner (or the next-highest control
                // source). Drop every override whose new controller is the leaving player, across
                // all three registries; a control-changing Aura they own already left above.
                self.play_permissions
                    .control_overrides
                    .retain(|&(_, controller, ..)| controller != player);
                self.play_permissions
                    .permanent_control_overrides
                    .retain(|&(_, controller, ..)| controller != player);
                self.play_permissions
                    .conditioned_control_overrides
                    .retain(|&(_, controller, ..)| controller != player);
                // Drop any now-removed objects off the stack and out of combat (disjoint
                // field borrows: the closure reads `objects`, retain mutates other fields).
                let objects = &self.objects;
                let removed = |o: ObjectId| matches!(objects[o as usize], Object::Removed { .. });
                self.stack.retain(|item| match item {
                    StackItem::Spell(id) => !removed(*id),
                    StackItem::Ability { source, .. } => !removed(*source),
                });
                self.combat.attackers.retain(|&a| !removed(a));
                // Counter and boost batches leave with the object they describe — an object that
                // has left the game has no ledger entry to explain, and the cleanup sweep reads
                // boost hosts straight off this registry.
                self.modifier_provenance
                    .counter_batches
                    .retain(|&(o, ..)| !removed(o));
                self.modifier_provenance
                    .modifiers
                    .retain(|m| !removed(m.host));
                self.combat.attack_targets.retain(|&(a, d)| {
                    !removed(a)
                        && d != Defender::Player(player)
                        && !d.object_id().is_some_and(removed)
                });
                self.combat
                    .blocks
                    .retain(|&(b, a)| !removed(b) && !removed(a));
                // CR 800.4a also purges the departing player's own outstanding pending trigger/
                // choice work — nobody is left to answer it. Phyrexian Arena's own upkeep drain
                // can eliminate its controller mid-upkeep while another upkeep trigger for that
                // same player is still queued (not yet placed on the stack); that queued entry
                // must be dropped here, not placed once tombstoned.
                self.pending_trigger_groups
                    .retain(|g| g.controller != player);
                self.pending_obligations
                    .retain(|obligation| !removed(obligation.object()));
                if self
                    .pending_choice
                    .as_ref()
                    .is_some_and(|c| c.player() == player)
                {
                    self.pending_choice = None;
                }
                self.resume.clear_for_removed(player, removed);
                self.pending_enter_bonus_counters
                    .retain(|&(object, _)| !removed(object));
                self.exile_time_counters.retain(|&(card, _)| !removed(card));
                // ponytail: `resolution_finish` is consumed synchronously in the same resolution
                // that sets it (`Game::finish_instant_sorcery_resolution`, right after the
                // spell's own effects run) — it never survives past one `PlayerLost` batch to go
                // stale, so it carries no cross-player state to purge here.
                self.delayed_triggers
                    .scheduled
                    .retain(|&(controller, ..)| controller != player);
                self.delayed_triggers
                    .pending_next_cast
                    .retain(|&(controller, ..)| controller != player);
                self.delayed_triggers
                    .pending_combat_damage_watch
                    .retain(|&(controller, ..)| controller != player);
                self.delayed_triggers
                    .pending_combat_damage_copy
                    .retain(|&(controller, ..)| controller != player);
            }
            Event::CardDrawn {
                player,
                object,
                from,
                card,
            } => {
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def: card,
                        owner: player,
                        zone: Zone::Hand,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, object);
                self.players[player.0 as usize]
                    .library
                    .retain(|&o| o != from);
                self.players[player.0 as usize].draws_this_turn += 1;
                self.drawn_this_turn.push(object);
            }
            Event::MulliganTaken {
                player,
                mulligans_taken,
                ..
            } => {
                self.players[player.0 as usize].mulligans_taken = mulligans_taken;
            }
            Event::HandKept { player } => {
                self.players[player.0 as usize].hand_kept = true;
            }
            Event::MulligansFinished => {
                self.mulliganing = false;
            }
            // Perpetual Timepiece's mandatory shuffle after the chosen graveyard cards enter the
            // library (CR 701.19-style). The order isn't event-sourced (like scry / `Game::
            // shuffle`'s other callers) — mutate the library directly.
            Event::LibraryShuffled { player } => self.shuffle(player),
            Event::LibraryHandSmoothed { player, hand_size } => {
                self.smoothed_shuffle_for_hand(player, hand_size)
            }
            // A reveal is not a zone change (CR 701.30) — the card stays exactly where it is;
            // nothing to mutate here.
            Event::RevealedTopOfLibrary { .. } | Event::RevealedFromHand { .. } => {}
            // A look is not a zone change either; it only records what one seat now knows. Snapshot
            // the hand as it stands — later draws are not part of what was looked at.
            Event::LookedAtHand { player, target } => {
                for card in self.hand(target) {
                    if !self.hand_cards_seen.contains(&(player, card)) {
                        self.hand_cards_seen.push((player, card));
                    }
                }
            }
            Event::PutOnBottomOfLibrary { player, card } => {
                // Same-zone reorder, not a zone change — no new object, just move it in the vec.
                let library = &mut self.players[player.0 as usize].library;
                library.retain(|&o| o != card);
                library.push(card);
            }
            Event::PutFromHandOnTop {
                card,
                from,
                def,
                player,
            } => {
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner: player,
                        zone: Zone::Library,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.players[player.0 as usize].library.insert(0, id);
            }
        }
    }

    /// The next living seat after `player`, wrapping around the table and skipping any
    /// eliminated players. Falls back to `player` if nobody else is left (game over).
    pub(crate) fn next_player(&self, player: PlayerId) -> PlayerId {
        let n = self.players.len() as u8;
        let mut next = (player.0 + 1) % n;
        for _ in 0..n {
            if !self.players[next as usize].lost {
                return PlayerId(next);
            }
            next = (next + 1) % n;
        }
        player
    }

    /// How many players are still in the game (haven't lost).
    pub(crate) fn living_player_count(&self) -> u8 {
        self.players.iter().filter(|p| !p.lost).count() as u8
    }

    /// Drop `object` from the current combat's attacker and blocker lists (CR 506.4) — shared by
    /// [`Event::Regenerated`]'s CR 701.15b removal, [`Event::RemovedFromCombat`]
    /// ([`Effect::Control(ControlEffect::RemoveFromCombat)`] — Spurnmage Advocate), and every control-change event
    /// ([`Event::ControlGained`]/[`Event::ControlGainedUntilEndOfTurn`]/
    /// [`Event::ConditionedControlGained`] — CR 506.4c, Goblin Cadets' "(This removes this
    /// creature from combat.)").
    fn remove_from_combat(&mut self, object: ObjectId, release_solely_blocked: bool) {
        self.combat.attackers.retain(|&a| a != object);
        self.combat.attack_targets.retain(|&(a, _)| a != object);
        self.combat
            .blocks
            .retain(|&(b, a)| b != object && a != object);
        // An attacker that left combat isn't a blocked creature any more either.
        self.combat.blocked_ever.retain(|&(_, a)| a != object);
        // CR 509.1h holds every attacker this creature blocked blocked even now that it's gone —
        // unless the effect is False Orders, whose second sentence is the printed exception.
        // Dropping only this blocker's pairs is exactly "blocked by only that creature": an
        // attacker a second creature also blocked still has that pair on the list.
        if release_solely_blocked {
            self.combat.blocked_ever.retain(|&(b, _)| b != object);
        }
    }
}
