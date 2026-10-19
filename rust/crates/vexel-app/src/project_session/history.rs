//! Bounded chronological history for combined scene and project-payload state.
//!
//! Admission guards perform every fallible operation before callers publish a
//! scene or payload mutation. A payload admission stages at most 1025 state
//! descriptors alongside the live journal (at most 1024 descriptors).

use vexel_project::MAX_BYTES;

pub(super) const MAX_STATES: usize = 1024;

/// Immutable payload snapshots report a stable identity shared by cheap
/// clones, plus the exact owned bytes retained by that snapshot. Cloning must
/// be cheap and allocation-free; all admission clones precede publication.
pub(super) trait PayloadSnapshot: Clone {
    fn identity(&self) -> usize;
    fn retained_bytes(&self) -> usize;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum JournalError {
    Capacity,
    IdentityExhausted,
    SnapshotIdentityConflict,
    #[cfg(test)]
    InvalidReachableWindow,
    ConcurrentChange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Transition {
    Initial,
    Scene,
    Payload,
}

#[derive(Clone, Debug)]
pub(super) struct JournalState<P> {
    app_state_id: u64,
    scene_state_id: u64,
    payload: P,
    transition: Transition,
}

impl<P> JournalState<P> {
    pub(super) fn app_state_id(&self) -> u64 {
        self.app_state_id
    }

    pub(super) fn scene_state_id(&self) -> u64 {
        self.scene_state_id
    }

    pub(super) fn payload(&self) -> &P {
        &self.payload
    }
}

/// Session-only combined state history. State identities are monotonically
/// allocated and are not reused when redo is discarded or old states evicted.
#[derive(Clone, Debug)]
pub(super) struct Journal<P> {
    states: Vec<JournalState<P>>,
    cursor: usize,
    next_app_state_id: u64,
    checkpoint_id: u64,
}

impl<P: PayloadSnapshot> Journal<P> {
    pub(super) fn try_new(scene_state_id: u64, payload: P) -> Result<Self, JournalError> {
        if payload.retained_bytes() > MAX_BYTES {
            return Err(JournalError::Capacity);
        }
        let mut states = Vec::new();
        reserve_states(&mut states, 1)?;
        states.push(JournalState {
            app_state_id: 1,
            scene_state_id,
            payload,
            transition: Transition::Initial,
        });
        Ok(Self {
            states,
            cursor: 0,
            next_app_state_id: 2,
            checkpoint_id: 1,
        })
    }

    pub(super) fn current(&self) -> &JournalState<P> {
        &self.states[self.cursor]
    }

    #[cfg(test)]
    pub(super) fn state_count(&self) -> usize {
        self.states.len()
    }

    #[cfg(test)]
    pub(super) fn checkpoint_id(&self) -> u64 {
        self.checkpoint_id
    }

    pub(super) fn is_dirty(&self) -> bool {
        self.current().app_state_id != self.checkpoint_id
    }

    /// Marks the current combined state as saved.
    pub(super) fn checkpoint(&mut self) {
        self.checkpoint_id = self.current().app_state_id;
    }

    /// Reserve and validate scene admission before running an editor closure.
    pub(super) fn prepare_scene(&mut self) -> Result<SceneAdmission<'_, P>, JournalError> {
        let id = self.allocate_id()?;
        reserve_states(&mut self.states, 1)?;
        let payload = self.current().payload.clone();
        Ok(SceneAdmission {
            journal: self,
            id,
            payload,
        })
    }

    /// Stage a branch-truncated payload transition before publishing its
    /// payload in the owning session.
    pub(super) fn prepare_payload(
        &mut self,
        payload: P,
    ) -> Result<PayloadAdmission<'_, P>, JournalError> {
        let id = self.allocate_id()?;
        let retained = &self.states[..=self.cursor];
        let candidate_count = retained
            .len()
            .checked_add(1)
            .ok_or(JournalError::Capacity)?;
        let mut candidate = Vec::new();
        reserve_states(&mut candidate, candidate_count)?;
        candidate.extend_from_slice(retained);
        candidate.push(JournalState {
            app_state_id: id,
            scene_state_id: self.current().scene_state_id,
            payload,
            transition: Transition::Payload,
        });
        if candidate.len() > MAX_STATES {
            candidate.drain(..candidate.len() - MAX_STATES);
        }
        payload_bytes(&candidate)?;
        Ok(PayloadAdmission {
            journal: self,
            candidate: Some(candidate),
            id,
        })
    }

    /// Return the previous journal state only when its scene remains
    /// navigable by the bridge. The caller must apply it before `commit_undo`.
    pub(super) fn undo_target(&self, reachable_scene_ids: &[u64]) -> Option<&JournalState<P>> {
        let target = self.states.get(self.cursor.checked_sub(1)?)?;
        reachable_scene_ids
            .contains(&target.scene_state_id)
            .then_some(target)
    }

    /// Return the next journal state only when its scene remains navigable by
    /// the bridge. The caller must apply it before `commit_redo`.
    pub(super) fn redo_target(&self, reachable_scene_ids: &[u64]) -> Option<&JournalState<P>> {
        let target = self.states.get(self.cursor.checked_add(1)?)?;
        reachable_scene_ids
            .contains(&target.scene_state_id)
            .then_some(target)
    }

