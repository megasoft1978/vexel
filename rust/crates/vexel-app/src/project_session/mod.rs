//! Window-free save/open session for the editable `.vexel` project.
//!
//! The bundle stores the exact `V1Document::source()` bytes as its scene.
//! Undo/redo history is session state and is NOT persisted: the v1 bundle has
//! no history section, so a reopened document starts clean with empty history.

mod actions;
#[cfg(test)]
mod tests;

pub use actions::{FileAction, shortcut_action, unsaved_changes_prompt};

use vexel_project::{self as project, BundleInput, Error as BundleError};
use vexel_project_io::{FileError, LoadFailure, project_file_load, project_file_save_as};
use vexel_v1_document::{V1Document, V1DocumentError};

const TIMELINE_JSON: &[u8] = b"{}";

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
    OutOfMemory,
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
            Self::OutOfMemory => "Not enough memory.",
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

fn manifest(width: u32, height: u32) -> Vec<u8> {
    format!("{{\"format\":\"vexel-project\",\"width\":{width},\"height\":{height}}}").into_bytes()
}

/// Writes `source` as a project through `vexel-project-io` (atomic publish).
/// Shared by `ProjectSession` and the MCP preview path.
pub fn write_project(
    path: &[u8],
    source: &[u8],
    width: u32,
    height: u32,
) -> Result<(), SessionError> {
    let manifest = manifest(width, height);
    let input = BundleInput {
        manifest_json: &manifest,
        scene_svg: source,
        timeline_json: TIMELINE_JSON,
        assets: &[],
    };
    let result = project_file_save_as(path, &input);
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
#[derive(Debug)]
pub struct ProjectSession {
    document: V1Document,
    path: Option<Vec<u8>>,
    /// Path of the `.svg` this untitled document was imported from, if any.
    /// Display only: Save never writes to it.
    svg_origin: Option<Vec<u8>>,
}

impl ProjectSession {
    pub fn new(document: V1Document) -> Self {
        Self {
            document,
            path: None,
            svg_origin: None,
        }
    }
    pub fn document(&self) -> &V1Document {
        &self.document
    }
    pub fn document_mut(&mut self) -> &mut V1Document {
        &mut self.document
    }
    pub fn path(&self) -> Option<&[u8]> {
        self.path.as_deref()
    }
    /// The imported `.svg` path for an untitled document opened from one.
    pub fn svg_origin(&self) -> Option<&[u8]> {
        self.svg_origin.as_deref()
    }
    pub fn is_dirty(&self) -> bool {
        self.document.is_dirty()
    }

    /// Saves to the current path. The document is marked saved only on success.
    pub fn save(&mut self) -> Result<(), SessionError> {
        let path = self.path.clone().ok_or(SessionError::NoPath)?;
        self.write(&path)
    }

    /// Saves to `path`; the session adopts it only on success.
    pub fn save_as(&mut self, path: &[u8]) -> Result<(), SessionError> {
        self.write(path)?;
        self.path = Some(path.to_vec());
        self.svg_origin = None;
        Ok(())
    }

    fn write(&mut self, path: &[u8]) -> Result<(), SessionError> {
        let source = self.document.source().map_err(SessionError::Document)?;
        write_project(path, &source, self.document.width(), self.document.height())?;
        self.document.mark_saved();
        Ok(())
    }

    /// Opens `path` into a fresh clean session with empty history. A `.svg`
    /// extension imports the SVG as an untitled document (Save then asks for a
    /// name and never overwrites the SVG); anything else is a `.vexel` project.
    pub fn open(path: &[u8]) -> Result<Self, SessionError> {
        if path.len() > 4 && path[path.len() - 4..].eq_ignore_ascii_case(b".svg") {
            return Self::open_svg(path);
        }
        let loaded = project_file_load(path).map_err(load_error)?;
        let document =
            V1Document::from_source(loaded.view().scene_svg).map_err(SessionError::InvalidScene)?;
        Ok(Self {
            document,
            path: Some(path.to_vec()),
            svg_origin: None,
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
        Ok(Self {
            document,
            path: None,
            svg_origin: Some(path.to_vec()),
        })
    }

    /// Replaces this session only if `path` opens successfully; on error the
    /// current document, history and path are untouched.
    pub fn open_replacing(&mut self, path: &[u8]) -> Result<(), SessionError> {
        *self = Self::open(path)?;
        Ok(())
    }
}
