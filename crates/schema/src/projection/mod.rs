//! Engine → wire projection: the single place new [`engine::Event`] and
//! [`engine::PendingChoice`] variants must be mapped before they reach a client.
//!
//! Per-viewer redaction (ADR 0006) lives here — the engine stays audience-unaware.
//! Exhaustive `match`es on engine enums are the compile-time gate: a new variant
//! without a projection arm is a build failure.

mod choice;
mod event;
mod privacy;

pub(crate) use choice::project_pending_choice;
pub(crate) use event::project_event;
