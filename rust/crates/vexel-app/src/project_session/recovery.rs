//! Application-session integration for native recovery capsules.

use super::payload::SessionPayload;
use super::recovery_store::{self, StoreError};
use super::{ProjectSession, SessionError};
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use vexel_project_io::{
    RECOVERY_MAX_BYTES, RecoveryDiscovery, RecoveryError, RecoveryInput, project_recovery_decode,
    project_recovery_discover, project_recovery_save,
};
use vexel_v1_document::V1Document;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RecoveryBaseline {
    pub(super) canonical_path: Vec<u8>,
    pub(super) source_size: u64,
    pub(super) modified_seconds: i64,
    pub(super) modified_nanoseconds: u32,
    pub(super) device: u64,
    pub(super) inode: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryStatus {
    Absent,
    Matching,
    Conflict,
}

/// Captures the canonical identity and source stamp immediately before loading.
pub(super) fn begin_open_baseline(path: &[u8]) -> Result<RecoveryBaseline, SessionError> {
    read_baseline(path)
}

/// Confirms the source remained identical while its project bundle was loaded.
pub(super) fn finish_open_baseline(
    path: &[u8],
    before: RecoveryBaseline,
) -> Result<RecoveryBaseline, SessionError> {
    let after = read_baseline(path)?;
    if before != after {
        return Err(SessionError::SourceChanged);
    }
    Ok(before)
}

/// Reads a fresh baseline after a successful save/publication.
pub(super) fn refresh_baseline(path: &[u8]) -> Result<RecoveryBaseline, SessionError> {
    read_baseline(path)
}

/// Returns whether the current file identity and full metadata stamp still
/// match the baseline captured when the project was opened.
pub(super) fn baseline_matches_current(
    path: &[u8],
    baseline: &RecoveryBaseline,
) -> Result<bool, SessionError> {
    let current = read_baseline(path)?;
    Ok(&current == baseline)
}

pub(super) fn canonical_path(path: &[u8]) -> Result<Vec<u8>, SessionError> {
    canonical_source(path)
}

pub(super) fn path_is_missing(path: &[u8]) -> Result<bool, SessionError> {
    if path.is_empty() || path.contains(&0) {
        return Err(SessionError::InvalidPath);
    }
    match fs::symlink_metadata(Path::new(OsStr::from_bytes(path))) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(_) => Err(SessionError::Unreadable),
    }
}

