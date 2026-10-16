//! AppKit panel calls. Each call needs the main thread and is re-checked here;
//! off the main thread every function returns its error value.

use super::{
    PanelBackend, PanelResult, UnsavedChoice, UnsavedText, checked_path, with_project_extension,
};
use crate::{MainThread, guard_native_call};
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSAlert, NSAlertStyle, NSOpenPanel, NSSavePanel};
use objc2_foundation::{NSArray, NSString, NSURL};

const MODAL_OK: isize = 1;
const FIRST_BUTTON: isize = 1000;

/// Re-validates the main-thread requirement at the call itself.
pub(crate) fn require_main() -> Option<MainThreadMarker> {
    MainThreadMarker::new()
}

fn url_path(url: Option<Retained<NSURL>>) -> PanelResult {
    match url.and_then(|u| u.path()) {
        Some(path) => checked_path(path.to_string().into_bytes()),
        None => PanelResult::Error,
    }
}

fn types(extensions: &[&str]) -> Retained<NSArray<NSString>> {
    let items: Vec<Retained<NSString>> = extensions.iter().map(|e| NSString::from_str(e)).collect();
    NSArray::from_retained_slice(&items)
}

/// Shows the Open panel for single files with the given extensions.
#[allow(deprecated)] // allowedFileTypes: baseline on macOS 13, avoids another framework.
pub fn open_panel(_main: MainThread, extensions: &[&str]) -> PanelResult {
    let Some(mtm) = require_main() else {
        return PanelResult::Error;
    };
    guard_native_call(|| {
        let panel = NSOpenPanel::openPanel(mtm);
        panel.setCanChooseFiles(true);
        panel.setCanChooseDirectories(false);
        panel.setAllowsMultipleSelection(false);
        panel.setAllowedFileTypes(Some(&types(extensions)));
        if panel.runModal() == MODAL_OK {
            url_path(panel.URL())
        } else {
            PanelResult::Cancelled
        }
    })
    .unwrap_or(PanelResult::Error)
}

/// Shows the Save panel; the returned path always ends with `.vexel`.
#[allow(deprecated)]
pub fn save_panel(_main: MainThread, suggested_name: &str) -> PanelResult {
    let Some(mtm) = require_main() else {
        return PanelResult::Error;
    };
    guard_native_call(|| {
        let panel = NSSavePanel::savePanel(mtm);
        panel.setAllowedFileTypes(Some(&types(&[super::PROJECT_EXTENSION])));
        panel.setCanCreateDirectories(true);
        panel.setNameFieldStringValue(&NSString::from_str(suggested_name));
        if panel.runModal() == MODAL_OK {
            match url_path(panel.URL()) {
                PanelResult::Path(path) => with_project_extension(path),
                other => other,
            }
        } else {
            PanelResult::Cancelled
        }
    })
    .unwrap_or(PanelResult::Error)
}

/// Shows the three-button unsaved-changes alert. Any failure answers Cancel.
pub fn confirm_unsaved(_main: MainThread, text: &UnsavedText<'_>) -> UnsavedChoice {
    let Some(mtm) = require_main() else {
        return UnsavedChoice::Cancel;
    };
    guard_native_call(|| {
        let alert = NSAlert::new(mtm);
        alert.setAlertStyle(NSAlertStyle::Warning);
        alert.setMessageText(&NSString::from_str(text.message));
        alert.setInformativeText(&NSString::from_str(text.detail));
        for title in [text.save, text.dont_save, text.cancel] {
            alert.addButtonWithTitle(&NSString::from_str(title));
        }
        match alert.runModal() - FIRST_BUTTON {
            0 => UnsavedChoice::Save,
            1 => UnsavedChoice::DontSave,
            _ => UnsavedChoice::Cancel,
        }
    })
    .unwrap_or(UnsavedChoice::Cancel)
}

/// Shows a single-button error alert.
pub fn show_error(_main: MainThread, title: &str, message: &str) {
    let Some(mtm) = require_main() else {
        return;
    };
    let _ = guard_native_call(|| {
        let alert = NSAlert::new(mtm);
        alert.setAlertStyle(NSAlertStyle::Warning);
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(message));
        alert.addButtonWithTitle(&NSString::from_str("OK"));
        alert.runModal();
    });
}

/// The real AppKit backend.
#[derive(Clone, Copy, Debug)]
pub struct NativePanels(MainThread);

impl NativePanels {
    pub fn new(main: MainThread) -> Self {
        Self(main)
    }
}

impl PanelBackend for NativePanels {
    fn open_panel(&self, extensions: &[&str]) -> PanelResult {
        open_panel(self.0, extensions)
    }
    fn save_panel(&self, suggested_name: &str) -> PanelResult {
        save_panel(self.0, suggested_name)
    }
    fn confirm_unsaved(&self, text: &UnsavedText<'_>) -> UnsavedChoice {
        confirm_unsaved(self.0, text)
    }
    fn show_error(&self, title: &str, message: &str) {
        show_error(self.0, title, message)
    }
}
