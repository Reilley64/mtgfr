//! Arena-style BO1 hand smoothing: two shuffle samples, keep closer land count.

use crate::*;

/// Whether sample A should be kept over sample B (ties → A).
pub(crate) fn sample_a_wins(lands_a: u32, lands_b: u32, expected: f64) -> bool {
    let dist_a = (lands_a as f64 - expected).abs();
    let dist_b = (lands_b as f64 - expected).abs();
    dist_a <= dist_b
}

impl Game {
    /// Two-sample BO1 land smoothing: shuffle twice via derive-per-op RNG, keep the
    /// library order whose top `hand_size` land count is closer to
    /// `hand_size * lands / len`. Ties -> sample A. Does not draw.
    pub(crate) fn smoothed_shuffle_for_hand(&mut self, player: PlayerId, hand_size: u8) {
        let len = self.players[player.0 as usize].library.len();
        if len < 2 {
            return;
        }

        let land_total = self.players[player.0 as usize]
            .library
            .iter()
            .filter(|&&id| CardFilter::Land.matches(card_def(self.def_id_of(id)).as_ref()))
            .count();
        let n = (hand_size as usize).min(len);
        let expected = n as f64 * land_total as f64 / len as f64;

        let baseline = self.players[player.0 as usize].library.clone();

        self.shuffle(player);
        let lands_a = self.count_lands_in_top(player, n);
        let order_a = self.players[player.0 as usize].library.clone();

        self.players[player.0 as usize].library = baseline;
        self.shuffle(player);
        let lands_b = self.count_lands_in_top(player, n);
        let order_b = self.players[player.0 as usize].library.clone();

        self.players[player.0 as usize].library = if sample_a_wins(lands_a, lands_b, expected) {
            order_a
        } else {
            order_b
        };
    }

    /// Smoothed shuffle for `hand_size`, then draw up to that many cards.
    pub fn deal_smoothed_hand(&mut self, player: PlayerId, hand_size: u8) {
        self.smoothed_shuffle_for_hand(player, hand_size);
        for _ in 0..hand_size {
            self.draw_card(player);
        }
    }

    fn count_lands_in_top(&self, player: PlayerId, n: usize) -> u32 {
        self.players[player.0 as usize]
            .library
            .iter()
            .take(n)
            .filter(|&&id| CardFilter::Land.matches(card_def(self.def_id_of(id)).as_ref()))
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closer_sample_wins() {
        // expected 3.0: A has 3, B has 5 → A
        assert!(sample_a_wins(3, 5, 3.0));
        // expected 3.0: A has 1, B has 3 → B
        assert!(!sample_a_wins(1, 3, 3.0));
    }

    #[test]
    fn equal_distance_prefers_sample_a() {
        // |2-3| == |4-3|
        assert!(sample_a_wins(2, 4, 3.0));
    }
}

#[cfg(test)]
mod game_tests {
    use super::*;

    fn card(name: &'static str) -> CardDef {
        let _ = cards::get_by_name(name).unwrap_or_else(|| panic!("unknown card {name:?}"));
        let kind = match name {
            "Forest" => CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            },
            "Shock" => CardKind::Spell {
                speed: SpellSpeed::Instant,
            },
            _ => panic!("unknown card {name:?}"),
        };
        CardDef {
            name,
            id: "",
            default_print: "",
            cost: Cost::FREE,
            kind,
            legendary: false,
            uncounterable: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
            identity_pips: empty_slice(),
            colors: empty_slice(),
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            free_cast_if: None,
            alternative_cost: None,
            cast_only_during_combat: false,
            cast_only_before_attackers: false,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: empty_slice(),
            otags: empty_slice(),
            cycling: None,
            cycling_sacrifice: SacrificeCost::None,
            flashback: None,
            echo: None,
            cumulative_upkeep: None,
            recover: None,
            bestow: None,
            morph: None,
            evoke: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            enchant: None,
            enchant_graveyard: false,
            back: None,
            adventure: None,
            halves: empty_slice(),
            suspend: None,
            vanishing: None,
            cast_x_max: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: empty_slice(),
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
        }
    }