    pub(super) fn commit_undo(&mut self, expected_app_state_id: u64) -> Result<(), JournalError> {
        let Some(target_index) = self.cursor.checked_sub(1) else {
            return Err(JournalError::ConcurrentChange);
        };
        if self.states[target_index].app_state_id != expected_app_state_id {
            return Err(JournalError::ConcurrentChange);
        }
        self.cursor = target_index;
        Ok(())
    }

    pub(super) fn commit_redo(&mut self, expected_app_state_id: u64) -> Result<(), JournalError> {
        let target_index = self.cursor + 1;
        if self
            .states
            .get(target_index)
            .map(|state| state.app_state_id)
            != Some(expected_app_state_id)
        {
            return Err(JournalError::ConcurrentChange);
        }
        self.cursor = target_index;
        Ok(())
    }

    /// Removes a prefix whose scene states have been evicted by the bridge.
    #[cfg(test)]
    pub(super) fn reconcile_reachable(
        &mut self,
        reachable_scene_ids: &[u64],
    ) -> Result<usize, JournalError> {
        if !reachable_scene_ids.contains(&self.current().scene_state_id) {
            return Err(JournalError::InvalidReachableWindow);
        }
        let prefix_len = self
            .states
            .iter()
            .take(self.cursor)
            .take_while(|state| !reachable_scene_ids.contains(&state.scene_state_id))
            .count();
        if prefix_len == 0 {
            return Ok(0);
        }
        self.states.drain(..prefix_len);
        self.cursor -= prefix_len;
        Ok(prefix_len)
    }

    fn allocate_id(&self) -> Result<u64, JournalError> {
        if self.next_app_state_id == u64::MAX {
            return Err(JournalError::IdentityExhausted);
        }
        Ok(self.next_app_state_id)
    }
}

/// A scene edit may run only while this exclusive admission guard is alive.
pub(super) struct SceneAdmission<'a, P> {
    journal: &'a mut Journal<P>,
    id: u64,
    payload: P,
}

impl<P: PayloadSnapshot> SceneAdmission<'_, P> {
    /// Finalize the observed document state without allocating. `None` means
    /// no journal transition was committed (a no-op or unreachable state).
    pub(super) fn finish(
        self,
        final_scene_id: u64,
        mut reachable: impl FnMut(u64) -> bool,
    ) -> Option<u64> {
        if final_scene_id == self.journal.current().scene_state_id || !reachable(final_scene_id) {
            return None;
        }
        let journal = self.journal;
        let coalesce = journal.current().transition == Transition::Scene
            && !reachable(journal.current().scene_state_id);
        journal.states.truncate(journal.cursor + 1);
        if coalesce {
            let current = &mut journal.states[journal.cursor];
            current.app_state_id = self.id;
            current.scene_state_id = final_scene_id;
        } else {
            journal.states.push(JournalState {
                app_state_id: self.id,
                scene_state_id: final_scene_id,
                payload: self.payload,
                transition: Transition::Scene,
            });
            journal.cursor += 1;
        }
        journal.next_app_state_id = self.id + 1;
        if journal.states.len() > MAX_STATES {
            journal.states.drain(..journal.states.len() - MAX_STATES);
        }
        journal
            .states
            .retain(|state| reachable(state.scene_state_id));
        journal.cursor = journal.states.len() - 1;
        Some(self.id)
    }
}

/// A prevalidated payload transition; dropping it leaves logical journal
/// state unchanged even if its temporary descriptor allocation succeeded.
pub(super) struct PayloadAdmission<'a, P> {
    journal: &'a mut Journal<P>,
    candidate: Option<Vec<JournalState<P>>>,
    id: u64,
}

impl<P> PayloadAdmission<'_, P> {
    /// Publish the staged state without allocating or returning an error.
    pub(super) fn commit(mut self) -> u64 {
        let candidate = self.candidate.take().expect("admission owns staged states");
        self.journal.states = candidate;
        self.journal.cursor = self.journal.states.len() - 1;
        self.journal.next_app_state_id = self.id + 1;
        self.id
    }
}

/// Bounded pairwise identity accounting avoids a fallible or infallible map
/// allocation while detecting inconsistent identity metadata.
fn payload_bytes<P: PayloadSnapshot>(states: &[JournalState<P>]) -> Result<usize, JournalError> {
    let mut total = 0usize;
    for (index, state) in states.iter().enumerate() {
        let identity = state.payload.identity();
        let bytes = state.payload.retained_bytes();
        let mut seen = false;
        for previous in &states[..index] {
            if previous.payload.identity() == identity {
                if previous.payload.retained_bytes() != bytes {
                    return Err(JournalError::SnapshotIdentityConflict);
                }
                seen = true;
            }
        }
        if !seen {
            total = total.checked_add(bytes).ok_or(JournalError::Capacity)?;
            if total > MAX_BYTES {
                return Err(JournalError::Capacity);
            }
        }
    }
    Ok(total)
}

// Every descriptor reservation passes through this seam so tests can reject a
// request deterministically without relying on allocator exhaustion.
fn reserve_states<P>(
    states: &mut Vec<JournalState<P>>,
    additional: usize,
) -> Result<(), JournalError> {
    #[cfg(test)]
    if FAIL_NEXT_RESERVATION.with(|fail| fail.replace(false)) {
        return Err(JournalError::Capacity);
    }
    states
        .try_reserve_exact(additional)
        .map_err(|_| JournalError::Capacity)
}

#[cfg(test)]
std::thread_local! {
    static FAIL_NEXT_RESERVATION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn fail_next_reservation_for_test() {
    FAIL_NEXT_RESERVATION.with(|fail| fail.set(true));
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
