//! The card-DSL vocabulary: the enums and structs a card TOML is written in.
//!
//! Pure definitions and small pure helpers — no `Game` state, no I/O. The engine implements
//! the rules logic *around* these types; they are defined here so the TOML surface, the
//! generated JSON Schema, and `DSL_REFERENCE.md` all project from one source.
//!
//! Cross-cutting CR glossary: individual [`Effect`] / [`Keyword`] variants cite the rules they
//! model. Not owned by one chapter — start at `docs/CR_INDEX.md` for reverse lookup.

mod card;
#[path = "effect/mod.rs"]
mod effect;
mod filter;
mod mana;
mod trigger;

pub use card::*;
pub use effect::*;
pub use filter::*;
pub use mana::*;
pub use trigger::*;
