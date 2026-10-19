//! Undo/redo availability, read from the combined session journal. The shell
//! keeps no counter: edits, loads and MCP operations update the session state.

use crate::session::{AppDocument, EditAction};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Availability {
    pub undo: bool,
    pub redo: bool,
}

impl Availability {
    /// Reads combined scene and payload journal availability.
    pub fn of(doc: &AppDocument) -> Self {
        Self {
            undo: doc.can_undo(),
            redo: doc.can_redo(),
        }
    }
}

/// Runs undo or redo on `doc`; an unavailable action changes nothing. The one
/// function behind the toolbar buttons and Cmd+Z / Shift+Cmd+Z. True when the
/// document changed.
pub fn apply(doc: &mut AppDocument, action: EditAction) -> bool {
    let available = Availability::of(doc);
    match action {
        EditAction::Undo if available.undo => doc.undo().unwrap_or(false),
        EditAction::Redo if available.redo => doc.redo().unwrap_or(false),
        _ => false,
    }
}
