//! The one document session of the shell (window-free).
//!
//! [`AppDocument`] owns the only [`V1Document`] (inside a [`ProjectSession`]).
//! Everything that changes the drawing -- tools, undo/redo, MCP preview
//! operations, File > Open -- goes through it, and the compose view renders
//! from [`AppDocument::source_bytes`] whenever [`AppDocument::revision`] moves.
//! `revision` is bumped only when the source bytes actually change (or the
//! whole document is replaced), so a no-op edit never triggers a re-render.
//!
//! Hook boundary (see [`hooks`]): drawing tools and animation reach the
//! document only through [`AppDocument::edit`] / [`AppDocument::apply`]; this
//! module depends on neither.

mod grants;
mod hooks;
mod mcp;
pub(crate) mod mcp_png;
mod mcp_tools;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_mcp;
#[cfg(test)]
mod tests_playback;

pub use grants::{GrantError, McpGrants};
pub use hooks::{DocumentEditor, EditAction, EditOutcome, PlaybackHost, edit_shortcut};
pub use mcp::{McpHost, McpOutcome};

use crate::project_session::{ProjectSession, SessionError};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use vexel_v1_document::{V1Document, V1DocumentError};

/// Window title for a document name (`None` = untitled) and dirty state.
/// A clean untitled document keeps the bare app name.
pub fn window_title(name: Option<&str>, dirty: bool) -> String {
    match (name, dirty) {
        (None, false) => "Vexel".to_owned(),
        (None, true) => "Vexel \u{2014} Untitled \u{2022}".to_owned(),
        (Some(name), false) => format!("Vexel \u{2014} {name}"),
        (Some(name), true) => format!("Vexel \u{2014} {name} \u{2022}"),
    }
}

fn file_name(path: &[u8]) -> Option<String> {
    let name = path.rsplit(|b| *b == b'/').next()?;
    (!name.is_empty()).then(|| String::from_utf8_lossy(name).into_owned())
}

#[derive(Debug)]
pub struct AppDocument {
    session: ProjectSession,
    revision: u64,
}

impl AppDocument {
    /// A blank, clean, untitled document at revision 0.
    pub fn new(width: u32, height: u32) -> Result<Self, V1DocumentError> {
        Ok(Self::from_document(V1Document::new(width, height)?))
    }

    pub fn from_document(document: V1Document) -> Self {
        Self {
            session: ProjectSession::new(document),
            revision: 0,
        }
    }

    /// Increases whenever the source changes (edit, undo, redo, open).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn source_bytes(&self) -> Result<Vec<u8>, SessionError> {
        self.session
            .document()
            .source()
            .map_err(SessionError::Document)
    }

    pub fn document(&self) -> &V1Document {
        self.session.document()
    }

    pub fn is_dirty(&self) -> bool {
        self.session.is_dirty()
    }

    pub fn path(&self) -> Option<&[u8]> {
        self.session.path()
    }

    pub fn size(&self) -> (u32, u32) {
        let doc = self.session.document();
        (doc.width(), doc.height())
    }

    /// File name of the project, or of the imported SVG; `None` when untitled.
    pub fn display_name(&self) -> Option<String> {
        self.session
            .path()
            .or_else(|| self.session.svg_origin())
            .and_then(file_name)
    }

    /// Full path of the project, or of the SVG an untitled document was
    /// imported from; `None` for a blank document.
    pub fn origin_path(&self) -> Option<&[u8]> {
        self.session.path().or_else(|| self.session.svg_origin())
    }

    /// Window title: named by the project path only (an imported SVG is
    /// untitled), with a dirty marker.
    pub fn title(&self) -> String {
        let name = self.session.path().and_then(file_name);
        window_title(name.as_deref(), self.is_dirty())
    }

    /// Opens `path` by extension: `.svg` as an untitled document, anything
    /// else as a `.vexel` project. On failure nothing changes.
    pub fn open_path(&mut self, path: &Path) -> Result<(), SessionError> {
        self.session.open_replacing(path.as_os_str().as_bytes())?;
        self.revision += 1;
        Ok(())
    }

    /// Records that the flow replaced the whole document (File > Open).
    pub(crate) fn mark_replaced(&mut self) {
        self.revision += 1;
    }

    /// The session for the file flow (open/save panels). Callers that replace
    /// the document must follow with [`AppDocument::mark_replaced`].
    pub(crate) fn session(&self) -> &ProjectSession {
        &self.session
    }
    pub(crate) fn session_for_flow(&mut self) -> &mut ProjectSession {
        &mut self.session
    }

    pub fn save(&mut self) -> Result<(), SessionError> {
        self.session.save()
    }
    pub fn save_as(&mut self, path: &[u8]) -> Result<(), SessionError> {
        self.session.save_as(path)
    }

    /// Runs one edit. `revision` moves only when the source bytes changed,
    /// whether or not the closure reported success (a failing edit that left
    /// partial changes must still re-render).
    pub fn edit<R, E>(&mut self, f: impl FnOnce(&mut V1Document) -> Result<R, E>) -> Result<R, E> {
        let before = self.session.document().source().ok();
        let result = f(self.session.document_mut());
        let after = self.session.document().source().ok();
        if before.is_none() || before != after {
            self.revision += 1;
        }
        result
    }

    pub fn undo(&mut self) -> Result<(), V1DocumentError> {
        self.edit(V1Document::undo)
    }
    pub fn redo(&mut self) -> Result<(), V1DocumentError> {
        self.edit(V1Document::redo)
    }
}
