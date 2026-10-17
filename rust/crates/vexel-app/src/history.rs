//! In-memory redo cursor. `V1Document` has no availability query (a core
//! accessor is the proper follow-up), so the shell counts: a successful undo
//! adds one redoable step, a successful redo removes one, and any other change
//! of the document revision (new edit, load, MCP operation) clears them all.

use crate::session::{AppDocument, EditAction};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct History {
    redo_depth: u32,
    seen: u64,
}

impl History {
    pub fn redo_depth(&self) -> u32 {
        self.redo_depth
    }

    pub fn can_redo(&self) -> bool {
        self.redo_depth > 0
    }

    /// Notes the document revision; a change not made by [`History::apply`]
    /// is a new edit or a load. True when the redo availability changed.
    pub fn observe(&mut self, revision: u64) -> bool {
        if revision == self.seen {
            return false;
        }
        self.seen = revision;
        std::mem::take(&mut self.redo_depth) > 0
    }

    /// Runs undo or redo on `doc`. Redo while nothing is redoable is a no-op.
    /// Returns true when the document was changed.
    pub fn apply(&mut self, doc: &mut AppDocument, action: EditAction) -> bool {
        self.observe(doc.revision());
        let ok = match action {
            EditAction::Undo => doc.undo().is_ok(),
            EditAction::Redo if self.can_redo() => doc.redo().is_ok(),
            EditAction::Redo => return false,
        };
        self.seen = doc.revision();
        if ok {
            match action {
                EditAction::Undo => self.redo_depth += 1,
                EditAction::Redo => self.redo_depth -= 1,
            }
        }
        ok
    }
}
