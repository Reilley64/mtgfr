//! Live-table priority chrome (turn-priority-and-stack spec / turn-priority-and-stack spec / 0029): yields, stack hold, and dwell.
//!
//! Owned by [`crate::Table`] as `chrome`; mutate only via [`crate::session::TableSession`]
//! (or the `pub(crate)` accessors below used by the hold timer). gRPC adapters never poke chrome.

/// Debug-only logical chrome persisted by a checkpoint. Timer and dwell instants are excluded.
#[cfg(debug_assertions)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DebugChromeSnapshot {
    pub yields: [bool; 4],
    pub turn_yields: [bool; 4],
    pub hold_requested: bool,
}

/// Live-table priority chrome knobs. Fields are private — see module docs.
#[derive(Debug, Default)]
pub struct ChromeState {
    /// Per-seat "don't care" yields: a yielded seat is auto-passed while the stack is
    /// non-empty. Cleared whenever the stack empties.
    yields: [bool; 4],
    /// Per-seat turn yield (turn-priority-and-stack spec): auto-pass until that seat's turn / until they act,
    /// or End Turn while they are active.
    turn_yields: [bool; 4],
    /// Active stack-hold (uncontested resolve pause): seq + when the hold started
    /// (`tokio::time::Instant` so hold timers honor the test paused clock).
    stack_hold: Option<(u64, tokio::time::Instant)>,
    /// Per-seat helpless stack dwell (hover pause). Cleared when the hold ends.
    stack_dwell: [bool; 4],
}

impl ChromeState {
    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub(crate) fn debug_snapshot(&self) -> DebugChromeSnapshot {
        DebugChromeSnapshot {
            yields: self.yields,
            turn_yields: self.turn_yields,
            hold_requested: self.stack_hold.is_some(),
        }
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub(crate) fn restore_debug_snapshot(&mut self, snapshot: DebugChromeSnapshot) {
        self.yields = snapshot.yields;
        self.turn_yields = snapshot.turn_yields;
        self.clear_hold();
    }

    pub fn yields(&self) -> &[bool; 4] {
        &self.yields
    }

    pub fn turn_yields(&self) -> &[bool; 4] {
        &self.turn_yields
    }

    pub fn stack_hold(&self) -> Option<(u64, tokio::time::Instant)> {
        self.stack_hold
    }

    pub fn any_dwell(&self) -> bool {
        self.stack_dwell.iter().any(|&d| d)
    }

    pub(crate) fn arm_yield(&mut self, seat: usize) {
        self.yields[seat] = true;
    }

    pub(crate) fn set_turn_yield_flag(&mut self, seat: usize, enabled: bool) {
        self.turn_yields[seat] = enabled;
    }

    /// Stack-yield + turn-yield flags for [`crate::session`] auto-advance (one borrow).
    pub(crate) fn skip_flags_mut(&mut self) -> (&mut [bool; 4], &mut [bool; 4]) {
        (&mut self.yields, &mut self.turn_yields)
    }

    pub(crate) fn begin_hold(&mut self, seq: u64, now: tokio::time::Instant) {
        self.stack_hold = Some((seq, now));
        self.stack_dwell = [false; 4];
    }

    pub(crate) fn clear_hold(&mut self) {
        self.stack_hold = None;
        self.stack_dwell = [false; 4];
    }

    /// Clear all transient priority chrome after an authoritative debug candidate passes every
    /// precommit gate. Detached hold timers become stale when the table sequence advances.
    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    pub(crate) fn clear_for_debug_commit(&mut self) {
        self.yields = [false; 4];
        self.turn_yields = [false; 4];
        self.clear_hold();
    }

    /// Clear hold only when it still matches `seq` (stale timer eviction).
    pub(crate) fn clear_hold_if_seq(&mut self, seq: u64) {
        if self.stack_hold.is_some_and(|(s, _)| s == seq) {
            self.clear_hold();
        }
    }

    pub(crate) fn set_dwell_flag(&mut self, seat: usize, dwelling: bool) {
        self.stack_dwell[seat] = dwelling;
    }

    /// Test / session fixtures that need to reset stack-yield without going through verbs.
    #[cfg(test)]
    pub(crate) fn set_yields_for_test(&mut self, yields: [bool; 4]) {
        self.yields = yields;
    }

    /// Test fixture: stamp an active hold as the scheduler would.
    #[cfg(test)]
    pub(crate) fn stamp_hold_for_test(&mut self, seq: u64, now: tokio::time::Instant) {
        self.begin_hold(seq, now);
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn debug_snapshot_preserves_logical_flags_but_restore_drops_timer_and_dwell() {
        let now = tokio::time::Instant::now();
        let mut live = ChromeState::default();
        live.arm_yield(0);
        live.set_turn_yield_flag(1, true);
        live.begin_hold(7, now);
        live.set_dwell_flag(2, true);

        let snapshot = live.debug_snapshot();
        assert_eq!(snapshot.yields, [true, false, false, false]);
        assert_eq!(snapshot.turn_yields, [false, true, false, false]);
        assert!(snapshot.hold_requested);

        let mut restored = ChromeState::default();
        restored.begin_hold(99, now);
        restored.set_dwell_flag(3, true);
        restored.restore_debug_snapshot(snapshot);

        assert_eq!(*restored.yields(), snapshot.yields);
        assert_eq!(*restored.turn_yields(), snapshot.turn_yields);
        assert!(restored.stack_hold().is_none());
        assert!(!restored.any_dwell());
    }
}
