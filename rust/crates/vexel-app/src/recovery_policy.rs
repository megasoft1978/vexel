//! Pure scheduling policy for automatic project recovery snapshots.
//!
//! The caller supplies monotonic time and reports edits and scheduled
//! attempts. This module does not own a clock, worker, or document state.

use std::time::{Duration, Instant};

const INITIAL_DEBOUNCE: Duration = Duration::from_secs(2);
const RETRY_INTERVAL: Duration = Duration::from_secs(15);

/// Tracks whether a dirty session is eligible for its next recovery snapshot.
#[derive(Clone, Debug, Default)]
pub struct RecoveryPolicy {
    dirty_since: Option<Instant>,
    last_scheduled: Option<Instant>,
}

impl RecoveryPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks the beginning of a dirty period. Further edits do not postpone
    /// the first snapshot, so continuous editing still receives coverage.
    pub fn mark_dirty(&mut self, now: Instant) {
        if self.dirty_since.is_none() {
            self.dirty_since = Some(now);
        }
    }

    /// Clears the schedule after the session becomes clean or ineligible.
    pub fn mark_clean(&mut self) {
        self.dirty_since = None;
        self.last_scheduled = None;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }

    /// Returns true once the initial debounce or subsequent retry interval
    /// has elapsed. `now` must be from the same monotonic clock as prior calls.
    pub fn is_due(&self, now: Instant) -> bool {
        let Some(dirty_since) = self.dirty_since else {
            return false;
        };
        let due_at = self
            .last_scheduled
            .map(|last| last + RETRY_INTERVAL)
            .unwrap_or(dirty_since + INITIAL_DEBOUNCE);
        now >= due_at
    }

    /// Records that a snapshot attempt was scheduled. Failed preparation or
    /// publication is retried no sooner than the same 15-second interval.
    pub fn record_scheduled(&mut self, now: Instant) {
        if self.dirty_since.is_some() {
            self.last_scheduled = Some(now);
        }
    }

    /// Replaces a pending snapshot with the newest one, preserving a single
    /// bounded pending slot. Returns whether an older pending value existed.
    pub fn coalesce_pending<T>(pending: &mut Option<T>, newer: T) -> bool {
        let replaced = pending.is_some();
        *pending = Some(newer);
        replaced
    }
}

#[cfg(test)]
#[path = "recovery_policy_tests.rs"]
mod tests;