fn read_baseline(path: &[u8]) -> Result<RecoveryBaseline, SessionError> {
    if path.is_empty() || path.contains(&0) {
        return Err(SessionError::InvalidPath);
    }
    let requested_path = Path::new(OsStr::from_bytes(path));
    let requested_metadata = fs::metadata(requested_path).map_err(|_| SessionError::Unreadable)?;
    if !requested_metadata.is_file() {
        return Err(SessionError::InvalidPath);
    }
    let canonical_path = canonical_source(path)?;
    let metadata = fs::metadata(Path::new(OsStr::from_bytes(&canonical_path)))
        .map_err(|_| SessionError::Unreadable)?;
    if !metadata.is_file() {
        return Err(SessionError::InvalidPath);
    }
    Ok(RecoveryBaseline {
        canonical_path,
        source_size: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec() as u32,
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

/// Materializes and publishes an immutable capture. This is called from the
/// recovery worker; owner-thread preparation never reaches this path.
pub(super) fn publish_recovery_snapshot(
    snapshot: &super::recovery_snapshot::RecoverySnapshot,
    published: &mut bool,
) -> Result<(), SessionError> {
    let canonical = &snapshot.baseline.canonical_path;
    let capsule = capsule_path_for_canonical(canonical)?;
    validate_existing_capsule(&capsule, canonical)?;

    let mut scene = Vec::new();
    scene
        .try_reserve_exact(snapshot.scene.byte_length())
        .map_err(|_| SessionError::OutOfMemory)?;
    scene.resize(snapshot.scene.byte_length(), 0);
    snapshot
        .scene
        .materialize(&mut scene)
        .map_err(|error| SessionError::Document(vexel_v1_document::V1DocumentError::Edit(error)))?;

    let mut save_result = Ok(());
    let prepared = snapshot
        .payload
        .try_with_bundle_input(&scene, &mut || true, |project| {
            let input = RecoveryInput {
                source_path: canonical,
                source_size: snapshot.baseline.source_size,
                source_modified_seconds: snapshot.baseline.modified_seconds,
                source_modified_nanoseconds: snapshot.baseline.modified_nanoseconds,
                captured_time_nanoseconds: snapshot.captured_time_ns,
                project: *project,
            };
            let result = project_recovery_save(&capsule, &input);
            *published = result.published;
            save_result = match result.error {
                RecoveryError::Ok => Ok(()),
                RecoveryError::Durability if result.published => {
                    Err(SessionError::RecoveryNotDurable)
                }
                RecoveryError::Durability => Err(SessionError::RecoveryUnavailable),
                RecoveryError::OutOfMemory => Err(SessionError::OutOfMemory),
                RecoveryError::Limit | RecoveryError::PathLimit => Err(SessionError::TooLarge),
                RecoveryError::InvalidArgument => Err(SessionError::InvalidPath),
                _ => Err(SessionError::RecoveryUnavailable),
            };
            Ok(())
        });
    prepared.map_err(|()| SessionError::OutOfMemory)?;
    save_result
}

impl ProjectSession {
    /// Prepares a bounded immutable view for automatic recovery publication.
    /// The editor source is shared by descriptor; its SVG bytes are materialized
    /// only when the worker publishes the snapshot.
    pub fn prepare_recovery_snapshot(
        &self,
        epoch: u64,
        revision: u64,
        captured_time_ns: u64,
    ) -> Result<Option<super::recovery_snapshot::RecoverySnapshot>, SessionError> {
        if !self.is_dirty() {
            return Ok(None);
        }
        let baseline = match (self.path.as_deref(), self.baseline.as_ref()) {
            (Some(_), Some(baseline)) => baseline,
            (None, None) => {
                // Untitled documents and imported SVG sessions have no native
                // recovery destination.
                return Ok(None);
            }
            _ => return Err(SessionError::RecoveryUnavailable),
        };
        let scene = self
            .document
            .edit_snapshot()
            .map_err(SessionError::Document)?;
        let mut canonical_path = Vec::new();
        canonical_path
            .try_reserve_exact(baseline.canonical_path.len())
            .map_err(|_| SessionError::OutOfMemory)?;
        canonical_path.extend_from_slice(&baseline.canonical_path);
        let baseline = RecoveryBaseline {
            canonical_path,
            source_size: baseline.source_size,
            modified_seconds: baseline.modified_seconds,
            modified_nanoseconds: baseline.modified_nanoseconds,
            device: baseline.device,
            inode: baseline.inode,
        };
        Ok(Some(super::recovery_snapshot::RecoverySnapshot {
            baseline,
            scene,
            payload: self.payload.clone(),
            captured_time_ns,
            epoch,
            revision,
        }))
    }

    /// Captures the complete current native project into the private per-user store.
    /// Source SVG imports and untitled sessions have no recovery destination.
    pub fn capture_recovery(&mut self, captured_time_ns: u64) -> Result<(), SessionError> {
        if !self.is_dirty() {
            return Ok(());
        }
        let source = self
            .path
            .as_deref()
            .ok_or(SessionError::RecoveryUnavailable)?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(SessionError::RecoveryUnavailable)?;
        let baseline_matches = baseline_matches_current(source, baseline)?;
        let canonical = &baseline.canonical_path;
        let capsule = capsule_path_for_canonical(canonical)?;
        validate_existing_capsule(&capsule, canonical)?;
        let scene = self.document.source().map_err(SessionError::Document)?;
        let mut result = Ok(());
        let mut capsule_published = false;
        let prepared = self
            .payload
            .try_with_bundle_input(&scene, &mut || true, |project| {
                let input = RecoveryInput {
                    source_path: canonical,
                    source_size: baseline.source_size,
                    source_modified_seconds: baseline.modified_seconds,
                    source_modified_nanoseconds: baseline.modified_nanoseconds,
                    captured_time_nanoseconds: captured_time_ns,
                    project: *project,
                };
                let capsule_result = project_recovery_save(&capsule, &input);
                capsule_published = capsule_result.published;
                result = match capsule_result.error {
                    RecoveryError::Ok => Ok(()),
                    RecoveryError::Durability if capsule_result.published => {
                        Err(SessionError::RecoveryNotDurable)
                    }
                    RecoveryError::Durability => Err(SessionError::RecoveryUnavailable),
                    RecoveryError::OutOfMemory => Err(SessionError::OutOfMemory),
                    RecoveryError::Limit | RecoveryError::PathLimit => Err(SessionError::TooLarge),
                    RecoveryError::InvalidArgument => Err(SessionError::InvalidPath),
                    _ => Err(SessionError::RecoveryUnavailable),
                };
                Ok(())
            });
        prepared.map_err(|()| SessionError::OutOfMemory)?;
        if capsule_published {
            self.recovery_conflict = !baseline_matches;
        }
        result
    }

    /// Checks the capsule for this session's existing native project.
    pub fn recovery_status(&self) -> Result<RecoveryStatus, SessionError> {
        let Some(source) = self.path.as_deref() else {
            return Ok(RecoveryStatus::Absent);
        };
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(SessionError::RecoveryUnavailable)?;
        let baseline_matches = baseline_matches_current(source, baseline)?;
        let capsule = capsule_path_for_canonical(&baseline.canonical_path)?;
        let canonical = &baseline.canonical_path;
        match project_recovery_discover(canonical, &capsule) {
            RecoveryDiscovery::Absent => Ok(RecoveryStatus::Absent),
            RecoveryDiscovery::Matching { capsule_bytes } if baseline_matches => {
                validate_capsule_contents(&capsule_bytes, canonical)?;
                Ok(RecoveryStatus::Matching)
            }
            RecoveryDiscovery::Matching { capsule_bytes }
            | RecoveryDiscovery::Conflict { capsule_bytes } => {
                validate_capsule_contents(&capsule_bytes, canonical)?;
                Ok(RecoveryStatus::Conflict)
            }
            RecoveryDiscovery::Invalid {
                error: RecoveryError::OutOfMemory,
            } => Err(SessionError::OutOfMemory),
            RecoveryDiscovery::Invalid {
                error: RecoveryError::PathLimit | RecoveryError::Limit,
            } => Err(SessionError::TooLarge),
            RecoveryDiscovery::Invalid {
                error: RecoveryError::PathMismatch,
            } => Err(SessionError::RecoveryInvalid),
            RecoveryDiscovery::Invalid { error } => {
                if error == RecoveryError::Io {
                    Err(SessionError::RecoveryUnavailable)
                } else {
                    Err(SessionError::RecoveryInvalid)
                }
            }
        }
    }

    /// Adopts a validated capsule only after the new document, payload, and path
    /// association have all been prepared successfully.
    pub fn restore_recovery(&mut self) -> Result<(), SessionError> {
        let source = self
            .path
            .as_deref()
            .ok_or(SessionError::RecoveryUnavailable)?;
        let current_baseline = self
            .baseline
            .as_ref()
            .ok_or(SessionError::RecoveryUnavailable)?;
        let baseline_matches = baseline_matches_current(source, current_baseline)?;
        let canonical = &current_baseline.canonical_path;
        let capsule = capsule_path_for_canonical(canonical)?;
        let (bytes, conflict) = match project_recovery_discover(canonical, &capsule) {
            RecoveryDiscovery::Matching { capsule_bytes } => (capsule_bytes, !baseline_matches),
            RecoveryDiscovery::Conflict { capsule_bytes } => (capsule_bytes, true),
            RecoveryDiscovery::Absent => return Err(SessionError::RecoveryInvalid),
            RecoveryDiscovery::Invalid {
                error: RecoveryError::OutOfMemory,
            } => return Err(SessionError::OutOfMemory),
            RecoveryDiscovery::Invalid {
                error: RecoveryError::Io,
            } => return Err(SessionError::RecoveryUnavailable),
            RecoveryDiscovery::Invalid { .. } => return Err(SessionError::RecoveryInvalid),
        };
        let view = project_recovery_decode(&bytes, Some(canonical))
            .map_err(|failure| map_recovery_error(failure.error))?;
        let document =
            V1Document::from_source(view.project.scene_svg).map_err(SessionError::InvalidScene)?;
        let payload = SessionPayload::try_from_view_with(view.project, &mut || true)
            .map_err(|()| SessionError::OutOfMemory)?;
        let journal = super::journal_integration::new_journal(&document, payload.clone())?;
        let mut restored_baseline = RecoveryBaseline {
            canonical_path: Vec::new(),
            source_size: view.source_size,
            modified_seconds: view.source_modified_seconds,
            modified_nanoseconds: view.source_modified_nanoseconds,
            device: current_baseline.device,
            inode: current_baseline.inode,
        };
        restored_baseline
            .canonical_path
            .try_reserve_exact(view.source_path.len())
            .map_err(|_| SessionError::OutOfMemory)?;
        restored_baseline
            .canonical_path
            .extend_from_slice(view.source_path);
        let mut replacement_path = Vec::new();
        replacement_path
            .try_reserve_exact(source.len())
            .map_err(|_| SessionError::OutOfMemory)?;
        replacement_path.extend_from_slice(source);

        self.document = document;
        self.payload = payload;
        self.journal = journal;
        self.path = Some(replacement_path);
        self.svg_origin = None;
        self.baseline = Some(restored_baseline);
        self.recovery_dirty = true;
        self.recovery_conflict = conflict;
        Ok(())
    }

    /// Explicitly discards this session's recovery capsule.
    pub fn discard_recovery(&mut self) -> Result<(), SessionError> {
        if self.path.is_none() {
            return Ok(());
        }
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(SessionError::RecoveryUnavailable)?;
        cleanup_recovery_for_canonical(&baseline.canonical_path)?;
        Ok(())
    }
}

pub(super) fn cleanup_recovery_for_path(path: &[u8]) -> Result<(), SessionError> {
    let canonical = canonical_source(path)?;
    cleanup_recovery_for_canonical(&canonical)
}

pub(super) fn cleanup_recovery_for_baseline(
    baseline: &RecoveryBaseline,
) -> Result<(), SessionError> {
    cleanup_recovery_for_canonical(&baseline.canonical_path)
}

fn cleanup_recovery_for_canonical(canonical: &[u8]) -> Result<(), SessionError> {
    let capsule = capsule_path_for_canonical(canonical)?;
    if !validate_existing_capsule(&capsule, canonical)? {
        return Ok(());
    }
    match fs::remove_file(Path::new(OsStr::from_bytes(&capsule))) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(SessionError::RecoveryCleanup),
    }
}

/// Checks the embedded identity without requiring the source to still exist.
/// A hash-key collision must never replace or discard another source's capsule.
fn validate_existing_capsule(capsule: &[u8], canonical: &[u8]) -> Result<bool, SessionError> {
    let path = Path::new(OsStr::from_bytes(capsule));
    let path_metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(SessionError::RecoveryUnavailable),
    };
    if !path_metadata.file_type().is_file() {
        return Err(SessionError::RecoveryInvalid);
    }
    let file = fs::File::open(path).map_err(|_| SessionError::RecoveryUnavailable)?;
    let metadata = file
        .metadata()
        .map_err(|_| SessionError::RecoveryUnavailable)?;
    if !metadata.is_file()
        || metadata.dev() != path_metadata.dev()
        || metadata.ino() != path_metadata.ino()
    {
        return Err(SessionError::RecoveryInvalid);
    }
    if metadata.len() > RECOVERY_MAX_BYTES as u64 {
        return Err(SessionError::TooLarge);
    }
    let mut bytes = Vec::new();
    let mut bounded = (&file).take(RECOVERY_MAX_BYTES as u64 + 1);
    let mut chunk = [0; 8192];
    loop {
        let count = bounded
            .read(&mut chunk)
            .map_err(|_| SessionError::RecoveryUnavailable)?;
        if count == 0 {
            break;
        }
        if count > RECOVERY_MAX_BYTES - bytes.len() {
            return Err(SessionError::TooLarge);
        }
        bytes
            .try_reserve(count)
            .map_err(|_| SessionError::OutOfMemory)?;
        bytes.extend_from_slice(&chunk[..count]);
    }
    project_recovery_decode(&bytes, Some(canonical))
        .map_err(|failure| map_recovery_error(failure.error))?;
    let after = fs::symlink_metadata(path).map_err(|_| SessionError::RecoveryUnavailable)?;
    if !after.file_type().is_file()
        || metadata.dev() != after.dev()
        || metadata.ino() != after.ino()
        || metadata.len() != after.len()
        || metadata.mtime() != after.mtime()
        || metadata.mtime_nsec() != after.mtime_nsec()
    {
        return Err(SessionError::RecoveryInvalid);
    }
    Ok(true)
}

fn canonical_source(path: &[u8]) -> Result<Vec<u8>, SessionError> {
    recovery_store::canonical_source_path(path).map_err(map_store_error)
}

fn capsule_path_for_canonical(path: &[u8]) -> Result<Vec<u8>, SessionError> {
    recovery_store::capsule_path_for_canonical(path).map_err(map_store_error)
}

fn map_store_error(error: StoreError) -> SessionError {
    match error {
        StoreError::InvalidPath => SessionError::InvalidPath,
        StoreError::OutOfMemory => SessionError::OutOfMemory,
        StoreError::Unavailable => SessionError::RecoveryUnavailable,
    }
}

fn map_recovery_error(error: RecoveryError) -> SessionError {
    match error {
        RecoveryError::OutOfMemory => SessionError::OutOfMemory,
        RecoveryError::Limit | RecoveryError::PathLimit => SessionError::TooLarge,
        RecoveryError::PathMismatch
        | RecoveryError::Corrupt
        | RecoveryError::UnsupportedVersion
        | RecoveryError::InvalidProject => SessionError::RecoveryInvalid,
        RecoveryError::InvalidArgument => SessionError::InvalidPath,
        _ => SessionError::RecoveryUnavailable,
    }
}

fn validate_capsule_contents(bytes: &[u8], expected_path: &[u8]) -> Result<(), SessionError> {
    let view = project_recovery_decode(bytes, Some(expected_path))
        .map_err(|failure| map_recovery_error(failure.error))?;
    V1Document::from_source(view.project.scene_svg).map_err(SessionError::InvalidScene)?;
    SessionPayload::try_from_view_with(view.project, &mut || true)
        .map(|_| ())
        .map_err(|()| SessionError::OutOfMemory)
}
