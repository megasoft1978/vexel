//! Window-free save/open session for the editable `.vexel` project.
//!
//! The bundle stores the exact `V1Document::source()` bytes as its scene.
//! Undo/redo history is session state and is NOT persisted: the v1 bundle has
//! no history section, so a reopened document starts clean with empty history.

mod actions;
mod edit_error;
mod history;
mod journal_integration;
#[cfg(test)]
mod journal_integration_tests;
mod native_commands;
mod payload;
mod recovery;
mod recovery_snapshot;
mod recovery_store;
#[cfg(test)]
mod recovery_tests;
mod retarget;
mod svg_asset;
#[cfg(test)]
mod svg_asset_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_reopen_native;
mod timeline_edit;
#[cfg(test)]
mod timeline_edit_tests;

pub use actions::{FileAction, shortcut_action, unsaved_changes_prompt};
pub use edit_error::SessionEditError;
pub use native_commands::{MAX_SCENE_ID_BYTES, MAX_SCENE_IDS, NativeCommandError, SceneIdsError};
pub use recovery::RecoveryStatus;
pub use recovery_snapshot::RecoverySnapshot;
pub use svg_asset::{OwnedProjectAsset, SvgAssetError};
pub use timeline_edit::{ProjectTimelineCandidate, ProjectTimelineError};

#[cfg(test)]
pub(crate) fn with_recovery_test_home<R>(home: &std::path::Path, action: impl FnOnce() -> R) -> R {
    recovery_store::with_test_home(home, action)
}

use history::Journal;
use payload::SessionPayload;
use vexel_project::{self as project, Error as BundleError};
use vexel_project_io::{FileError, LoadFailure, project_file_load, project_file_save_as};
use vexel_v1_document::{V1Document, V1DocumentError};

/// Typed, user-presentable failure. No variant carries partial data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionError {
    /// `save` called before the document has a path.
    NoPath,
    /// The path is empty, too long, contains NUL, or is not a regular file.
    InvalidPath,
    /// The file could not be read (missing, permissions, I/O).
    Unreadable,
    /// Not a Vexel project (bad magic).
    NotAProject,
    /// Written by an unsupported format version.
    UnsupportedVersion,
    /// Truncated or checksum/structure damage; offset is of the first fault.
    Corrupt {
        offset: usize,
    },
    /// Exceeds a documented size limit.
    TooLarge,
    /// The bundle is valid but its scene is not an openable V1 SVG.
    InvalidScene(V1DocumentError),
    /// The project could not be written; an existing file is unchanged.
    WriteFailed,
    /// Written, but durability (fsync) could not be confirmed.
    NotDurable,
    /// The save was durable, but its new source identity could not be recorded.
    PublishedStateUnknown,
    OutOfMemory,
    /// A conflict recovery can only be saved under a new path.
    RecoveryConflict,
    /// The source changed while the project was being opened.
    SourceChanged,
    /// The private recovery store could not be read or written safely.
    RecoveryUnavailable,
    /// A recovery capsule was published, but its directory durability is unknown.
    RecoveryNotDurable,
    /// The project was saved, but its recovery capsule could not be removed.
    RecoveryCleanup,
    /// The recovery capsule is invalid or does not match this project.
    RecoveryInvalid,
    /// The combined project history could not admit or navigate a state.
    HistoryUnavailable,
    Document(V1DocumentError),
}

