//! Zone queries, draw/shuffle, and mana-pool helpers.
//! Primary: CR 400 (zones), CR 121 (drawing a card), CR 106.4 (mana pool).
//!
//! Zone membership and library/hand operations; mana pool empties as turn-based
//! actions elsewhere. Deferred / gaps: per-deck increments under `docs/fidelity/` (fidelity-grind skill).

use crate::*;

impl Game {
    /// The amount of `color` mana currently in `player`'s pool.
    pub fn mana_in_pool(&self, player: PlayerId, color: Color) -> u8 {
        self.players[player.0 as usize].mana_pool.colored[color.index()]
    }

    /// Total mana floating in `player`'s pool, of every kind.
    pub fn floating_mana(&self, player: PlayerId) -> u32 {
        self.players[player.0 as usize].mana_pool.total()
    }

    /// The player's current mana pool.
    pub fn mana_pool(&self, player: PlayerId) -> &ManaPool {
        &self.players[player.0 as usize].mana_pool
    }

    /// The amount of colorless `{C}` mana currently in `player`'s pool.
    pub fn colorless_in_pool(&self, player: PlayerId) -> u8 {
        self.players[player.0 as usize].mana_pool.colorless
    }

    /// Test/setup helper: add a comfortable amount of every color plus colorless to `player`'s
    /// pool so cost-agnostic tests can cast without arranging lands.
    pub fn fund_mana(&mut self, player: PlayerId) {
        for mana in [
            Mana::Color(Color::White),
            Mana::Color(Color::Blue),
            Mana::Color(Color::Black),
            Mana::Color(Color::Red),
            Mana::Color(Color::Green),
            Mana::Colorless,
        ] {
            self.apply(&Event::ManaAdded {
                player,
                mana,
                amount: 20,
                persist: false,
            });
        }
    }

    /// The zone an object currently occupies — following its lineage if the id has since
    /// moved on (so an old id still reports where the card ended up).
    pub fn zone_of(&self, object: ObjectId) -> Zone {
        match &self.objects[object as usize] {
            Object::Card(c) => c.zone,
            Object::Spell(_) => Zone::Stack,
            Object::Permanent(_) => Zone::Battlefield,
            Object::Moved { to } => self.zone_of(*to),
            Object::Removed { .. } => panic!("object {object} has left the game"),
        }
    }

    /// Create a card on the bottom of `player`'s library, returning its id. Public alongside the
    /// other `spawn_*` test/setup helpers so a server test can stock a library and avoid decking
    /// its active player at the draw step.
    pub fn spawn_in_library(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let def = intern_card_def(def);
        let id = self.create_object(
            None,
            Object::Card(Card {
                def,
                owner: player,
                zone: Zone::Library,
                commander: false,
                face_down: false,
            }),
        );
        self.players[player.0 as usize].library.push(id);
        id
    }

    /// Test/setup helper: replace `player`'s library with `defs` in order — index 0
    /// becomes the top of the library (drawn first). Returns the object ids in order.
    pub fn stack_library(&mut self, player: PlayerId, defs: &[CardDef]) -> Vec<ObjectId> {
        self.players[player.0 as usize].library.clear();
        defs.iter()
            .cloned()
            .map(|def| self.spawn_in_library(player, def))
            .collect()
    }

    /// Shuffle `player`'s library with a derive-per-op PRNG (Fisher-Yates via unbiased indices).
    pub fn shuffle(&mut self, player: PlayerId) {
        let len = self.players[player.0 as usize].library.len();
        if len < 2 {
            return;
        }
        let mut order: Vec<usize> = (0..len).collect();
        self.with_op_rng(player, |rng| {
            for i in (1..len).rev() {
                let j = rng.gen_index(i + 1);
                order.swap(i, j);
            }
        });
        let lib = &mut self.players[player.0 as usize].library;
        let mut next = Vec::with_capacity(len);
        for i in order {
            next.push(lib[i]);
        }
        *lib = next;
    }

