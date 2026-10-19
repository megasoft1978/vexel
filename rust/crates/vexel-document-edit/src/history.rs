use crate::{EditError, Editor};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    pub label: &'static str,
    pub state_cursor: usize,
    pub is_current: bool,
    pub is_saved: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryState {
    pub count: usize,
    pub cursor: usize,
    pub saved_cursor: Option<usize>,
}

impl Editor {
    /// Returns the opaque identity of the current source state.
    ///
    /// State identities are stable for the lifetime of this editor. Callers
    /// must treat them as opaque tokens: they are not document content hashes
    /// and they may cease to be reachable after a branch, coalescing edit, or
    /// bounded-history eviction.
    pub fn history_bridge_state_id(&self) -> u64 {
        self.state_id
    }

    /// Returns the opaque identity at a reachable history cursor, where zero
    /// is the oldest retained state and `history_count()` is the newest.
    /// Enumerating this range lets an app journal trim states removed by the
    /// editor's bounded history without inspecting editor internals.
    pub fn history_bridge_state_id_at(&self, cursor: usize) -> Option<u64> {
        if cursor > self.history.len() {
            return None;
        }
        if cursor == 0 {
            return Some(
                self.history
                    .first()
                    .map_or(self.state_id, |command| command.before_state),
            );
        }
        self.history
            .get(cursor - 1)
            .map(|command| command.after_state)
    }

    /// Returns the current cursor for a reachable opaque state identity.
    pub fn history_bridge_cursor_for(&self, state_id: u64) -> Option<usize> {
        (0..=self.history.len())
            .find(|cursor| self.history_bridge_state_id_at(*cursor) == Some(state_id))
    }

    /// Reports whether `state_id` remains within the bounded reachable history.
    pub fn history_bridge_can_navigate(&self, state_id: u64) -> bool {
        self.history_bridge_cursor_for(state_id).is_some()
    }

    /// Moves to a reachable state atomically. A stale or foreign state token
    /// is rejected without changing the editor.
    pub fn history_bridge_navigate(&mut self, state_id: u64) -> Result<(), EditError> {
        let cursor = self
            .history_bridge_cursor_for(state_id)
            .ok_or(EditError::InvalidArgument)?;
        self.jump_to(cursor)
    }

    /// Closes the current command's coalescing group. The next edit cannot
    /// merge with an edit before this boundary, including after undo.
    pub fn history_bridge_boundary(&mut self) {
        if self.cursor > 0 {
            if let Some(command) = self.history.get_mut(self.cursor - 1) {
                command.coalescible = false;
            }
        }
    }

    pub fn history_state(&self) -> HistoryState {
        let saved_cursor = if self.saved_state
            == self
                .history
                .first()
                .map_or(self.state_id, |c| c.before_state)
        {
            Some(0)
        } else {
            self.history
                .iter()
                .position(|c| c.after_state == self.saved_state)
                .map(|i| i + 1)
        };
        HistoryState {
            count: self.history.len(),
            cursor: self.cursor,
            saved_cursor,
        }
    }
    pub fn history_entry(&self, index: usize) -> Option<HistoryEntry> {
        let command = self.history.get(index)?;
        Some(HistoryEntry {
            label: command.label,
            state_cursor: index + 1,
            is_current: self.cursor == index + 1,
            is_saved: command.after_state == self.saved_state,
        })
    }
    pub fn history_saved_cursor(&self) -> Option<usize> {
        self.history_state().saved_cursor
    }
    pub fn history_relabel_latest(&mut self, action: crate::HistoryAction) {
        if self.cursor == self.history.len() {
            if let Some(command) = self.history.last_mut() {
                command.label = action.label();
            }
        }
    }
    pub fn history_jump(&mut self, state_cursor: usize) -> Result<(), EditError> {
        self.jump_to(state_cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EditLimits, HistoryAction};
    use std::sync::Arc;

    fn editor(history_commands: usize) -> Editor {
        Editor::create(
            &Arc::new(b"<svg x='0'/>".to_vec()),
            EditLimits {
                history_commands,
                history_bytes: 1024,
            },
        )
        .editor
        .unwrap()
    }

    fn edit_value(editor: &mut Editor, value: u8) {
        editor
            .commit_replace(8, 1, &[value], 0, false, HistoryAction::EditAttribute)
            .unwrap();
    }

    #[test]
    fn bridge_navigation_is_atomic_and_branching_drops_redo_tokens() {
        let mut editor = editor(8);
        let initial = editor.history_bridge_state_id();
        edit_value(&mut editor, b'1');
        let first = editor.history_bridge_state_id();
        edit_value(&mut editor, b'2');
        let second = editor.history_bridge_state_id();

        editor.history_bridge_navigate(first).unwrap();
        assert_eq!(editor.history_bridge_state_id(), first);
        assert!(editor.history_bridge_can_navigate(second));
        assert_eq!(
            editor.history_bridge_navigate(u64::MAX),
            Err(EditError::InvalidArgument)
        );
        assert_eq!(editor.history_bridge_state_id(), first);
        assert_eq!(editor.history_cursor(), 1);

        edit_value(&mut editor, b'3');
        let branch = editor.history_bridge_state_id();
        assert_ne!(branch, second);
        assert!(!editor.history_bridge_can_navigate(second));
        assert!(editor.history_bridge_can_navigate(initial));
    }

    #[test]
    fn explicit_boundary_prevents_attribute_coalescing() {
        let mut editor = Editor::create(&Arc::new(b"<svg x='0'/>".to_vec()), EditLimits::default())
            .editor
            .unwrap();
        let root = editor.root().unwrap();
        editor.set_attribute(root, b"x", b"1").unwrap();
        let first = editor.history_bridge_state_id();
        editor.history_bridge_boundary();
        let root = editor.root().unwrap();
        editor.set_attribute(root, b"x", b"2").unwrap();

        assert_eq!(editor.history_count(), 2);
        assert_eq!(editor.history_bridge_state_id_at(1), Some(first));
        assert_ne!(editor.history_bridge_state_id(), first);
    }

    #[test]
    fn bounded_history_exposes_eviction_through_reachable_state_window() {
        let mut editor = editor(2);
        let initial = editor.history_bridge_state_id();
        edit_value(&mut editor, b'1');
        let first = editor.history_bridge_state_id();
        edit_value(&mut editor, b'2');
        edit_value(&mut editor, b'3');

        assert_eq!(editor.history_count(), 2);
        assert_eq!(editor.history_bridge_state_id_at(0), Some(first));
        assert!(!editor.history_bridge_can_navigate(initial));
        assert!(editor.history_bridge_can_navigate(first));
    }
}
