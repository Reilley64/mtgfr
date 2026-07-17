//! Memoized effective characteristics for battlefield objects.
//!
//! Cache for [`characteristics`](crate::characteristics) — no CR chapter of its own.
//! Invalidated on relevant [`Event`]s. ADR 0003 additive recompute, not CR 613 layers.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::*;

/// Cached effective power/toughness/keywords per battlefield object, invalidated by
/// [`Game::invalidate_characteristics_cache`] when relevant [`Event`]s apply (ADR 0003 —
/// additive recompute, not CR 613 layers).
#[derive(Clone, Default)]
pub(crate) struct CharacteristicsCache {
    power: HashMap<ObjectId, i32>,
    toughness: HashMap<ObjectId, i32>,
    keywords: HashMap<ObjectId, Box<[Keyword]>>,
}

impl CharacteristicsCache {
    pub fn power(&self, object: ObjectId) -> Option<i32> {
        self.power.get(&object).copied()
    }

    pub fn set_power(&mut self, object: ObjectId, value: i32) {
        self.power.insert(object, value);
    }

    pub fn toughness(&self, object: ObjectId) -> Option<i32> {
        self.toughness.get(&object).copied()
    }

    pub fn set_toughness(&mut self, object: ObjectId, value: i32) {
        self.toughness.insert(object, value);
    }

    pub fn keywords(&self, object: ObjectId) -> Option<&[Keyword]> {
        self.keywords.get(&object).map(|k| k.as_ref())
    }

    pub fn set_keywords(&mut self, object: ObjectId, value: Vec<Keyword>) {
        self.keywords.insert(object, value.into_boxed_slice());
    }

    pub fn invalidate_object(&mut self, object: ObjectId) {
        self.power.remove(&object);
        self.toughness.remove(&object);
        self.keywords.remove(&object);
    }

    pub fn invalidate_owner(&mut self, game: &Game, owner: PlayerId) {
        for id in game.battlefield() {
            if game.owner_of(id) == owner {
                self.invalidate_object(id);
            }
        }
    }

    pub fn invalidate_all_battlefield(&mut self, game: &Game) {
        for id in game.battlefield() {
            self.invalidate_object(id);
        }
    }
}

/// Interior-mutable slot so characteristic queries can memoize under `&Game` without
/// breaking the engine's `&self` read API. `Mutex` (not `RefCell`) keeps `Game: Send` for
/// server clones. Cloned games copy the warmed cache as-is.
#[derive(Default)]
pub(crate) struct CharacteristicsCacheCell(pub(crate) Mutex<CharacteristicsCache>);

impl Clone for CharacteristicsCacheCell {
    fn clone(&self) -> Self {
        Self(Mutex::new(
            self.0.lock().expect("characteristics cache lock").clone(),
        ))
    }
}

impl CharacteristicsCacheCell {
    pub(crate) fn read<R>(&self, f: impl FnOnce(&CharacteristicsCache) -> R) -> R {
        f(&self.0.lock().expect("characteristics cache lock"))
    }

    pub(crate) fn write<R>(&self, f: impl FnOnce(&mut CharacteristicsCache) -> R) -> R {
        f(&mut self.0.lock().expect("characteristics cache lock"))
    }
}

