//! Reveal-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_reveal_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            // Goblin Guide's attack trigger: reveal the defender's top card; land, to hand.
            Effect::RevealTopToHand { filter, defender } => {
                let defender = defender.expect("filled from attack context when placed");
                let Some(&card) = self.players[defender.0 as usize].library.first() else {
                    return Vec::new(); // an empty library reveals nothing (CR 120.3-ish).
                };
                let def = self.def_of(card);
                let mut events = vec![Event::RevealedTopOfLibrary {
                    player: defender,
                    card,
                    def,
                }];
                if filter.matches(def) {
                    events.push(Event::SearchedToHand {
                        player: defender,
                        object: self.next_object_id(),
                        from: card,
                        card: def,
                    });
                }
                events
            }
            // Open the Way: reveal from the top until X lands are found (or the library runs
            // out, CR 120-style "as many as possible"); each land goes to `matched_dest`
            // (battlefield tapped), every other revealed card to `rest_dest` (bottom of
            // library). Deterministic given the library, so no player choice is involved.
            Effect::RevealUntil {
                filter,
                count,
                matched_dest,
                matched_tapped,
                rest_dest,
            } => {
                let goal = self.resolve_count(count, controller, source, target, x);
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                let mut matched = 0;
                for &card in &self.players[controller.0 as usize].library {
                    if matched >= goal {
                        break; // cards past the stop point stay on top, untouched.
                    }
                    let def = self.def_of(card);
                    events.push(Event::RevealedTopOfLibrary {
                        player: controller,
                        card,
                        def,
                    });
                    if !filter.matches(def) {
                        match rest_dest {
                            RestDest::Bottom => {
                                events.push(Event::PutOnBottomOfLibrary {
                                    player: controller,
                                    card,
                                });
                            }
                            RestDest::Hand => {
                                events.push(Event::SearchedToHand {
                                    player: controller,
                                    object: next,
                                    from: card,
                                    card: def,
                                });
                                next += 1;
                            }
                        }
                        continue;
                    }
                    matched += 1;
                    match matched_dest {
                        SearchDest::Battlefield => {
                            events.push(Event::SearchedToBattlefield {
                                permanent: next,
                                from: card,
                                controller,
                                tapped: matched_tapped,
                            });
                        }
                        SearchDest::Hand => {
                            events.push(Event::SearchedToHand {
                                player: controller,
                                object: next,
                                from: card,
                                card: def,
                            });
                        }
                        // ponytail: no pool card sets `matched_dest = "library_top"` on
                        // `reveal_until`/`reveal_top_cards` — this routine already processes the
                        // library strictly top-down, so once every miss ahead of a match has been
                        // routed away by `rest_dest`, the match sits on top with nothing further
                        // to do. Give this a real move event if a card ever needs it.
                        SearchDest::LibraryTop => {}
                        // ponytail: no pool card sets `matched_dest = "graveyard"` on
                        // `reveal_until`/`reveal_top_cards` either (Buried Alive's #172 search is
                        // a genuine `search_library`, not a top-down reveal) — wire an
                        // `Event::Milled` arm here, mirroring `SearchDest::Graveyard`'s
                        // `search_library` arm, from the first card that needs it.
                        SearchDest::Graveyard => {}
                    }
                    next += 1;
                }
                events
            }
            // Animist's Awakening: reveal exactly the top `count` cards (not "until N match" —
            // `RevealUntil`'s sibling), stopping early on a short library (CR 120.3 "as many as
            // possible"). Every match goes to `matched_dest`, deployed untapped instead of
            // `matched_tapped` when `deploy_untapped_if` holds (spell mastery); every other
            // revealed card goes to `rest_dest`.
            Effect::RevealTopCards {
                count,
                filter,
                matched_dest,
                matched_tapped,
                rest_dest,
                deploy_untapped_if,
            } => {
                let goal = self.resolve_count(count, controller, source, target, x);
                let tapped = matched_tapped
                    && !deploy_untapped_if.is_some_and(|condition| {
                        self.condition_holds(condition, TriggerContext::of(controller))
                    });
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for &card in self.players[controller.0 as usize]
                    .library
                    .iter()
                    .take(goal as usize)
                {
                    let def = self.def_of(card);
                    events.push(Event::RevealedTopOfLibrary {
                        player: controller,
                        card,
                        def,
                    });
                    if !filter.matches(def) {
                        match rest_dest {
                            RestDest::Bottom => {
                                events.push(Event::PutOnBottomOfLibrary {
                                    player: controller,
                                    card,
                                });
                            }
                            RestDest::Hand => {
                                events.push(Event::SearchedToHand {
                                    player: controller,
                                    object: next,
                                    from: card,
                                    card: def,
                                });
                                next += 1;
                            }
                        }
                        continue;
                    }
                    match matched_dest {
                        SearchDest::Battlefield => {
                            events.push(Event::SearchedToBattlefield {
                                permanent: next,
                                from: card,
                                controller,
                                tapped,
                            });
                        }
                        SearchDest::Hand => {
                            events.push(Event::SearchedToHand {
                                player: controller,
                                object: next,
                                from: card,
                                card: def,
                            });
                        }
                        // ponytail: no pool card sets `matched_dest = "library_top"` on
                        // `reveal_until`/`reveal_top_cards` — see the sibling arm in
                        // `RevealUntil`'s resolution above for why this is a genuine no-op today.
                        SearchDest::LibraryTop => {}
                        // ponytail: see the sibling arm in `RevealUntil`'s resolution above — no
                        // pool card sets `matched_dest = "graveyard"` on `reveal_top_cards` either.
                        SearchDest::Graveyard => {}
                    }
                    next += 1;
                }
                events
            }
            // Keen Duelist's upkeep trigger: both players reveal their top card, each loses life
            // to the *other's* mana value, then each puts their own revealed card into hand.
            Effect::RevealTopAndDrainMutual => {
                let Some(Target::Player(opponent)) = target else {
                    panic!("reveal-top-and-drain-mutual resolves with a chosen opponent target");
                };
                let you = self.players[controller.0 as usize].library.first().copied();
                let them = self.players[opponent.0 as usize].library.first().copied();
                let mut events = Vec::new();
                if let Some(card) = you {
                    events.push(Event::RevealedTopOfLibrary {
                        player: controller,
                        card,
                        def: self.def_of(card),
                    });
                }
                if let Some(card) = them {
                    events.push(Event::RevealedTopOfLibrary {
                        player: opponent,
                        card,
                        def: self.def_of(card),
                    });
                }
                if let Some(card) = them {
                    events.push(Event::LifeChanged {
                        player: controller,
                        amount: -(self.def_of(card).mana_value() as i32),
                        source: Some(source),
                    });
                }
                if let Some(card) = you {
                    events.push(Event::LifeChanged {
                        player: opponent,
                        amount: -(self.def_of(card).mana_value() as i32),
                        source: Some(source),
                    });
                }
                let mut next = self.next_object_id();
                if let Some(card) = you {
                    events.push(Event::SearchedToHand {
                        player: controller,
                        object: next,
                        from: card,
                        card: self.def_of(card),
                    });
                    next += 1;
                }
                if let Some(card) = them {
                    events.push(Event::SearchedToHand {
                        player: opponent,
                        object: next,
                        from: card,
                        card: self.def_of(card),
                    });
                }
                events
            }

            _ => unreachable!("reveal family mint received a non-family effect"),
        }
    }
}