    /// Shuffle `player`'s library, then put `card` on top (CR 701.19 — Enlightened Tutor/Sterling
    /// Grove's "reveal it, then shuffle and put that card on top"). Pulls `card` out first so the
    /// shuffle can't relocate it — equivalent to shuffling everything and then overriding its
    /// position, since a uniform shuffle treats every card symmetrically before the override.
    /// Same-zone reorder, not a zone change (CR 400.7) — `card` keeps its object id.
    pub(crate) fn shuffle_then_put_on_top(&mut self, player: PlayerId, card: ObjectId) {
        self.players[player.0 as usize]
            .library
            .retain(|&o| o != card);
        self.shuffle(player);
        self.players[player.0 as usize].library.insert(0, card);
    }

    /// Draw the top card of `player`'s library into their hand. Drawing from an
    /// empty library flags the player to lose on the next SBA sweep (rule 104.3c).
    pub fn draw_card(&mut self, player: PlayerId) -> Vec<Event> {
        let events = self.draw_events(player, 1);
        self.apply_all(&events);
        events
    }

    /// The dredgers `player` may use to replace a single draw (CR 702.52): each card in their own
    /// graveyard carrying `dredge = Some(n)` whose N does not exceed the library size (CR 702.52a
    /// makes dredge illegal when the library is too small). Returns `(graveyard object id, N)` per
    /// eligible dredger; empty when none qualify, so the single-draw choke draws normally.
    pub(crate) fn dredge_options(&self, player: PlayerId) -> Vec<(ObjectId, u8)> {
        let library = self.players[player.0 as usize].library.len();
        self.graveyard_of(player)
            .into_iter()
            .filter_map(|id| {
                let n = self.def_of(id).dredge?;
                (library >= n as usize).then_some((id, n))
            })
            .collect()
    }

    /// Who controls a draw-replacing Chains of Mephistopheles, if one is on the battlefield —
    /// "If a player would draw a card except the first one they draw in each of their draw steps,
    /// that player discards a card instead." (CR 614). It replaces *every* player's draws, whoever
    /// controls it, so unlike [`Game::controls_static`] this scan isn't player-scoped. The
    /// controller is what stamps the substituted discard's cause (Psychic Purge reads it — the
    /// discard is caused by Chains' own static ability, not by whatever asked for the draw).
    ///
    /// ponytail: two Chains behave as one — the first found is the cause, and a second copy
    ///   neither doubles the replacement (CR 614.5 already exempts the draw it grants) nor gets a
    ///   say in the cause. Return the full set and pause on the ordering if a real deck runs two.
    pub(crate) fn chains_controller(&self) -> Option<PlayerId> {
        self.battlefield().into_iter().find_map(|id| {
            self.functional_abilities(id)
                .iter()
                .any(|ability| {
                    ability.timing == Timing::Static
                        && matches!(
                            ability.effect,
                            Effect::Static(
                                StaticEffect::DrawsAfterTheFirstEachDrawStepBecomeDiscardThenDraw
                            )
                        )
                })
                .then(|| self.controller_of(id))
        })
    }

    /// Whether the draw `player` is about to take is Chains of Mephistopheles' exempt one —
    /// "the first one they draw in each of their draw steps". A draw in any other step, or one
    /// `player` takes during *someone else's* draw step, is never exempt.
    fn is_first_draw_of_own_draw_step(&self, player: PlayerId) -> bool {
        self.step == Step::Draw
            && self.active_player == player
            && self.players[player.0 as usize].draws_this_draw_step == 0
    }

    /// The one draw funnel: draw for each seat in `seats` (in the order given, `count` cards
    /// each), every draw individually replaceable, then run `after`.
    ///
    /// Each draw is its own event (CR 121.2) and may be replaced by dredge (CR 702.52) or by
    /// Chains of Mephistopheles' discard (CR 614) — both of which *pause* for an answer, so the
    /// batch can't run to completion here. It parks in [`ResumeState::draw_batch`] and the
    /// answering handler re-enters [`Game::run_draw_batch`]; whatever the caller meant to do after
    /// the draws rides along as `after` rather than following this call.
    ///
    /// ponytail: one batch at a time — a second call while one is parked replaces it. Nothing
    /// reaches here from inside a batch: a draw only applies events, and `apply` merely *enqueues*
    /// the triggers it fires. Make `draw_batch` a stack if a replacement ever draws re-entrantly.
    pub(crate) fn draw_with_replacements(
        &mut self,
        seats: Vec<(PlayerId, u32)>,
        after: DrawAfter,
        events: &mut Vec<Event>,
    ) {
        self.resume.draw_batch = Some(DrawBatch {
            seats,
            after,
            paused: false,
        });
        self.run_draw_batch(events);
    }

