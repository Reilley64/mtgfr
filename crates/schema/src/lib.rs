//! The wire protocol: intent/delta envelopes and per-viewer visibility filtering.
//!
//! The engine emits canonical, full-information [`engine::Event`]s. Before an event
//! reaches a player it is passed through [`redact`], which strips information that
//! player may not legally see (a drawn card's identity is private to its owner).
//! Redaction lives here, never in the engine — the engine stays audience-unaware.
//!
//! These types carry `utoipa::ToSchema` so the server can emit an OpenAPI document
//! for the TypeScript client (see ADR 0001).

mod answer_protocol;
mod catalog;
mod dto;
mod event;
mod intent;
mod projection;
mod snapshot;
#[cfg(test)]
pub(crate) mod test_support;

pub use answer_protocol::{Answer, encode_answer};
pub use catalog::{CatalogCard, catalog_card, color_identity};
pub use dto::{
    ActionView, ChoiceItem, CombatView, CommanderDamageView, CreateTableResponse, Credentials,
    DeckCardEntry, DeckDetail, DeckError, DeckSummary, JoinRequest, LobbyView, Me, ModalView,
    ModeView, ObjectView, PendingChoiceView, PlayerView, ReadyRequest, SaveDeckRequest, SeatView,
    SignupCredentials, StackObjectView, StartRequest, VisibleState, WireCost, WireKind,
};
pub use event::{DeltaEnvelope, VisibleEvent, redact, spectator_redact};
pub use intent::{
    IntentEnvelope, WireAttack, WireBlock, WireDamage, WireIntent, WireModeChoice, WireTarget,
    to_intent, to_intent_for_seat,
};
pub use snapshot::{SPECTATOR_VIEWER, StreamFrame, ViewExtras, complete_visible};

/// Mirror of [`engine::ObjectId`] for the wire.
pub type ObjectId = u32;
