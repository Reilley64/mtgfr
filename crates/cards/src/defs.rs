//! Interned card definitions (`CardId` → `Arc<CardDef>`).
//!
//! Zone objects and events store [`CardId`] instead of embedding a fat [`CardDef`].
//! See OpenSpec [`card-dsl`](../../../openspec/specs/card-dsl/spec.md) and
//! [`engine`](../../../openspec/specs/engine/spec.md).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::CardDef;

/// Stable handle into the process-global card-definition intern table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardId(pub u32);

#[derive(Default)]
struct InternTable {
    defs: Vec<Arc<CardDef>>,
    by_oracle_id: HashMap<&'static str, CardId>,
}

fn table() -> &'static Mutex<InternTable> {
    static TABLE: OnceLock<Mutex<InternTable>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(InternTable::default()))
}

/// Intern `def` and return its stable [`CardId`].
///
/// Real card defs reuse the same handle whenever their non-empty Scryfall oracle id matches an
/// existing entry. Test stubs with an empty `id` still get a fresh handle on every call.
pub fn intern_card_def(def: CardDef) -> CardId {
    let mut guard = table().lock().expect("card def intern table poisoned");
    if !def.id.is_empty()
        && let Some(&id) = guard.by_oracle_id.get(def.id)
    {
        return id;
    }
    let id = CardId(guard.defs.len() as u32);
    if !def.id.is_empty() {
        guard.by_oracle_id.insert(def.id, id);
    }
    guard.defs.push(Arc::new(def));
    id
}

/// Shared definition for `id`. Panics if `id` was never returned by [`intern_card_def`].
pub fn card_def(id: CardId) -> Arc<CardDef> {
    let guard = table().lock().expect("card def intern table poisoned");
    guard
        .defs
        .get(id.0 as usize)
        .cloned()
        .unwrap_or_else(|| panic!("unknown CardId({})", id.0))
}

/// How many defs are interned. Lets the engine's interning tests assert "this zone change did
/// not reintern" without reaching into the table.
pub fn interned_len() -> usize {
    table()
        .lock()
        .expect("card def intern table poisoned")
        .defs
        .len()
}
