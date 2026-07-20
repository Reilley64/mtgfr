//! Live tables and the in-process registry that owns them (ADR 0005 / 0021).
//!
//! One table per `table_id`; born already seeded (see [`Table::seeded`] + [`crate::decks::seed_game`]).
//! Deck seeding stays in [`crate::decks`]; stream subscribe stays in [`crate::stream`].

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use engine::Game;
use schema::SeedSeat;
use tokio::sync::broadcast;

use crate::chrome::ChromeState;
use crate::session::{Broadcast, PublishedDelta};

/// One seated player: the user who owns the seat and their display name (public — every
/// viewer's [`schema::PlayerView::username`] shows it).
#[derive(Debug, Clone, Default)]
pub struct Seat {
    pub user_id: Option<i64>,
    pub username: Option<String>,
}

/// A table: up to four seats playing a live game. One `Table` per `table_id` in the registry,
/// born with its game already running (see [`Table::seeded`]).
pub struct Table {
    /// The four seats (a Commander table). Only the first `seats.len()`-many via `seed` are
    /// filled; a table always seeds with 2..=4 players.
    pub seats: [Seat; 4],
    /// The host user (whoever started the game on the BFF side) — kept for display/audit; no
    /// handler on this side gates on it anymore.
    pub host: Option<i64>,
    /// The live game.
    pub game: Option<Game>,
    /// The PRNG seed the game was seeded with (recorded so a replay reproduces the shuffle).
    /// Meaningful only once `game` is `Some`.
    pub seed: u64,
    /// Monotonic delta sequence number; the snapshot watermark for resume.
    pub seq: u64,
    /// Monotonic publish id for the stream fan-out (gRPC broadcast). Advances on every
    /// broadcast, including hold-only ticks that keep game `seq` unchanged (dwell must not kill
    /// the hold timer).
    pub broadcast_seq: u64,
    pub tx: broadcast::Sender<Broadcast>,
    /// Auto-pass / stack-hold / dwell policy — see [`crate::chrome::ChromeState`].
    pub chrome: ChromeState,
    /// Per-seat Card id → Printing UUID from the seat's deck (art preference for ObjectView).
    pub prints: [std::collections::HashMap<String, String>; 4],
    /// When `Game.Stream` last went quiet (`None` = has/had listeners, grace not armed).
    /// Seed starts `Some(now)`; subscribe clears to `None`; drain arms `Some(now)` on the
    /// first no-listener sweep so reconnect grace is not skipped off a stale seed timestamp.
    pub(crate) quiet_since: Option<Instant>,
}

impl Table {
    /// An empty table shell (no seats, no game) — a test-support builder; production tables are
    /// always born via [`Table::seeded`].
    #[cfg(test)]
    pub fn empty() -> Table {
        Self::shell()
    }

    fn shell() -> Table {
        let (tx, _rx) = broadcast::channel(256);
        Table {
            seats: Default::default(),
            host: None,
            game: None,
            seed: 0,
            seq: 0,
            broadcast_seq: 0,
            tx,
            chrome: ChromeState::default(),
            prints: Default::default(),
            quiet_since: Some(Instant::now()),
        }
    }

    /// Build a table with its seats filled from an already-resolved lobby, ready for
    /// [`Table::game`] to be set by [`crate::decks::seed_game`]. `seats` is ordered by seat
    /// index (2..=4).
    pub fn seeded(host_user_id: i64, seats: &[SeedSeat]) -> Table {
        let mut table = Self::shell();
        table.host = Some(host_user_id);
        for (i, seat) in seats.iter().enumerate() {
            table.seats[i] = Seat {
                user_id: Some(seat.user_id),
                username: Some(seat.username.clone()),
            };
        }
        table
    }

    /// Milliseconds until the stack-hold would resolve, or `0` if no hold is active.
    /// Deadline math lives in the session module (shared with the hold poll loop).
    pub fn stack_hold_remaining_ms(&self) -> u32 {
        crate::session::stack_hold_remaining_ms(self.chrome.stack_hold(), self.chrome.any_dwell())
    }