impl SessionError {
    pub fn user_message(self) -> &'static str {
        match self {
            Self::NoPath => "This document has not been saved yet. Choose Save As.",
            Self::InvalidPath => "The selected path is not usable.",
            Self::Unreadable => "The file could not be read.",
            Self::NotAProject => "This file is not a Vexel project.",
            Self::UnsupportedVersion => {
                "This project was saved by an unsupported version of Vexel."
            }
            Self::Corrupt { .. } => "The project file is damaged or incomplete.",
            Self::TooLarge => "The project file exceeds the supported size.",
            Self::InvalidScene(_) => "The project does not contain a supported drawing.",
            Self::WriteFailed => "The project could not be saved; the previous file is unchanged.",
            Self::NotDurable => "The project was saved but could not be confirmed on disk.",
            Self::PublishedStateUnknown => {
                "The file was saved, but Vexel could not confirm its source identity. Reopen it to continue."
            }
            Self::OutOfMemory => "Not enough memory.",
            Self::RecoveryConflict => {
                "This recovery came from a project that has changed. Use Save As to keep both versions."
            }
            Self::SourceChanged => {
                "The project source changed. Reopen it, or use Save As to keep both versions."
            }
            Self::RecoveryUnavailable => "The recovery store is unavailable for this project.",
            Self::RecoveryNotDurable => {
                "The recovery snapshot was written, but its durability could not be confirmed."
            }
            Self::RecoveryCleanup => {
                "The project was saved, but its recovery snapshot could not be removed."
            }
            Self::RecoveryInvalid => {
                "The recovery snapshot is damaged or belongs to another project."
            }
            Self::HistoryUnavailable => "The project history could not be updated.",
            Self::Document(_) => "The document could not be read for saving.",
        }
    }
}

fn load_error(failure: LoadFailure) -> SessionError {
    if let Some(bundle) = failure.bundle_failure {
        return match bundle.error {
            BundleError::BadMagic => SessionError::NotAProject,
            BundleError::UnsupportedVersion => SessionError::UnsupportedVersion,
            BundleError::Limit => SessionError::TooLarge,
            BundleError::OutOfMemory => SessionError::OutOfMemory,
            _ => SessionError::Corrupt {
                offset: bundle.error_offset,
            },
        };
    }
    match failure.file_error {
        FileError::PathLimit | FileError::InvalidArgument => SessionError::InvalidPath,
        FileError::OutOfMemory => SessionError::OutOfMemory,
        FileError::Limit => SessionError::TooLarge,
        _ => SessionError::Unreadable,
    }
}

/// Writes `source` as a project through `vexel-project-io` (atomic publish).
/// Shared by `ProjectSession` and the MCP preview path.
pub fn write_project(
    path: &[u8],
    source: &[u8],
    width: u32,
    height: u32,
) -> Result<(), SessionError> {
    write_project_with_payload(path, source, &SessionPayload::defaults(width, height))
}

fn write_project_with_payload(
    path: &[u8],
    source: &[u8],
    payload: &SessionPayload,
) -> Result<(), SessionError> {
    let result = payload.with_bundle_input(source, |input| project_file_save_as(path, input));
    match result.error {
        FileError::Ok => Ok(()),
        FileError::Durability => Err(SessionError::NotDurable),
        FileError::OutOfMemory => Err(SessionError::OutOfMemory),
        FileError::Limit => Err(SessionError::TooLarge),
        FileError::PathLimit => Err(SessionError::InvalidPath),
        FileError::InvalidArgument if result.bundle_error == project::Error::Ok => {
            Err(SessionError::InvalidPath)
        }
        _ => Err(SessionError::WriteFailed),
    }
}

/// One open document plus its file association.
/// Token for an open edit group (see `ProjectSession::begin_group`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditGroup(history::GroupToken);

#[derive(Debug)]
pub struct ProjectSession {
    document: V1Document,
    payload: SessionPayload,
    journal: Journal<SessionPayload>,
    path: Option<Vec<u8>>,
    /// Path of the `.svg` this untitled document was imported from, if any.
    /// Display only: Save never writes to it.
    svg_origin: Option<Vec<u8>>,
    /// Recovery snapshots are unsaved work even though the parsed SVG starts clean.
    recovery_dirty: bool,
    /// A changed source must not be overwritten by normal Save.
    recovery_conflict: bool,
    /// Metadata and identity captured around the native project load.
    baseline: Option<recovery::RecoveryBaseline>,
    /// Scene ids of the source state named by the first member (the scene
    /// history state id), so timeline edits do not re-serialize and re-tokenize.
    scene_ids: std::cell::RefCell<Option<(u64, Vec<Vec<u8>>)>>,
}

