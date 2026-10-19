use super::super::{ProjectSession, recovery_store};
use crate::project_session::with_recovery_test_home;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use vexel_project::{Asset, BundleInput};
use vexel_v1_document::V1Document;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vexel-recovery-snapshot-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
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

fn bytes(path: &Path) -> &[u8] {
    path.as_os_str().as_bytes()
}

fn project_bytes(scene: &[u8]) -> Vec<u8> {
    let assets = [Asset {
        name: b"preserved.bin",
        media_type: b"application/octet-stream",
        data: b"opaque payload",
    }];
    vexel_project::encode(&BundleInput {
        manifest_json: br#"{"format":"vexel-project","future":true}"#,
        scene_svg: scene,
        timeline_json: br#"{"timeline":"opaque"}"#,
        assets: &assets,
    })
    .unwrap()
}

#[test]
fn snapshot_shares_immutable_scene_and_payload_across_later_edits() {
    let temp = TempDir::new();
    let home = temp.file("home");
    let projects = temp.file("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let source = projects.join("source.vexel");
    let initial = V1Document::new(80, 64).unwrap().source().unwrap();
    fs::write(&source, project_bytes(&initial)).unwrap();

    with_recovery_test_home(&home, || {
        let mut session = ProjectSession::open(bytes(&source)).unwrap();
        session
            .edit(|document| document.add_rectangle(2.0, 3.0, 22.0, 18.0, "#123456"))
            .unwrap();
        let captured_scene = session.document().source().unwrap();
        let snapshot = session
            .prepare_recovery_snapshot(7, 11, 123_456)
            .unwrap()
            .unwrap();

        session
            .edit(|document| document.add_rectangle(30.0, 31.0, 44.0, 48.0, "#654321"))
            .unwrap();
        let newer_scene = session.document().source().unwrap();
        assert_ne!(captured_scene, newer_scene);

        let result = snapshot.publish();
        assert_eq!(result.epoch, 7);
        assert_eq!(result.revision, 11);
        assert!(result.published);
        assert!(!result.durability_uncertain);
        assert_eq!(result.error, None);

        let canonical = recovery_store::canonical_source_path(bytes(&source)).unwrap();
        let capsule = recovery_store::capsule_path_for_canonical(&canonical).unwrap();
        let capsule_path = Path::new(std::ffi::OsStr::from_bytes(&capsule));
        let capsule_bytes = fs::read(capsule_path).unwrap();
        let recovered =
            vexel_project_io::project_recovery_decode(&capsule_bytes, Some(&canonical)).unwrap();
        assert_eq!(recovered.captured_time_nanoseconds, 123_456);
        assert_eq!(recovered.project.scene_svg, captured_scene);
        assert_eq!(
            recovered.project.manifest_json,
            br#"{"format":"vexel-project","future":true}"#
        );
        assert_eq!(recovered.project.timeline_json, br#"{"timeline":"opaque"}"#);
        assert_eq!(
            recovered.project.asset_at(0).unwrap().data,
            b"opaque payload"
        );
        assert!(session.is_dirty());
        assert_ne!(
            session.document().source().unwrap(),
            recovered.project.scene_svg
        );
    });
}

#[test]
fn snapshot_preparation_skips_clean_and_untitled_sessions() {
    let mut session = ProjectSession::new(V1Document::new(16, 16).unwrap()).unwrap();
    assert!(
        session
            .prepare_recovery_snapshot(1, 1, 1)
            .unwrap()
            .is_none()
    );
    session
        .edit(|document| document.add_rectangle(1.0, 1.0, 4.0, 4.0, "#abcdef"))
        .unwrap();
    assert!(
        session
            .prepare_recovery_snapshot(1, 2, 2)
            .unwrap()
            .is_none()
    );
}

#[test]
fn dirty_saved_native_snapshot_captures_open_baseline_and_identity() {
    let temp = TempDir::new();
    let home = temp.file("home");
    let projects = temp.file("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let source = projects.join("baseline.vexel");
    let initial = V1Document::new(48, 48).unwrap().source().unwrap();
    fs::write(&source, project_bytes(&initial)).unwrap();

    with_recovery_test_home(&home, || {
        let mut session = ProjectSession::open(bytes(&source)).unwrap();
        session
            .edit(|document| document.add_rectangle(3.0, 5.0, 11.0, 12.0, "#abcdef"))
            .unwrap();
        let baseline = session.baseline.as_ref().unwrap().clone();
        let snapshot = session
            .prepare_recovery_snapshot(9, 12, 345)
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.baseline, baseline);
    });
}

#[test]
fn clean_session_skips_snapshot_preparation_without_store_access() {
    let session = ProjectSession::new(V1Document::new(16, 16).unwrap()).unwrap();
    assert!(matches!(
        session.prepare_recovery_snapshot(1, 1, 1),
        Ok(None)
    ));
}
