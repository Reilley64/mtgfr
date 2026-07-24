//! Resolution-local scratch for "this way" Amounts and related riders.
//!
//! Turn-scoped tallies stay on [`crate::Game`]; these fields are overwritten per resolving
//! step / Sequence and read by [`crate::Game::resolve_amount`]. Kept off the top-level `Game`
//! field list so resolution locality is one module, not leaky Game interface growth.

use crate::state::{DestroyedThisWay, PowerExiledThisWay};
use crate::{CardFilter, ObjectId, PlayerId, SearchDest};

/// Scratch for one effect resolution (or Sequence of steps sharing a pause/resume).
#[derive(Clone, Default)]
pub(crate) struct ResolutionFrame {
    /// Permanents this resolution's own [`Effect::Destroy(DestroyEffect::DestroyAll)`](crate::Effect::Destroy(DestroyEffect::DestroyAll)) destroyed
    /// (`Amount::PermanentsDestroyedThisWay`). Overwritten each DestroyAll, not accumulated.
    pub(crate) destroyed_this_way: Vec<DestroyedThisWay>,
    /// Nonland cards this resolution's [`Effect::Choice(ChoiceEffect::EachPlayerExilesFromGraveyard)`](crate::Effect::Choice(ChoiceEffect::EachPlayerExilesFromGraveyard))
    /// exiled (`Amount::NonlandCardsExiledThisWay`).
    pub(crate) nonland_cards_exiled_this_way: u32,
    /// Cards this resolution's own [`Effect::Dig(DigEffect::SearchLibrary)`](crate::Effect::Dig(DigEffect::SearchLibrary)) step
    /// just moved to an [`Exile`](crate::SearchDest::Exile) destination (Trench Gorger's "the
    /// number of cards exiled this way", `Amount::CardsExiledBySearchThisWay`). Reset to 0 when
    /// the search begins ([`Game::run_look_pause`](crate::Game::run_look_pause)'s
    /// `Effect::Dig(DigEffect::SearchLibrary)` arm), incremented per pick
    /// ([`Game::search_library`](crate::Game::search_library)).
    pub(crate) cards_exiled_by_search_this_way: u32,
    /// Total mana every player paid into this resolution's join-forces round
    /// ([`Effect::Choice(ChoiceEffect::JoinForcesPayMana)`](crate::Effect::Choice(ChoiceEffect::JoinForcesPayMana))), read by
    /// [`Amount::ManaPaidThisWay`](crate::Amount::ManaPaidThisWay). Reset when the round begins,
    /// the join-forces twin of [`Self::council_past_votes`].
    pub(crate) join_forces_mana: u32,
    /// Council's-dilemma tallies for this resolution's vote round.
    pub(crate) council_past_votes: u32,
    pub(crate) council_present_votes: u32,
    /// Total mana value milled by this resolution's [`Effect::Mill(MillEffect::MillSelf)`](crate::Effect::Mill(MillEffect::MillSelf)).
    pub(crate) milled_mana_value_this_way: u32,
    /// Card id + mana value from [`Effect::Dig(DigEffect::ExileTargetGraveyardCardRecordManaValue)`](crate::Effect::Dig(DigEffect::ExileTargetGraveyardCardRecordManaValue)).
    pub(crate) surge_exiled_card: Option<(ObjectId, u32)>,
    /// The mana value of the **nonland** card this resolution returned from a graveyard to its
    /// owner's hand (Vengeful Rebirth's "If you return a nonland card to your hand this way …
    /// damage equal to that card's mana value"), read by
    /// [`Amount::ReturnedNonlandCardManaValue`](crate::Amount::ReturnedNonlandCardManaValue).
    /// `None` when nothing came back from a graveyard this resolution, or a *land* card did —
    /// written on every [`Event::ReturnedToHand`](crate::Event::ReturnedToHand) apply (`apply.rs`,
    /// like [`Self::vanished_permanent_owner`]) and cleared as each spell resolution begins, so a
    /// fizzled return clause can't leak a previous resolution's value into the damage clause.
    pub(crate) returned_nonland_card_mana_value: Option<u32>,
    /// Creature controller + power from this resolution's [`Effect::Destroy(DestroyEffect::ExileAll)`](crate::Effect::Destroy(DestroyEffect::ExileAll)).
    pub(crate) power_exiled_this_way: Vec<PowerExiledThisWay>,
    /// Whether the edict controller sacrificed during this resolution's
    /// [`Effect::Choice(ChoiceEffect::EachPlayerSacrifices)`](crate::Effect::Choice(ChoiceEffect::EachPlayerSacrifices)).
    pub(crate) sacrificed_by_edict_controller: bool,
    /// Last-known owner of a token that just ceased to exist ([`Event::TokenCeasedToExist`](crate::Event::TokenCeasedToExist),
    /// CR 111.7) — written unconditionally whenever that event applies (`apply.rs`), read by
    /// [`Game::owner_of_shared_target`](crate::Game::owner_of_shared_target) so a *later* step in
    /// the same [`Sequence`](crate::Effect::Sequence) (Oblation's `target_owner_draws` rider,
    /// after its own tuck step vanished a token target) can still resolve the owner instead of
    /// panicking on `Object::Removed`. Object ids are never reused, so matching on the id makes
    /// a stale entry from an unrelated earlier vanish harmless.
    pub(crate) vanished_permanent_owner: Option<(ObjectId, PlayerId)>,
    /// An all-players [`Effect::Dig(DigEffect::SearchLibrary)`](crate::Effect::Dig(DigEffect::SearchLibrary)) fan-out's (Veteran
    /// Explorer) still-to-search players and the filter/destination/count each of their searches
    /// restarts fresh with. `None` once the last queued player's search ends (or for any
    /// single-searcher search, which never populates this).
    pub(crate) search_fanout: Option<SearchFanout>,
}

/// Continuation state for [`SearchScope::AllPlayers`](crate::SearchScope::AllPlayers): the
/// players still to search (APNAP order) after the one currently paused, plus the fixed
/// filter/destination/count/overflow every one of their searches restarts with (each player's
/// own [`PendingChoice::SearchLibrary`](crate::PendingChoice::SearchLibrary) re-pauses/mutates
/// independently of this template — see [`Game::search_library`](crate::Game::search_library)).
#[derive(Debug, Clone)]
pub(crate) struct SearchFanout {
    pub(crate) remaining: Vec<PlayerId>,
    pub(crate) filter: CardFilter,
    pub(crate) to_zone: SearchDest,
    pub(crate) tapped: bool,
    pub(crate) count: u8,
    pub(crate) overflow: Option<SearchDest>,
}