impl ProjectSession {
    pub fn new(document: V1Document) -> Result<Self, SessionError> {
        let payload = SessionPayload::defaults(document.width(), document.height());
        let journal = journal_integration::new_journal(&document, payload.clone())?;
        Ok(Self {
            document,
            payload,
            journal,
            path: None,
            svg_origin: None,
            recovery_dirty: false,
            recovery_conflict: false,
            baseline: None,
            scene_ids: std::cell::RefCell::new(None),
        })
    }
    pub fn document(&self) -> &V1Document {
        &self.document
    }

    pub(crate) fn history_state_id(&self) -> u64 {
        self.journal.current().app_state_id()
    }
    pub fn path(&self) -> Option<&[u8]> {
        self.path.as_deref()
    }
    /// The imported `.svg` path for an untitled document opened from one.
    pub fn svg_origin(&self) -> Option<&[u8]> {
        self.svg_origin.as_deref()
    }
    pub fn is_dirty(&self) -> bool {
        self.document.is_dirty() || self.journal.is_dirty() || self.recovery_dirty
    }

    /// Runs one scene edit after the journal has admitted its possible state.
    /// The final observed editor state is recorded even when the operation
    /// returns an error after making a partial change.
    pub fn edit<R, E>(
        &mut self,
        edit: impl FnOnce(&mut V1Document) -> Result<R, E>,
    ) -> Result<R, SessionEditError<E>> {
        journal_integration::edit(self, edit)
    }

    /// Starts an atomic edit group; undo/redo are disabled until it closes.
    /// Nested groups are rejected with `HistoryUnavailable`.
    pub fn begin_group(&mut self) -> Result<EditGroup, SessionError> {
        journal_integration::begin_group(self).map(EditGroup)
    }

    /// Collapses the group's edits into one undo step.
    pub fn end_group(&mut self, group: EditGroup) -> Result<(), SessionError> {
        journal_integration::end_group(self, group.0)
    }

    /// Restores the scene and payload from before the group and drops its edits.
    pub fn rollback_group(&mut self, group: EditGroup) -> Result<(), SessionError> {
        journal_integration::rollback_group(self, group.0)
    }

    pub fn undo(&mut self) -> Result<bool, SessionError> {
        journal_integration::undo(self)
    }

    pub fn redo(&mut self) -> Result<bool, SessionError> {
        journal_integration::redo(self)
    }

    pub fn can_undo(&self) -> bool {
        journal_integration::can_undo(self)
    }

    pub fn can_redo(&self) -> bool {
        journal_integration::can_redo(self)
    }

    /// Saves to the current path. The document is marked saved only on success.
    pub fn save(&mut self) -> Result<(), SessionError> {
        if self.recovery_conflict {
            return Err(SessionError::RecoveryConflict);
        }
        let path = self.path.clone().ok_or(SessionError::NoPath)?;
        if let Some(baseline) = self.baseline.as_ref() {
            if !recovery::path_is_missing(&path)?
                && !recovery::baseline_matches_current(&path, baseline)?
            {
                return Err(SessionError::SourceChanged);
            }
        }
        self.write(&path)?;
        self.recovery_dirty = false;
        self.recovery_conflict = false;
        recovery::cleanup_recovery_for_path(&path).map_err(|_| SessionError::RecoveryCleanup)?;
        Ok(())
    }

    /// Saves to `path`; the session adopts it only on success.
    pub fn save_as(&mut self, path: &[u8]) -> Result<(), SessionError> {
        if self.recovery_conflict && self.path.as_deref() == Some(path) {
            return Err(SessionError::RecoveryConflict);
        }
        if let (Some(baseline), Ok(target)) =
            (self.baseline.as_ref(), recovery::canonical_path(path))
        {
            if baseline.canonical_path == target {
                if self.recovery_conflict {
                    return Err(SessionError::RecoveryConflict);
                }
                if !recovery::path_is_missing(path)?
                    && !recovery::baseline_matches_current(path, baseline)?
                {
                    return Err(SessionError::SourceChanged);
                }
            }
        }
        let previous_path = self.path.clone();
        let previous_baseline = self.baseline.clone();
        self.write(path)?;
        let cleanup = previous_baseline
            .as_ref()
            .map(recovery::cleanup_recovery_for_baseline)
            .or_else(|| {
                previous_path
                    .as_deref()
                    .map(recovery::cleanup_recovery_for_path)
            })
            .transpose();
        self.path = Some(path.to_vec());
        self.svg_origin = None;
        self.recovery_dirty = false;
        self.recovery_conflict = false;
        cleanup.map_err(|_| SessionError::RecoveryCleanup)?;
        Ok(())
    }

