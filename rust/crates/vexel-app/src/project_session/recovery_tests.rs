use super::{ProjectSession, RecoveryStatus};
use crate::project_session::recovery_store::with_test_home;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use vexel_project::{self as project, Asset, BundleInput};
use vexel_v1_document::V1Document;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vexel-session-recovery-{}-{}",
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

fn encoded_project(scene: &[u8]) -> Vec<u8> {
    let assets = [Asset {
        name: b"opaque.bin",
        media_type: b"application/x-vexel-test",
        data: b"\0\xffraw-asset",
    }];
    project::encode(&BundleInput {
        manifest_json: br#"{ "format":"vexel-project", "future": {"raw": [1, 2]} }"#,
        scene_svg: scene,
        timeline_json: br#"{ "unknown": "timeline bytes" }"#,
        assets: &assets,
    })
    .unwrap()
}

#[test]
fn open_baseline_rejects_source_changes_during_bundle_load() {
    let temp = TempDir::new();
    let path = temp.file("changing.vexel");
    let initial = V1Document::new(32, 32).unwrap().source().unwrap();
    fs::write(&path, encoded_project(&initial)).unwrap();
    let baseline = super::recovery::begin_open_baseline(bytes(&path)).unwrap();
    let mut changed = V1Document::new(32, 32).unwrap();
    changed
        .add_rectangle(1.0, 1.0, 12.0, 14.0, "#ff0000")
        .unwrap();
    fs::write(&path, encoded_project(&changed.source().unwrap())).unwrap();
    assert_eq!(
        super::recovery::finish_open_baseline(bytes(&path), baseline),
        Err(super::SessionError::SourceChanged)
    );
}

