//! AppDocument access to native animation data and journal commands.

use crate::project_session::{NativeCommandError, ProjectTimelineCandidate};
use crate::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings};

impl AppDocument {
    /// Returns the current bounded timeline and asset snapshot without
    /// changing the journal or dirty state.
    pub fn native_timeline_candidate(
        &self,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
    ) -> Result<ProjectTimelineCandidate, NativeCommandError<()>> {
        self.session.timeline_candidate(settings, limits)
    }

    /// Edits native timeline data as one combined project-journal transition.
    /// Payload-only edits advance `state_revision()` while leaving the
    /// source-only `revision()` unchanged.
    pub fn edit_timeline<E>(
        &mut self,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
        edit: impl FnOnce(&ProjectTimelineCandidate) -> Result<ProjectTimelineCandidate, E>,
    ) -> Result<bool, NativeCommandError<E>> {
        let changed = self.session.edit_timeline(settings, limits, edit)?;
        if changed {
            self.state_revision += 1;
        }
        Ok(changed)
    }
}
