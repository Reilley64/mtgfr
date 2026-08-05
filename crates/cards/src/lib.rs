//! The card DSL: the vocabulary a card is written in, plus the pool that is written in it.
//!
//! [`types`] defines the enums and structs of the DSL — [`CardDef`], [`Effect`], filters, mana,
//! triggers. [`toml_surface`] is the document shape `toml::from_str` parses, and the source the
//! committed JSON Schema and `DSL_REFERENCE.md` are generated from. The `engine` crate depends
//! on this one and implements the rules logic *around* these types; the vocabulary itself never
//! reaches for game state.
//!
//! The rest of the crate is data: one TOML file per card under `data/`, plus token profiles
//! under `data/tokens/`, loaded once into registries of [`CardDef`]. Deserialize interns owned
//! strings and load-once data to `'static` where useful, then clones small handles from the
//! bounded pool as needed. File I/O lives here, keeping the engine free of it (`CLAUDE.md`).
//!
//! Token profiles load first and are installed via [`install_token_defs`] so `create_token`'s
//! `token = "<oracle-id>"` resolves during deckable-card deserialize. Tokens are **not** in
//! [`registry`] — the catalog/deck builder only sees castable cards.

/// Card-DSL deserialization (the `card-dsl` feature): manual impls for the types whose TOML
/// spelling differs structurally from their Rust shape, plus interning/default helpers
/// referenced by the `cfg_attr` serde derives on the vocabulary types.
#[cfg(feature = "card-dsl")]
mod de;
mod defs;
#[cfg(feature = "card-dsl")]
pub mod toml_surface;
pub mod types;

/// Install / look up token profiles (`data/tokens/*.toml`) for card-DSL load.
#[cfg(feature = "card-dsl")]
pub use de::{install_token_defs, token_def};
pub use defs::{CardId, card_def, intern_card_def, interned_len};
#[cfg(feature = "card-dsl")]
pub use toml_surface::CardToml;
pub use types::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use jsonschema::Validator;
use serde_json::Value as JsonValue;

const CARD_SCHEMA_JSON: &str = include_str!("../schema/card.schema.json");
const TOKEN_SCHEMA_JSON: &str = include_str!("../schema/token.schema.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TomlSchemaKind {
    Card,
    Token,
}

struct Pool {
    /// Primary key: Scryfall oracle id ([`CardDef::id`]).
    by_id: HashMap<String, CardDef>,
    /// Secondary: printed name → CardDef (authoring, tests, fuzzy display).
    by_name: HashMap<String, CardDef>,
}

struct TokenPool {
    by_id: HashMap<String, CardDef>,
}

static POOL: OnceLock<Pool> = OnceLock::new();
static TOKEN_POOL: OnceLock<TokenPool> = OnceLock::new();

fn pool() -> &'static Pool {
    POOL.get_or_init(load_from_data_dir)
}

fn token_pool() -> &'static TokenPool {
    let _ = pool();
    TOKEN_POOL
        .get()
        .expect("token pool installed during card load")
}

/// The loaded card registry, keyed by Card id (Scryfall oracle id). Deckable cards only.
pub fn registry() -> &'static HashMap<String, CardDef> {
    &pool().by_id
}

/// The card with the given Card id, if it exists in the pool.
pub fn get(id: &str) -> Option<CardDef> {
    pool().by_id.get(id).cloned()
}

/// The card with the given printed name, if it exists in the pool.
pub fn get_by_name(name: &str) -> Option<CardDef> {
    pool().by_name.get(name).cloned()
}

/// Token profiles from `data/tokens/`, keyed by Scryfall oracle id.
pub fn token_registry() -> &'static HashMap<String, CardDef> {
    &token_pool().by_id
}

/// The token profile with the given Scryfall oracle id, if it exists.
pub fn get_token(id: &str) -> Option<CardDef> {
    token_pool().by_id.get(id).cloned()
}

pub fn validate_toml_str(text: &str) -> Result<(), Vec<String>> {
    validate_toml_str_as(TomlSchemaKind::Card, text)
}