impl Game {
    /// Drop cached P/T/keywords entries made stale by `event`. Called at the start of
    /// [`Game::apply`] so pre-mutation owner/attachment facts are still readable.
    pub(crate) fn invalidate_characteristics_cache(&self, event: &Event) {
        self.characteristics_cache.write(|cache| match *event {
            Event::CountersPlaced { object, .. }
            | Event::TempBoost { object, .. }
            | Event::BasePtSetUntilEndOfTurn { object, .. }
            | Event::TypesAddedUntilEndOfTurn { object, .. }
            | Event::TempBoostsEnded { object }
            | Event::KeywordsStripped { object, .. } => {
                cache.invalidate_object(object);
            }
            // A Backup grant (CR 702.166) adds the source's keywords to the target — drop its
            // cached keyword set.
            Event::AbilitiesGranted { target, .. } => cache.invalidate_object(target),
            // The grants clear at cleanup; every target loses the granted keywords (read the
            // still-live list before it's emptied by the applying event).
            Event::GrantedAbilitiesEnded => {
                for &(target, _) in &self.abilities_granted_until_eot {
                    cache.invalidate_object(target);
                }
            }
            Event::AttachedTo { object, host } => {
                // The *old* host (pre-mutation — this runs before the event applies) loses
                // whatever this Aura/Equipment was granting it, same as the new host gains it;
                // both need their cached P/T/keywords dropped (Shielded by Faith moving off its
                // first creature must drop that creature's cached indestructible). (CR 702.12)
                let old_host = self.as_permanent(object).and_then(|p| p.attached_to);
                cache.invalidate_object(object);
                if let Some(host) = host {
                    cache.invalidate_object(host);
                }
                if let Some(old_host) = old_host {
                    cache.invalidate_object(old_host);
                }
            }
            // ponytail: board-wide invalidation on every permanent entering (correct but coarse,
            // mirroring `LifeChanged` below) — a cross-owner count anthem (Yavimaya Enchantress's
            // "for each enchantment on the battlefield") goes stale on an *opponent's* entry, since
            // no owner-side event touches the anthem's own controller. Narrow to "only when a
            // cross-owner `per_permanent` anthem exists on the board" if recompute cost ever shows
            // up in a profile.
            Event::PermanentEntered { .. } => {
                cache.invalidate_all_battlefield(self);
            }
            Event::LandPlayed { player, .. }
            | Event::TokenCreated {
                controller: player, ..
            } => {
                cache.invalidate_owner(self, player);
            }
            // A hand-size static (Empyrial Armor's `grant_to_attached`) depends on the player's
            // hand count, which these events change: a draw/tutor-to-hand grows it, a cast
            // shrinks it (a discard rides `MovedToGraveyard` below instead, already covered).
            Event::CardDrawn { player, .. } | Event::SearchedToHand { player, .. } => {
                cache.invalidate_owner(self, player);
            }
            Event::SpellCast { controller, .. } => {
                cache.invalidate_owner(self, controller);
            }
            // ponytail: board-wide invalidation on every non-cast battlefield entry (correct but
            // coarse, same cross-owner reasoning as `PermanentEntered` above — a searched/
            // reanimated/manifested/put-onto-battlefield enchantment can belong to any player).
            Event::ReanimatedToBattlefield { .. }
            | Event::FlickeredToBattlefield { .. }
            | Event::SearchedToBattlefield { .. }
            | Event::Manifested { .. }
            | Event::PutOntoBattlefieldFromHand { .. } => {
                cache.invalidate_all_battlefield(self);
            }
            // Turning face up swaps the anonymous 2/2 for the real card's characteristics.
            Event::TurnedFaceUp { permanent } => cache.invalidate_object(permanent),
            // An enter-as-copy overwrites name/types/subtypes/P/T/keywords wholesale; a copied
            // anthem-lord could also buff the controller's other creatures, so drop the whole
            // board (CR 706/707.2 — Altered Ego, Cursed Mirror; the same at the until-EOT revert).
            Event::BecameCopy { object, .. } => {
                cache.invalidate_owner(self, self.controller_of(object));
            }
            // ponytail: board-wide invalidation on every permanent leaving the battlefield
            // (correct but coarse, same cross-owner reasoning as `PermanentEntered` above — a
            // leaving enchantment can belong to any player, not just this anthem's controller).
            Event::MovedToGraveyard { from, .. }
            | Event::MovedToExile { from, .. }
            | Event::ReturnedToHand { from, .. }
            | Event::TuckedToLibrary { from, .. } => {
                let attached_host = self.as_permanent(from).and_then(|p| p.attached_to);
                cache.invalidate_all_battlefield(self);
                if let Some(host) = attached_host {
                    cache.invalidate_object(host);
                }
            }
            Event::TokenCeasedToExist {
                controller, token, ..
            } => {
                let attached_host = self.as_permanent(token).and_then(|p| p.attached_to);
                cache.invalidate_owner(self, controller);
                if let Some(host) = attached_host {
                    cache.invalidate_object(host);
                }
            }
            Event::AttackerDeclared { object, .. } => {
                cache.invalidate_object(object);
            }
            // A chosen-type-gated anthem's newly-set source affects every creature the
            // controller owns, same scope as `LandPlayed`/`TokenCreated` above.
            Event::CreatureTypeChosen { object, .. } => {
                cache.invalidate_owner(self, self.owner_of(object));
            }
            // Flickering Ward's answered "choose a color" changed the protection keyword the Aura (CR 702.21, CR 303.4)
            // (`object`) grants its enchanted host — invalidate that host's cached keywords.
            Event::ColorChosen { object, .. } => {
                if let Some(host) = self.as_permanent(object).and_then(|p| p.attached_to) {
                    cache.invalidate_object(host);
                }
            }
            // The city's-blessing-gated anthem's condition just flipped for every creature this
            // player owns, same scope as `CreatureTypeChosen` above.
            Event::CitysBlessingGained { player } => {
                cache.invalidate_owner(self, player);
            }
            // A life change can flip a life-total-gated anthem (Bloodghast's "haste as long as an
            // opponent has 10 or less life") for any creature on the board, so drop the whole
            // battlefield's cache like `CombatCleared`.
            // ponytail: board-wide invalidation on every life change (correct but coarse); narrow
            // to the changed player's opponents' permanents if a life-heavy game shows this hot.
            Event::LifeChanged { .. } => {
                cache.invalidate_all_battlefield(self);
            }
            Event::CombatCleared => {
                cache.invalidate_all_battlefield(self);
            }
            // Phasing in/out can move a static source (an anthem, an attached Aura's grant) into
            // or out of every scan at once — drop the whole board's cache, like `CombatCleared`.
            // The phasing permanent and its attachments are dropped explicitly too: they're
            // excluded from `battlefield()` while phased, so `invalidate_all_battlefield` misses
            // them (their own stale keywords/P/T must clear on phase-in).
            Event::PhasedOut { object } | Event::PhasedIn { object } => {
                cache.invalidate_all_battlefield(self);
                cache.invalidate_object(object);
                for attached in self.attachments(object) {
                    cache.invalidate_object(attached);
                }
            }
            _ => {}
        });
    }
}
