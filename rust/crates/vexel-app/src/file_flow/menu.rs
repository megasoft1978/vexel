//! Single table for File menu titles, key equivalents and shortcuts.

use crate::project_session::{FileAction, ProjectSession};
use vexel_platform_apple::MenuItemSpec;

/// One File menu entry. `key_code` is the ANSI virtual key code that
/// `shortcut_action` must map (with Command, plus Shift when `shift`) to `action`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuEntry {
    pub action: FileAction,
    pub title: &'static str,
    pub key: &'static str,
    pub key_code: u16,
    pub shift: bool,
}

/// Menu order; the item tag is the index.
pub const FILE_MENU: [MenuEntry; 3] = [
    MenuEntry {
        action: FileAction::Open,
        title: "Open\u{2026}",
        key: "o",
        key_code: 31,
        shift: false,
    },
    MenuEntry {
        action: FileAction::Save,
        title: "Save",
        key: "s",
        key_code: 1,
        shift: false,
    },
    MenuEntry {
        action: FileAction::SaveAs,
        title: "Save As\u{2026}",
        key: "s",
        key_code: 1,
        shift: true,
    },
];

/// Platform menu specs built from [`FILE_MENU`].
pub fn menu_specs() -> Vec<MenuItemSpec<'static>> {
    FILE_MENU
        .iter()
        .enumerate()
        .map(|(tag, entry)| MenuItemSpec {
            title: entry.title,
            key: entry.key,
            shift: entry.shift,
            tag: tag as u32,
        })
        .collect()
}

/// Maps a menu tag back to its action.
pub fn action_for_tag(tag: u32) -> Option<FileAction> {
    FILE_MENU.get(tag as usize).map(|entry| entry.action)
}

/// Item enablement in [`FILE_MENU`] order: Save is disabled only when the
/// document is clean and already has a path.
pub fn menu_enabled(session: &ProjectSession) -> [bool; 3] {
    let save = session.is_dirty() || session.path().is_none();
    FILE_MENU.map(|entry| match entry.action {
        FileAction::Save => save,
        FileAction::Open | FileAction::SaveAs => true,
    })
}