    fn mixed_deck() -> Vec<CardDef> {
        let mut deck = vec![card("Forest"); 20];
        deck.extend(std::iter::repeat_n(card("Shock"), 20));
        deck
    }

    fn count_lands_top(game: &Game, player: PlayerId, n: usize) -> u32 {
        game.players[player.0 as usize]
            .library
            .iter()
            .take(n)
            .filter(|&&id| CardFilter::Land.matches(card_def(game.def_id_of(id)).as_ref()))
            .count() as u32
    }

    #[test]
    fn smoothed_shuffle_advances_op_iteration_by_two() {
        let mut game = Game::with_master_seed(1, [7; 32]);
        let p = PlayerId(0);
        game.stack_library(p, &mixed_deck());
        let before = game.players[p.0 as usize].op_iteration;
        game.smoothed_shuffle_for_hand(p, 7);
        assert_eq!(game.players[p.0 as usize].op_iteration, before + 2);
    }

    #[test]
    fn smoothed_shuffle_picks_closer_of_two_samples() {
        let deck = mixed_deck();
        let mut diverging_seed = None;
        for seed_byte in 0u8..=255 {
            let mut master = [0u8; 32];
            master[0] = seed_byte;
            let mut probe = Game::with_master_seed(1, master);
            let p = PlayerId(0);
            probe.stack_library(p, &deck);
            let baseline = probe.players[p.0 as usize].library.clone();
            probe.shuffle(p);
            let lands_a = count_lands_top(&probe, p, 7);
            probe.players[p.0 as usize].library = baseline;
            probe.shuffle(p);
            let lands_b = count_lands_top(&probe, p, 7);
            if lands_a != lands_b {
                diverging_seed = Some(master);
                break;
            }
        }
        let master = diverging_seed.expect("need a seed where samples differ");

        let mut expected_game = Game::with_master_seed(1, master);
        let p = PlayerId(0);
        expected_game.stack_library(p, &deck);
        let baseline = expected_game.players[p.0 as usize].library.clone();
        let land_total = baseline
            .iter()
            .filter(|&&id| CardFilter::Land.matches(card_def(expected_game.def_id_of(id)).as_ref()))
            .count();
        let expected = 7.0 * land_total as f64 / baseline.len() as f64;

        expected_game.shuffle(p);
        let lands_a = count_lands_top(&expected_game, p, 7);
        let order_a = expected_game.players[p.0 as usize].library.clone();
        expected_game.players[p.0 as usize].library = baseline.clone();
        expected_game.shuffle(p);
        let lands_b = count_lands_top(&expected_game, p, 7);
        let order_b = expected_game.players[p.0 as usize].library.clone();
        let want = if sample_a_wins(lands_a, lands_b, expected) {
            order_a
        } else {
            order_b
        };

        let mut game = Game::with_master_seed(1, master);
        game.stack_library(p, &deck);
        game.smoothed_shuffle_for_hand(p, 7);
        assert_eq!(game.players[p.0 as usize].library, want);
    }

    #[test]
    fn deal_smoothed_hand_draws_n_and_burns_two_ops() {
        let mut game = Game::with_master_seed(1, [7; 32]);
        let p = PlayerId(0);
        game.stack_library(p, &mixed_deck());
        game.deal_smoothed_hand(p, 7);
        assert_eq!(game.hand(p).len(), 7);
        assert_eq!(game.library_size(p), 33);
        assert_eq!(game.op_iteration(p), 2);
    }

    #[test]
    fn ordinary_shuffle_still_burns_one_op() {
        let mut game = Game::with_master_seed(1, [3; 32]);
        let p = PlayerId(0);
        game.stack_library(p, &vec![card("Forest"); 10]);
        let before = game.op_iteration(p);
        game.shuffle(p);
        assert_eq!(game.op_iteration(p), before + 1);
    }

    #[test]
    fn singleton_library_burns_no_ops() {
        let mut game = Game::with_master_seed(1, [1; 32]);
        let p = PlayerId(0);
        game.stack_library(p, &[card("Forest")]);
        game.smoothed_shuffle_for_hand(p, 7);
        assert_eq!(game.op_iteration(p), 0);
    }
}
