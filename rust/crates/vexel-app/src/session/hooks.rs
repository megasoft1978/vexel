//! Narrow hook points between the session and its collaborators.
//!
//! * Drawing tools: `tools::ToolController` methods take `&mut V1Document`, so
//!   the shell calls `app.edit(|doc| tools.pointer(event, doc, &map))`. Any type
//!   with the same shape can implement [`DocumentEditor`] and be run with
//!   [`AppDocument::apply`].
//! * Animation: the ANIM clock requests frames through a [`PlaybackHost`];
//!   [`AppDocument::request_frame`] hands the host the current source bytes and
//!   revision, so playback renders from the same document as the editor.
//! * Pointer mapping: `compose::PointMap` implements `tools::CanvasMap`.

use super::AppDocument;
use crate::compose::PointMap;
use crate::project_session::SessionError;
use crate::tools::CanvasMap;
use vexel_platform_apple::Modifiers;
use vexel_v1_document::V1Document;

/// Anything that mutates the document (a tool gesture, an animation edit).
pub trait DocumentEditor {
    type Error;
    fn apply(&mut self, doc: &mut V1Document) -> Result<(), Self::Error>;
}

/// Whether an applied edit changed the source (i.e. moved `revision`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditOutcome {
    Unchanged,
    Changed,
}

/// Receives frame requests from the animation clock.
pub trait PlaybackHost {
    fn request_frame(&mut self, source: &[u8], revision: u64, seconds: f64);
}

impl AppDocument {
    /// Runs `editor` through [`AppDocument::edit`].
    pub fn apply<E: DocumentEditor>(&mut self, editor: &mut E) -> Result<EditOutcome, E::Error> {
        let before = self.revision();
        self.edit(|doc| editor.apply(doc))?;
        Ok(if self.revision() == before {
            EditOutcome::Unchanged
        } else {
            EditOutcome::Changed
        })
    }

    /// Asks `host` to render `seconds` of the current source.
    pub fn request_frame(
        &self,
        host: &mut dyn PlaybackHost,
        seconds: f64,
    ) -> Result<(), SessionError> {
        let source = self.source_bytes()?;
        host.request_frame(&source, self.revision(), seconds);
        Ok(())
    }
}

impl CanvasMap for PointMap {
    fn to_doc(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        self.canvas_to_doc(x, y)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditAction {
    Undo,
    Redo,
}

const KEY_Z: u16 = 6;

/// Cmd+Z and Shift+Cmd+Z (ANSI virtual key code); other modifiers map to nothing.
pub fn edit_shortcut(code: u16, modifiers: Modifiers) -> Option<EditAction> {
    let command = modifiers.0 & Modifiers::COMMAND != 0;
    let shift = modifiers.0 & Modifiers::SHIFT != 0;
    let others = modifiers.0 & (Modifiers::OPTION | Modifiers::CONTROL) != 0;
    (command && !others && code == KEY_Z).then_some(if shift {
        EditAction::Redo
    } else {
        EditAction::Undo
    })
}
