//! Retargets native timeline tracks from one scene id to another, ahead of
//! the document edit that renames the element.

use super::native_commands::{NativeCommandError, copy_assets};
use super::timeline_edit::{ProjectTimelineCandidate, ProjectTimelineError};
use super::{ProjectSession, SessionError};
use vexel_timeline::{EditLimits, EmptyTimelineSettings};

impl ProjectSession {
    /// Points every track that targets `old` at `new` as one journal
    /// transition. The new bytes are validated against the scene ids with
    /// `old` replaced by `new`, so call this before renaming the element.
    /// Returns `false` when no track targets `old`.
    pub fn retarget_timeline(
        &mut self,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
        old: &[u8],
        new: &[u8],
    ) -> Result<bool, NativeCommandError<()>> {
        let candidate = self.timeline_candidate(settings, limits)?;
        if !candidate.is_editable() {
            return Ok(false);
        }
        let mut starts = Vec::new();
        {
            let view = candidate.view().map_err(NativeCommandError::Timeline)?;
            let base = view.source().as_ptr() as usize;
            for index in 0..view.track_count() {
                if let Some(track) = view.track_at(index).filter(|track| track.target == old) {
                    starts.push(track.target.as_ptr() as usize - base);
                }
            }
        }
        if starts.is_empty() {
            return Ok(false);
        }
        let bytes = candidate.timeline_bytes();
        let mut out = Vec::new();
        out.try_reserve(bytes.len() + starts.len() * new.len())
            .map_err(|_| NativeCommandError::Timeline(ProjectTimelineError::OutOfMemory))?;
        let mut cursor = 0;
        for start in starts {
            out.extend_from_slice(&bytes[cursor..start]);
            out.extend_from_slice(new);
            cursor = start + old.len();
        }
        out.extend_from_slice(&bytes[cursor..]);
        let mut scene_ids = self.drawing_scene_ids()?;
        for id in &mut scene_ids {
            if id == old {
                *id = new.to_vec();
            }
        }
        let assets = copy_assets(candidate.assets()).map_err(NativeCommandError::Timeline)?;
        let edited = ProjectTimelineCandidate::open(&out, scene_ids, assets, settings, limits)
            .map_err(NativeCommandError::Timeline)?;
        let replacement = self
            .payload
            .with_timeline_candidate(&edited)
            .map_err(NativeCommandError::Session)?;
        let admission = self
            .journal
            .prepare_payload(replacement.clone())
            .map_err(|_| NativeCommandError::Session(SessionError::HistoryUnavailable))?;
        self.document.history_boundary();
        self.payload = replacement;
        admission.commit();
        Ok(true)
    }
}
