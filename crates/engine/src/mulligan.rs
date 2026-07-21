//! Pre-game simultaneous mulligans.
//!
//! Constructors stay in the ordinary playable state for direct engine tests; callers opt into this
//! setup phase with [`Game::begin_mulligans`] after libraries are shuffled and opening hands drawn.

use crate::*;

pub fn hand_size_after_mulligans(mulligans_taken: u8) -> u8 {
    7u8.saturating_sub(mulligans_taken.saturating_sub(1))
}

impl Game {
    pub fn begin_mulligans(&mut self) {
        self.mulliganing = true;
        for player in &mut self.players {
            player.hand_kept = false;
            player.mulligans_taken = 0;
        }
        self.refresh_actions();
    }

    pub fn mulliganing(&self) -> bool {
        self.mulliganing
    }

    pub fn hand_kept(&self, player: PlayerId) -> bool {
        self.players[player.0 as usize].hand_kept
    }

    pub fn mulligans_taken(&self, player: PlayerId) -> u8 {
        self.players[player.0 as usize].mulligans_taken
    }

    pub(crate) fn keep_hand(&mut self, player: PlayerId) -> Result<Vec<Event>, Reject> {
        self.require_mulligan_decision(player)?;

        let mut events = Vec::new();
        self.push_apply(&mut events, Event::HandKept { player });
        self.finish_mulligans_if_all_kept(&mut events);
        Ok(events)
    }

    pub(crate) fn take_mulligan(&mut self, player: PlayerId) -> Result<Vec<Event>, Reject> {
        self.require_mulligan_decision(player)?;

        let next_mulligans = self.players[player.0 as usize]
            .mulligans_taken
            .saturating_add(1);
        let hand_size = hand_size_after_mulligans(next_mulligans);
        if hand_size < 1 {
            return Err(Reject::Mulliganing);
        }

        let mut events = Vec::new();
        let hand = self.hand_of(player);
        for (next, from) in (self.next_object_id()..).zip(hand) {
            self.push_apply(
                &mut events,
                Event::PutFromHandOnTop {
                    card: next,
                    from,
                    def: self.def_of(from),
                    player,
                },
            );
        }

        self.shuffle(player);
        for event in self.draw_events(player, hand_size as u32) {
            self.push_apply(&mut events, event);
        }
        self.push_apply(
            &mut events,
            Event::MulliganTaken {
                player,
                mulligans_taken: next_mulligans,
                hand_size,
            },
        );

        if hand_size == 1 {
            self.push_apply(&mut events, Event::HandKept { player });
            self.finish_mulligans_if_all_kept(&mut events);
        }

        Ok(events)
    }

    fn require_mulligan_decision(&self, player: PlayerId) -> Result<(), Reject> {
        if !self.mulliganing {
            return Err(Reject::Mulliganing);
        }
        let player = &self.players[player.0 as usize];
        if player.lost || player.hand_kept {
            return Err(Reject::Mulliganing);
        }
        Ok(())
    }

    fn finish_mulligans_if_all_kept(&mut self, events: &mut Vec<Event>) {
        let all_kept = self
            .players
            .iter()
            .all(|player| player.lost || player.hand_kept);
        if !all_kept {
            return;
        }

        self.push_apply(events, Event::MulligansFinished);
        events.extend(self.begin_first_turn_events());
    }
}
