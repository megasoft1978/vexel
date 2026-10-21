use std::cell::{Cell, RefCell};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use vexel_app::file_flow::FlowOutcome;
use vexel_app::file_ui::{FileUi, RecoveryStatusMessage};
use vexel_app::project_session::{ProjectSession, RecoveryStatus};
use vexel_platform_apple::{
    PanelBackend, PanelResult, RecoveryChoice, RecoveryText, UnsavedChoice, UnsavedText,
};
use vexel_v1_document::V1Document;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vexel-auto-recovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct HomeGuard(Option<std::ffi::OsString>);

impl HomeGuard {
    fn set(path: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        // This integration target has one test, so its isolated process has no
        // concurrent HOME readers while the temporary recovery store is used.
        unsafe { std::env::set_var("HOME", path) };
        Self(previous)
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(previous) => unsafe { std::env::set_var("HOME", previous) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}

#[derive(Clone)]
struct PanelScript {
    open_path: Rc<RefCell<Option<Vec<u8>>>>,
    save_path: Rc<RefCell<Option<Vec<u8>>>>,
    open_error: Rc<Cell<bool>>,
    save_error: Rc<Cell<bool>>,
    unsaved_choice: Rc<Cell<UnsavedChoice>>,
    recovery_choice: Rc<Cell<RecoveryChoice>>,
    errors_shown: Rc<Cell<usize>>,
}

impl PanelScript {
    fn new() -> Self {
        Self {
            open_path: Rc::new(RefCell::new(None)),
            save_path: Rc::new(RefCell::new(None)),
            open_error: Rc::new(Cell::new(false)),
            save_error: Rc::new(Cell::new(false)),
            unsaved_choice: Rc::new(Cell::new(UnsavedChoice::Cancel)),
            recovery_choice: Rc::new(Cell::new(RecoveryChoice::Cancel)),
            errors_shown: Rc::new(Cell::new(0)),
        }
    }
}

struct ScriptedPanels(PanelScript);

impl PanelBackend for ScriptedPanels {
    fn open_panel(&self, _: &[&str]) -> PanelResult {
        if self.0.open_error.replace(false) {
            PanelResult::Error
        } else {
            self.0
                .open_path
                .borrow_mut()
                .take()
                .map(PanelResult::Path)
                .unwrap_or(PanelResult::Cancelled)
        }
    }

    fn save_panel(&self, _: &str, _: &str) -> PanelResult {
        if self.0.save_error.replace(false) {
            PanelResult::Error
        } else {
            self.0
                .save_path
                .borrow_mut()
                .take()
                .map(PanelResult::Path)
                .unwrap_or(PanelResult::Cancelled)
        }
    }

    fn confirm_unsaved(&self, _: &UnsavedText<'_>) -> UnsavedChoice {
        self.0.unsaved_choice.get()
    }

    fn confirm_recovery(&self, _: &RecoveryText<'_>) -> RecoveryChoice {
        self.0.recovery_choice.get()
    }

    fn show_error(&self, _: &str, _: &str) {
        self.0.errors_shown.set(self.0.errors_shown.get() + 1);
    }
}

fn save_project(path: &Path, width: u32, height: u32) -> ProjectSession {
    let mut session = ProjectSession::new(V1Document::new(width, height).unwrap()).unwrap();
    session.save_as(path.as_os_str().as_bytes()).unwrap();
    session
}

fn save_project_with_recovery(path: &Path, width: u32, height: u32, color: &str) -> Vec<u8> {
    let mut session = save_project(path, width, height);
    session
        .edit(|document| document.add_rectangle(8.0, 9.0, 30.0, 25.0, color))
        .unwrap();
    let recovered_scene = session.document().source().unwrap();
    session.capture_recovery(42).unwrap();
    recovered_scene
}

fn wait_for_auto_recovery(ui: &mut FileUi<ScriptedPanels>) -> Instant {
    let dirty_since = Instant::now();
    let _ = ui.tick_recovery(dirty_since);
    let eligible_at = dirty_since + Duration::from_secs(3);
    let status = ui.tick_recovery(eligible_at);
    assert_ne!(status, Some(RecoveryStatusMessage::Unavailable));

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = ui.tick_recovery(eligible_at) {
            if status == RecoveryStatusMessage::SavedCurrent {
                return eligible_at;
            }
            assert_ne!(status, RecoveryStatusMessage::Unavailable);
        }
        assert!(
            Instant::now() < deadline,
            "automatic recovery did not publish"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn automatic_recovery_routes_open_save_and_close_without_losing_capsules() {
    let temp = TempDir::new();
    let home = temp.path().join("home");
    let projects = temp.path().join("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let _home = HomeGuard::set(&home);
    let script = PanelScript::new();

    let source_path = projects.join("source.vexel");
    *script.save_path.borrow_mut() = Some(source_path.as_os_str().as_bytes().to_vec());
    let mut ui = FileUi::new(ScriptedPanels(script.clone())).unwrap();
    assert_eq!(ui.menu(2), Some(FlowOutcome::Saved));
    assert_eq!(
        ui.document().path(),
        Some(source_path.as_os_str().as_bytes())
    );
    let original_project = fs::read(&source_path).unwrap();

    ui.document_mut()
        .edit(|document| document.add_rectangle(8.0, 9.0, 30.0, 25.0, "#345678"))
        .unwrap();
    let edited_scene = ui.document().source_bytes().unwrap();
    let published_at = wait_for_auto_recovery(&mut ui);
    let mut recovered = ProjectSession::open(source_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        recovered.recovery_status().unwrap(),
        RecoveryStatus::Matching
    );
    recovered.restore_recovery().unwrap();
    assert!(recovered.is_dirty());
    assert_eq!(recovered.document().source().unwrap(), edited_scene);
    assert_eq!(fs::read(&source_path).unwrap(), original_project);

    ui.document_mut()
        .edit(|document| document.add_rectangle(1.0, 2.0, 3.0, 4.0, "#abcdef"))
        .unwrap();
    assert_eq!(
        ui.tick_recovery(published_at),
        Some(RecoveryStatusMessage::NewerEditsPending)
    );
    assert_eq!(ui.tick_recovery(published_at), None);
    // The cadence has not elapsed; the complete capsule still covers the older scene.

    let edited_scene = ui.document().source_bytes().unwrap();
    assert_eq!(ui.menu(2), Some(FlowOutcome::Cancelled));
    assert_eq!(
        ui.document().path(),
        Some(source_path.as_os_str().as_bytes())
    );
    assert!(ui.document().is_dirty());
    assert_eq!(
        ProjectSession::open(source_path.as_os_str().as_bytes())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Matching
    );
    script.save_error.set(true);
    let errors_before = script.errors_shown.get();
    assert!(matches!(ui.menu(2), Some(FlowOutcome::Failed(_))));
    assert_eq!(script.errors_shown.get(), errors_before + 1);
    assert_eq!(
        ui.document().path(),
        Some(source_path.as_os_str().as_bytes())
    );
    assert_eq!(ui.document().source_bytes().unwrap(), edited_scene);

    assert_eq!(ui.save(), FlowOutcome::Saved);
    assert!(!ui.document().is_dirty());
    let saved = ProjectSession::open(source_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(saved.recovery_status().unwrap(), RecoveryStatus::Absent);
    assert_eq!(saved.document().source().unwrap(), edited_scene);

    let discard_path = projects.join("discard.vexel");
    let discard_scene = save_project_with_recovery(&discard_path, 96, 72, "#aa2233");
    *script.open_path.borrow_mut() = Some(discard_path.as_os_str().as_bytes().to_vec());
    script.recovery_choice.set(RecoveryChoice::Discard);
    assert_eq!(ui.menu(0), Some(FlowOutcome::Opened));
    assert_eq!(
        ui.document().path(),
        Some(discard_path.as_os_str().as_bytes())
    );
    assert!(!ui.document().is_dirty());
    assert_eq!(
        ui.document().source_bytes().unwrap(),
        ProjectSession::open(discard_path.as_os_str().as_bytes())
            .unwrap()
            .document()
            .source()
            .unwrap()
    );
    assert_ne!(ui.document().source_bytes().unwrap(), discard_scene);
    assert_eq!(
        ProjectSession::open(discard_path.as_os_str().as_bytes())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Absent
    );

    ui.document_mut()
        .edit(|document| document.add_rectangle(2.0, 3.0, 14.0, 12.0, "#202020"))
        .unwrap();
    let live_path = ui.document().path().unwrap().to_vec();
    let live_scene = ui.document().source_bytes().unwrap();
    let previous_snapshot_at = wait_for_auto_recovery(&mut ui);

    let cancel_path = projects.join("cancel.vexel");
    let _ = save_project_with_recovery(&cancel_path, 88, 64, "#2277aa");
    *script.open_path.borrow_mut() = Some(cancel_path.as_os_str().as_bytes().to_vec());
    script.recovery_choice.set(RecoveryChoice::Cancel);
    script.unsaved_choice.set(UnsavedChoice::DontSave);
    assert_eq!(ui.menu(0), Some(FlowOutcome::Cancelled));
    assert_eq!(ui.document().path(), Some(live_path.as_slice()));
    assert_eq!(ui.document().source_bytes().unwrap(), live_scene);
    assert!(ui.document().is_dirty());
    assert_eq!(
        ProjectSession::open(live_path.as_slice())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Matching
    );
    assert_eq!(
        ProjectSession::open(cancel_path.as_os_str().as_bytes())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Matching
    );

    let missing_path = projects.join("missing.vexel");
    *script.open_path.borrow_mut() = Some(missing_path.as_os_str().as_bytes().to_vec());
    let errors_before = script.errors_shown.get();
    assert!(matches!(ui.menu(0), Some(FlowOutcome::Failed(_))));
    assert_eq!(script.errors_shown.get(), errors_before + 1);
    assert_eq!(ui.document().path(), Some(live_path.as_slice()));
    assert_eq!(ui.document().source_bytes().unwrap(), live_scene);
    assert_eq!(
        ProjectSession::open(live_path.as_slice())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Matching
    );

    ui.document_mut()
        .edit(|document| document.add_rectangle(5.0, 7.0, 18.0, 16.0, "#303030"))
        .unwrap();
    let after_cancel_and_error_scene = ui.document().source_bytes().unwrap();
    let next_snapshot_at = previous_snapshot_at + Duration::from_secs(16);
    let status = ui.tick_recovery(next_snapshot_at);
    assert_ne!(status, Some(RecoveryStatusMessage::Unavailable));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = ui.tick_recovery(next_snapshot_at) {
            if status == RecoveryStatusMessage::SavedCurrent {
                break;
            }
            assert_ne!(status, RecoveryStatusMessage::Unavailable);
        }
        assert!(
            Instant::now() < deadline,
            "automatic recovery did not resume after cancelled and failed Open"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let mut resumed = ProjectSession::open(live_path.as_slice()).unwrap();
    assert_eq!(resumed.recovery_status().unwrap(), RecoveryStatus::Matching);
    resumed.restore_recovery().unwrap();
    assert_eq!(
        resumed.document().source().unwrap(),
        after_cancel_and_error_scene
    );

    let restore_path = projects.join("restore.vexel");
    let restore_scene = save_project_with_recovery(&restore_path, 80, 56, "#228844");
    *script.open_path.borrow_mut() = Some(restore_path.as_os_str().as_bytes().to_vec());
    script.recovery_choice.set(RecoveryChoice::Restore);
    assert_eq!(ui.menu(0), Some(FlowOutcome::Opened));
    assert_eq!(ui.document().source_bytes().unwrap(), restore_scene);
    assert!(ui.document().is_dirty());
    assert_eq!(
        ProjectSession::open(restore_path.as_os_str().as_bytes())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Matching
    );

    let saved_as_path = projects.join("restored-as.vexel");
    *script.save_path.borrow_mut() = Some(saved_as_path.as_os_str().as_bytes().to_vec());
    assert_eq!(ui.menu(2), Some(FlowOutcome::Saved));
    assert_eq!(
        ui.document().path(),
        Some(saved_as_path.as_os_str().as_bytes())
    );
    assert!(!ui.document().is_dirty());
    assert_eq!(
        ProjectSession::open(restore_path.as_os_str().as_bytes())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Absent
    );
    assert_eq!(
        ProjectSession::open(saved_as_path.as_os_str().as_bytes())
            .unwrap()
            .recovery_status()
            .unwrap(),
        RecoveryStatus::Absent
    );

    ui.document_mut()
        .edit(|document| document.add_rectangle(3.0, 4.0, 10.0, 11.0, "#444444"))
        .unwrap();
    let close_scene = ui.document().source_bytes().unwrap();
    let _ = wait_for_auto_recovery(&mut ui);
    script.unsaved_choice.set(UnsavedChoice::DontSave);
    assert_eq!(ui.close(), FlowOutcome::CloseWindow);
    let preserved = ProjectSession::open(saved_as_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        preserved.recovery_status().unwrap(),
        RecoveryStatus::Matching
    );
    let mut restored_after_close = preserved;
    restored_after_close.restore_recovery().unwrap();
    assert_eq!(
        restored_after_close.document().source().unwrap(),
        close_scene
    );
}
