//! Library-look / search pause family — [`Effect::Dig(DigEffect::LookAtTop)`], [`Effect::Dig(DigEffect::DistributeTop)`],
//! [`Effect::Dig(DigEffect::SearchLibrary)`].
//!
//! Pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`]; this module only raises the choice.

use crate::*;

impl Game {
    /// Pause on SelectFromTop / DistributeTop / SearchLibrary for the matching effect.
    pub(crate) fn run_look_pause(&mut self, effect: Effect, ctx: ResolveCtx) {
        let ResolveCtx {
            controller, target, ..
        } = ctx;
        match effect {
            // Look at the top N, select up to `up_to` matching cards into `dest`, rest to `rest`
            // (Quandrix Apprentice). Pauses on a SelectFromTop choice.
            Effect::Dig(DigEffect::LookAtTop {
                count,
                filter,
                up_to,
                min,
                dest,
                dest_tapped,
                rest,
                mv_budget,
            }) => pending::raise(
                self,
                pending::ChoiceRequest::SelectFromTop {
                    player: controller,
                    count,
                    filter,
                    up_to,
                    min,
                    dest,
                    dest_tapped,
                    rest,
                    mv_budget,
                },
            ),
            // Look at the top N, route one card each to hand / bottom / exile-may-play
            // (Expressive Iteration). Pauses on a DistributeTop choice.
            Effect::Dig(DigEffect::DistributeTop {
                count,
                to_hand,
                to_bottom,
                to_exile_may_play,
            }) => pending::raise(
                self,
                pending::ChoiceRequest::DistributeTop {
                    player: controller,
                    count,
                    to_hand,
                    to_bottom,
                    to_exile_may_play,
                },
            ),
            // A library search (fetchlands / tutors) pauses on a SearchLibrary choice. Usually
            // the ability's own controller searches; Path to Exile/Assassin's Trophy hand the
            // search to the exiled/destroyed permanent's controller instead (CR 701.19 doesn't
            // require the searcher to be the ability's controller). `AllPlayers` (Veteran
            // Explorer) instead fans one search out to every living player in APNAP order (CR
            // 101.4): this player searches now, the rest queue in `resolution_frame.search_fanout`
            // for `Game::search_library` to continue once this player's own search (and its
            // shuffle, CR 701.19f) finishes — see that fan-out's doc comment.
            Effect::Dig(DigEffect::SearchLibrary {
                filter,
                to_zone,
                tapped,
                searcher,
                count,
                overflow,
                count_amount,
                optional,
            }) => {
                // Collective Voyage's "up to X basic land cards, where X is the total amount of
                // mana paid this way": resolve the dynamic cap once, here, so every seat of an
                // `AllPlayers` fan-out searches for the same X.
                let count = match count_amount {
                    None => count,
                    Some(amount) => self
                        .resolve_amount(amount, controller, ctx.source, target, ctx.x)
                        .clamp(0, u8::MAX as i32) as u8,
                };
                // Trench Gorger's "the number of cards exiled this way" is resolution-scoped, not
                // accumulated across searches — reset at the top of every `SearchLibrary`, mirroring
                // `NonlandCardsExiledThisWay`'s own reset at its fan-out's start.
                self.resolution_frame.cards_exiled_by_search_this_way = 0;
                let searching_player = match searcher {
                    SearchScope::You => {
                        self.resolution_frame.search_fanout = None;
                        controller
                    }
                    SearchScope::TargetController => {
                        self.resolution_frame.search_fanout = None;
                        self.controller_of(expect_object_target(
                            target,
                            "a search effect's target-controller",
                        ))
                    }
                    SearchScope::AllPlayers => {
                        let mut order = self.apnap_order();
                        if order.is_empty() {
                            // No living player to search — unreachable while resolution runs.
                            self.resolution_frame.search_fanout = None;
                            return;
                        }
                        let first = order.remove(0);
                        self.resolution_frame.search_fanout = Some(SearchFanout {
                            remaining: order,
                            filter,
                            to_zone,
                            tapped,
                            count,
                            overflow,
                        });
                        first
                    }
                };
                // Printed "may search" (White Orchid Phantom): pause for a yes/no *before* any
                // search begins. Declining skips the search and its shuffle (CR 701.19c only
                // applies once a search actually starts). Accepting re-runs this effect with
                // `optional = false` and `searcher = You` under the answering player
                // (`Game::answer_may`). An AllPlayers fan-out keeps its queue; declining walks to
                // the next seat via `continue_search_fanout`.
                if optional {
                    pending::raise(
                        self,
                        pending::ChoiceRequest::MayYesNo {
                            player: searching_player,
                            source: ctx.source,
                            effect: Effect::Dig(DigEffect::SearchLibrary {
                                filter,
                                to_zone,
                                tapped,
                                searcher: SearchScope::You,
                                count,
                                overflow,
                                count_amount: None,
                                optional: false,
                            }),
                            // Mid-resolution may-search: accept runs the search inline (not as a
                            // new stack object). Ability-level optional tutors keep Default.
                            resume: crate::MayYesNoResume::ResolveInline,
                        },
                    );
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::SearchLibrary {
                        player: searching_player,
                        filter,
                        dest: to_zone,
                        tapped,
                        count,
                        overflow,
                    },
                );
                // Stranglehold denies `searching_player` (CR 701.19): the raise above came back
                // without pausing. In an `AllPlayers` fan-out (Veteran Explorer), walk straight to
                // the next queued player instead of silently dropping the rest of the fan-out;
                // outside a fan-out this is a no-op (`continue_search_fanout` sees no live queue).
                if !self.resolution_is_paused() {
                    self.continue_search_fanout();
                }
            }
            _ => unreachable!("look pause family received a non-family effect"),
        }
    }
}
