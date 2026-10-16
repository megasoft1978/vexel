//! Glue between the native window and `file_flow`: owns the session, routes
//! shortcuts, menu activations and close requests, and keeps menu state current.

use crate::file_flow::{
    FlowOutcome, action_for_tag, handle, handle_close, menu_enabled, menu_specs,
};
use crate::project_session::{ProjectSession, shortcut_action};
use vexel_platform_apple::{Modifiers, NativeApp, PanelBackend, WindowError, WindowId};
use vexel_v1_document::V1Document;

const BLANK_SIZE: (u32, u32) = (800, 600);

pub struct FileUi<B: PanelBackend> {
    session: ProjectSession,
    backend: B,
}

impl<B: PanelBackend> FileUi<B> {
    /// Starts with a blank, clean, untitled document.
    pub fn new(backend: B) -> Option<Self> {
        let document = V1Document::new(BLANK_SIZE.0, BLANK_SIZE.1).ok()?;
        Some(Self {
            session: ProjectSession::new(document),
            backend,
        })
    }

    /// Installs the File menu on `window` and applies the initial enablement.
    pub fn install(&self, app: &mut NativeApp, window: WindowId) -> Result<(), WindowError> {
        app.install_file_menu(window, &menu_specs())?;
        self.refresh_menu(app)
    }

    pub fn refresh_menu(&self, app: &NativeApp) -> Result<(), WindowError> {
        app.set_file_menu_enabled(&menu_enabled(&self.session))
    }

    /// Window title: app name plus the document name once it has a path.
    pub fn title(&self) -> String {
        match self.session.path() {
            Some(path) => {
                let name = path.rsplit(|b| *b == b'/').next().unwrap_or_default();
                format!("Vexel \u{2014} {}", String::from_utf8_lossy(name))
            }
            None => "Vexel".to_owned(),
        }
    }

    pub fn key(&mut self, code: u16, modifiers: Modifiers) -> Option<FlowOutcome> {
        let action = shortcut_action(code, modifiers)?;
        Some(handle(action, &mut self.session, &self.backend))
    }

    pub fn menu(&mut self, tag: u32) -> Option<FlowOutcome> {
        let action = action_for_tag(tag)?;
        Some(handle(action, &mut self.session, &self.backend))
    }

    pub fn close(&mut self) -> FlowOutcome {
        handle_close(&mut self.session, &self.backend)
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