#[test]
fn dirty_recovery_round_trips_scene_and_opaque_payload() {
    let temp = TempDir::new();
    let home = temp.file("home");
    let projects = temp.file("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let source_path = projects.join("document.vexel");
    let baseline = V1Document::new(240, 160).unwrap().source().unwrap();
    fs::write(&source_path, encoded_project(&baseline)).unwrap();
    with_test_home(&home, || {
        let mut session = ProjectSession::open(bytes(&source_path)).unwrap();
        session
            .document_mut()
            .add_rectangle(10.0, 20.0, 70.0, 80.0, "#123456")
            .unwrap();
        let recovered_scene = session.document().source().unwrap();
        session.capture_recovery(1234).unwrap();
        assert_eq!(session.recovery_status().unwrap(), RecoveryStatus::Matching);

        let mut restored = ProjectSession::open(bytes(&source_path)).unwrap();
        restored.restore_recovery().unwrap();
        assert!(restored.is_dirty());
        assert_eq!(restored.path(), Some(bytes(&source_path)));
        assert_eq!(restored.document().source().unwrap(), recovered_scene);
        assert_eq!(restored.document().history_count(), 0);

        let save_path = projects.join("restored.vexel");
        restored.save_as(bytes(&save_path)).unwrap();
        let loaded = vexel_project_io::project_file_load(bytes(&save_path)).unwrap();
        let view = loaded.view();
        assert_eq!(view.scene_svg, recovered_scene);
        assert_eq!(
            view.manifest_json,
            br#"{ "format":"vexel-project", "future": {"raw": [1, 2]} }"#
        );
        assert_eq!(view.timeline_json, br#"{ "unknown": "timeline bytes" }"#);
        assert_eq!(
            view.asset_at(0),
            Some(Asset {
                name: b"opaque.bin",
                media_type: b"application/x-vexel-test",
                data: b"\0\xffraw-asset",
            })
        );
    });
}

#[test]
fn conflict_restore_remains_dirty_and_discard_removes_capsule() {
    let temp = TempDir::new();
    let home = temp.file("home");
    let projects = temp.file("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let source_path = projects.join("conflict.vexel");
    let baseline = V1Document::new(32, 32).unwrap().source().unwrap();
    fs::write(&source_path, encoded_project(&baseline)).unwrap();
    with_test_home(&home, || {
        let mut edited = ProjectSession::open(bytes(&source_path)).unwrap();
        edited
            .document_mut()
            .add_rectangle(1.0, 1.0, 4.0, 4.0, "#abcdef")
            .unwrap();
        let mut changed_document = V1Document::new(32, 32).unwrap();
        changed_document
            .add_rectangle(2.0, 2.0, 10.0, 12.0, "#654321")
            .unwrap();
        fs::write(
            &source_path,
            encoded_project(&changed_document.source().unwrap()),
        )
        .unwrap();
        edited.capture_recovery(5678).unwrap();
        assert_eq!(
            edited.recovery_status().unwrap(),
            RecoveryStatus::Conflict,
            "a source change before the first capsule must conflict with the open baseline"
        );
        let source_after_external_change = fs::read(&source_path).unwrap();
        assert_eq!(edited.save(), Err(super::SessionError::RecoveryConflict));
        assert_eq!(
            fs::read(&source_path).unwrap(),
            source_after_external_change,
            "a source changed after open must not be overwritten by Save"
        );

        let mut restored = ProjectSession::open(bytes(&source_path)).unwrap();
        assert_eq!(
            restored.recovery_status().unwrap(),
            RecoveryStatus::Conflict
        );
        restored.restore_recovery().unwrap();
        assert!(restored.is_dirty());
        assert_eq!(restored.save(), Err(super::SessionError::RecoveryConflict));
        restored.discard_recovery().unwrap();
        assert!(restored.is_dirty());
        assert_eq!(restored.save(), Err(super::SessionError::RecoveryConflict));
        assert_eq!(restored.recovery_status().unwrap(), RecoveryStatus::Absent);
    });
}

#[test]
fn successful_save_cleans_capsule_and_failed_discard_keeps_recovery_dirty() {
    let temp = TempDir::new();
    let home = temp.file("home");
    let projects = temp.file("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let source_path = projects.join("cleanup.vexel");
    let baseline = V1Document::new(48, 48).unwrap().source().unwrap();
    fs::write(&source_path, encoded_project(&baseline)).unwrap();

    with_test_home(&home, || {
        let mut session = ProjectSession::open(bytes(&source_path)).unwrap();
        session
            .document_mut()
            .add_rectangle(3.0, 3.0, 12.0, 9.0, "#112233")
            .unwrap();
        session.capture_recovery(9012).unwrap();
        assert_eq!(session.recovery_status().unwrap(), RecoveryStatus::Matching);
        session.save().unwrap();
        assert!(!session.is_dirty());
        assert_eq!(session.recovery_status().unwrap(), RecoveryStatus::Absent);

        session
            .document_mut()
            .add_rectangle(20.0, 20.0, 10.0, 8.0, "#445566")
            .unwrap();
        session.capture_recovery(9013).unwrap();
        let canonical = super::recovery_store::canonical_source_path(bytes(&source_path)).unwrap();
        let capsule = super::recovery_store::capsule_path_for_canonical(&canonical).unwrap();
        let capsule_path = Path::new(std::ffi::OsStr::from_bytes(&capsule));
        fs::remove_file(capsule_path).unwrap();
        fs::create_dir(capsule_path).unwrap();

        assert_eq!(
            session.discard_recovery(),
            Err(super::SessionError::RecoveryInvalid)
        );
        assert!(session.is_dirty());
        assert_eq!(
            session.save(),
            Err(super::SessionError::RecoveryCleanup),
            "the project is published even when post-save cleanup fails"
        );
        assert!(!session.is_dirty());
        assert!(capsule_path.is_dir());
    });
}

#[test]
fn durable_save_with_unavailable_new_baseline_reports_published_state() {
    let temp = TempDir::new();
    let projects = temp.file("projects");
    fs::create_dir(&projects).unwrap();
    let source_path = projects.join("published.vexel");
    let baseline = V1Document::new(48, 48).unwrap().source().unwrap();
    fs::write(&source_path, encoded_project(&baseline)).unwrap();

    let mut session = ProjectSession::open(bytes(&source_path)).unwrap();
    session
        .document_mut()
        .add_rectangle(1.0, 2.0, 13.0, 17.0, "#123abc")
        .unwrap();
    let new_scene = session.document().source().unwrap();
    let old_baseline = session.baseline.clone();
    assert_eq!(
        session.write_with_baseline_refresh(bytes(&source_path), |_| {
            Err(super::SessionError::Unreadable)
        }),
        Err(super::SessionError::PublishedStateUnknown)
    );

    let published = vexel_project_io::project_file_load(bytes(&source_path)).unwrap();
    assert_eq!(published.view().scene_svg, new_scene);
    assert_eq!(session.path(), Some(bytes(&source_path)));
    assert_eq!(session.baseline, old_baseline);
    assert!(session.is_dirty());
}

#[test]
fn corrupt_capsule_is_typed_and_does_not_partially_replace_the_session() {
    let temp = TempDir::new();
    let home = temp.file("home");
    let projects = temp.file("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let source_path = projects.join("corrupt.vexel");
    let baseline = V1Document::new(64, 64).unwrap().source().unwrap();
    fs::write(&source_path, encoded_project(&baseline)).unwrap();

    with_test_home(&home, || {
        let mut session = ProjectSession::open(bytes(&source_path)).unwrap();
        session
            .document_mut()
            .add_rectangle(4.0, 5.0, 16.0, 14.0, "#778899")
            .unwrap();
        session.capture_recovery(3344).unwrap();
        let live_scene = session.document().source().unwrap();
        let live_path = session.path().unwrap().to_vec();
        let canonical = super::recovery_store::canonical_source_path(bytes(&source_path)).unwrap();
        let capsule = super::recovery_store::capsule_path_for_canonical(&canonical).unwrap();
        fs::write(
            Path::new(std::ffi::OsStr::from_bytes(&capsule)),
            b"truncated recovery capsule",
        )
        .unwrap();

        assert_eq!(
            session.recovery_status(),
            Err(super::SessionError::RecoveryInvalid)
        );
        assert_eq!(
            session.restore_recovery(),
            Err(super::SessionError::RecoveryInvalid)
        );
        assert_eq!(session.document().source().unwrap(), live_scene);
        assert_eq!(session.path(), Some(live_path.as_slice()));
        assert!(session.is_dirty());
    });
}

#[test]
fn mismatched_capsule_identity_is_preserved_by_capture_and_cleanup() {
    let temp = TempDir::new();
    let home = temp.file("home");
    let projects = temp.file("projects");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&projects).unwrap();
    let source_a = projects.join("a.vexel");
    let source_b = projects.join("b.vexel");
    let source_a_scene = V1Document::new(32, 32).unwrap().source().unwrap();
    let source_b_scene = V1Document::new(64, 64).unwrap().source().unwrap();
    fs::write(&source_a, encoded_project(&source_a_scene)).unwrap();
    fs::write(&source_b, encoded_project(&source_b_scene)).unwrap();

    with_test_home(&home, || {
        let canonical_a = super::recovery_store::canonical_source_path(bytes(&source_a)).unwrap();
        let canonical_b = super::recovery_store::canonical_source_path(bytes(&source_b)).unwrap();
        let capsule = super::recovery_store::capsule_path_for_canonical(&canonical_a).unwrap();
        let assets_b = [Asset {
            name: b"opaque.bin",
            media_type: b"application/x-vexel-test",
            data: b"\0\xffraw-asset",
        }];
        let project_b = BundleInput {
            manifest_json: br#"{ "format":"vexel-project", "future": {"raw": [1, 2]} }"#,
            scene_svg: &source_b_scene,
            timeline_json: br#"{ "unknown": "timeline bytes" }"#,
            assets: &assets_b,
        };
        let metadata_b = fs::metadata(&source_b).unwrap();
        let planted = vexel_project_io::project_recovery_save(
            &capsule,
            &vexel_project_io::RecoveryInput {
                source_path: &canonical_b,
                source_size: metadata_b.len(),
                source_modified_seconds: metadata_b.mtime(),
                source_modified_nanoseconds: metadata_b.mtime_nsec() as u32,
                captured_time_nanoseconds: 77,
                project: project_b,
            },
        );
        assert_eq!(planted.error, vexel_project_io::RecoveryError::Ok);
        let original_capsule = fs::read(Path::new(std::ffi::OsStr::from_bytes(&capsule))).unwrap();

        let mut session = ProjectSession::open(bytes(&source_a)).unwrap();
        session
            .document_mut()
            .add_rectangle(2.0, 3.0, 12.0, 15.0, "#abcdef")
            .unwrap();
        assert_eq!(
            session.capture_recovery(78),
            Err(super::SessionError::RecoveryInvalid)
        );
        assert_eq!(
            session.discard_recovery(),
            Err(super::SessionError::RecoveryInvalid)
        );
        assert_eq!(
            fs::read(Path::new(std::ffi::OsStr::from_bytes(&capsule))).unwrap(),
            original_capsule
        );
        assert!(session.is_dirty());
    });
}
