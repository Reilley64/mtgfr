//! Engine data types: cards, effects, events, intents, zones, and their helpers.
//!
//! Pure definitions and small pure helpers — no `Game` state. Moved out of `lib.rs`
//! verbatim in the module split; see docs/superpowers/specs/.
//!
//! Cross-cutting CR glossary: individual [`Effect`] / [`Event`] / [`Intent`] /
//! [`Keyword`] / [`PendingChoice`] variants cite the rules they model. Not owned by
//! one chapter — start at `docs/CR_INDEX.md` for reverse lookup.

mod card;
#[path = "effect/mod.rs"]
mod effect;
mod filter;
mod inspect;
mod mana;
mod stack;
mod trigger;

pub use card::*;
pub use effect::*;
pub use filter::*;
pub use inspect::*;
pub use mana::*;
pub use stack::*;
pub use trigger::*;