    /// Draw out the parked [`DrawBatch`], one draw at a time, until it is paid or a replacement
    /// pauses. A no-op when no batch is parked, so an answer handler can call it unconditionally.
    pub(crate) fn run_draw_batch(&mut self, events: &mut Vec<Event>) {
        loop {
            let Some(mut batch) = self.resume.draw_batch.take() else {
                return;
            };
            batch.seats.retain(|&(_, count)| count > 0);
            let Some(&(player, _)) = batch.seats.first() else {
                self.finish_draw_batch(batch.after, batch.paused, events);
                return;
            };
            // Charge the draw before taking it: a pause parks the batch as it stands, and the
            // handler that resumes must not hand this seat the same draw twice.
            batch.seats[0].1 -= 1;
            self.resume.draw_batch = Some(batch);
            if !self.draw_one_or_park(player, true, events) {
                return;
            }
        }
    }

    /// [`Game::draw_one_with_replacements`], marking the parked batch as having paused so
    /// [`Game::finish_draw_batch`] knows the interrupted caller is owed a resume.
    fn draw_one_or_park(
        &mut self,
        player: PlayerId,
        apply_chains: bool,
        events: &mut Vec<Event>,
    ) -> bool {
        if self.draw_one_with_replacements(player, apply_chains, events) {
            return true;
        }
        if let Some(batch) = &mut self.resume.draw_batch {
            batch.paused = true;
        }
        false
    }

    /// Whatever the finished batch's caller owes next.
    fn finish_draw_batch(&mut self, after: DrawAfter, paused: bool, events: &mut Vec<Event>) {
        match after {
            DrawAfter::Nothing => {}
            // The step loop advances itself when the draw ran straight through; it bailed out at
            // its own `pending_choice` check only if a replacement paused, so the step is ours to
            // resume only then.
            DrawAfter::DrawStep if paused => events.extend(self.advance_step()),
            DrawAfter::DrawStep => {}
            // Trade Secrets: "Target opponent draws two cards, then you draw up to four cards."
            DrawAfter::TradeSecretsCaster {
                caster,
                opponent,
                source,
                max,
            } => crate::pending::raise_choice(
                self,
                PendingChoice::MayDrawUpTo {
                    player: caster,
                    max,
                    effect: Effect::Choice(ChoiceEffect::MayDrawUpToThenOpponentMayRepeat {
                        count: Amount::Fixed(i32::from(max)),
                    }),
                    resume: MayDrawUpToResume::TradeSecretsRepeat { opponent, source },
                },
            ),
            // Trade Secrets: "…Then that player may repeat this process as many times as they choose."
            DrawAfter::TradeSecretsRepeat {
                caster,
                opponent,
                source,
                max,
            } => crate::pending::raise_choice(
                self,
                PendingChoice::MayYesNo {
                    player: opponent,
                    source,
                    effect: Effect::Choice(ChoiceEffect::MayDrawUpToThenOpponentMayRepeat {
                        count: Amount::Fixed(i32::from(max)),
                    }),
                    resume: MayYesNoResume::TradeSecretsRepeat { caster, max },
                },
            ),
        }
    }

    /// One draw for `player`, replacements applied. Returns `false` when a replacement raised a
    /// pause instead of drawing — the answering handler finishes that draw and re-enters
    /// [`Game::run_draw_batch`].
    ///
    /// `apply_chains` is CR 614.5: the draw Chains of Mephistopheles *itself* generates gets no
    /// second visit from the replacement that generated it (dredge may still replace it — a
    /// different effect).
    /// ponytail: Chains is offered before dredge on a draw both could replace. CR 616.1 gives the
    /// drawing player that ordering; add the ordering pause if a real deck ever runs both.
    fn draw_one_with_replacements(
        &mut self,
        player: PlayerId,
        apply_chains: bool,
        events: &mut Vec<Event>,
    ) -> bool {
        if apply_chains
            && self.chains_controller().is_some()
            && !self.is_first_draw_of_own_draw_step(player)
        {
            let hand = self.hand_of(player);
            // "If the player doesn't discard a card this way, they mill a card." — an empty hand
            // is the only way to not discard, since the discard itself isn't optional.
            if hand.is_empty() {
                let evs = self.mill_events(player, 1);
                self.apply_all(&evs);
                events.extend(evs);
                return true;
            }
            crate::pending::raise_choice(
                self,
                PendingChoice::DiscardCards {
                    player,
                    hand,
                    count: 1,
                    or_one_matching: None,
                    draw_replacement: true,
                },
            );
            return false;
        }
        let eligible = self.dredge_options(player);
        if !eligible.is_empty() {
            crate::pending::raise_choice(self, PendingChoice::ChooseDredge { player, eligible });
            return false;
        }
        let evs = self.draw_events(player, 1);
        self.apply_all(&evs);
        events.extend(evs);
        true
    }

