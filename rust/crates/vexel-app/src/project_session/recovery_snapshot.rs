//! Immutable recovery captures prepared by the owner and serialized by a worker.

use super::{SessionError, payload::SessionPayload, recovery::RecoveryBaseline};
use vexel_v1_document::EditSnapshot;

/// An owned, immutable view of a dirty native session.
///
/// The scene snapshot shares immutable source owners with the editor and the
/// payload clone shares its immutable project data. No live session reference
/// crosses the worker boundary.
pub struct RecoverySnapshot {
    pub(super) baseline: RecoveryBaseline,
    pub(super) scene: EditSnapshot,
    pub(super) payload: SessionPayload,
    pub(super) captured_time_ns: u64,
    pub(super) epoch: u64,
    pub(super) revision: u64,
}

impl RecoverySnapshot {
    /// Serializes and atomically publishes this snapshot to the private store.
    /// Call this only on the recovery worker.
    pub(crate) fn publish(self) -> crate::recovery_job::RecoveryJobResult {
        let epoch = self.epoch;
        let revision = self.revision;
        let mut published = false;
        let result = super::recovery::publish_recovery_snapshot(&self, &mut published);
        crate::recovery_job::RecoveryJobResult {
            epoch,
            revision,
            published,
            durability_uncertain: result == Err(SessionError::RecoveryNotDurable),
            error: result.err(),
        }
    }
}

#[cfg(test)]
#[path = "recovery_snapshot_tests.rs"]
mod tests;
