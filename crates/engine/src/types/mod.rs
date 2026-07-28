//! Engine runtime types: zone objects, events, intents, and their helpers.
//!
//! Pure definitions and small pure helpers — no `Game` state. The card-DSL vocabulary
//! (`CardDef`, `Effect`, filters, mana, triggers) lives in the `cards` crate and is
//! re-exported from the engine root; what remains here is the state machine's own object
//! model.
//!
//! Cross-cutting CR glossary: individual [`Event`] / [`Intent`] / [`PendingChoice`] variants
//! cite the rules they model. Not owned by one chapter — start at `docs/CR_INDEX.md` for
//! reverse lookup.

mod inspect;
mod object;
mod stack;

pub use inspect::*;
pub(crate) use object::*;
pub use stack::*;
