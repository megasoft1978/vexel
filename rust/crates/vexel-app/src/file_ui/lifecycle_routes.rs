//! Shared lifecycle entry points for shell startup and MCP file operations.

use super::*;
use crate::animation::{driver::Driver, playback::LoopClock};
use crate::compose::DocumentView;
use crate::preview::Operation;
use crate::session::{McpHost, McpOutcome};
use std::path::Path;

impl<B: PanelBackend> FileUi<B> {
    /// Opens the startup argument under the same barrier as native File Open.
    pub fn open_startup(&mut self, view: &mut DocumentView, path: &Path, stage: (u32, u32)) {
        self.with_lifecycle(|document, _| view.open(document, path, stage));
    }

    /// MCP file operations hold the barrier across the entire host transaction.
    pub fn handle_mcp<C: LoopClock>(
        &mut self,
        host: &mut McpHost,
        operation: Operation,
        driver: &mut Driver<C>,
    ) -> Result<McpOutcome, &'static str> {
        if matches!(
            &operation,
            Operation::Save | Operation::SaveAs { .. } | Operation::Open { .. }
        ) {
            self.with_lifecycle(|document, _| host.handle_with(document, operation, driver))
        } else {
            host.handle_with(self.document_mut(), operation, driver)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_session::{
        ProjectSession, RecoveryStatus, SessionError, with_recovery_test_home,
    };
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use vexel_platform_apple::{
        PanelResult, RecoveryChoice, RecoveryText, UnsavedChoice, UnsavedText,
    };
    use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};
    use vexel_v1_document::V1Document;

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
        fn confirm_recovery(&self, _: &RecoveryText<'_>) -> RecoveryChoice {
            RecoveryChoice::Cancel
        }
        fn show_error(&self, _: &str, _: &str) {}
    }

    struct PathBackend {
        path: Vec<u8>,
        save_error: bool,
        unsaved: UnsavedChoice,
        recovery: RecoveryChoice,
    }
    impl PanelBackend for PathBackend {
        fn open_panel(&self, _: &[&str]) -> PanelResult {
            PanelResult::Path(self.path.clone())
        }
        fn save_panel(&self, _: &str) -> PanelResult {
            if self.save_error {
                PanelResult::Error
            } else {
                PanelResult::Path(self.path.clone())
            }
        }
        fn confirm_unsaved(&self, _: &UnsavedText<'_>) -> UnsavedChoice {
            self.unsaved
        }
        fn confirm_recovery(&self, _: &RecoveryText<'_>) -> RecoveryChoice {
            self.recovery
        }
        fn show_error(&self, _: &str, _: &str) {}
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "vexel-file-ui-recovery-{}-{}",
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

    fn path_bytes(path: &Path) -> &[u8] {
        path.as_os_str().as_bytes()
    }

    fn dirty_ui() -> FileUi<Never> {
        let mut ui = FileUi::new(Never).unwrap();
        ui.document_mut()
            .edit(|document| document.add_rectangle(1.0, 1.0, 8.0, 9.0, "#123456"))
            .unwrap();
        ui
    }

    fn published_result(epoch: u64, revision: u64) -> RecoveryJobResult {
        RecoveryJobResult {
            epoch,
            revision,
            published: true,
            durability_uncertain: false,
            error: None,
        }
    }

    fn gated_file_action<B: PanelBackend + Send + 'static>(
        mut ui: FileUi<B>,
        action: impl FnOnce(&mut FileUi<B>) -> FlowOutcome + Send + 'static,
    ) -> FlowOutcome {
        use std::sync::mpsc;

        let job = ui.recovery_coordinator.recovery.as_ref().unwrap();
        let (active_tx, active_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::channel();
        assert!(job.submit_test_task(move || {
            active_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            published_result(1, 1)
        }));
        active_rx.recv().unwrap();
        let (pending_tx, pending_rx) = mpsc::channel();
        assert!(job.submit_test_task(move || {
            pending_tx.send(()).unwrap();
            published_result(1, 2)
        }));

        let (paused_tx, paused_rx) = mpsc::channel();
        ui.recovery_coordinator
            .acknowledge_next_pause(move || paused_tx.send(()).unwrap());
        let (finished_tx, finished_rx) = mpsc::channel();
        let lifecycle = std::thread::spawn(move || {
            let outcome = action(&mut ui);
            finished_tx.send(()).unwrap();
            (ui, outcome)
        });
        paused_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        // The acknowledgement is sent only after selection is paused and the
        // pending task is removed; the active publish is still gated here.
        assert!(finished_rx.recv_timeout(Duration::from_millis(50)).is_err());
        release_tx.send(()).unwrap();
        finished_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let (ui, outcome) = lifecycle.join().unwrap();
        let _pause = ui.recovery_coordinator.recovery.as_ref().unwrap().pause();
        assert!(pending_rx.try_recv().is_err());
        drop(_pause);
        let job = ui.recovery_coordinator.recovery.as_ref().unwrap();
        let (resumed_tx, resumed_rx) = mpsc::channel();
        let resumed = job.submit_test_task(move || {
            resumed_tx.send(()).unwrap();
            published_result(3, 3)
        });
        if outcome == FlowOutcome::CloseWindow {
            assert!(!resumed);
        } else {
            assert!(resumed);
            resumed_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        }
        outcome
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

    #[test]
    fn recovery_result_status_reports_a_published_older_revision() {
        let result = RecoveryJobResult {
            epoch: 4,
            revision: 8,
            published: true,
            durability_uncertain: false,
            error: None,
        };
        assert_eq!(
            RecoveryCoordinator::result_status(result, 4, 9),
            Some(RecoveryStatusMessage::SavedOlderRevision)
        );
        assert_eq!(
            RecoveryCoordinator::result_status(result, 4, 8),
            Some(RecoveryStatusMessage::SavedCurrent)
        );
        assert_eq!(RecoveryCoordinator::result_status(result, 3, 9), None);
        assert_eq!(RecoveryCoordinator::result_status(result, 4, 7), None);

        let failed = RecoveryJobResult {
            error: Some(SessionError::RecoveryUnavailable),
            published: false,
            ..result
        };
        assert_eq!(
            RecoveryCoordinator::result_status(failed, 4, 9),
            Some(RecoveryStatusMessage::PublicationFailed(
                SessionError::RecoveryUnavailable
            ))
        );
    }

    #[test]
    fn uncertain_publication_and_lifecycle_coverage_remain_typed() {
        let uncertain = RecoveryJobResult {
            durability_uncertain: true,
            error: Some(SessionError::RecoveryNotDurable),
            ..published_result(1, 1)
        };
        assert_eq!(
            RecoveryCoordinator::result_status(uncertain, 1, 1),
            Some(RecoveryStatusMessage::DurabilityUncertain)
        );
        let mut ui = dirty_ui();
        let revision = ui.document().state_revision();
        ui.recovery_coordinator.published = Some((1, revision));
        assert_eq!(ui.save(), FlowOutcome::Cancelled);
        assert_eq!(ui.recovery_coordinator.published, Some((1, revision)));
        ui.with_lifecycle(|document, _| {
            *document = AppDocument::new(32, 24).unwrap();
        });
        assert_eq!(ui.recovery_coordinator.published, None);
        assert_eq!(
            ui.tick_recovery(Instant::now()),
            Some(RecoveryStatusMessage::NotRequired)
        );
    }

    #[test]
    fn payload_only_native_edit_changes_recovery_stamp_without_source_revision() {
        let mut ui = FileUi::new(Never).unwrap();
        let before = RecoverySessionStamp::capture(ui.document());
        let source_revision = ui.document().revision();
        let settings = EmptyTimelineSettings {
            ticks_per_second: 24,
            duration_ticks: 240,
            fps_numerator: 24,
            fps_denominator: 1,
            loop_mode: LoopMode::Once,
            onion_before: 0,
            onion_after: 0,
            onion_opacity: 0.5,
        };

        assert!(
            ui.document_mut()
                .edit_timeline(settings, EditLimits::default(), |candidate| {
                    candidate.set_loop(LoopMode::Repeat)
                })
                .unwrap()
        );

        let after = RecoverySessionStamp::capture(ui.document());
        assert_eq!(ui.document().revision(), source_revision);
        assert_eq!(after.revision, before.revision + 1);
        assert!(after.dirty);
    }

    #[test]
    fn canceled_or_failed_lifecycle_preserves_retry_interval() {
        let start = Instant::now();
        let stamp = RecoverySessionStamp {
            revision: 12,
            path: Some(b"/projects/current.vexel".to_vec()),
            dirty: true,
        };
        let mut policy = RecoveryPolicy::new();
        policy.mark_dirty(start);
        policy.record_scheduled(start + Duration::from_secs(2));

        RecoveryCoordinator::reconcile_policy(
            &mut policy,
            stamp.clone(),
            stamp.clone(),
            start + Duration::from_secs(3),
        );
        // A subsequent canceled/failed lifecycle on the same session must
        // retain the original 15-second max-rate window.
        RecoveryCoordinator::reconcile_policy(
            &mut policy,
            stamp.clone(),
            stamp,
            start + Duration::from_secs(4),
        );
        assert!(!policy.is_due(start + Duration::from_secs(16)));
        assert!(policy.is_due(start + Duration::from_secs(17)));
    }

    #[test]
    fn canceled_file_actions_keep_the_existing_retry_window() {
        let mut ui = FileUi::new(Never).unwrap();
        ui.document_mut()
            .edit(|document| document.add_rectangle(1.0, 1.0, 8.0, 9.0, "#123456"))
            .unwrap();
        let now = Instant::now();
        let scheduled = now - Duration::from_secs(2);
        let dirty_since = now - Duration::from_secs(4);
        ui.recovery_coordinator
            .recovery_policy
            .mark_dirty(dirty_since);
        ui.recovery_coordinator
            .recovery_policy
            .record_scheduled(scheduled);

        assert_eq!(ui.menu(0), Some(FlowOutcome::Cancelled));
        assert_eq!(ui.save(), FlowOutcome::Cancelled);
        assert_eq!(ui.menu(2), Some(FlowOutcome::Cancelled));
        assert_eq!(ui.close(), FlowOutcome::KeepOpen);
        assert!(
            !ui.recovery_coordinator
                .recovery_policy
                .is_due(Instant::now() + Duration::from_secs(11))
        );
        assert!(
            ui.recovery_coordinator
                .recovery_policy
                .is_due(Instant::now() + Duration::from_secs(13))
        );
    }

    #[test]
    fn replaced_dirty_session_starts_a_fresh_debounce_window() {
        let start = Instant::now();
        let before = RecoverySessionStamp {
            revision: 12,
            path: Some(b"/projects/current.vexel".to_vec()),
            dirty: true,
        };
        let after = RecoverySessionStamp {
            revision: 13,
            path: Some(b"/projects/other.vexel".to_vec()),
            dirty: true,
        };
        let mut policy = RecoveryPolicy::new();
        policy.mark_dirty(start);
        policy.record_scheduled(start + Duration::from_secs(2));

        RecoveryCoordinator::reconcile_policy(
            &mut policy,
            before,
            after,
            start + Duration::from_secs(3),
        );
        assert!(!policy.is_due(start + Duration::from_secs(4)));
        assert!(policy.is_due(start + Duration::from_secs(5)));
    }

    #[test]
    fn lifecycle_barrier_waits_for_active_publish_and_drops_pending_publish() {
        assert_eq!(
            gated_file_action(dirty_ui(), FileUi::save),
            FlowOutcome::Cancelled
        );
        assert_eq!(
            gated_file_action(dirty_ui(), |ui| ui.menu(2).unwrap()),
            FlowOutcome::Cancelled
        );
        assert_eq!(
            gated_file_action(dirty_ui(), |ui| ui.menu(0).unwrap()),
            FlowOutcome::Cancelled
        );
        assert_eq!(
            gated_file_action(dirty_ui(), FileUi::close),
            FlowOutcome::KeepOpen
        );
    }

    #[test]
    fn gated_recovery_discard_open_removes_capsule_after_active_publish() {
        let temp = TempDir::new();
        let home = temp.file("home");
        let projects = temp.file("projects");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&projects).unwrap();
        let path = projects.join("open.vexel");

        with_recovery_test_home(&home, || {
            let mut source = ProjectSession::new(V1Document::new(48, 48).unwrap()).unwrap();
            source.save_as(path_bytes(&path)).unwrap();
            source
                .edit(|document| document.add_rectangle(2.0, 3.0, 16.0, 19.0, "#123456"))
                .unwrap();
            source.capture_recovery(123).unwrap();
            assert_eq!(source.recovery_status().unwrap(), RecoveryStatus::Matching);

            let ui = FileUi::new(PathBackend {
                path: path_bytes(&path).to_vec(),
                save_error: false,
                unsaved: UnsavedChoice::DontSave,
                recovery: RecoveryChoice::Discard,
            })
            .unwrap();
            let action_home = home.clone();
            assert_eq!(
                gated_file_action(ui, move |ui| {
                    with_recovery_test_home(&action_home, || ui.menu(0).unwrap())
                }),
                FlowOutcome::Opened
            );
            let opened = ProjectSession::open(path_bytes(&path)).unwrap();
            assert_eq!(opened.recovery_status().unwrap(), RecoveryStatus::Absent);
        });
    }

    #[test]
    fn gated_save_save_as_and_close_preserve_lifecycle_transactions() {
        let temp = TempDir::new();
        let home = temp.file("home");
        let projects = temp.file("projects");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&projects).unwrap();
        with_recovery_test_home(&home, || {
            let save_as_path = projects.join("save-as.vexel");
            let mut save_as_ui = FileUi::new(PathBackend {
                path: path_bytes(&save_as_path).to_vec(),
                save_error: false,
                unsaved: UnsavedChoice::Cancel,
                recovery: RecoveryChoice::Cancel,
            })
            .unwrap();
            save_as_ui
                .document_mut()
                .edit(|document| document.add_rectangle(1.0, 2.0, 9.0, 10.0, "#123456"))
                .unwrap();
            assert_eq!(
                gated_file_action(save_as_ui, |ui| ui.menu(2).unwrap()),
                FlowOutcome::Saved
            );
            assert!(
                ProjectSession::open(path_bytes(&save_as_path))
                    .unwrap()
                    .document()
                    .source()
                    .is_ok()
            );

            let save_path = projects.join("save.vexel");
            let mut saved = AppDocument::new(48, 48).unwrap();
            saved.save_as(path_bytes(&save_path)).unwrap();
            let mut save_ui = FileUi::new(PathBackend {
                path: path_bytes(&save_path).to_vec(),
                save_error: false,
                unsaved: UnsavedChoice::Cancel,
                recovery: RecoveryChoice::Cancel,
            })
            .unwrap();
            save_ui.document_mut().open_path(&save_path).unwrap();
            save_ui
                .document_mut()
                .edit(|document| document.add_rectangle(2.0, 3.0, 12.0, 14.0, "#654321"))
                .unwrap();
            assert_eq!(gated_file_action(save_ui, FileUi::save), FlowOutcome::Saved);
            assert!(
                !ProjectSession::open(path_bytes(&save_path))
                    .unwrap()
                    .is_dirty()
            );

            let close_path = projects.join("close.vexel");
            let mut recoverable = ProjectSession::new(V1Document::new(48, 48).unwrap()).unwrap();
            recoverable.save_as(path_bytes(&close_path)).unwrap();
            recoverable
                .edit(|document| document.add_rectangle(3.0, 4.0, 13.0, 15.0, "#abcdef"))
                .unwrap();
            recoverable.capture_recovery(321).unwrap();
            let mut close_ui = FileUi::new(PathBackend {
                path: path_bytes(&close_path).to_vec(),
                save_error: false,
                unsaved: UnsavedChoice::DontSave,
                recovery: RecoveryChoice::Cancel,
            })
            .unwrap();
            close_ui.document_mut().open_path(&close_path).unwrap();
            close_ui
                .document_mut()
                .edit(|document| document.add_rectangle(4.0, 5.0, 14.0, 16.0, "#fedcba"))
                .unwrap();
            assert_eq!(
                gated_file_action(close_ui, FileUi::close),
                FlowOutcome::CloseWindow
            );
            assert_eq!(
                ProjectSession::open(path_bytes(&close_path))
                    .unwrap()
                    .recovery_status()
                    .unwrap(),
                RecoveryStatus::Matching
            );
        });
    }

    #[test]
    fn gated_open_error_and_save_error_resume_worker_without_publishing_pending_work() {
        let temp = TempDir::new();
        let home = temp.file("home");
        fs::create_dir_all(&home).unwrap();
        with_recovery_test_home(&home, || {
            let missing = temp.file("missing.vexel");
            let open_ui = FileUi::new(PathBackend {
                path: path_bytes(&missing).to_vec(),
                save_error: false,
                unsaved: UnsavedChoice::Cancel,
                recovery: RecoveryChoice::Cancel,
            })
            .unwrap();
            assert!(matches!(
                gated_file_action(open_ui, |ui| ui.menu(0).unwrap()),
                FlowOutcome::Failed(SessionError::Unreadable)
            ));

            let mut save_ui = FileUi::new(PathBackend {
                path: path_bytes(&missing).to_vec(),
                save_error: true,
                unsaved: UnsavedChoice::Cancel,
                recovery: RecoveryChoice::Cancel,
            })
            .unwrap();
            save_ui
                .document_mut()
                .edit(|document| document.add_rectangle(1.0, 1.0, 8.0, 9.0, "#123456"))
                .unwrap();
            assert_eq!(
                gated_file_action(save_ui, FileUi::save),
                FlowOutcome::Failed(SessionError::InvalidPath)
            );
        });
    }

    #[test]
    fn startup_route_waits_and_drops_pending_on_replacement_and_error() {
        let temp = TempDir::new();
        let path = temp.file("startup.vexel");
        AppDocument::new(48, 36)
            .unwrap()
            .save_as(path_bytes(&path))
            .unwrap();
        for succeeds in [true, false] {
            let requested = if succeeds {
                path.clone()
            } else {
                temp.file("missing.vexel")
            };
            assert_eq!(
                gated_file_action(dirty_ui(), move |ui| {
                    let previous = ui.document().source_bytes().unwrap();
                    ui.open_startup(&mut DocumentView::new(), &requested, (48, 36));
                    if succeeds {
                        assert_eq!(ui.document().path(), Some(path_bytes(&requested)));
                        assert!(!ui.document().is_dirty());
                        assert!(ui.recovery_coordinator.published.is_none());
                        FlowOutcome::Opened
                    } else {
                        assert_eq!(ui.document().source_bytes().unwrap(), previous);
                        assert!(ui.document().is_dirty());
                        FlowOutcome::Failed(SessionError::Unreadable)
                    }
                }),
                if succeeds {
                    FlowOutcome::Opened
                } else {
                    FlowOutcome::Failed(SessionError::Unreadable)
                }
            );
        }
    }

    #[test]
    fn mcp_file_routes_wait_drop_pending_and_resume_on_success_or_error() {
        use crate::animation::playback::ManualClock;
        use crate::session::McpGrants;
        let temp = TempDir::new();
        let home = temp.file("home");
        let projects = temp.file("projects");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&projects).unwrap();
        with_recovery_test_home(&home, || {
            let source = projects.join("source.vexel");
            AppDocument::new(48, 36)
                .unwrap()
                .save_as(path_bytes(&source))
                .unwrap();
            for kind in 0..3 {
                for succeeds in [true, false] {
                    let mut ui = FileUi::new(Never).unwrap();
                    ui.document_mut().open_path(&source).unwrap();
                    ui.document_mut()
                        .edit(|document| document.add_rectangle(1.0, 2.0, 3.0, 4.0, "#123456"))
                        .unwrap();
                    let operation = match kind {
                        0 => Operation::Save,
                        1 => Operation::SaveAs {
                            path: projects.join("saved-as.vexel").to_str().unwrap().to_owned(),
                        },
                        _ => Operation::Open {
                            path: source.to_str().unwrap().to_owned(),
                        },
                    };
                    let grants = if succeeds {
                        McpGrants::from_dirs([&projects], Some(&home)).unwrap()
                    } else {
                        McpGrants::none()
                    };
                    let mut host =
                        McpHost::with_policy(grants.policy_with(None, Some(home.clone())));
                    assert_eq!(
                        gated_file_action(ui, move |ui| {
                            let previous = RecoverySessionStamp::capture(ui.document());
                            let result = ui.handle_mcp(
                                &mut host,
                                operation,
                                &mut Driver::new(ManualClock::default()),
                            );
                            if succeeds {
                                assert!(matches!(result, Ok(McpOutcome::Reply(_))));
                                assert!(!ui.document().is_dirty());
                                assert!(ui.recovery_coordinator.published.is_none());
                                FlowOutcome::Saved
                            } else {
                                assert!(result.is_err());
                                assert_eq!(RecoverySessionStamp::capture(ui.document()), previous);
                                FlowOutcome::Cancelled
                            }
                        }),
                        if succeeds {
                            FlowOutcome::Saved
                        } else {
                            FlowOutcome::Cancelled
                        }
                    );
                }
            }
        });
    }
}
