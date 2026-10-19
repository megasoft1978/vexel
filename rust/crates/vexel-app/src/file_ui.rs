//! Glue between the native window and `file_flow`: owns the document session,
//! routes shortcuts, menu activations and close requests, and keeps menu state
//! current.

use crate::file_flow::{
    FlowOutcome, action_for_tag, handle, handle_close, menu_enabled, menu_specs,
};
use crate::project_session::{FileAction, SessionError, shortcut_action};
use crate::recovery_job::{RecoveryJob, RecoveryJobResult};
use crate::recovery_policy::RecoveryPolicy;
use crate::session::AppDocument;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use vexel_platform_apple::{Modifiers, NativeApp, PanelBackend, WindowError, WindowId};

mod lifecycle_routes;

const BLANK_SIZE: (u32, u32) = (800, 600);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryStatusMessage {
    SavedCurrent,
    SavedOlderRevision,
    NewerEditsPending,
    PreparationFailed(SessionError),
    PublicationFailed(SessionError),
    DurabilityUncertain,
    Unavailable,
    NotRequired,
}

impl RecoveryStatusMessage {
    pub fn user_message(self) -> &'static str {
        match self {
            Self::SavedCurrent => "Recovery snapshot saved",
            Self::SavedOlderRevision => "Recovery snapshot saved; newer edits still need recovery",
            Self::NewerEditsPending => "Newer edits still need recovery",
            Self::PreparationFailed(error) | Self::PublicationFailed(error) => error.user_message(),
            Self::DurabilityUncertain => SessionError::RecoveryNotDurable.user_message(),
            Self::Unavailable => "Automatic recovery is unavailable",
            Self::NotRequired => "Recovery snapshot not required",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecoverySessionStamp {
    revision: u64,
    path: Option<Vec<u8>>,
    dirty: bool,
}

impl RecoverySessionStamp {
    fn capture(document: &AppDocument) -> Self {
        Self {
            revision: document.state_revision(),
            path: document.path().map(<[u8]>::to_vec),
            dirty: document.is_dirty(),
        }
    }
}

pub struct FileUi<B: PanelBackend> {
    doc: AppDocument,
    backend: B,
    recovery_coordinator: RecoveryCoordinator,
}

struct RecoveryCoordinator {
    recovery: Option<RecoveryJob>,
    recovery_policy: RecoveryPolicy,
    recovery_epoch: u64,
    recovery_status: Option<RecoveryStatusMessage>,
    // Last accepted publication identity; unchanged sessions keep their coverage.
    published: Option<(u64, u64)>,
    observed_revision: u64,
    #[cfg(test)]
    next_pause_ack: Option<Box<dyn FnOnce() + Send>>,
}

impl RecoveryCoordinator {
    fn new() -> Self {
        let recovery = RecoveryJob::spawn().ok();
        Self {
            recovery_status: recovery
                .is_none()
                .then_some(RecoveryStatusMessage::Unavailable),
            published: None,
            observed_revision: 0,
            recovery,
            recovery_policy: RecoveryPolicy::new(),
            recovery_epoch: 1,
            #[cfg(test)]
            next_pause_ack: None,
        }
    }

    fn result_status(
        result: RecoveryJobResult,
        epoch: u64,
        revision: u64,
    ) -> Option<RecoveryStatusMessage> {
        if result.epoch != epoch || result.revision > revision {
            return None;
        }
        Some(if result.durability_uncertain {
            RecoveryStatusMessage::DurabilityUncertain
        } else if let Some(error) = result.error {
            RecoveryStatusMessage::PublicationFailed(error)
        } else if result.published {
            if result.revision < revision {
                RecoveryStatusMessage::SavedOlderRevision
            } else {
                RecoveryStatusMessage::SavedCurrent
            }
        } else {
            RecoveryStatusMessage::Unavailable
        })
    }

    fn reconcile_policy(
        policy: &mut RecoveryPolicy,
        before: RecoverySessionStamp,
        after: RecoverySessionStamp,
        now: Instant,
    ) {
        if !after.dirty {
            policy.mark_clean();
        // State revisions can move for ordinary edits (including native
        // timeline payload changes). Preserve the capture rate limit unless
        // the project location identifies a replacement session.
        } else if !before.dirty || before.path != after.path || !policy.is_dirty() {
            *policy = RecoveryPolicy::new();
            policy.mark_dirty(now);
        }
    }

    fn with_lifecycle<T>(
        &mut self,
        document: &mut AppDocument,
        action: impl FnOnce(&mut AppDocument) -> T,
    ) -> T {
        let before = RecoverySessionStamp::capture(document);
        let _pause = self.recovery.as_ref().map(|job| {
            #[cfg(test)]
            if let Some(acknowledge) = self.next_pause_ack.take() {
                return job.pause_test(acknowledge);
            }
            job.pause()
        });
        self.recovery_epoch = self.recovery_epoch.wrapping_add(1).max(1);
        let result = action(document);
        let after = RecoverySessionStamp::capture(document);
        if before != after || !after.dirty {
            self.published = None;
            self.recovery_status = Some(if after.dirty {
                RecoveryStatusMessage::NewerEditsPending
            } else {
                RecoveryStatusMessage::NotRequired
            });
        }
        Self::reconcile_policy(&mut self.recovery_policy, before, after, Instant::now());
        result
    }

    #[cfg(test)]
    fn acknowledge_next_pause(&mut self, acknowledge: impl FnOnce() + Send + 'static) {
        self.next_pause_ack = Some(Box::new(acknowledge));
    }

    fn tick(&mut self, document: &AppDocument, now: Instant) -> Option<RecoveryStatusMessage> {
        if let Some(result) = self.recovery.as_ref().and_then(RecoveryJob::poll_result) {
            if let Some(status) =
                Self::result_status(result, self.recovery_epoch, document.state_revision())
            {
                if result.published {
                    self.published = Some((result.epoch, result.revision));
                }
                self.recovery_status = Some(status);
            }
        }

        if !document.is_dirty() {
            self.recovery_policy.mark_clean();
            return self.recovery_status.take();
        }
        let changed = self.observed_revision != document.state_revision();
        self.observed_revision = document.state_revision();
        if changed
            && self
                .published
                .is_some_and(|(_, revision)| revision < document.state_revision())
            && self.recovery_status.is_none()
        {
            self.recovery_status = Some(RecoveryStatusMessage::NewerEditsPending);
        }
        self.recovery_policy.mark_dirty(now);
        if !self.recovery_policy.is_due(now) {
            return self.recovery_status.take();
        }
        self.recovery_policy.record_scheduled(now);
        let captured_time_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .min(u64::MAX as u128) as u64;
        let snapshot = match document.session().prepare_recovery_snapshot(
            self.recovery_epoch,
            document.state_revision(),
            captured_time_ns,
        ) {
            Ok(Some(snapshot)) => snapshot,
            Ok(None) => return self.recovery_status.take(),
            Err(error) => {
                self.recovery_status = Some(RecoveryStatusMessage::PreparationFailed(error));
                return self.recovery_status.take();
            }
        };
        let submitted = self
            .recovery
            .as_ref()
            .is_some_and(|job| job.submit(snapshot));
        if !submitted {
            self.recovery_status = Some(RecoveryStatusMessage::Unavailable);
        }
        self.recovery_status.take()
    }

    fn shutdown(&mut self) {
        if let Some(job) = &mut self.recovery {
            job.shutdown();
        }
    }
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
            recovery_coordinator: RecoveryCoordinator::new(),
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
        let mut replaced = false;
        let outcome = self.with_lifecycle(|doc, backend| {
            let outcome = handle(action, doc.session_for_flow(), backend);
            if outcome == FlowOutcome::Opened {
                doc.mark_replaced();
                replaced = true;
            }
            outcome
        });
        if replaced {
            self.recovery_coordinator.recovery_policy.mark_clean();
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
        let outcome =
            self.with_lifecycle(|doc, backend| handle_close(doc.session_for_flow(), backend));
        if outcome == FlowOutcome::CloseWindow {
            self.recovery_coordinator.shutdown();
        }
        outcome
    }

    /// Runs a document lifecycle operation behind the recovery publication
    /// barrier. MCP and startup replacement use the same boundary as File UI.
    pub fn with_lifecycle<T>(&mut self, action: impl FnOnce(&mut AppDocument, &B) -> T) -> T {
        self.recovery_coordinator
            .with_lifecycle(&mut self.doc, |document| action(document, &self.backend))
    }

    /// Advances bounded automatic recovery scheduling and returns one stable
    /// status message for the shell status strip, if anything changed.
    pub fn tick_recovery(&mut self, now: Instant) -> Option<RecoveryStatusMessage> {
        self.recovery_coordinator.tick(&self.doc, now)
    }
}

#[cfg(test)]
mod lifecycle_cadence_tests {
    use super::*;
    use crate::native_animation::{NativeTimelineEffect, import};
    use crate::project_session::ProjectSession;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use vexel_platform_apple::{
        PanelResult, RecoveryChoice, RecoveryText, UnsavedChoice, UnsavedText,
    };
    use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};
    use vexel_v1_document::V1Document;

    const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="24" viewBox="0 0 32 24"><rect id="shape" x="2" y="2" width="12" height="10" fill="#f00"/></svg>"##;

    struct ImportBackend(Vec<u8>);

    impl PanelBackend for ImportBackend {
        fn open_panel(&self, _: &[&str]) -> PanelResult {
            PanelResult::Path(self.0.clone())
        }
        fn save_panel(&self, _: &str) -> PanelResult {
            PanelResult::Cancelled
        }
        fn confirm_unsaved(&self, _: &UnsavedText<'_>) -> UnsavedChoice {
            UnsavedChoice::Cancel
        }
        fn confirm_recovery(&self, _: &RecoveryText<'_>) -> RecoveryChoice {
            RecoveryChoice::Cancel
        }
        fn show_error(&self, _: &str, _: &str) {}
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "vexel-file-ui-cadence-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn settings() -> EmptyTimelineSettings {
        EmptyTimelineSettings {
            ticks_per_second: 24,
            duration_ticks: 240,
            fps_numerator: 24,
            fps_denominator: 1,
            loop_mode: LoopMode::Repeat,
            onion_before: 0,
            onion_after: 0,
            onion_opacity: 0.5,
        }
    }

    #[test]
    fn successful_native_svg_import_preserves_scheduled_recovery_interval() {
        let temp = TempDir::new();
        let project_path = temp.file("project.vexel");
        let import_path = temp.file("drawing.svg");
        fs::write(&import_path, SVG).unwrap();
        let mut project = ProjectSession::new(V1Document::from_source(SVG).unwrap()).unwrap();
        project
            .save_as(project_path.as_os_str().as_bytes())
            .unwrap();

        let mut files = FileUi::with_size(
            ImportBackend(import_path.as_os_str().as_bytes().to_vec()),
            (32, 24),
        )
        .unwrap();
        files.document_mut().open_path(&project_path).unwrap();
        assert!(
            files
                .document_mut()
                .edit_timeline(settings(), EditLimits::default(), |candidate| {
                    candidate.set_loop(LoopMode::Once)
                })
                .unwrap()
        );
        assert!(files.document().is_dirty());

        let scheduled = Instant::now();
        files
            .recovery_coordinator
            .recovery_policy
            .mark_dirty(scheduled - Duration::from_secs(20));
        files
            .recovery_coordinator
            .recovery_policy
            .record_scheduled(scheduled);
        let source_revision = files.document().revision();
        let state_revision = files.document().state_revision();
        let effect = NativeTimelineEffect::ImportSvg {
            target_id: "shape".to_owned(),
            tick: 0,
        };

        assert_eq!(
            import::handle(&mut files, &effect, settings(), EditLimits::default()),
            import::ImportResult::Imported
        );
        assert_eq!(files.document().revision(), source_revision);
        assert_eq!(files.document().state_revision(), state_revision + 1);
        assert!(files.document().is_dirty());
        assert!(
            !files
                .recovery_coordinator
                .recovery_policy
                .is_due(scheduled + Duration::from_secs(14))
        );
        assert!(
            files
                .recovery_coordinator
                .recovery_policy
                .is_due(scheduled + Duration::from_secs(15))
        );
    }
}
