//! Shared counter-move helpers for target and combat answers.

use crate::*;

impl Game {
    pub(crate) fn remove_all_counters_events(&self, object: ObjectId) -> (Vec<Event>, i32) {
        let mut events = Vec::new();
        let mut removed = 0;
        let plus = self.permanent(object).plus_counters;
        if plus > 0 {
            events.push(Event::CountersPlaced {
                object,
                count: -plus,
                source_name: self.def_of(object).name,
            });
            removed += plus;
        }
        for &kind in CounterKind::ALL.iter() {
            let count = self.permanent(object).kind_counters[kind as usize] as i32;
            if count > 0 {
                events.push(Event::KindCountersPlaced {
                    object,
                    kind,
                    count: -count,
                });
                removed += count;
            }
        }
        (events, removed)
    }

    /// Move counters from `from` onto `to` ([`Effect::Counters(CountersEffect::MoveCounters)`]): +1/+1 counters always
    /// move, through the same replaceable-placement pipeline the destination's own +1/+1
    /// doublers would apply to any other "put a counter" (CR 614); `all_kinds` also moves every
    /// named kind present, raw (named kinds bypass that pipeline everywhere else in the pool —
    /// see [`Effect::Static(StaticEffect::EntersWithCounters)`]'s doc).
    pub(crate) fn move_counters(
        &mut self,
        from: ObjectId,
        to: ObjectId,
        all_kinds: bool,
        events: &mut Vec<Event>,
    ) {
        let plus = self.permanent(from).plus_counters;
        if plus > 0 {
            self.push_apply(
                events,
                Event::CountersPlaced {
                    object: from,
                    count: -plus,
                    source_name: self.def_of(from).name,
                },
            );
            let n = self.counters_after_replacements(to, plus);
            if n > 0 {
                self.push_apply(
                    events,
                    Event::CountersPlaced {
                        object: to,
                        count: n,
                        source_name: self.def_of(from).name,
                    },
                );
            }
        }
        if !all_kinds {
            return;
        }
        for &kind in CounterKind::ALL.iter() {
            let count = self.permanent(from).kind_counters[kind as usize] as i32;
            if count <= 0 {
                continue;
            }
            self.push_apply(
                events,
                Event::KindCountersPlaced {
                    object: from,
                    kind,
                    count: -count,
                },
            );
            self.push_apply(
                events,
                Event::KindCountersPlaced {
                    object: to,
                    kind,
                    count,
                },
            );
        }
    }

    /// Move +1/+1 counters from `from` onto several destinations at once
    /// ([`Effect::Counters(CountersEffect::MoveCounters)`]'s `distributed` mode, CR 601.2d): one combined removal from
    /// `from` for the summed total, then each destination's placement through the same
    /// replaceable-counters pipeline (CR 614) [`Self::move_counters`] uses for its single-
    /// destination case. `assignment` pairs were already validated (distinct, legal, ≥1 each,
    /// summing to at most the source's live count) by [`Self::divide_moved_counters`].
    pub(crate) fn move_counters_distributed(
        &mut self,
        from: ObjectId,
        assignment: &[(ObjectId, i32)],
        events: &mut Vec<Event>,
    ) {
        let total: i32 = assignment.iter().map(|&(_, n)| n).sum();
        if total == 0 {
            return; // "you may move any number" — declining to move any is a legal no-op.
        }
        self.push_apply(
            events,
            Event::CountersPlaced {
                object: from,
                count: -total,
                source_name: self.def_of(from).name,
            },
        );
        for &(to, n) in assignment {
            let n = self.counters_after_replacements(to, n);
            if n <= 0 {
                continue;
            }
            self.push_apply(
                events,
                Event::CountersPlaced {
                    object: to,
                    count: n,
                    source_name: self.def_of(from).name,
                },
            );
        }
    }
}
