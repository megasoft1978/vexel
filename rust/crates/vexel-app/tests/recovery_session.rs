use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use vexel_app::project_session::{ProjectSession, RecoveryStatus};
use vexel_v1_document::V1Document;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vexel-recovery-integration-{}-{}",
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

#[test]
fn saved_project_recovers_edits_and_save_as_preserves_original() {
    let temp = TempDir::new();
    let home = temp.path().join("home");
    let projects = temp.path().join("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let _home = HomeGuard::set(&home);

    let source_path = projects.join("source.vexel");
    let source = ProjectSession::new(V1Document::new(120, 90).unwrap());
    let mut source = source;
    source.save_as(source_path.as_os_str().as_bytes()).unwrap();
    let original_bytes = fs::read(&source_path).unwrap();

    let mut edited = ProjectSession::open(source_path.as_os_str().as_bytes()).unwrap();
    edited
        .document_mut()
        .add_rectangle(8.0, 9.0, 30.0, 25.0, "#345678")
        .unwrap();
    let recovered_scene = edited.document().source().unwrap();
    edited.capture_recovery(42).unwrap();

    let mut reopened = ProjectSession::open(source_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        reopened.recovery_status().unwrap(),
        RecoveryStatus::Matching
    );
    reopened.restore_recovery().unwrap();
    assert!(reopened.is_dirty());
    assert_eq!(reopened.document().source().unwrap(), recovered_scene);

    let restored_path = projects.join("restored.vexel");
    reopened
        .save_as(restored_path.as_os_str().as_bytes())
        .unwrap();
    assert_eq!(fs::read(&source_path).unwrap(), original_bytes);
    let saved = ProjectSession::open(restored_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(saved.document().source().unwrap(), recovered_scene);
}
