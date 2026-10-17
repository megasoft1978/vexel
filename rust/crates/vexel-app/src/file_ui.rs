//! Glue between the native window and `file_flow`: owns the document session,
//! routes shortcuts, menu activations and close requests, and keeps menu state
//! current.

use crate::file_flow::{
    FlowOutcome, action_for_tag, handle, handle_close, menu_enabled, menu_specs,
};
use crate::project_session::{FileAction, shortcut_action};
use crate::session::AppDocument;
use vexel_platform_apple::{Modifiers, NativeApp, PanelBackend, WindowError, WindowId};

const BLANK_SIZE: (u32, u32) = (800, 600);

pub struct FileUi<B: PanelBackend> {
    doc: AppDocument,
    backend: B,
}

impl<B: PanelBackend> FileUi<B> {
    /// Starts with a blank, clean, untitled document.
    pub fn new(backend: B) -> Option<Self> {
        Self::with_size(backend, BLANK_SIZE)
    }

    pub fn with_size(backend: B, size: (u32, u32)) -> Option<Self> {
        Some(Self {
            doc: AppDocument::new(size.0, size.1).ok()?,
            backend,
        })
    }

    pub fn document(&self) -> &AppDocument {
        &self.doc
    }

    pub fn document_mut(&mut self) -> &mut AppDocument {
        &mut self.doc
    }

    /// Installs the File menu on `window` and applies the initial enablement.
    pub fn install(&self, app: &mut NativeApp, window: WindowId) -> Result<(), WindowError> {
        app.install_file_menu(window, &menu_specs())?;
        self.refresh_menu(app)
    }

    pub fn refresh_menu(&self, app: &NativeApp) -> Result<(), WindowError> {
        app.set_file_menu_enabled(&menu_enabled(self.doc.session()))
    }

    /// Window title: app name, document name once it has a path, dirty marker.
    pub fn title(&self) -> String {
        self.doc.title()
    }

    fn run(&mut self, action: FileAction) -> FlowOutcome {
        let outcome = handle(action, self.doc.session_for_flow(), &self.backend);
        if outcome == FlowOutcome::Opened {
            self.doc.mark_replaced();
        }
        outcome
    }

    pub fn key(&mut self, code: u16, modifiers: Modifiers) -> Option<FlowOutcome> {
        let action = shortcut_action(code, modifiers)?;
        Some(self.run(action))
    }

    /// The Save toolbar button: the same flow as Cmd+S (an untitled document
    /// opens the Save panel).
    pub fn save(&mut self) -> FlowOutcome {
        self.run(FileAction::Save)
    }

    /// The panel backend, for the Export panels.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn menu(&mut self, tag: u32) -> Option<FlowOutcome> {
        let action = action_for_tag(tag)?;
        Some(self.run(action))
    }

    pub fn close(&mut self) -> FlowOutcome {
        handle_close(self.doc.session_for_flow(), &self.backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vexel_platform_apple::{PanelResult, UnsavedChoice, UnsavedText};

    struct Never;
    impl PanelBackend for Never {
        fn open_panel(&self, _: &[&str]) -> PanelResult {
            PanelResult::Cancelled
        }
        fn save_panel(&self, _: &str) -> PanelResult {
            PanelResult::Cancelled
        }
        fn confirm_unsaved(&self, _: &UnsavedText<'_>) -> UnsavedChoice {
            UnsavedChoice::Cancel
        }
        fn show_error(&self, _: &str, _: &str) {}
    }

    #[test]
    fn key_and_menu_routes_reach_the_flow() {
        let mut ui = FileUi::new(Never).unwrap();
        let cmd = Modifiers(Modifiers::COMMAND);
        // Cmd+S on an untitled document reaches the (cancelled) save panel.
        assert_eq!(ui.key(1, cmd), Some(FlowOutcome::Cancelled));
        assert_eq!(ui.key(1, Modifiers(Modifiers::CONTROL)), None);
        assert_eq!(ui.key(0, cmd), None);
        assert_eq!(ui.menu(0), Some(FlowOutcome::Cancelled));
        assert_eq!(ui.menu(9), None);
        assert_eq!(ui.close(), FlowOutcome::CloseWindow);
        assert_eq!(ui.title(), "Vexel");
    }
}