    /// Fan out the current hold remaining without bumping game `seq` (dwell / countdown sync).
    /// Private to chrome — only [`crate::session::TableSession`] calls this.
    pub(crate) fn publish_hold_tick(&mut self) {
        let Some(game) = self.game.as_ref() else {
            return;
        };
        self.broadcast_seq += 1;
        let _ = self.tx.send(std::sync::Arc::new(PublishedDelta {
            seq: self.seq,
            broadcast_seq: self.broadcast_seq,
            events: vec![],
            game: game.clone(),
            auto_actions: vec![],
            yields: *self.chrome.yields(),
            turn_yields: *self.chrome.turn_yields(),
            stack_hold_remaining_ms: self.stack_hold_remaining_ms(),
        }));
    }

    /// The seat index a user holds, if any.
    pub fn seat_of(&self, user_id: i64) -> Option<u8> {
        self.seats
            .iter()
            .position(|s| s.user_id == Some(user_id))
            .map(|i| i as u8)
    }
}

/// The registry of live tables, keyed by `table_id`. Single instance, in-process — no Redis
/// (ADR 0005). Ids are random 128-bit hex, so they're unguessable.
#[derive(Default)]
pub struct Registry {
    tables: HashMap<String, Table>,
}

/// How long a started table may sit with zero `Game.Stream` subscribers before drain treats it
/// as abandoned ("seats vacated" in the deploy PRD). Long enough for a reconnect blip; short
/// enough that ghost tables from closed browsers don't pin Terminating pods for the full grace.
pub const ABANDONED_TABLE_GRACE: Duration = Duration::from_secs(60);