    /// The draw Chains of Mephistopheles owes after its discard — "If the player discards a card
    /// this way, they draw a card." — followed by the rest of the interrupted batch.
    pub(crate) fn finish_chains_draw(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        if !self.draw_one_or_park(player, false, events) {
            return;
        }
        self.run_draw_batch(events);
    }

    /// The events for `player` drawing `count` cards — pure (the caller applies them).
    /// Each successful draw mints a new hand-object id (`next + i`), matching the arena
    /// slots `apply` will push into.
    pub(crate) fn draw_events(&self, player: PlayerId, count: u32) -> Vec<Event> {
        let library = self.players[player.0 as usize].library.clone();
        let mut next = self.next_object_id();
        (0..count as usize)
            .map(|i| match library.get(i) {
                Some(&from) => {
                    let event = Event::CardDrawn {
                        player,
                        object: next,
                        from,
                        card: self.def_id_of(from),
                    };
                    next += 1;
                    event
                }
                None => Event::DrewFromEmptyLibrary { player },
            })
            .collect()
    }

    /// The events for `player` milling the top `count` cards of their library into their
    /// graveyard — pure (the caller applies them). A library shorter than `count` mills only
    /// what's there; milling never sets the empty-draw flag, so it can't cause a loss. Each
    /// milled card mints a new graveyard-object id (`next + i`), matching the arena slots
    /// `apply` will push into.
    /// Mill `count` cards from each of `players`, in the order given.
    ///
    /// Ids are minted in one pass across every seat's batch — [`Game::mill_events`] can't be
    /// called once per player here, since each call restarts from the same not-yet-applied
    /// `next_object_id` and the second seat would reuse the first's ids.
    pub(crate) fn mill_events_for(&self, players: &[PlayerId], count: u32) -> Vec<Event> {
        let mut next = self.next_object_id();
        let mut events = Vec::new();
        for &player in players {
            let library = self.players[player.0 as usize].library.clone();
            for &from in library.iter().take(count as usize) {
                events.push(Event::Milled {
                    player,
                    card: next,
                    from,
                });
                next += 1;
            }
        }
        events
    }

    pub(crate) fn mill_events(&self, player: PlayerId, count: u32) -> Vec<Event> {
        let library = self.players[player.0 as usize].library.clone();
        let mut next = self.next_object_id();
        library
            .iter()
            .take(count as usize)
            .map(|&from| {
                let event = Event::Milled {
                    player,
                    card: next,
                    from,
                };
                next += 1;
                event
            })
            .collect()
    }

    /// The events for `player` impulse-exiling the top `count` cards of their library face-up with
    /// permission to play them until end of turn (or until the end of their next turn, if
    /// `until_next_turn` — Atsushi's exile mode) — pure (the caller applies them). Mirrors
    /// [`Self::mill_events`]: a short library exiles only what's there; each mints a new exile id.
    /// Intet, the Dreamer's mode instead exiles face down and grants a free, source-scoped
    /// permission — `face_down` / `free_while_source` (the granting permanent).
    pub(crate) fn exile_top_may_play_events(
        &self,
        player: PlayerId,
        count: u32,
        until_next_turn: bool,
        face_down: bool,
        free_while_source: Option<ObjectId>,
    ) -> Vec<Event> {
        let library = self.players[player.0 as usize].library.clone();
        let mut next = self.next_object_id();
        library
            .iter()
            .take(count as usize)
            .map(|&from| {
                let event = Event::ExiledFromLibraryMayPlay {
                    player,
                    card: next,
                    from,
                    until_next_turn,
                    face_down,
                    free_while_source,
                };
                next += 1;
                event
            })
            .collect()
    }
}
