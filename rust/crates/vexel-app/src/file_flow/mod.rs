//! Pure Open / Save / Save As / close flow over `ProjectSession`.
//!
//! All dialogs go through [`PanelBackend`], so tests drive the whole flow with a
//! fake. Every cancel leaves the session untouched; a failed save or open never
//! replaces the live document.

mod dedup;
mod menu;
mod recovery;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_recovery;

pub use dedup::{DEDUP_WINDOW, ShortcutDedup};
pub use menu::{FILE_MENU, MenuEntry, action_for_tag, menu_enabled, menu_specs};

use crate::project_session::{FileAction, ProjectSession, SessionError, unsaved_changes_prompt};
use vexel_platform_apple::{PanelBackend, PanelResult, UnsavedChoice, UnsavedText};

/// Extensions the Open panel offers: `.svg` opens as an untitled import
/// (compose path), `.vexel` as a project.
pub const OPEN_EXTENSIONS: &[&str] = &["svg", "vexel"];

/// What the caller must do after a flow step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowOutcome {
    Saved,
    Opened,
    /// The user cancelled; nothing changed.
    Cancelled,
    /// An alert with the error's `user_message()` was shown; nothing changed.
    Failed(SessionError),
    CloseWindow,
    KeepOpen,
}

impl FlowOutcome {
    /// Whether the session changed (the frame must be redrawn).
    pub fn redraw(self) -> bool {
        matches!(self, Self::Saved | Self::Opened)
    }

    /// Status-area text, or `None` when nothing happened.
    pub fn status(self) -> Option<&'static str> {
        match self {
            Self::Saved => Some("Saved"),
            Self::Opened => Some("Opened"),
            Self::Failed(error) => Some(error.user_message()),
            Self::Cancelled | Self::KeepOpen | Self::CloseWindow => None,
        }
    }
}

fn file_name(path: &[u8]) -> Option<String> {
    let name = path.rsplit(|b| *b == b'/').next()?;
    (!name.is_empty()).then(|| String::from_utf8_lossy(name).into_owned())
}

fn fail(backend: &dyn PanelBackend, title: &str, error: SessionError) -> FlowOutcome {
    backend.show_error(title, error.user_message());
    FlowOutcome::Failed(error)
}

fn save_as(session: &mut ProjectSession, backend: &dyn PanelBackend) -> FlowOutcome {
    let suggested = session
        .path()
        .and_then(file_name)
        .unwrap_or_else(|| "Untitled.vexel".to_owned());
    match backend.save_panel(&suggested) {
        PanelResult::Cancelled => FlowOutcome::Cancelled,
        PanelResult::Error => fail(backend, "Could not save", SessionError::InvalidPath),
        PanelResult::Path(path) => match session.save_as(&path) {
            Ok(()) => FlowOutcome::Saved,
            Err(error) => fail(backend, "Could not save", error),
        },
    }
}

fn save(session: &mut ProjectSession, backend: &dyn PanelBackend) -> FlowOutcome {
    if session.path().is_none() {
        return save_as(session, backend);
    }
    match session.save() {
        Ok(()) => FlowOutcome::Saved,
        Err(error) => fail(backend, "Could not save", error),
    }
}

/// Asks the unsaved-changes question; `Some(outcome)` means stop with it.
fn resolve_unsaved(
    session: &mut ProjectSession,
    backend: &dyn PanelBackend,
    action: FileAction,
) -> Option<FlowOutcome> {
    let name = session.path().and_then(file_name);
    let prompt = unsaved_changes_prompt(name.as_deref(), action);
    let text = UnsavedText {
        message: &prompt.message,
        detail: prompt.detail,
        save: prompt.confirm,
        dont_save: prompt.discard,
        cancel: prompt.cancel,
    };
    match backend.confirm_unsaved(&text) {
        UnsavedChoice::Cancel => Some(FlowOutcome::Cancelled),
        UnsavedChoice::DontSave => None,
        UnsavedChoice::Save => match save(session, backend) {
            FlowOutcome::Saved => None,
            other => Some(other),
        },
    }
}

fn open(session: &mut ProjectSession, backend: &dyn PanelBackend) -> FlowOutcome {
    if session.is_dirty() {
        if let Some(stop) = resolve_unsaved(session, backend, FileAction::Open) {
            return stop;
        }
    }
    match backend.open_panel(OPEN_EXTENSIONS) {
        PanelResult::Cancelled => FlowOutcome::Cancelled,
        PanelResult::Error => fail(backend, "Could not open", SessionError::InvalidPath),
        PanelResult::Path(path) => recovery::open_selected(session, &path, backend),
    }
}

/// Runs one file action against the session.
pub fn handle(
    action: FileAction,
    session: &mut ProjectSession,
    backend: &dyn PanelBackend,
) -> FlowOutcome {
    match action {
        FileAction::Save => save(session, backend),
        FileAction::SaveAs => save_as(session, backend),
        FileAction::Open => open(session, backend),
    }
}

/// Window close request: clean closes, dirty asks first.
pub fn handle_close(session: &mut ProjectSession, backend: &dyn PanelBackend) -> FlowOutcome {
    if !session.is_dirty() {
        return FlowOutcome::CloseWindow;
    }
    match resolve_unsaved(session, backend, FileAction::Save) {
        None => FlowOutcome::CloseWindow,
        Some(_) => FlowOutcome::KeepOpen,
    }
}
