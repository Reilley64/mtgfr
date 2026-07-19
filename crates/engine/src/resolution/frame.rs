//! Resolution-local scratch for "this way" Amounts and related riders.
//!
//! Turn-scoped tallies stay on [`crate::Game`]; these fields are overwritten per resolving
//! step / Sequence and read by [`crate::Game::resolve_amount`]. Kept off the top-level `Game`
//! field list so resolution locality is one module, not leaky Game interface growth.

use crate::state::{DestroyedThisWay, PowerExiledThisWay};
use crate::{ObjectId, PlayerId};

/// Scratch for one effect resolution (or Sequence of steps sharing a pause/resume).
#[derive(Clone, Default)]
pub(crate) struct ResolutionFrame {
    /// Permanents this resolution's own [`Effect::DestroyAll`](crate::Effect::DestroyAll) destroyed
    /// (`Amount::PermanentsDestroyedThisWay`). Overwritten each DestroyAll, not accumulated.
    pub(crate) destroyed_this_way: Vec<DestroyedThisWay>,
    /// Nonland cards this resolution's [`Effect::EachPlayerExilesFromGraveyard`](crate::Effect::EachPlayerExilesFromGraveyard)
    /// exiled (`Amount::NonlandCardsExiledThisWay`).
    pub(crate) nonland_cards_exiled_this_way: u32,
    /// Council's-dilemma tallies for this resolution's vote round.
    pub(crate) council_past_votes: u32,
    pub(crate) council_present_votes: u32,
    /// Total mana value milled by this resolution's [`Effect::MillSelf`](crate::Effect::MillSelf).
    pub(crate) milled_mana_value_this_way: u32,
    /// Card id + mana value from [`Effect::ExileTargetGraveyardCardRecordManaValue`](crate::Effect::ExileTargetGraveyardCardRecordManaValue).
    pub(crate) surge_exiled_card: Option<(ObjectId, u32)>,
    /// Creature controller + power from this resolution's [`Effect::ExileAll`](crate::Effect::ExileAll).
    pub(crate) power_exiled_this_way: Vec<PowerExiledThisWay>,
    /// Whether the edict controller sacrificed during this resolution's
    /// [`Effect::EachPlayerSacrifices`](crate::Effect::EachPlayerSacrifices).
    pub(crate) sacrificed_by_edict_controller: bool,
    /// Last-known owner of a token that just ceased to exist ([`Event::TokenCeasedToExist`](crate::Event::TokenCeasedToExist),
    /// CR 111.7) — written unconditionally whenever that event applies (`apply.rs`), read by
    /// [`Game::owner_of_shared_target`](crate::Game::owner_of_shared_target) so a *later* step in
    /// the same [`Sequence`](crate::Effect::Sequence) (Oblation's `target_owner_draws` rider,
    /// after its own tuck step vanished a token target) can still resolve the owner instead of
    /// panicking on `Object::Removed`. Object ids are never reused, so matching on the id makes
    /// a stale entry from an unrelated earlier vanish harmless.
    pub(crate) vanished_permanent_owner: Option<(ObjectId, PlayerId)>,
}