pub fn validate_toml_str_as(kind: TomlSchemaKind, text: &str) -> Result<(), Vec<String>> {
    let toml_value = toml::from_str::<toml::Value>(text)
        .map_err(|err| vec![format!("TOML parse error: {err}")])?;
    let instance = serde_json::to_value(toml_value)
        .map_err(|err| vec![format!("TOML to JSON conversion failed: {err}")])?;
    let validator = schema_validator(kind);
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|err| format!("{}: {err}", err.instance_path()))
        .collect();

    if errors.is_empty() {
        return Ok(());
    }

    Err(errors)
}

pub fn validate_toml_path(kind: TomlSchemaKind, path: impl AsRef<Path>) -> Result<(), Vec<String>> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .map_err(|err| vec![format!("{}: reading TOML failed: {err}", path.display())])?;

    validate_toml_str_as(kind, &text).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| format!("{}: {error}", path.display()))
            .collect()
    })
}

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))
}

fn schema_validator(kind: TomlSchemaKind) -> Validator {
    let schema_text = match kind {
        TomlSchemaKind::Card => CARD_SCHEMA_JSON,
        TomlSchemaKind::Token => TOKEN_SCHEMA_JSON,
    };
    let schema = serde_json::from_str::<JsonValue>(schema_text).expect("card schema JSON parses");
    jsonschema::validator_for(&schema).expect("card schema compiles")
}

fn load_toml_file(path: &Path) -> CardDef {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    toml::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

fn load_token_defs(dir: &Path) {
    let tokens_dir = dir.join("tokens");
    let entries = std::fs::read_dir(&tokens_dir)
        .unwrap_or_else(|e| panic!("reading token data dir {}: {e}", tokens_dir.display()));

    let mut by_id_owned: HashMap<String, CardDef> = HashMap::new();
    let mut engine_map: HashMap<&'static str, CardDef> = HashMap::new();

    for entry in entries {
        let path = entry.expect("token data dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let def = load_toml_file(&path);
        if def.id.is_empty() {
            panic!(
                "{}: token CardDef.id (Scryfall oracle id) is required",
                path.display()
            );
        }
        // `default_print` is optional for token profiles (fidelity increment #97): a token
        // predating printed token cards (e.g. Legends) has no Scryfall printing to key at all.
        // An empty `default_print` already renders as the card back client-side
        // (`client/app/domain/ui/card-art.ts` `cardArtUrl`), so there's no synthetic UUID to
        // invent — leaving it empty is the faithful representation of "no printing exists".
        if by_id_owned
            .insert(def.id.to_string(), def.clone())
            .is_some()
        {
            panic!("{}: duplicate token id {}", path.display(), def.id);
        }
        // `def.id` is already leaked/`'static` from CardDef deserialize.
        engine_map.insert(def.id, def);
    }

    TOKEN_POOL
        .set(TokenPool { by_id: by_id_owned })
        .unwrap_or_else(|_| panic!("token pool installed twice"));
    install_token_defs(engine_map);
}

fn load_from_data_dir() -> Pool {
    let dir = data_dir();
    // Tokens first so `token = "<id>"` resolves while parsing deckable cards.
    load_token_defs(&dir);

    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("reading card data dir {dir:?}: {e}"));

    let mut by_id = HashMap::new();
    let mut by_name = HashMap::new();
    for entry in entries {
        let path = entry.expect("card data dir entry").path();
        // Non-recursive: `data/tokens/` is loaded separately above.
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let def = load_toml_file(&path);
        if def.id.is_empty() {
            panic!(
                "{}: CardDef.id (Scryfall oracle id) is required",
                path.display()
            );
        }
        if def.default_print.is_empty() {
            panic!(
                "{}: CardDef.default_print (Scryfall card UUID) is required",
                path.display()
            );
        }
        if by_id.insert(def.id.to_string(), def.clone()).is_some() {
            panic!("{}: duplicate Card id {}", path.display(), def.id);
        }
        if by_name.insert(def.name.to_string(), def.clone()).is_some() {
            panic!("{}: duplicate card name {}", path.display(), def.name);
        }
    }
    Pool { by_id, by_name }
}
