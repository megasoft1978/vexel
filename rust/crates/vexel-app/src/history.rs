//! Undo/redo availability, read straight from the document's own history
//! (`V1Document::history_cursor` / `history_count`). The shell keeps no
//! counter: a new edit, a load or an MCP operation changes the core cursor and
//! therefore the availability, with nothing to reset here.

use crate::session::{AppDocument, EditAction};
use vexel_v1_document::V1Document;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Availability {
    pub undo: bool,
    pub redo: bool,
}

impl Availability {
    /// Undo is possible iff `cursor > 0`, redo iff `cursor < count`.
    pub fn of(doc: &V1Document) -> Self {
        Self {
            undo: doc.history_cursor() > 0,
            redo: doc.history_cursor() < doc.history_count(),
        }
    }
}

/// Runs undo or redo on `doc`; an unavailable action changes nothing. The one
/// function behind the toolbar buttons and Cmd+Z / Shift+Cmd+Z. True when the
/// document changed.
pub fn apply(doc: &mut AppDocument, action: EditAction) -> bool {
    let available = Availability::of(doc.document());
    match action {
        EditAction::Undo if available.undo => doc.undo().is_ok(),
        EditAction::Redo if available.redo => doc.redo().is_ok(),
        _ => false,
    }
}
