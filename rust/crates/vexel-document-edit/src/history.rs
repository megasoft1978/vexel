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
