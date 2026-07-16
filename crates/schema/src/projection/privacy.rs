//! Shared per-viewer privacy gates for projection. Every private field routes
//! through one of these so a new variant can't forget the guard and leak a hand.

use crate::dto::ChoiceItem;

/// A value belonging to `owner` is visible only when `viewer` *is* that owner — an
/// opponent, and a spectator (`viewer == None`), both get `None`.
pub(crate) fn redact_private<T>(
    owner: engine::PlayerId,
    viewer: Option<engine::PlayerId>,
    value: T,
) -> Option<T> {
    (viewer == Some(owner)).then_some(value)
}

/// The "labeled item list" shape a pending choice uses (a scry look, a tutor's
/// matches, a discardable hand): visible to `owner` only. An opponent, or a
/// spectator (`viewer == None`), gets an empty list — the count elsewhere on the
/// view stays public.
pub(crate) fn private_items(
    owner: engine::PlayerId,
    viewer: Option<engine::PlayerId>,
    ids: Vec<engine::ObjectId>,
    label: impl FnOnce(Vec<engine::ObjectId>) -> Vec<ChoiceItem>,
) -> Vec<ChoiceItem> {
    if viewer == Some(owner) {
        label(ids)
    } else {
        Vec::new()
    }
}