impl Registry {
    /// Insert a new table. Returns `false` if `table_id` is already registered (no overwrite).
    ///
    /// `HashMap::try_insert` is still unstable (`map_try_insert`); Entry matches that semantics.
    pub(crate) fn try_insert(&mut self, table_id: String, table: Table) -> bool {
        use std::collections::hash_map::Entry;
        match self.tables.entry(table_id) {
            Entry::Vacant(slot) => {
                slot.insert(table);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub(crate) fn get(&self, table_id: &str) -> Option<&Table> {
        self.tables.get(table_id)
    }

    pub(crate) fn get_mut(&mut self, table_id: &str) -> Option<&mut Table> {
        self.tables.get_mut(table_id)
    }

    pub(crate) fn remove(&mut self, table_id: &str) -> Option<Table> {
        self.tables.remove(table_id)
    }

    /// Live games this instance holds (every table is born already seeded — see
    /// [`Table::seeded`] — so this is simply how many tables are registered).
    pub fn active_table_count(&self) -> usize {
        self.tables.values().filter(|t| t.game.is_some()).count()
    }

    /// Drop started tables that have had no stream subscribers for at least `grace`.
    /// Returns how many were removed. Call from the SIGTERM drain loop.
    ///
    /// `quiet_since = None` means the table had listeners (or just lost them without a sweep
    /// yet) — the first no-listener observation *arms* grace from `now` instead of treating a
    /// stale seed timestamp as the start of quiet.
    pub fn evict_abandoned(&mut self, now: Instant, grace: Duration) -> usize {
        let before = self.tables.len();
        self.tables.retain(|_, table| {
            if table.game.is_none() {
                return true;
            }
            if table.tx.receiver_count() > 0 {
                table.quiet_since = None;
                return true;
            }
            match table.quiet_since {
                None => {
                    table.quiet_since = Some(now);
                    true
                }
                Some(since) => now.saturating_duration_since(since) < grace,
            }
        });
        before - self.tables.len()
    }
}

/// Lock the table registry, tolerating a poisoned mutex. A panic under the lock quarantines
/// just that one table (C3, see `session::TableSession::apply`); it must not brick every other
/// table by leaving every later `lock()` panic on the poison.
pub fn lock(reg: &Mutex<Registry>) -> std::sync::MutexGuard<'_, Registry> {
    reg.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::PlayerId;
    use std::sync::Mutex;

    #[test]
    fn try_insert_rejects_a_duplicate_table_id() {
        let mut registry = Registry::default();
        assert!(registry.try_insert("t1".to_string(), Table::empty()));
        assert!(registry.get("t1").is_some());
        assert!(
            !registry.try_insert("t1".to_string(), Table::empty()),
            "duplicate id must not overwrite"
        );
        assert_eq!(
            registry.active_table_count(),
            0,
            "empty shells are not active"
        );
    }

    #[test]
    fn lock_survives_a_poisoned_registry() {
        // C3: a panic under the lock poisons the mutex; `lock()` must still hand back a usable
        // guard instead of propagating the poison and bricking every later request.
        let reg = Mutex::new(Registry::default());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _held = reg.lock().unwrap();
            panic!("engine blew up under the lock");
        }));
        assert!(reg.is_poisoned(), "the panic poisoned the mutex");
        assert_eq!(
            lock(&reg).active_table_count(),
            0,
            "lock() recovers the guard anyway"
        );
    }

    #[test]
    fn active_table_count_counts_every_seeded_table() {
        let mut registry = Registry::default();
        assert_eq!(
            registry.active_table_count(),
            0,
            "a fresh registry is empty"
        );

        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        assert!(registry.try_insert("live".to_string(), table));
        assert_eq!(
            registry.active_table_count(),
            1,
            "a seeded table counts as active"
        );
    }

    /// Drain waits on `active_table_count() == 0`. A seeded game with no stream subscribers is
    /// "seats vacated" (DEPLOYMENT.md) — it must not block SIGTERM forever the way production
    /// Terminating pods did (ghost `active_tables` long after players left).
    #[test]
    fn abandoned_table_with_no_stream_subscribers_is_evicted_for_drain() {
        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        // Quiet since long before grace — stands in for a table abandoned hours ago.
        table.quiet_since = Some(Instant::now() - Duration::from_secs(120));
        assert!(registry.try_insert("ghost".to_string(), table));
        assert_eq!(registry.active_table_count(), 1);

        let removed = registry.evict_abandoned(Instant::now(), Duration::from_secs(60));
        assert_eq!(removed, 1, "no-listener table past grace is abandoned");
        assert_eq!(
            registry.active_table_count(),
            0,
            "drain can reach zero after eviction"
        );
    }

    #[test]
    fn table_with_a_live_stream_subscriber_survives_drain_eviction() {
        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        table.quiet_since = Some(Instant::now() - Duration::from_secs(120));
        let _rx = table.tx.subscribe();
        assert!(registry.try_insert("watched".to_string(), table));

        let removed = registry.evict_abandoned(Instant::now(), Duration::from_secs(60));
        assert_eq!(removed, 0, "a live Game.Stream keeps the table for drain");
        assert_eq!(registry.active_table_count(), 1);
    }

    #[test]
    fn abandoned_table_inside_reconnect_grace_is_kept() {
        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        table.quiet_since = Some(Instant::now() - Duration::from_secs(30));
        assert!(registry.try_insert("blip".to_string(), table));

        let removed = registry.evict_abandoned(Instant::now(), Duration::from_secs(60));
        assert_eq!(removed, 0, "brief disconnects stay within grace");
        assert_eq!(registry.active_table_count(), 1);
    }

    /// Bugbot: a long-lived game whose streams drop before the first drain sweep still has
    /// seed-era `quiet_since` unless subscribe cleared it — the first quiet sweep must *arm*
    /// grace from `now`, not instant-evict off the seed timestamp.
    #[test]
    fn previously_watched_table_gets_grace_from_first_quiet_sweep() {
        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        // Subscribe cleared the seed quiet mark; streams then dropped with no further sweep.
        table.quiet_since = None;
        assert!(registry.try_insert("played".to_string(), table));

        let grace = Duration::from_secs(60);
        let t0 = Instant::now();
        assert_eq!(
            registry.evict_abandoned(t0, grace),
            0,
            "first quiet sweep arms grace instead of evicting"
        );
        assert_eq!(registry.active_table_count(), 1);
        assert_eq!(
            registry.evict_abandoned(t0 + Duration::from_secs(30), grace),
            0,
            "still inside grace"
        );
        assert_eq!(
            registry.evict_abandoned(t0 + grace, grace),
            1,
            "evicted once grace elapses from the arming sweep"
        );
        assert_eq!(registry.active_table_count(), 0);
    }
}
