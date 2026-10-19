use super::FlowOutcome;
use super::recovery::open_selected;
use crate::project_session::{ProjectSession, RecoveryStatus, with_recovery_test_home};
use std::{
    cell::Cell,
    fs,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};
use vexel_platform_apple::{
    PanelBackend, PanelResult, RecoveryChoice, RecoveryText, UnsavedChoice, UnsavedText,
};
use vexel_v1_document::V1Document;

struct Backend {
    choice: Cell<RecoveryChoice>,
    prompt: Cell<usize>,
}

impl Default for Backend {
    fn default() -> Self {
        Self {
            choice: Cell::new(RecoveryChoice::Cancel),
            prompt: Cell::new(0),
        }
    }
}

impl PanelBackend for Backend {
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
        self.prompt.set(self.prompt.get() + 1);
        self.choice.get()
    }
    fn show_error(&self, _: &str, _: &str) {}
}

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-recovery-flow-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn home(&self) -> PathBuf {
        let path = self.file("home");
        fs::create_dir_all(&path).unwrap();
        path
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bytes(path: &Path) -> &[u8] {
    path.as_os_str().as_bytes()
}

fn saved(path: &Path) -> ProjectSession {
    let document = V1Document::new(120, 80).unwrap();
    let mut session = ProjectSession::new(document);
    session.save_as(bytes(path)).unwrap();
    session
}

fn dirty(session: &mut ProjectSession) {
    session
        .document_mut()
        .add_rectangle_with_id(2.0, 3.0, 20.0, 12.0, "#123456", "recovered")
        .unwrap();
}

fn recovery(path: &Path) -> ProjectSession {
    let mut session = saved(path);
    dirty(&mut session);
    session.capture_recovery(42).unwrap();
    session
}

#[test]
fn matching_cancel_preserves_the_live_session_and_snapshot() {
    let dir = Dir::new();
    with_recovery_test_home(&dir.home(), || {
        let path = dir.file("matching.vexel");
        let source = saved(&path);
        let snapshot = recovery(&path);
        assert_eq!(
            snapshot.recovery_status().unwrap(),
            RecoveryStatus::Matching
        );
        let mut live = ProjectSession::new(V1Document::new(40, 40).unwrap());
        dirty(&mut live);
        let before = live.document().source().unwrap();
        let backend = Backend::default();
        assert_eq!(
            open_selected(&mut live, bytes(&path), &backend),
            FlowOutcome::Cancelled
        );
        assert_eq!(backend.prompt.get(), 1);
        assert_eq!(live.document().source().unwrap(), before);
        assert_eq!(
            snapshot.recovery_status().unwrap(),
            RecoveryStatus::Matching
        );
        assert_eq!(source.path(), Some(bytes(&path)));
    });
}

#[test]
fn restore_adopts_dirty_snapshot_only_after_success() {
    let dir = Dir::new();
    with_recovery_test_home(&dir.home(), || {
        let path = dir.file("restore.vexel");
        let snapshot = recovery(&path);
        let recovered = snapshot.document().source().unwrap();
        let backend = Backend::default();
        backend.choice.set(RecoveryChoice::Restore);
        let mut live = ProjectSession::new(V1Document::new(40, 40).unwrap());
        assert_eq!(
            open_selected(&mut live, bytes(&path), &backend),
            FlowOutcome::Opened
        );
        assert_eq!(live.document().source().unwrap(), recovered);
        assert!(live.is_dirty());
        assert_eq!(live.path(), Some(bytes(&path)));
        assert_eq!(backend.prompt.get(), 1);
    });
}

#[test]
fn discard_adopts_normal_open_and_cancel_is_the_only_prompt_free_stop() {
    let dir = Dir::new();
    with_recovery_test_home(&dir.home(), || {
        let path = dir.file("discard.vexel");
        let original = saved(&path).document().source().unwrap();
        let snapshot = recovery(&path);
        let backend = Backend::default();
        backend.choice.set(RecoveryChoice::Discard);
        let mut live = ProjectSession::new(V1Document::new(40, 40).unwrap());
        assert_eq!(
            open_selected(&mut live, bytes(&path), &backend),
            FlowOutcome::Opened
        );
        assert_eq!(live.document().source().unwrap(), original);
        assert!(!live.is_dirty());
        assert_eq!(snapshot.recovery_status().unwrap(), RecoveryStatus::Absent);
        assert_eq!(backend.prompt.get(), 1);
    });
}

#[test]
fn absent_recovery_opens_normally_without_a_prompt() {
    let dir = Dir::new();
    with_recovery_test_home(&dir.home(), || {
        let path = dir.file("plain.vexel");
        let expected = saved(&path).document().source().unwrap();
        let backend = Backend::default();
        let mut live = ProjectSession::new(V1Document::new(40, 40).unwrap());
        assert_eq!(
            open_selected(&mut live, bytes(&path), &backend),
            FlowOutcome::Opened
        );
        assert_eq!(live.document().source().unwrap(), expected);
        assert_eq!(backend.prompt.get(), 0);
    });
}

#[test]
fn conflict_restore_is_explicit_and_keeps_changed_source_bytes() {
    let dir = Dir::new();
    with_recovery_test_home(&dir.home(), || {
        let path = dir.file("conflict.vexel");
        let snapshot = recovery(&path);
        let mut source = ProjectSession::open(bytes(&path)).unwrap();
        dirty(&mut source);
        let scene = source.document().source().unwrap();
        let bundle = vexel_project::BundleInput {
            manifest_json: br#"{}"#,
            scene_svg: &scene,
            timeline_json: br#"{}"#,
            assets: &[],
        };
        let result = vexel_project_io::project_file_save_as(bytes(&path), &bundle);
        assert_eq!(result.error, vexel_project_io::FileError::Ok);
        let changed_source = fs::read(&path).unwrap();
        assert_eq!(
            snapshot.recovery_status().unwrap(),
            RecoveryStatus::Conflict
        );

        let backend = Backend::default();
        backend.choice.set(RecoveryChoice::Restore);
        let mut live = ProjectSession::new(V1Document::new(40, 40).unwrap());
        assert_eq!(
            open_selected(&mut live, bytes(&path), &backend),
            FlowOutcome::Opened
        );
        assert!(live.is_dirty());
        assert_eq!(fs::read(&path).unwrap(), changed_source);
        assert_eq!(backend.prompt.get(), 1);
    });
}