    fn write(&mut self, path: &[u8]) -> Result<(), SessionError> {
        self.write_with_baseline_refresh(path, recovery::refresh_baseline)
    }

    fn write_with_baseline_refresh(
        &mut self,
        path: &[u8],
        refresh: impl FnOnce(&[u8]) -> Result<recovery::RecoveryBaseline, SessionError>,
    ) -> Result<(), SessionError> {
        let source = self.document.source().map_err(SessionError::Document)?;
        write_project_with_payload(path, &source, &self.payload)?;
        self.baseline = Some(refresh(path).map_err(|_| SessionError::PublishedStateUnknown)?);
        self.document.mark_saved();
        self.journal.checkpoint();
        Ok(())
    }

    /// Opens `path` into a fresh clean session with empty history. A `.svg`
    /// extension imports the SVG as an untitled document (Save then asks for a
    /// name and never overwrites the SVG); anything else is a `.vexel` project.
    pub fn open(path: &[u8]) -> Result<Self, SessionError> {
        Self::open_with_payload_reserver(path, || true)
    }

    fn open_with_payload_reserver(
        path: &[u8],
        mut reserve_allowed: impl FnMut() -> bool,
    ) -> Result<Self, SessionError> {
        if path.len() > 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".svg") {
            return Self::open_svg(path);
        }
        let before = recovery::begin_open_baseline(path)?;
        let loaded = project_file_load(path).map_err(load_error)?;
        let baseline = recovery::finish_open_baseline(path, before)?;
        let view = loaded.view();
        let document =
            V1Document::from_source(view.scene_svg).map_err(SessionError::InvalidScene)?;
        let payload = SessionPayload::try_from_view_with(view, &mut reserve_allowed)
            .map_err(|()| SessionError::OutOfMemory)?;
        let journal = journal_integration::new_journal(&document, payload.clone())?;
        Ok(Self {
            document,
            payload,
            journal,
            path: Some(path.to_vec()),
            svg_origin: None,
            recovery_dirty: false,
            recovery_conflict: false,
            baseline: Some(baseline),
            scene_ids: std::cell::RefCell::new(None),
        })
    }

    fn open_svg(path: &[u8]) -> Result<Self, SessionError> {
        use std::os::unix::ffi::OsStrExt;
        let opened =
            crate::compose::open_path(std::path::Path::new(std::ffi::OsStr::from_bytes(path)))
                .map_err(|error| match error {
                    crate::compose::OpenError::InvalidPath => SessionError::InvalidPath,
                    crate::compose::OpenError::TooLarge => SessionError::TooLarge,
                    _ => SessionError::Unreadable,
                })?;
        let document =
            V1Document::from_source(&opened.bytes).map_err(SessionError::InvalidScene)?;
        let payload = SessionPayload::defaults(document.width(), document.height());
        let journal = journal_integration::new_journal(&document, payload.clone())?;
        Ok(Self {
            document,
            payload,
            journal,
            path: None,
            svg_origin: Some(path.to_vec()),
            recovery_dirty: false,
            recovery_conflict: false,
            baseline: None,
            scene_ids: std::cell::RefCell::new(None),
        })
    }

    /// Replaces this session only if `path` opens successfully; on error the
    /// current document, history and path are untouched.
    pub fn open_replacing(&mut self, path: &[u8]) -> Result<(), SessionError> {
        *self = Self::open(path)?;
        Ok(())
    }

    #[cfg(test)]
    fn open_replacing_with_payload_reserver(
        &mut self,
        path: &[u8],
        reserve_allowed: impl FnMut() -> bool,
    ) -> Result<(), SessionError> {
        *self = Self::open_with_payload_reserver(path, reserve_allowed)?;
        Ok(())
    }
}
