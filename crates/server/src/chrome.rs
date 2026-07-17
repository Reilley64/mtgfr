//! Live-table priority chrome (ADR 0007 / 0026 / 0027 / 0029): yields, stack hold, and dwell.
//!
//! Owned by [`crate::decks::Table`] as `chrome`; mutate only via [`crate::session::TableSession`]
//! (or the `pub(crate)` accessors below used by the hold timer). gRPC adapters never poke chrome.

/// Live-table priority chrome knobs. Fields are private — see module docs.
#[derive(Debug, Default)]
pub struct ChromeState {
    /// Per-seat "don't care" yields: a yielded seat is auto-passed while the stack is
    /// non-empty. Cleared whenever the stack empties.
    yields: [bool; 4],
    /// Per-seat turn yield (ADR 0029): auto-pass until that seat's turn / until they act.
    turn_yields: [bool; 4],
    /// Active stack-hold (uncontested resolve pause): seq + when the hold started
    /// (`tokio::time::Instant` so hold timers honor the test paused clock).
    stack_hold: Option<(u64, tokio::time::Instant)>,
    /// Per-seat helpless stack dwell (hover pause). Cleared when the hold ends.
    stack_dwell: [bool; 4],
}

impl ChromeState {
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
