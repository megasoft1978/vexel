//! Pure menu/keyboard mapping and prompt text for file actions.

use vexel_platform_apple::Modifiers;

const KEY_S: u16 = 1;
const KEY_O: u16 = 31;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileAction {
    Save,
    SaveAs,
    Open,
}

/// Cmd+S, Shift+Cmd+S, Cmd+O (ANSI virtual key codes); any other modifier
/// combination maps to nothing.
pub fn shortcut_action(code: u16, modifiers: Modifiers) -> Option<FileAction> {
    let shift = modifiers.0 & Modifiers::SHIFT != 0;
    let command = modifiers.0 & Modifiers::COMMAND != 0;
    let others = modifiers.0 & (Modifiers::OPTION | Modifiers::CONTROL) != 0;
    if !command || others {
        return None;
    }
    match (code, shift) {
        (KEY_S, false) => Some(FileAction::Save),
        (KEY_S, true) => Some(FileAction::SaveAs),
        (KEY_O, false) => Some(FileAction::Open),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Prompt {
    pub message: String,
    pub detail: &'static str,
    pub confirm: &'static str,
    pub discard: &'static str,
    pub cancel: &'static str,
}

/// Text for the unsaved-changes sheet shown before `action` would replace the
/// current document. `name` is the file name, or `None` for an untitled one.
pub fn unsaved_changes_prompt(name: Option<&str>, action: FileAction) -> Prompt {
    let shown = name.unwrap_or("Untitled");
    Prompt {
        message: format!("Do you want to save the changes made to \u{201c}{shown}\u{201d}?"),
        detail: if action == FileAction::Open {
            "Your changes will be lost if you do not save them before opening another project."
        } else {
            "Your changes will be lost if you do not save them."
        },
        confirm: if name.is_some() {
            "Save"
        } else {
            "Save As\u{2026}"
        },
        discard: "Don\u{2019}t Save",
        cancel: "Cancel",
    }
}
