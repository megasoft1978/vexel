//! Window-free guard against one Cmd+S press running a file action twice
//! (once as the menu key equivalent, once as `keyDown:`).

use crate::project_session::FileAction;
use std::time::Duration;

/// How long after one delivery the same action from the other path is a duplicate.
pub const DEDUP_WINDOW: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Source {
    Menu,
    Key,
}

/// Remembers the last delivered file action. `now` is any monotonic time
/// (for example elapsed since process start); the caller injects it.
#[derive(Debug, Default)]
pub struct ShortcutDedup {
    last: Option<(Source, FileAction, Duration)>,
}

impl ShortcutDedup {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a keyboard-originated `action` should run. `false` when the
    /// menu delivered the same action within [`DEDUP_WINDOW`].
    pub fn allow_key(&mut self, action: FileAction, now: Duration) -> bool {
        self.allow(Source::Key, action, now)
    }

    /// Whether a menu-originated `action` should run. `false` when the
    /// keyboard delivered the same action within [`DEDUP_WINDOW`].
    pub fn allow_menu(&mut self, action: FileAction, now: Duration) -> bool {
        self.allow(Source::Menu, action, now)
    }

    fn allow(&mut self, source: Source, action: FileAction, now: Duration) -> bool {
        let duplicate = self.last.is_some_and(|(prev_source, prev_action, at)| {
            prev_source != source && prev_action == action && now.saturating_sub(at) < DEDUP_WINDOW
        });
        if !duplicate {
            self.last = Some((source, action, now));
        }
        !duplicate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn menu_save_then_key_save_in_the_window_runs_once() {
        let mut dedup = ShortcutDedup::new();
        let mut saves = 0;
        if dedup.allow_menu(FileAction::Save, ms(1000)) {
            saves += 1;
        }
        if dedup.allow_key(FileAction::Save, ms(1010)) {
            saves += 1;
        }
        assert_eq!(saves, 1);
    }

    #[test]
    fn key_save_alone_runs_once() {
        let mut dedup = ShortcutDedup::new();
        assert!(dedup.allow_key(FileAction::Save, ms(5)));
    }

    #[test]
    fn two_presses_a_second_apart_run_twice() {
        let mut dedup = ShortcutDedup::new();
        assert!(dedup.allow_key(FileAction::Save, ms(0)));
        assert!(dedup.allow_key(FileAction::Save, ms(1000)));
        assert!(dedup.allow_menu(FileAction::Save, ms(2000)));
        assert!(dedup.allow_key(FileAction::Save, ms(3000)));
    }

    #[test]
    fn menu_then_key_after_the_window_runs_twice() {
        let mut dedup = ShortcutDedup::new();
        assert!(dedup.allow_menu(FileAction::Save, ms(0)));
        assert!(dedup.allow_key(FileAction::Save, DEDUP_WINDOW));
    }

    #[test]
    fn save_then_save_as_in_the_window_runs_twice() {
        let mut dedup = ShortcutDedup::new();
        assert!(dedup.allow_menu(FileAction::Save, ms(0)));
        assert!(dedup.allow_key(FileAction::SaveAs, ms(10)));
    }

    #[test]
    fn key_then_menu_in_the_window_runs_once() {
        let mut dedup = ShortcutDedup::new();
        assert!(dedup.allow_key(FileAction::Save, ms(0)));
        assert!(!dedup.allow_menu(FileAction::Save, ms(10)));
    }
}
