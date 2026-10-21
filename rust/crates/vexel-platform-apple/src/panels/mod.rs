//! Native Open / Save panels, unsaved-changes alert and error alert.
//!
//! Everything here returns plain bytes and small enums; no Objective-C object
//! leaves the crate. The pure helpers and the [`PanelBackend`] seam are tested
//! without showing any panel. Native calls run only on the main thread.

mod native;
#[cfg(test)]
mod tests;

pub use native::{
    NativePanels, confirm_recovery, confirm_unsaved, open_panel, save_panel, show_error,
};

/// Longest path accepted from a panel, in bytes.
pub const MAX_PANEL_PATH: usize = 1024;
/// Extension (without dot) used for Vexel projects.
pub const PROJECT_EXTENSION: &str = "vexel";

/// Result of a file panel. `Path` is a UTF-8 path, 1..=1024 bytes, no NUL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PanelResult {
    Path(Vec<u8>),
    Cancelled,
    Error,
}

/// Answer to the unsaved-changes alert.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsavedChoice {
    Save,
    DontSave,
    Cancel,
}

/// Answer to an explicit recovery decision for an opened native project.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryChoice {
    Restore,
    Discard,
    Cancel,
}

/// Text of the unsaved-changes alert.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsavedText<'a> {
    pub message: &'a str,
    pub detail: &'a str,
    pub save: &'a str,
    pub dont_save: &'a str,
    pub cancel: &'a str,
}

/// Text for the recovery alert. Conflict wording is supplied by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryText<'a> {
    pub message: &'a str,
    pub detail: &'a str,
    pub restore: &'a str,
    pub discard: &'a str,
    pub cancel: &'a str,
}

/// Swappable front for the native dialogs; the app tests use a fake.
pub trait PanelBackend {
    fn open_panel(&self, extensions: &[&str]) -> PanelResult;
    /// `extension` is the only type the panel allows and appends.
    fn save_panel(&self, suggested_name: &str, extension: &str) -> PanelResult;
    fn confirm_unsaved(&self, text: &UnsavedText<'_>) -> UnsavedChoice;
    fn confirm_recovery(&self, _text: &RecoveryText<'_>) -> RecoveryChoice {
        RecoveryChoice::Cancel
    }
    fn show_error(&self, title: &str, message: &str);
}

/// Bounds a panel path: non-empty, at most [`MAX_PANEL_PATH`] bytes, no NUL.
pub fn checked_path(path: Vec<u8>) -> PanelResult {
    if path.is_empty() || path.len() > MAX_PANEL_PATH || path.contains(&0) {
        PanelResult::Error
    } else {
        PanelResult::Path(path)
    }
}

/// Appends `.vexel` unless the path already ends with it (ASCII case-insensitive).
/// The result is bounded like any other panel path.
pub fn with_project_extension(path: Vec<u8>) -> PanelResult {
    with_extension(path, PROJECT_EXTENSION)
}

/// Appends `.<extension>` unless the path already ends with it (ASCII case-insensitive).
pub fn with_extension(mut path: Vec<u8>, extension: &str) -> PanelResult {
    let suffix = format!(".{extension}");
    let has = path.len() > suffix.len()
        && path[path.len() - suffix.len()..].eq_ignore_ascii_case(suffix.as_bytes());
    if !has {
        if path.len().saturating_add(suffix.len()) > MAX_PANEL_PATH {
            return PanelResult::Error;
        }
        path.extend_from_slice(suffix.as_bytes());
    }
    checked_path(path)
}
