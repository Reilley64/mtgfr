//! Event application and state-based actions.
//!
//! Primary: CR 704 (state-based actions). Also the apply path that mutates board facts
//! from [`Event`]s after each intent.
//! Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

impl Game {
    /// Whether `host` is a legal object for `attachment` (an Aura or Equipment) to be attached to
    /// right now: `attachment`'s own `def.enchant` filter re-checked against its live host, or the
    /// default "enchant creature" filter when it has none (a plain Aura, or Equipment — the DSL
    /// has no attach-filter surface for Equipment beyond "must be a creature"). Used both at an
    /// Aura's cast-time legality re-check (CR 303.4f) and by the CR 704.5m/n state-based action
    /// below, so an Aura like Confiscate ("Enchant permanent") isn't held to the default
    /// enchant-creature restriction its `enchant` filter doesn't actually impose.
    pub(crate) fn attachment_host_legal(&self, attachment: ObjectId, host: ObjectId) -> bool {
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

    /// Re-check state-based actions and return the events they produce.
    /// A player at 0-or-less life loses; a creature with lethal marked damage dies.
    pub(crate) fn check_state_based_actions(&self) -> Vec<Event> {
        let mut events = Vec::new();

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
            let Object::Permanent(p) = self.objects[id as usize] else {
                continue;
            };
            // A creature with lethal marked damage dies (CR 704.5g); a planeswalker with 0 loyalty
            // is put into its owner's graveyard (CR 704.5i).
            let dies = match p.def.kind {
                // CR 702.103e: a bestowed permanent that's attached is an Aura, not a creature —
                // the toughness-≤0 / lethal-damage creature death SBAs don't apply to it.
                CardKind::Creature { .. } if !self.is_bestowed_and_attached(id) => {
                    let toughness = self.toughness(id);
                    // 0-or-less toughness is a death SBA even for an indestructible creature (CR 702.12, CR 704)
                    // (CR 704.5f); lethal damage / deathtouch is not, if it's indestructible
                    // (CR 702.12b).
                    toughness <= 0
                        || (!self.has_keyword(id, Keyword::Indestructible)
                            && (p.marked_damage >= toughness || p.deathtouched))
                }
                CardKind::Planeswalker { .. } => p.loyalty <= 0,
                _ => false,
            };
            if dies {
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
            let Object::Permanent(p) = self.objects[id as usize] else {
                continue;
            };
            let host_illegal = match p.attached_to {
                // unattached Aura is illegal, unless it's this Aura awaiting its host choice
                None => matches!(p.def.kind, CardKind::Aura) && awaiting_host != Some(id),
                Some(host) => !self.attachment_host_legal(id, host) || leaving.contains(&host),
            };
            if !host_illegal {
                continue;
            }
            if matches!(p.def.kind, CardKind::Aura) {
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
        // draw from an empty library, or who took lethal commander damage loses (CR 704.5a/c/g).
        for (id, player) in self.players.iter().enumerate() {
            if player.lost {
                continue;
            }
            let lethal_commander_damage = player
                .commander_damage
                .iter()
                .any(|&(_, amount)| amount >= LETHAL_COMMANDER_DAMAGE);
            if player.life <= 0 || player.attempted_empty_draw || lethal_commander_damage {
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
            sba.extend(self.take_serra_lifegain_events());
            if sba.is_empty() {
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

    /// Drain the Serra Paragon rider's queued life gains (CR 118.9): each controller whose tagged
    /// permanent was exiled on death this batch gains 2 life. Emitted as an [`Event::LifeChanged`]
    /// (so "whenever you gain life" watchers fire) from the SBA fixpoint sweep, mirroring how
    /// [`Game::check_leaves_battlefield_illusions`] emits its own rules-driven follow-ups — the
    /// draining is `&mut` so the queue self-clears and can't re-fire (each dead permanent enqueues
    /// exactly once). `source: None` — the granting Serra Paragon may itself already be gone.
    fn take_serra_lifegain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending_serra_lifegain)
            .into_iter()
            .map(|player| Event::LifeChanged {
                player,
                amount: self.life_gain_after_replacements(player, 2),
                source: None,
            })
            .collect()
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
            .filter(|&&(_, thief, condition)| !self.control_condition_holds(thief, condition))
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
            if matches!(self.objects[source as usize], Object::Permanent(_)) {
                continue; // the source is still on the battlefield — the link is still live.
            }
            let Object::Card(card) = self.objects[exiled as usize] else {
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
            if matches!(self.objects[source as usize], Object::Permanent(_)) {
                continue; // the source is still on the battlefield — the link is still live.
            }
            let Object::Card(card) = self.objects[exiled as usize] else {
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
                def,
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
            .temp_boosts
            .retain(|&(o, ..)| o != object);
        if let Object::Permanent(p) = &mut self.objects[object as usize] {
            p.plus_counters = 0;
            p.temp_power = 0;
            p.temp_toughness = 0;
            p.base_pt_set_eot = None;
            p.added_types_eot = TypeSet::NONE;
            p.added_subtypes_eot = &[];
            p.temp_keywords = &[];
        }
    }

    /// Recompute `plus_counters` / `temp_*` on the permanent from provenance batches — batches are
    /// the write path; aggregates stay a derived cache for hot characteristic / cleanup scans.
    pub(crate) fn resync_modifier_aggregates(&mut self, object: ObjectId) {
        let counters: i32 = self
            .modifier_provenance
            .counter_batches
            .iter()
            .filter(|&&(o, _, _)| o == object)
            .map(|&(_, c, _)| c)
            .sum();
        let boosts: Vec<_> = self
            .modifier_provenance
            .temp_boosts
            .iter()
            .copied()
            .filter(|&(o, ..)| o == object)
            .collect();
        let temp_power: i32 = boosts.iter().map(|&(_, p, _, _, _)| p).sum();
        let temp_toughness: i32 = boosts.iter().map(|&(_, _, t, _, _)| t).sum();
        let temp_keywords: &'static [Keyword] = match boosts.as_slice() {
            [] => &[],
            [(_, _, _, keywords, _)] => *keywords,
            many => {
                let mut union: Vec<Keyword> = Vec::new();
                for &(_, _, _, keywords, _) in many {
                    for &k in keywords {
                        if !union.contains(&k) {
                            union.push(k);
                        }
                    }
                }
                if union.is_empty() {
                    &[]
                } else {
                    Box::leak(union.into_boxed_slice())
                }
            }
        };
        let Object::Permanent(p) = &mut self.objects[object as usize] else {
            return;
        };
        p.plus_counters = counters;
        p.temp_power = temp_power;
        p.temp_toughness = temp_toughness;
        p.temp_keywords = temp_keywords;
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
        match *event {
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
                kicked,
                bought_back,
                strive_count,
                replicate_count,
                bestowed,
                face_down,
                evoked,
                spent_colors,
            } => {
                let (def, commander) = match self.objects[from as usize] {
                    Object::Card(c) => (c.def, c.commander),
                    _ => panic!("cast source {from} is not a card"),
                };
                // Cast zone is read off `from` before `create_object` below moves it onto the
                // stack (CR 601's default cast zone — Dirgur Focusmage's "from your hand").
                let from_zone = self.zone_of(from);
                let cast_from_hand = from_zone == Zone::Hand;
                // Serra Paragon (CR 118.9): a permanent spell cast from the graveyard by neither
                // flashback nor escape can only be its once-per-turn permission — flashback/escape (CR 702.34, CR 702.19, CR 500)
                // set their own flags, and no permanent card has retrace. The tag rides to the
                // resulting permanent so it gains the exile-and-gain-2-life rider.
                let serra_recursion = from_zone == Zone::Graveyard
                    && !flashback
                    && !escape
                    && !matches!(def.kind, CardKind::Spell { .. });
                // CR 107.3: a static cast-X modification (Unbound Flourishing) doubles the value of
                // X on the caster's permanent X-spells *after* payment. This is the single point
                // where the spell's X is frozen for its whole life, so the doubled value flows to
                // enters-with-X counters and every `Amount::X` reader downstream.
                let x = self.cast_x_after_replacements(controller, &def, x);
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
                        modes,
                        copy: false,
                        flashback,
                        escape,
                        cast_from_hand,
                        damage_division: DamageAssignment::default(),
                        damage_division_players: [None; MAX_TARGETS],
                        counter_division: DamageAssignment::default(),
                        sacrifice_count,
                        kicked,
                        bought_back,
                        strive_count,
                        replicate_count,
                        serra_recursion,
                        bestowed,
                        face_down,
                        evoked,
                        spent_colors,
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
                // Feeds the `has_x` `nth_each_turn` gate (Nev, Zimone Infinite Analyst) —
                // SpellFilter::HasXInCost's own predicate (characteristics.rs).
                if def.cost.x > 0 {
                    self.players[controller.0 as usize].x_spells_cast_this_turn += 1;
                }
                // Feeds Condition::CastInstantOrSorceryThisTurn (Hall of Oracles's activation gate),
                // Amount::GreatestInstantOrSorceryManaValueCastThisTurn (Rootha, Mastering the
                // Moment's "X is the greatest mana value among instant and sorcery spells you've
                // cast this turn"), and Amount::OnePlusInstantsAndSorceriesCastThisTurn (Rionya,
                // Fire Dancer's "X is one plus the number of instant and sorcery spells you've
                // cast this turn").
                if matches!(def.kind, CardKind::Spell { .. }) {
                    let player = &mut self.players[controller.0 as usize];
                    player.instant_or_sorcery_cast_this_turn = true;
                    player.greatest_instant_or_sorcery_mana_value_cast_this_turn = player
                        .greatest_instant_or_sorcery_mana_value_cast_this_turn
                        .max(def.mana_value());
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
                let front = self.def_of(source);
                let adventure = *front
                    .adventure
                    .expect("an adventure cast's source card has an adventure half");
                let commander = self.is_commander(source);
                let id = self.create_object(
                    Some(source),
                    Object::Spell(Spell {
                        def: adventure,
                        controller,
                        targets: TargetList::single(target),
                        targets_second: TargetList::default(),
                        commander,
                        x,
                        modes: Modes::default(),
                        copy: false,
                        flashback: false,
                        escape: false,
                        // Cast from the card's owner's hand (CR 601's default cast zone).
                        cast_from_hand: true,
                        damage_division: DamageAssignment::default(),
                        damage_division_players: [None; MAX_TARGETS],
                        counter_division: DamageAssignment::default(),
                        sacrifice_count: 0,
                        kicked: false,
                        bought_back: false,
                        strive_count: 0,
                        replicate_count: 0,
                        serra_recursion: false,
                        bestowed: false,
                        face_down: false,
                        evoked: false,
                        // ponytail: an adventure cast still pays real mana (`settle_payment` runs
                        // above), but no adventure card checks color-spent yet — wire this from
                        // the same `Event::ManaSpent` snapshot `Event::SpellCast` uses
                        // (`Game::cast_adventure`) if one ever does.
                        spent_colors: [false; Color::COUNT],
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
                    Object::Spell(match self.objects[original as usize] {
                        Object::Spell(src) => Spell {
                            controller,
                            commander: false,
                            copy: true,
                            ..src
                        },
                        _ => Spell {
                            def: self.def_of(original),
                            controller,
                            targets: TargetList::default(),
                            targets_second: TargetList::default(),
                            commander: false,
                            x: 0,
                            modes: Modes::default(),
                            copy: true,
                            flashback: false,
                            escape: false,
                            cast_from_hand: false,
                            damage_division: DamageAssignment::default(),
                            damage_division_players: [None; MAX_TARGETS],
                            counter_division: DamageAssignment::default(),
                            sacrifice_count: 0,
                            kicked: false,
                            bought_back: false,
                            strive_count: 0,
                            replicate_count: 0,
                            serra_recursion: false,
                            bestowed: false,
                            face_down: false,
                            evoked: false,
                            // A copy pays no cost (CR 707.10) — nothing was spent to "cast" it.
                            spent_colors: [false; Color::COUNT],
                        },
                    }),
                );
                assert_eq!(id, copy);
                self.stack.push(StackItem::Spell(copy));
            }
            Event::SpellCeasedToExist { spell } => {
                self.remove_spell_from_stack(spell);
                self.objects[spell as usize] = Object::Removed;
            }
            Event::PreparedChanged { object, prepared } => {
                self.permanent_mut(object).prepared = prepared;
            }
            Event::LeveledUp { source, level } => {
                self.permanent_mut(source).level = level;
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
            Event::ColorChosen { object, color } => {
                self.permanent_mut(object).chosen_color = Some(color);
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
                let back = *self
                    .def_of(source)
                    .back
                    .expect("a prepared cast's source has a back face");
                let id = self.create_object(
                    None,
                    Object::Spell(Spell {
                        def: back,
                        controller,
                        targets: TargetList::single(target),
                        targets_second: TargetList::default(),
                        commander: false,
                        x,
                        modes: Modes::default(),
                        // "Cast a **copy**" (CR): it ceases to exist on resolve rather than
                        // becoming a graveyard card (there is no card behind it).
                        copy: true,
                        flashback: false,
                        escape: false,
                        // Cast from the source permanent's prepared state, not the hand.
                        cast_from_hand: false,
                        damage_division: DamageAssignment::default(),
                        damage_division_players: [None; MAX_TARGETS],
                        counter_division: DamageAssignment::default(),
                        sacrifice_count: 0,
                        kicked: false,
                        bought_back: false,
                        strive_count: 0,
                        replicate_count: 0,
                        serra_recursion: false,
                        bestowed: false,
                        face_down: false,
                        evoked: false,
                        // ponytail: a prepared cast still pays real mana (`settle_payment` runs
                        // in `Game::cast_prepared`), but no prepare card checks color-spent yet —
                        // wire this from the same `Event::ManaSpent` snapshot `Event::SpellCast`
                        // uses if one ever does.
                        spent_colors: [false; Color::COUNT],
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
            } => {
                self.stack.push(StackItem::Ability {
                    controller,
                    source,
                    effect,
                    target,
                    targets_second,
                    x,
                });
            }
            Event::AbilityResolved { .. } => {
                // The resolving ability is always the top of the stack.
                debug_assert!(matches!(self.stack.last(), Some(StackItem::Ability { .. })));
                self.stack.pop();
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
                    for player in &mut self.players {
                        player.life_gained_this_turn = 0;
                        player.spells_cast_this_turn = 0;
                        player.x_spells_cast_this_turn = 0;
                        player.draws_this_turn = 0;
                        player.life_losses_this_turn = 0;
                        player.creatures_died_this_turn = 0;
                        player.modified_creature_died_this_turn = false;
                        player.nontoken_creatures_entered_this_turn = 0;
                        player.land_entered_under_your_control_this_turn = false;
                        player.card_left_graveyard_this_turn = false;
                        player.instant_or_sorcery_cast_this_turn = false;
                        player.greatest_instant_or_sorcery_mana_value_cast_this_turn = 0;
                        player.instants_and_sorceries_cast_this_turn = 0;
                        player.flash_permission_this_turn = false;
                        player.channel_colorless_mana_this_turn = false;
                        player.graveyard_play_used_this_turn = false;
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
                    // ponytail: `ScheduleThisTurnCombatDamageCopy`'s CR 603.7 "this turn" watch
                    // is repeatable (unlike `pending_next_cast`'s one-shot), but the same
                    // turn-boundary reasoning applies — nothing reads it after this Untap step
                    // without a fresh arm, so clearing it here is CR-equivalent to per-entry
                    // cleanup-step expiry.
                    self.delayed_triggers.pending_combat_damage_copy.clear();
                    // "Attacks this turn if able" (Furygale Flocking) expires at the turn
                    // boundary, the same "this turn" scope as the tallies above.
                    self.combat_extras.must_attack.clear();
                    // ponytail: "Prevent all combat damage … this turn" (Inkshield) shields expire
                    // at the next Untap — combat is always within the turn, so a combat-only shield
                    // cleared here is behavior-exact for "this turn", the same idiom `must_attack`
                    // and `pending_next_cast` use.
                    self.combat_extras.combat_damage_prevention_shields.clear();
                    // "Entered the battlefield this turn" (Oran-Rief, the Vastwood) expires at
                    // the same turn boundary — every battlefield permanent's, not just the
                    // active player's (a new turn, anyone's, ends "this turn").
                    for id in self.battlefield() {
                        self.permanent_mut(id).entered_this_turn = false;
                    }
                } else if step == Step::EndCombat {
                    // CR "this combat": an `ArmCombatDamageWatch` watch that never fired this
                    // combat expires here, same silent-clear shape as `pending_next_cast`'s own
                    // turn-boundary expiry above.
                    self.delayed_triggers.pending_combat_damage_watch.clear();
                }
            }
            Event::LandPlayed {
                permanent,
                from,
                player,
            } => {
                let def = self.def_of(from);
                let commander = self.is_commander(from);
                // Serra Paragon (CR 118.9): a land can only be played from the graveyard under its
                // once-per-turn permission (no other effect plays lands from there), so a
                // graveyard land-play consumes that permission and the land gains the rider.
                let serra_recursion = self.zone_of(from) == Zone::Graveyard;
                let mut perm = fresh_permanent(def, player, false, commander);
                perm.serra_recursion = serra_recursion;
                // A land's own `enters_tapped` is unconditional; a conditional gate (check
                // lands, slowlands, reveal lands) is resolved here instead, at this one ETB site.
                perm.tapped = self.enters_tapped(def, player);
                let id = self.create_object(Some(from), Object::Permanent(perm));
                assert_eq!(id, permanent);
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
                // Remove the regenerated creature from combat (CR 701.15b) — drop it as attacker
                // and blocker, and any blocks naming it as the attacker.
                self.combat.attackers.retain(|&a| a != object);
                self.combat.attack_targets.retain(|&(a, _)| a != object);
                self.combat
                    .blocks
                    .retain(|&(b, a)| b != object && a != object);
            }
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
                self.resync_modifier_aggregates(object);
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
                            (a.timing, a.effect),
                            (Timing::Static, Effect::ControlAttached)
                        )
                    });
                    if grants_control {
                        self.permanent_mut(host).summoning_sick = true;
                    }
                }
            }
            Event::TempBoost {
                object,
                power,
                toughness,
                keywords,
                source_name,
            } => {
                self.modifier_provenance.temp_boosts.push((
                    object,
                    power,
                    toughness,
                    keywords,
                    source_name,
                ));
                self.resync_modifier_aggregates(object);
            }
            Event::BasePtSetUntilEndOfTurn {
                object,
                power,
                toughness,
            } => {
                self.permanent_mut(object).base_pt_set_eot = Some((power, toughness));
            }
            Event::TypesAddedUntilEndOfTurn {
                object,
                types,
                subtypes,
                colors,
            } => {
                let p = self.permanent_mut(object);
                p.added_types_eot = types;
                p.added_subtypes_eot = subtypes;
                p.added_colors_eot = colors;
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
                let p = self.permanent_mut(object);
                p.added_types = add_types;
                p.added_subtypes = add_subtypes;
                p.set_base_pt = Some((base_power, base_toughness));
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
            // A permanent became a copy of another creature as it entered (CR 706/707.2). Overwrite
            // its `def` with the copied `def`; for an until-EOT copy, stash the original first so
            // cleanup can restore it (Cursed Mirror). `CardDef: Copy`, so both are plain moves.
            Event::BecameCopy {
                object,
                def,
                until_eot,
            } => {
                let p = self.permanent_mut(object);
                if until_eot {
                    // Leak the original printed def to `'static` (like `CardDef::back`) so the
                    // revert reference lives on the `Copy` `Permanent`. Bounded — one leak per
                    // until-EOT copy, freed only at process exit — the same shape as the
                    // `KeywordsStripped` union leak below.
                    p.reverts_to_def_eot = Some(Box::leak(Box::new(p.def)));
                }
                p.def = def;
            }
            Event::TempBoostsEnded { object } => {
                self.modifier_provenance
                    .temp_boosts
                    .retain(|&(o, ..)| o != object);
                self.resync_modifier_aggregates(object);
                let p = self.permanent_mut(object);
                p.temp_lost_keywords = &[];
                p.base_pt_set_eot = None;
                p.added_types_eot = TypeSet::NONE;
                p.added_subtypes_eot = &[];
                p.added_colors_eot = &[];
                // Revert an until-EOT enter-as-copy to the printed permanent (CR 514.2 — Cursed
                // Mirror's "become a copy … until end of turn").
                if let Some(printed) = p.reverts_to_def_eot.take() {
                    p.def = *printed;
                }
            }
            Event::KeywordsStripped { object, keywords } => {
                let p = self.permanent_mut(object);
                if p.temp_lost_keywords.is_empty() {
                    p.temp_lost_keywords = keywords;
                } else {
                    // Same union-not-clobber shape as `TempBoost` above, for a second strip
                    // landing on the same permanent the same turn.
                    // ponytail: leaks a small deduped Vec to keep `Permanent: Copy` — bounded by
                    // one leak per multi-strip collision per turn, freed only at process exit.
                    let mut union: Vec<Keyword> = p.temp_lost_keywords.to_vec();
                    for k in keywords {
                        if !union.contains(k) {
                            union.push(*k);
                        }
                    }
                    p.temp_lost_keywords = Box::leak(union.into_boxed_slice());
                }
            }
            Event::ControlGainedUntilEndOfTurn {
                object,
                controller,
                source_name,
            } => {
                self.play_permissions
                    .control_overrides
                    .push((object, controller, source_name));
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
                self.play_permissions
                    .permanent_control_overrides
                    .push((object, controller));
            }
            Event::ConditionedControlGained {
                object,
                controller,
                condition,
            } => {
                self.play_permissions
                    .conditioned_control_overrides
                    .push((object, controller, condition));
            }
            Event::ConditionedControlEnded { object } => self
                .play_permissions
                .conditioned_control_overrides
                .retain(|&(o, ..)| o != object),
            Event::AttackerDeclared { object, defender } => {
                self.combat.attackers.push(object);
                self.combat.attack_targets.push((object, defender));
            }
            Event::TokenEnteredAttacking { token, defender } => {
                self.combat.attackers.push(token);
                self.combat.attack_targets.push((token, defender));
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
            Event::DelayedTriggersFired { fire_at } => self
                .delayed_triggers
                .scheduled
                .retain(|&(_, _, f, _)| f != fire_at),
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
            } => {
                let def = self.def_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Card(Card {
                        def,
                        owner: self.owner_of(from),
                        zone: Zone::Exile,
                        commander,
                        face_down: false,
                    }),
                );
                assert_eq!(id, card);
                self.players[player.0 as usize]
                    .library
                    .retain(|&o| o != from);
                self.play_permissions
                    .play_from_exile
                    .push((card, player, until_next_turn));
            }
            // Herald of Amity's dig: exile face-up, no permission attached — the follow-up
            // choice grants `CastFromExileFreePermissionGranted` for at most one of the batch.
            Event::ExiledFromLibraryToChooseCastFree {
                player,
                card,
                from,
                face_down,
            } => {
                let def = self.def_of(from);
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
                let def = self.def_of(from);
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
                self.combat.blocks.push((blocker, attacker))
            }
            Event::CombatDamageDivided {
                attacker,
                assignment,
            } => self.combat.damage.push((attacker, assignment.pairs())),
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
            // A marker only — `Game::queue_combat_damage_triggers` reads it off the events batch
            // in `enqueue_triggers`, but it mutates no state of its own (the life loss it
            // accompanies already applied via `LifeChanged`).
            Event::CombatDamageDealtToPlayer { .. } => {}
            // A marker only — the prevented damage's absence (no `LifeChanged`) and the Inkling
            // mints (accompanying `TokenCreated` events) carry all the state; this event mutates
            // nothing itself.
            Event::CombatDamagePrevented { .. } => {}
            Event::MovedToCommandZone { card, from } => {
                let def = self.def_of(from);
                let owner = self.owner_of(from);
                if matches!(self.objects[from as usize], Object::Permanent(_)) {
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
            Event::ManaEmptied {
                player,
                end_of_turn,
            } => {
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
                    evoked,
                    spent_colors,
                ) = match self.objects[from as usize] {
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
                        s.evoked,
                        s.spent_colors,
                    ),
                    _ => panic!("PermanentEntered source {from} is not a spell"),
                };
                let id = self.create_object(
                    Some(from),
                    Object::Permanent(fresh_permanent(def, owner, true, commander)),
                );
                assert_eq!(id, permanent);
                // See `Permanent::entered_with_x`'s doc — locked in here while `from` is still
                // the resolving Spell, before `remove_spell_from_stack` below takes it away.
                self.permanent_mut(permanent).entered_with_x = x;
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
                // Evoke (CR 702.74a): an evoked spell's resulting permanent is sacrificed the
                // instant it enters — the self-sacrifice fires as its own trigger, queued
                // alongside the permanent's ETB triggers (`Game::enqueue_triggers`), so an ETB
                // payoff (Mulldrifter's draw two) still resolves first.
                self.permanent_mut(permanent).evoked = evoked;
                // See `Permanent::spent_colors`'s doc — same "read it before the spell is gone"
                // idiom as `entered_with_x` above (Court Hussar's "unless {W} was spent to cast it").
                self.permanent_mut(permanent).spent_colors = spent_colors;
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
                let def = self.def_of(from);
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
            } => {
                let id = self.create_object(None, Object::Permanent(fresh_token(def, controller)));
                assert_eq!(id, token);
            }
            Event::TokenCeasedToExist {
                token,
                controller,
                def,
            } => {
                // CR 603.6c/704.5m last-known information: capture the Aura(s) attached to this
                // token *before* it vanishes, so `Trigger::EnchantedCreatureDies` can still find
                // them once the token's arena slot (and the Aura's own `attached_to`) is gone —
                // see `Game::dying_creature_attachments`.
                if matches!(def.kind, CardKind::Creature { .. }) {
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
                    self.batch_trigger_scratch.dying_creature_stats.push((
                        token,
                        self.power(token),
                        self.plus_counters(token),
                    ));
                    // CR 700.4/701.29 last-known information: a token ceasing to exist is a
                    // "died" too — read `is_modified` before `Object::Removed` below erases its
                    // attachments/counters. Feeds `Condition::ModifiedCreatureDiedThisTurn`.
                    if self.is_modified(token) {
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
                self.objects[token as usize] = Object::Removed;
            }
            Event::DamageMarked { object, amount, .. } => {
                self.permanent_mut(object).marked_damage += amount
            }
            // A pure signal event for trigger-scanning (`Game::queue_sacrifice_triggers`) — the
            // actual zone change is a separate event (`MovedToGraveyard`/`MovedToCommandZone`/
            // `TokenCeasedToExist`) emitted alongside it at the same call site.
            Event::Sacrificed { .. } => {}
            Event::MovedToGraveyard { card, from } => {
                // Feeds `Amount::PermanentsDiedThisTurn` (Ominous Harvest's Gravestorm): `from`
                // being a live battlefield `Object::Permanent` (not a hand/exile/stack card
                // heading to the graveyard by discard, resolution, or counter) is exactly CR
                // 700.4's "died" — put into a graveyard from the battlefield. A token's death is
                // the separate `TokenCeasedToExist` event, not counted here (see that `Amount`
                // variant's doc).
                if matches!(self.objects[from as usize], Object::Permanent(_)) {
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
                }
                let def = self.def_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                // CR 603.6c/704.5m last-known information: capture the Aura(s) attached to this
                // creature *before* `create_object` tombstones it — whether this death was a
                // state-based action (lethal damage) or a direct effect (Destroy), the Aura's own (CR 704, CR 303.4, CR 120.3)
                // orphan-to-graveyard SBA hasn't run yet, so it's still attached right now. Read (CR 704, CR 303.4, CR 403.5)
                // back by `Trigger::EnchantedCreatureDies` in `enqueue_triggers`; see
                // `Game::dying_creature_attachments`.
                if matches!(def.kind, CardKind::Creature { .. }) {
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
                    self.batch_trigger_scratch.dying_creature_stats.push((
                        from,
                        self.power(from),
                        self.plus_counters(from),
                    ));
                    // CR 700.4/701.29 last-known information: read `is_modified` before
                    // `clear_modifier_provenance`/`create_object` below tear down its
                    // attachments/counters. Feeds `Condition::ModifiedCreatureDiedThisTurn`
                    // (Intermediate Chirography's Level 3 morbid-of-modified end step). Keyed by
                    // controller ("died under *your* control", CR 700.4) — the sibling
                    // `creatures_died_this_turn` tally uses `dead_controller` too, not owner.
                    if self.is_modified(from) {
                        let controller = self.controller_of(from);
                        self.players[controller.0 as usize].modified_creature_died_this_turn = true;
                    }
                }
                if matches!(self.objects[from as usize], Object::Permanent(_)) {
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
                let def = self.def_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                if matches!(self.objects[from as usize], Object::Permanent(_)) {
                    // CR 603.10a last-known information — see `MovedToGraveyard`'s
                    // `permanents_left_battlefield` push above.
                    self.batch_trigger_scratch
                        .permanents_left_battlefield
                        .push((from, self.attached_to(from)));
                    self.clear_modifier_provenance(from);
                }
                // Serra Paragon's rider (CR 118.9): a tagged permanent redirected to exile on death
                // (see `graveyard_or_command`) owes its controller 2 life. Captured here, off the
                // still-live `Object::Permanent`, before `create_object` below tombstones it; the
                // SBA sweep drains it into an `Event::LifeChanged`. (CR 704)
                if self.as_permanent(from).is_some_and(|p| p.serra_recursion) {
                    self.pending_serra_lifegain.push(owner);
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
                let def = self.def_of(from);
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
                let def = self.def_of(from);
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
                let def = self.def_of(from);
                let commander = self.is_commander(from);
                let id = self.create_object(
                    Some(from),
                    Object::Permanent(fresh_permanent(def, controller, true, commander)),
                );
                assert_eq!(id, permanent);
            }
            Event::ReturnedToHand { card, from } => {
                // A bounce sends the permanent to its *owner's* hand, not the caster's.
                let def = self.def_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                if matches!(self.objects[from as usize], Object::Permanent(_)) {
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
            }
            Event::TuckedToLibrary { card, from, to_top } => {
                let def = self.def_of(from);
                let owner = self.owner_of(from);
                let commander = self.is_commander(from);
                if matches!(self.objects[from as usize], Object::Permanent(_)) {
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
                if to_top {
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
                let def = self.def_of(from);
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
                let def = self.def_of(from);
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
                let def = self.def_of(from);
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
                let def = self.def_of(from);
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
            Event::PlayerLost { player } => {
                self.players[player.0 as usize].lost = true;
                // CR 800.4a: everything the departing player owns leaves the game. This pool
                // has no control-changing effects, so nothing they merely control-but-not-own
                // needs handing back. ponytail: their pending triggers/choices aren't purged —
                // no pool card lets a player die with those outstanding.
                for slot in self.objects.iter_mut() {
                    let owned = match slot {
                        Object::Card(c) => c.owner == player,
                        Object::Spell(s) => s.controller == player,
                        Object::Permanent(p) => p.owner == player,
                        Object::Moved { .. } | Object::Removed => false,
                    };
                    if owned {
                        *slot = Object::Removed;
                    }
                }
                // Drop any now-removed objects off the stack and out of combat (disjoint
                // field borrows: the closure reads `objects`, retain mutates other fields).
                let objects = &self.objects;
                let removed = |o: ObjectId| matches!(objects[o as usize], Object::Removed);
                self.stack.retain(|item| match *item {
                    StackItem::Spell(id) => !removed(id),
                    StackItem::Ability { source, .. } => !removed(source),
                });
                self.combat.attackers.retain(|&a| !removed(a));
                self.combat
                    .attack_targets
                    .retain(|&(a, d)| !removed(a) && d != player);
                self.combat
                    .blocks
                    .retain(|&(b, a)| !removed(b) && !removed(a));
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
            }
            // Perpetual Timepiece's mandatory shuffle after the chosen graveyard cards enter the
            // library (CR 701.19-style). The order isn't event-sourced (like scry / `Game::
            // shuffle`'s other callers) — mutate the library directly.
            Event::LibraryShuffled { player } => self.shuffle(player),
            // A reveal is not a zone change (CR 701.30) — the card stays exactly where it is;
            // nothing to mutate here.
            Event::RevealedTopOfLibrary { .. } => {}
            Event::PutOnBottomOfLibrary { player, card } => {
                // Same-zone reorder, not a zone change — no new object, just move it in the vec.
                let library = &mut self.players[player.0 as usize].library;
                library.retain(|&o| o != card);
                library.push(card);
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
}
